//! dBASE III/IV: a 32-byte-per-field header over fixed-width records.
//!
//! The format has no delimiters and no encoding marker, which is exactly why an import of one is a
//! byte-level job: field names, widths and types are declared once in the header, then every record
//! is the same length. Nothing here is guessed.
//!
//! Refused by name rather than mis-read: dBASE II (`0x02`) has a different header entirely, and
//! Visual FoxPro (`0x30`) puts a null-flags bitmap in each record that would silently shift every
//! column by one. Memo fields (`M`) arrive empty — the text lives in a sibling `.dbt`, and reading
//! it is a different parser.

use std::io::BufRead;

use nook_core::{Error, Result};

/// One field declaration from the header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    /// `C`, `N`, `F`, `D`, `L` or `M`.
    pub kind: char,
    pub length: usize,
}

/// The parsed header: what the records look like.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub version: u8,
    pub record_count: usize,
    pub record_length: usize,
    pub fields: Vec<Field>,
}

impl Header {
    pub fn columns(&self) -> Vec<String> {
        self.fields.iter().map(|f| f.name.clone()).collect()
    }
}

/// Records from a DBF stream, one at a time.
pub struct Rows<R: BufRead> {
    reader: R,
    header: Header,
    /// Records still to be read, from the header's count.
    left: usize,
    buf: Vec<u8>,
    /// Deleted records are skipped, not imported: the flag is a tombstone, not data.
    deleted: usize,
    /// Record number, for an error a human can act on.
    at: usize,
    done: bool,
}

impl<R: BufRead> Rows<R> {
    pub fn columns(&self) -> Vec<String> {
        self.header.columns()
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    /// How many records carried the deletion flag. Read after the iterator is exhausted.
    pub fn deleted(&self) -> usize {
        self.deleted
    }
}

/// Open a DBF and parse its header, ready to yield records.
pub fn open(path: &std::path::Path) -> Result<Rows<std::io::BufReader<std::fs::File>>> {
    let file = std::fs::File::open(path)
        .map_err(|e| Error::io(format!("open {}: {e}", path.display())))?;
    let mut reader = std::io::BufReader::with_capacity(64 * 1024, file);
    let header = read_header(&mut reader, path)?;
    let left = header.record_count;
    let buf = vec![0; header.record_length];
    Ok(Rows {
        reader,
        header,
        left,
        buf,
        deleted: 0,
        at: 0,
        done: false,
    })
}

fn read_header<R: BufRead>(reader: &mut R, path: &std::path::Path) -> Result<Header> {
    let mut fixed = [0u8; 32];
    reader
        .read_exact(&mut fixed)
        .map_err(|_| Error::config(format!("{} is too short to be a DBF", path.display())))?;
    let version = fixed[0];
    match version {
        0x03 | 0x83 | 0x04 | 0x8B => {}
        0x02 => {
            return Err(Error::config(
                "dBASE II tables are not supported: their header layout is different. Export to \
                 dBASE III first",
            ))
        }
        0x30..=0x32 => {
            return Err(Error::config(
                "this is a Visual FoxPro table, not dBASE III/IV: each record starts with a \
                 null-flags bitmap that would shift every column. Export it to dBASE III first",
            ))
        }
        other => {
            return Err(Error::config(format!(
                "unknown DBF version byte {other:#04x}: only dBASE III (0x03/0x83) and dBASE IV \
                 (0x04/0x8b) are read"
            )))
        }
    }
    let record_count = u32::from_le_bytes([fixed[4], fixed[5], fixed[6], fixed[7]]) as usize;
    let header_length = u16::from_le_bytes([fixed[8], fixed[9]]) as usize;
    let record_length = u16::from_le_bytes([fixed[10], fixed[11]]) as usize;
    if header_length < 33 {
        return Err(Error::config(format!(
            "{} declares a {header_length}-byte header, too short for one field",
            path.display()
        )));
    }
    if record_length == 0 {
        return Err(Error::config(format!(
            "{} declares zero-length records",
            path.display()
        )));
    }
    // The field descriptors run to the 0x0D terminator; reading header_length - 32 of them keeps a
    // truncated file from being read as a shorter, valid one.
    let mut descriptors = vec![0u8; header_length - 32];
    reader.read_exact(&mut descriptors).map_err(|_| {
        Error::config(format!(
            "{} ends inside its header: it claims {header_length} bytes",
            path.display()
        ))
    })?;
    let mut fields: Vec<Field> = Vec::new();
    let mut width_sum = 1; // the deletion flag
    for chunk in descriptors.as_chunks::<32>().0 {
        if chunk[0] == 0x0D {
            break;
        }
        let name = field_name(&chunk[..11]);
        let kind = chunk[11] as char;
        let length = chunk[16] as usize;
        if !matches!(kind, 'C' | 'N' | 'F' | 'D' | 'L' | 'M') {
            return Err(Error::config(format!(
                "field {name} has type {kind:?}, which is not dBASE III/IV (C, N, F, D, L, M)"
            )));
        }
        if length == 0 {
            return Err(Error::config(format!("field {name} declares zero width")));
        }
        width_sum += length;
        fields.push(Field { name, kind, length });
    }
    if fields.is_empty() {
        return Err(Error::config(format!(
            "{} has no field descriptors",
            path.display()
        )));
    }
    if width_sum > record_length {
        return Err(Error::config(format!(
            "{} declares {width_sum} bytes of fields in {record_length}-byte records",
            path.display()
        )));
    }
    Ok(Header {
        version,
        record_count,
        record_length,
        fields,
    })
}

/// Field names are NUL-padded ASCII in an 11-byte slot.
fn field_name(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

impl<R: BufRead> Iterator for Rows<R> {
    type Item = Result<Vec<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.done && self.left > 0 {
            self.left -= 1;
            self.at += 1;
            match self.reader.read_exact(&mut self.buf) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    self.done = true;
                    return Some(Err(Error::config(format!(
                        "record {} is truncated: the header promised {} records",
                        self.at, self.header.record_count
                    ))));
                }
                Err(e) => {
                    self.done = true;
                    return Some(Err(Error::io(format!("read record {}: {e}", self.at))));
                }
            }
            if self.buf[0] == 0x2A {
                self.deleted += 1;
                continue;
            }
            let mut at = 1;
            let mut row = Vec::with_capacity(self.header.fields.len());
            for field in &self.header.fields {
                let cell = &self.buf[at..at + field.length];
                at += field.length;
                row.push(decode(cell, field.kind));
            }
            return Some(Ok(row));
        }
        self.done = true;
        None
    }
}

/// One fixed-width cell as text.
///
/// A DBF carries no encoding marker. UTF-8 is tried first because that is what modern exports
/// write; a failure falls back to Latin-1, which is what dBASE itself wrote.
fn decode(cell: &[u8], kind: char) -> String {
    match kind {
        'C' => text(cell),
        // Numeric and float are right-justified, so the padding is on the left.
        'N' | 'F' => text(cell).trim_start().to_string(),
        'D' => {
            let raw = text(cell);
            if raw.len() == 8 && raw.bytes().all(|b| b.is_ascii_digit()) {
                format!("{}-{}-{}", &raw[0..4], &raw[4..6], &raw[6..8])
            } else {
                // All spaces or all zeroes is how dBASE writes an empty date.
                String::new()
            }
        }
        'L' => match cell.first().copied().unwrap_or(b' ') {
            b'T' | b't' | b'Y' | b'y' => "true".to_string(),
            b'F' | b'f' | b'N' | b'n' => "false".to_string(),
            _ => String::new(),
        },
        // The text is in a sibling .dbt; the cell holds a block number.
        'M' => String::new(),
        _ => text(cell),
    }
}

fn text(cell: &[u8]) -> String {
    let trimmed = match cell.iter().rposition(|b| *b != b' ') {
        Some(end) => &cell[..=end],
        None => return String::new(),
    };
    match std::str::from_utf8(trimmed) {
        Ok(s) => s.to_string(),
        Err(_) => trimmed.iter().map(|b| *b as char).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a dBASE III file. The fixture is written here rather than committed as a binary so
    /// every byte the parser relies on is visible in the test that depends on it.
    fn build(fields: &[(&str, char, usize)], records: &[(&str, Vec<&str>)]) -> Vec<u8> {
        let record_length: usize = 1 + fields.iter().map(|f| f.2).sum::<usize>();
        let header_length = 32 + fields.len() * 32 + 1;
        let mut out = Vec::new();
        out.push(0x03);
        out.extend_from_slice(&[126, 1, 1]);
        out.extend_from_slice(&(records.len() as u32).to_le_bytes());
        out.extend_from_slice(&(header_length as u16).to_le_bytes());
        out.extend_from_slice(&(record_length as u16).to_le_bytes());
        out.extend_from_slice(&[0u8; 20]);
        for (name, kind, length) in fields {
            let mut slot = [0u8; 32];
            slot[..name.len()].copy_from_slice(name.as_bytes());
            slot[11] = *kind as u8;
            slot[16] = *length as u8;
            out.extend_from_slice(&slot);
        }
        out.push(0x0D);
        for (flag, values) in records {
            out.push(if *flag == "deleted" { 0x2A } else { 0x20 });
            for ((_, kind, length), value) in fields.iter().zip(values) {
                let mut cell = vec![b' '; *length];
                let bytes = value.as_bytes();
                let take = bytes.len().min(*length);
                if *length > 0 {
                    // Character, date and logical are left-justified; numbers are right-justified.
                    let at = if matches!(kind, 'N' | 'F') && take < *length {
                        *length - take
                    } else {
                        0
                    };
                    cell[at..at + take].copy_from_slice(&bytes[..take]);
                }
                out.extend_from_slice(&cell);
            }
        }
        out.push(0x1A);
        out
    }

    fn parse(bytes: Vec<u8>) -> (Vec<String>, Vec<Vec<String>>, usize) {
        let mut reader = std::io::BufReader::new(std::io::Cursor::new(bytes));
        let header = read_header(&mut reader, std::path::Path::new("fixture.dbf")).unwrap();
        let left = header.record_count;
        let buf = vec![0; header.record_length];
        let mut rows = Rows {
            reader,
            header,
            left,
            buf,
            deleted: 0,
            at: 0,
            done: false,
        };
        let columns = rows.columns();
        let mut out = Vec::new();
        for row in &mut rows {
            out.push(row.unwrap());
        }
        (columns, out, rows.deleted())
    }

    #[test]
    fn reads_character_numeric_date_and_logical_fields() {
        let bytes = build(
            &[
                ("NAME", 'C', 10),
                ("AMOUNT", 'N', 8),
                ("BORN", 'D', 8),
                ("ACTIVE", 'L', 1),
            ],
            &[
                ("active", vec!["Ada", "42", "18151210", "T"]),
                ("active", vec!["Grace", "-7", "19060809", "F"]),
            ],
        );
        let (columns, rows, deleted) = parse(bytes);
        assert_eq!(columns, vec!["NAME", "AMOUNT", "BORN", "ACTIVE"]);
        assert_eq!(
            rows,
            vec![
                vec!["Ada", "42", "1815-12-10", "true"],
                vec!["Grace", "-7", "1906-08-09", "false"],
            ]
        );
        assert_eq!(deleted, 0);
    }

    #[test]
    fn deleted_records_are_skipped_and_counted() {
        let bytes = build(
            &[("NAME", 'C', 5)],
            &[
                ("active", vec!["keep"]),
                ("deleted", vec!["gone"]),
                ("active", vec!["also"]),
            ],
        );
        let (_, rows, deleted) = parse(bytes);
        assert_eq!(rows, vec![vec!["keep"], vec!["also"]]);
        assert_eq!(deleted, 1);
    }

    #[test]
    fn an_empty_date_is_empty_not_zero() {
        let bytes = build(&[("BORN", 'D', 8)], &[("active", vec!["        "])]);
        let (_, rows, _) = parse(bytes);
        assert_eq!(rows, vec![vec![""]]);
    }

    #[test]
    fn a_visual_foxpro_table_is_refused_by_name() {
        let mut bytes = build(&[("NAME", 'C', 5)], &[]);
        bytes[0] = 0x30;
        let mut reader = std::io::BufReader::new(std::io::Cursor::new(bytes));
        let err = read_header(&mut reader, std::path::Path::new("fox.dbf")).unwrap_err();
        assert!(err.to_string().contains("Visual FoxPro"), "{err}");
    }

    #[test]
    fn a_field_wider_than_the_record_is_refused() {
        let mut bytes = build(&[("NAME", 'C', 5)], &[]);
        // Record length at offset 10: claim less than the field needs.
        bytes[10] = 3;
        bytes[11] = 0;
        let mut reader = std::io::BufReader::new(std::io::Cursor::new(bytes));
        let err = read_header(&mut reader, std::path::Path::new("bad.dbf")).unwrap_err();
        assert!(err.to_string().contains("bytes of fields"), "{err}");
    }

    #[test]
    fn latin1_bytes_survive_the_round_trip() {
        let mut bytes = build(&[("NAME", 'C', 4)], &[]);
        // Append one record holding 0xE9 ("é" in Latin-1, invalid on its own as UTF-8).
        bytes.pop();
        bytes.push(0x20);
        bytes.extend_from_slice(&[0xE9, b' ', b' ', b' ']);
        bytes.push(0x1A);
        bytes[4..8].copy_from_slice(&1u32.to_le_bytes());
        let (_, rows, _) = parse(bytes);
        assert_eq!(rows, vec![vec!["é"]]);
    }
}
