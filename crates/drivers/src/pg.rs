//! PostgreSQL driver.
//!
//! Uses the simple query protocol on purpose: it hands every value back as text, so `numeric`,
//! `int8` and `timestamptz` keep full precision without a per-type decoder, and multi-statement
//! scripts work the way the editor submits them.
//! ponytail: text protocol only, no binary results and no bind parameters — add a prepared path
//! when a feature needs parameter binding or binary blobs at scale.

use std::time::{Duration, Instant};

use nook_core::{ConnectionConfig, Dialect, DriverKind, Error, Result, Value};
use nook_model::{
    Catalog, Column, DbProcess, DbUser, ForeignKey, Index, ResultSet, Routine, Schema, Sequence,
    ServerInfo, Table, TableKind, Trigger,
};
use postgres::types::Type;
use postgres::{Client, NoTls, SimpleQueryMessage};
use postgres_native_tls::MakeTlsConnector;

use crate::{guard_read_only, Driver, MAX_ROWS};

pub struct Postgres {
    client: Client,
    cfg: ConnectionConfig,
    database: String,
    in_tx: bool,
    /// `pg_backend_pid()`, read once at open. Stable for the connection's life, and the id another
    /// session needs to cancel this one's work.
    pid: Option<u32>,
}

impl Postgres {
    pub fn open(cfg: &ConnectionConfig) -> Result<Self> {
        let mut client = connect(cfg)?;
        // One round-trip at open, so a cancel never has to interrupt the connection it wants to
        // cancel. A failure here is not fatal: the session works, cancellation says it cannot.
        let pid = client
            .query_opt("SELECT pg_backend_pid()", &[])
            .ok()
            .flatten()
            .map(|row| row.get::<_, i32>(0) as u32);
        Ok(Postgres {
            client,
            database: cfg.database.clone(),
            in_tx: false,
            cfg: cfg.clone(),
            pid,
        })
    }

    fn simple(&mut self, sql: &str) -> Result<Vec<SimpleQueryMessage>> {
        self.client
            .simple_query(sql)
            .map_err(|e| Error::query(describe(&e, None)))
    }

    /// Column names and server types for a statement. Prepared but never executed: this is how the
    /// grid gets real `numeric(10,2)` / `timestamptz` labels while values still arrive as text.
    fn declared_columns(&mut self, sql: &str) -> Vec<(String, Type)> {
        match self.client.prepare(sql) {
            Ok(stmt) => stmt
                .columns()
                .iter()
                .map(|c| (c.name().to_string(), c.type_().clone()))
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    fn string_rows(&mut self, sql: &str) -> Result<Vec<Vec<String>>> {
        let messages = self.simple(sql)?;
        let mut out: Vec<Vec<String>> = Vec::new();
        let mut ncols = 0usize;
        for m in messages {
            match m {
                SimpleQueryMessage::RowDescription(desc) => ncols = desc.len(),
                SimpleQueryMessage::Row(r) => {
                    if out.len() >= MAX_ROWS {
                        break;
                    }
                    let mut row = Vec::with_capacity(ncols);
                    for i in 0..ncols {
                        row.push(r.get(i).unwrap_or("").to_string());
                    }
                    out.push(row);
                }
                _ => {}
            }
        }
        Ok(out)
    }

    fn one(&mut self, sql: &str) -> Option<String> {
        self.string_rows(sql)
            .ok()
            .and_then(|rows| rows.into_iter().next())
            .and_then(|r| r.into_iter().next())
    }
}

fn connect(cfg: &ConnectionConfig) -> Result<Client> {
    let (host, port) = cfg.target();
    let mut pc = postgres::Config::new();
    pc.host(&host)
        .port(port)
        .connect_timeout(Duration::from_secs(cfg.connect_timeout_s.clamp(1, 300)));
    if !cfg.user.is_empty() {
        pc.user(&cfg.user);
    }
    if let Some(pw) = &cfg.password {
        pc.password(pw);
    }
    if !cfg.database.is_empty() {
        pc.dbname(&cfg.database);
    }
    pc.ssl_mode(match cfg.ssl {
        nook_core::SslMode::Disable => postgres::config::SslMode::Disable,
        nook_core::SslMode::Prefer => postgres::config::SslMode::Prefer,
        _ => postgres::config::SslMode::Require,
    });
    let target = if cfg.ssh.enabled {
        format!("{host}:{port} via ssh tunnel")
    } else {
        format!("{host}:{port}")
    };
    if cfg.ssl == nook_core::SslMode::Disable {
        return pc
            .connect(NoTls)
            .map_err(|e| Error::connect(format!("{target}: {}", describe(&e, Some(cfg)))));
    }
    let verify = matches!(
        cfg.ssl,
        nook_core::SslMode::VerifyFull | nook_core::SslMode::VerifyCa
    );
    let builder = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(!verify)
        .danger_accept_invalid_hostnames(!verify)
        .build()
        .map_err(|e| Error::config(format!("tls: {e}")))?;
    pc.connect(MakeTlsConnector::new(builder))
        .map_err(|e| Error::connect(format!("{target}: {}", describe(&e, Some(cfg)))))
}

/// The sentence to show for a failed connect. tokio-postgres's `Display` prints the error's *kind*
/// alone, so the server's own answer to a rejected login arrives as the bare "db error", and its
/// "password missing" — raised when the server asks and the config carries none — arrives as the bare
/// "invalid configuration". Both leave the user with nothing to act on.
fn describe(error: &postgres::Error, cfg: Option<&ConnectionConfig>) -> String {
    if let Some(db) = error.as_db_error() {
        return db.to_string();
    }
    let text = error.to_string();
    // On a connect, host, port and user are always set, which leaves "password missing" as the one
    // `Kind::Config` failure there is — and `Kind::Config`'s own message never reaches `Display`. If
    // the wording ever changes, the raw text is what the user reads: no worse than before.
    if text == "invalid configuration" && cfg.is_some_and(|cfg| cfg.password.is_none()) {
        return "no password: this profile carries none and the server asked for one — type it in \
                Password and save, so the keychain keeps it"
            .to_string();
    }
    text
}

/// Raw column bytes, decoded by `decode_raw` instead of by `postgres-types`.
///
/// Needed because the prepared path gets binary results and the crate's `FromSql` impls only cover a
/// handful of types (no `numeric`, no `json` without a feature flag).
#[derive(Debug)]
struct Raw(Vec<u8>);

impl Raw {
    /// Parameters are sent as text, which every type accepts through its input function.
    fn from_value(v: &Value) -> Raw {
        Raw(match v {
            Value::Null => return Raw(Vec::new()),
            other => other.as_text().into_bytes(),
        })
    }
}

impl postgres::types::ToSql for Raw {
    fn to_sql(
        &self,
        _ty: &Type,
        out: &mut postgres::types::private::BytesMut,
    ) -> std::result::Result<postgres::types::IsNull, Box<dyn std::error::Error + Sync + Send>>
    {
        // An empty payload is how a NULL is spelled here; every non-NULL value is non-empty text.
        if self.0.is_empty() {
            return Ok(postgres::types::IsNull::Yes);
        }
        out.extend_from_slice(&self.0);
        Ok(postgres::types::IsNull::No)
    }

    fn accepts(_ty: &Type) -> bool {
        true
    }

    /// The payload is text, so the bind must declare text — the default is binary, which would make
    /// the server read `"42"` as four bytes of an int4.
    fn encode_format(&self, _ty: &Type) -> postgres::types::Format {
        postgres::types::Format::Text
    }

    fn to_sql_checked(
        &self,
        ty: &Type,
        out: &mut postgres::types::private::BytesMut,
    ) -> std::result::Result<postgres::types::IsNull, Box<dyn std::error::Error + Sync + Send>>
    {
        self.to_sql(ty, out)
    }
}

impl<'a> postgres::types::FromSql<'a> for Raw {
    fn from_sql(
        _ty: &Type,
        raw: &'a [u8],
    ) -> std::result::Result<Raw, Box<dyn std::error::Error + Sync + Send>> {
        Ok(Raw(raw.to_vec()))
    }

    fn accepts(_ty: &Type) -> bool {
        true
    }
}

/// Turn one binary column payload into a cell, by the type the server declared.
fn decode_raw(bytes: &[u8], ty: &Type, column: &str) -> Result<Value> {
    let be = |n: usize| -> Option<i64> {
        if bytes.len() < n {
            return None;
        }
        let mut buf = [0u8; 8];
        buf[8 - n..].copy_from_slice(&bytes[..n]);
        Some(i64::from_be_bytes(buf))
    };
    match *ty {
        Type::BOOL => Ok(Value::Bool(bytes.first().copied().unwrap_or(0) != 0)),
        Type::INT2 => be(2).map(Value::Int).ok_or_else(|| short(column, ty)),
        Type::INT4 => be(4).map(Value::Int).ok_or_else(|| short(column, ty)),
        Type::INT8 => be(8).map(Value::Int).ok_or_else(|| short(column, ty)),
        Type::OID => be(4).map(|v| Value::UInt(v as u64)).ok_or_else(|| short(column, ty)),
        Type::FLOAT4 => be(4)
            .map(|v| Value::Float(f32::from_bits(v as u32) as f64))
            .ok_or_else(|| short(column, ty)),
        Type::FLOAT8 => be(8)
            .map(|v| Value::Float(f64::from_bits(v as u64)))
            .ok_or_else(|| short(column, ty)),
        Type::BYTEA => Ok(Value::Bytes(bytes.to_vec())),
        Type::JSON | Type::JSONB => Ok(Value::Json(
            // jsonb carries a one-byte version prefix.
            String::from_utf8_lossy(if *ty == Type::JSONB { &bytes[1..] } else { bytes })
                .into_owned(),
        )),
        Type::DATE => {
            // Days since 2000-01-01.
            let days = be(4).ok_or_else(|| short(column, ty))?;
            Ok(Value::Date(civil_from_days(days).to_string()))
        }
        Type::TIMESTAMP | Type::TIMESTAMPTZ => {
            // Microseconds since 2000-01-01 00:00:00 UTC.
            let us = be(8).ok_or_else(|| short(column, ty))?;
            Ok(Value::Timestamp(format_timestamp(us)))
        }
        Type::TIME => {
            let us = be(8).ok_or_else(|| short(column, ty))?;
            let s = us / 1_000_000;
            Ok(Value::Time(format!(
                "{:02}:{:02}:{:02}",
                s / 3600,
                (s / 60) % 60,
                s % 60
            )))
        }
        Type::NUMERIC => Ok(Value::text(decode_numeric(bytes).ok_or_else(|| {
            Error::query(format!(
                "column {column}: numeric payload of {} bytes is not readable",
                bytes.len()
            ))
        })?)),
        ref t if is_text_type(t) => Ok(Value::text(String::from_utf8_lossy(bytes).into_owned())),
        ref t => Err(Error::unsupported(format!(
            "column {column}: a prepared statement cannot render type {} yet — run the statement              without parameters, or cast the column to text",
            t.name()
        ))),
    }
}

fn short(column: &str, ty: &Type) -> Error {
    Error::query(format!("column {column}: expected a {} payload", ty.name()))
}

fn is_text_type(ty: &Type) -> bool {
    matches!(
        *ty,
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME | Type::UNKNOWN | Type::CHAR
    ) || matches!(ty.name(), "citext" | "uuid" | "xml" | "ltree")
}

/// Postgres `numeric` on the wire: ndigits, weight, sign, dscale, then base-10000 digits.
fn decode_numeric(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 8 {
        return None;
    }
    let ndigits = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
    let weight = i16::from_be_bytes([bytes[2], bytes[3]]);
    let sign = u16::from_be_bytes([bytes[4], bytes[5]]);
    let dscale = u16::from_be_bytes([bytes[6], bytes[7]]) as usize;
    if sign == 0xC000 {
        return Some("NaN".into());
    }
    let negative = sign == 0x4000;
    let mut digits: Vec<u16> = Vec::with_capacity(ndigits);
    for i in 0..ndigits {
        let at = 8 + i * 2;
        if at + 1 >= bytes.len() {
            return None;
        }
        digits.push(u16::from_be_bytes([bytes[at], bytes[at + 1]]));
    }
    // Integer part: digits 0..=weight; fraction: weight+1.. with `dscale` decimal places.
    let mut whole = String::new();
    if weight < 0 {
        whole.push('0');
    } else {
        for i in 0..=weight as usize {
            let d = digits.get(i).copied().unwrap_or(0);
            if i == 0 {
                whole.push_str(&d.to_string());
            } else {
                whole.push_str(&format!("{d:04}"));
            }
        }
    }
    let mut frac = String::new();
    let mut i = (weight + 1).max(0) as usize;
    while frac.len() < dscale {
        let d = digits.get(i).copied().unwrap_or(0);
        frac.push_str(&format!("{d:04}"));
        i += 1;
    }
    frac.truncate(dscale);
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str(&whole);
    if dscale > 0 {
        out.push('.');
        out.push_str(&frac);
    }
    Some(out)
}

/// Days since 1970-01-01 to a `YYYY-MM-DD` date.
fn civil_from_days(days: i64) -> String {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Microseconds since 2000-01-01 UTC to `YYYY-MM-DD HH:MM:SS[.ffffff]`.
fn format_timestamp(us_since_2000: i64) -> String {
    const EPOCH_OFFSET_US: i64 = 946_684_800 * 1_000_000;
    let total = us_since_2000 + EPOCH_OFFSET_US;
    let secs = total.div_euclid(1_000_000);
    let micros = total.rem_euclid(1_000_000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let date = civil_from_days(days);
    let mut out = format!(
        "{date} {:02}:{:02}:{:02}",
        rem / 3600,
        (rem / 60) % 60,
        rem % 60
    );
    if micros > 0 {
        out.push_str(&format!(".{micros:06}"));
    }
    out
}

fn parse_value(raw: Option<&str>, ty: &Type) -> Value {
    let Some(text) = raw else { return Value::Null };
    match *ty {
        Type::BOOL => Value::Bool(text == "t" || text == "true"),
        Type::INT2 | Type::INT4 | Type::INT8 | Type::OID => text
            .parse::<i64>()
            .map(Value::Int)
            .unwrap_or_else(|_| Value::text(text)),
        Type::FLOAT4 | Type::FLOAT8 => text
            .parse::<f64>()
            .map(Value::Float)
            .unwrap_or_else(|_| Value::text(text)),
        Type::JSON | Type::JSONB => Value::Json(text.to_string()),
        Type::DATE => Value::Date(text.to_string()),
        Type::TIME | Type::TIMETZ => Value::Time(text.to_string()),
        Type::TIMESTAMP | Type::TIMESTAMPTZ => Value::Timestamp(text.to_string()),
        Type::BYTEA => {
            let hex = text.strip_prefix("\\x").unwrap_or(text);
            match decode_hex(hex) {
                Some(b) => Value::Bytes(b),
                None => Value::text(text),
            }
        }
        _ => Value::text(text),
    }
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i + 1 < bytes.len() {
        let hi = (bytes[i] as char).to_digit(16)?;
        let lo = (bytes[i + 1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
        i += 2;
    }
    Some(out)
}

/// Postgres prints booleans as `true`/`false` through `::text` and `t`/`f` in the wire format.
fn pg_bool(cell: &str) -> bool {
    matches!(
        cell.trim().to_ascii_lowercase().as_str(),
        "t" | "true" | "yes" | "1"
    )
}

fn split_list(cell: &str) -> Vec<String> {
    cell.split('|')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Fallback type labels when the statement could not be prepared (DDL, multi-statement scripts).
fn infer_types(rows: &[Vec<Value>], columns: usize) -> Vec<String> {
    (0..columns)
        .map(|i| {
            rows.iter()
                .filter_map(|r| r.get(i))
                .find(|v| !v.is_null())
                .map(|v| v.type_name().to_ascii_uppercase())
                .unwrap_or_else(|| "TEXT".into())
        })
        .collect()
}

const EXCLUDE_SCHEMAS: &str =
    "'information_schema','pg_catalog','pg_toast','pg_temp_1','pg_toast_temp_1'";

impl Driver for Postgres {
    fn kind(&self) -> DriverKind {
        DriverKind::Postgres
    }

    fn dialect(&self) -> Dialect {
        Dialect::Postgres
    }

    fn current_database(&self) -> Option<String> {
        Some(self.database.clone())
    }

    fn server_info(&mut self) -> Result<ServerInfo> {
        let version = self
            .one("SELECT version()")
            .ok_or_else(|| Error::connect("could not read server version"))?;
        let short = version.split(" on ").next().unwrap_or(&version).to_string();
        let size = self
            .one("SELECT pg_database_size(current_database())::text")
            .and_then(|s| s.parse::<i64>().ok());
        let uptime = self
            .one("SELECT EXTRACT(EPOCH FROM (now() - pg_postmaster_start_time()))::bigint::text")
            .and_then(|s| s.parse::<i64>().ok());
        Ok(ServerInfo {
            version: short,
            driver: "PostgreSQL".into(),
            database: Some(self.database.clone()),
            host: format!("{}:{}", self.cfg.target().0, self.cfg.target().1),
            uptime_s: uptime,
            size_bytes: size,
            read_only: self.cfg.read_only,
        })
    }

    fn run(&mut self, sql: &str, limit: usize) -> Result<ResultSet> {
        if sql.trim().is_empty() {
            return Err(Error::query("empty statement"));
        }
        guard_read_only(self.cfg.read_only, sql)?;
        let started = Instant::now();
        let limit = limit.max(1);
        // Types come from a prepared (but unexecuted) statement; values come from the simple
        // protocol as text, which keeps numeric and timestamps exact.
        let declared = self.declared_columns(sql);
        let messages = self.simple(sql)?;
        let mut rs = ResultSet::default();
        let mut names: Vec<String> = declared.iter().map(|(n, _)| n.clone()).collect();
        let mut truncated = false;
        for m in messages {
            match m {
                SimpleQueryMessage::RowDescription(desc) if names.is_empty() => {
                    names = desc.iter().map(|c| c.name().to_string()).collect();
                }
                SimpleQueryMessage::Row(row) => {
                    if rs.rows.len() >= limit {
                        truncated = true;
                        continue;
                    }
                    let mut vals = Vec::with_capacity(names.len().max(row.len()));
                    for i in 0..row.len() {
                        let ty = declared.get(i).map(|(_, t)| t.clone());
                        vals.push(match ty.as_ref() {
                            Some(t) => parse_value(row.get(i), t),
                            None => match row.get(i) {
                                Some(v) => Value::text(v),
                                None => Value::Null,
                            },
                        });
                    }
                    rs.rows.push(vals);
                }
                SimpleQueryMessage::CommandComplete(n) => rs.affected += n,
                _ => {}
            }
        }
        rs.columns = names;
        rs.column_types = if declared.is_empty() {
            infer_types(&rs.rows, rs.columns.len())
        } else {
            declared.iter().map(|(_, t)| t.name().to_string()).collect()
        };
        rs.truncated = truncated;
        rs.elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        Ok(rs)
    }

    /// A real prepared statement: the server binds the values, and the statement it prepares carries
    /// `$n` placeholders rather than the data.
    ///
    /// Results arrive in the binary format, so each column is decoded by its declared type. A type
    /// this driver cannot decode is refused by name rather than rendered as hex or an empty cell.
    fn run_prepared(&mut self, sql: &str, values: &[Value], limit: usize) -> Result<ResultSet> {
        if sql.trim().is_empty() {
            return Err(Error::query("empty statement"));
        }
        guard_read_only(self.cfg.read_only, sql)?;
        let started = Instant::now();
        let limit = limit.max(1);
        let bound: Vec<Raw> = values.iter().map(Raw::from_value).collect();
        let params: Vec<&(dyn postgres::types::ToSql + Sync)> = bound
            .iter()
            .map(|v| v as &(dyn postgres::types::ToSql + Sync))
            .collect();
        let rows = self
            .client
            .query(sql, &params)
            .map_err(|e| Error::query(describe(&e, None)))?;
        let mut rs = ResultSet::default();
        for row in &rows {
            if rs.columns.is_empty() {
                rs.columns = row.columns().iter().map(|c| c.name().to_string()).collect();
                rs.column_types = row
                    .columns()
                    .iter()
                    .map(|c| c.type_().name().to_string())
                    .collect();
            }
            if rs.rows.len() >= limit {
                rs.truncated = true;
                break;
            }
            let mut vals = Vec::with_capacity(rs.columns.len());
            for (i, column) in row.columns().iter().enumerate() {
                let raw: Option<Raw> = row
                    .try_get(i)
                    .map_err(|e| Error::query(describe(&e, None)))?;
                vals.push(match raw {
                    None => Value::Null,
                    Some(raw) => decode_raw(&raw.0, column.type_(), column.name())?,
                });
            }
            rs.rows.push(vals);
        }
        rs.elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        Ok(rs)
    }

    fn explain_plan(&mut self, sql: &str, analyze: bool) -> Result<nook_model::plan::Plan> {
        // JSON where the server can produce it: parsing Postgres's text plan is brittle in exactly
        // the way a normaliser exists to avoid.
        let head = if analyze {
            "EXPLAIN (ANALYZE, FORMAT JSON, BUFFERS)"
        } else {
            "EXPLAIN (FORMAT JSON)"
        };
        let rs = self.run(&format!("{head} {sql}"), 100)?;
        let text = rs
            .rows
            .first()
            .and_then(|r| r.first())
            .map(|v| v.display())
            .ok_or_else(|| Error::query("the server returned no plan"))?;
        let json: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| Error::query(format!("the plan is not JSON: {e}")))?;
        nook_model::plan::from_postgres(&json)
            .ok_or_else(|| Error::query("the plan has no \"Plan\" node"))
    }

    fn backend_pid(&self) -> Option<u32> {
        // `pg_backend_pid()` is the session's own id, and it is stable for the life of the
        // connection — no round-trip needed here, and none is possible: `&self`.
        self.pid
    }

    fn cancel(&mut self, pid: u64) -> Result<()> {
        // The sibling connection runs this, never the one being cancelled.
        let sql = format!("SELECT pg_cancel_backend({pid})");
        let messages = self.simple(&sql)?;
        let ok = messages
            .iter()
            .find_map(|m| match m {
                SimpleQueryMessage::Row(r) => r.get(0),
                _ => None,
            })
            .map(|v| v == "t" || v == "true")
            .unwrap_or(false);
        if ok {
            Ok(())
        } else {
            Err(Error::query(format!(
                "pg_cancel_backend({pid}) reported that the backend was not cancelled"
            )))
        }
    }

    fn catalog(&mut self) -> Result<Catalog> {
        let started = Instant::now();
        let schemas: Vec<String> = self
            .string_rows(&format!(
                "SELECT nspname FROM pg_namespace WHERE nspname NOT IN ({EXCLUDE_SCHEMAS}) \
                 AND nspname NOT LIKE 'pg_temp%' ORDER BY nspname"
            ))?
            .into_iter()
            .filter_map(|r| r.into_iter().next())
            .collect();

        let table_rows = self.string_rows(&format!(
            "SELECT n.nspname, c.relname, c.relkind::text, COALESCE(obj_description(c.oid, 'pg_class'), ''), \
             GREATEST(c.reltuples::bigint, 0)::text, pg_total_relation_size(c.oid)::text \
             FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE c.relkind IN ('r','p','v','m','f') AND n.nspname NOT IN ({EXCLUDE_SCHEMAS}) \
             AND n.nspname NOT LIKE 'pg_temp%' ORDER BY n.nspname, c.relname"
        ))?;

        let column_rows = self.string_rows(&format!(
            "SELECT c.table_schema, c.table_name, c.column_name, c.ordinal_position::text, \
             COALESCE(c.column_default, ''), c.is_nullable, \
             CASE WHEN c.data_type = 'USER-DEFINED' THEN c.udt_name ELSE c.data_type END, \
             COALESCE(c.character_maximum_length::text, ''), \
             COALESCE(c.numeric_precision::text, ''), COALESCE(c.numeric_scale::text, '') \
             FROM information_schema.columns c \
             WHERE c.table_schema NOT IN ({EXCLUDE_SCHEMAS}) \
             ORDER BY c.table_schema, c.table_name, c.ordinal_position"
        ))?;

        let pk_rows = self.string_rows(&format!(
            "SELECT tc.table_schema, tc.table_name, kcu.column_name \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
               ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema \
             WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_schema NOT IN ({EXCLUDE_SCHEMAS})"
        ))?;

        let index_rows = self.string_rows(&format!(
            "SELECT n.nspname, t.relname, i.relname, ix.indisunique::text, ix.indisprimary::text, \
             COALESCE(array_to_string(ARRAY(SELECT pg_get_indexdef(ix.indexrelid, k + 1, true) \
               FROM generate_subscripts(ix.indkey, 1) AS k ORDER BY k), '|'), '') \
             FROM pg_index ix JOIN pg_class i ON i.oid = ix.indexrelid \
             JOIN pg_class t ON t.oid = ix.indrelid \
             JOIN pg_namespace n ON n.oid = t.relnamespace \
             WHERE n.nspname NOT IN ({EXCLUDE_SCHEMAS}) ORDER BY 1,2,3"
        ))?;

        let fk_rows = self.string_rows(&format!(
            "SELECT tc.constraint_name, tc.table_schema, tc.table_name, kcu.column_name, \
             ccu.table_schema, ccu.table_name, ccu.column_name, rc.delete_rule, rc.update_rule \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
               ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema \
             JOIN information_schema.constraint_column_usage ccu \
               ON ccu.constraint_name = tc.constraint_name AND ccu.table_schema = tc.table_schema \
             JOIN information_schema.referential_constraints rc \
               ON rc.constraint_name = tc.constraint_name AND rc.constraint_schema = tc.table_schema \
             WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_schema NOT IN ({EXCLUDE_SCHEMAS})"
        ))?;

        let mut out = Catalog {
            driver: self
                .server_info()
                .map(|i| i.version)
                .unwrap_or_else(|_| "PostgreSQL".into()),
            database: Some(self.database.clone()),
            databases: self.databases().unwrap_or_default(),
            schemas: schemas
                .iter()
                .map(|name| Schema {
                    name: name.clone(),
                    tables: vec![],
                    ..Default::default()
                })
                .collect(),
            fetched_ms: 0,
        };

        for row in &table_rows {
            if row.len() < 6 {
                continue;
            }
            let schema_name = row[0].clone();
            let Some(schema) = out.schemas.iter_mut().find(|s| s.name == schema_name) else {
                continue;
            };
            let kind = match row[2].as_str() {
                "v" => TableKind::View,
                "m" => TableKind::MatView,
                "f" => TableKind::Table,
                _ => TableKind::Table,
            };
            schema.tables.push(Table {
                schema: Some(schema_name.clone()),
                name: row[1].clone(),
                kind,
                comment: row[3].clone(),
                row_estimate: row[4].parse::<i64>().ok(),
                size_bytes: row[5].parse::<i64>().ok(),
                ..Table::default()
            });
        }

        for row in &column_rows {
            if row.len() < 10 {
                continue;
            }
            let Some(table) = out
                .schema(&row[0])
                .and_then(|s| s.tables.iter().find(|t| t.name == row[1]))
                .cloned()
            else {
                continue;
            };
            let col = Column {
                name: row[2].clone(),
                type_name: row[6].clone(),
                nullable: row[5].eq_ignore_ascii_case("YES"),
                default: if row[4].is_empty() {
                    None
                } else {
                    Some(row[4].clone())
                },
                ordinal: row[3].parse().unwrap_or(0),
                max_len: row[7].parse().ok(),
                numeric_precision: row[8].parse().ok(),
                numeric_scale: row[9].parse().ok(),
                ..Column::default()
            };
            if let Some(schema) = out.schemas.iter_mut().find(|s| s.name == row[0]) {
                if let Some(t) = schema.tables.iter_mut().find(|t| t.name == table.name) {
                    t.columns.push(col);
                }
            }
        }

        for row in &pk_rows {
            if row.len() < 3 {
                continue;
            }
            if let Some(schema) = out.schemas.iter_mut().find(|s| s.name == row[0]) {
                if let Some(t) = schema.tables.iter_mut().find(|t| t.name == row[1]) {
                    if let Some(c) = t.columns.iter_mut().find(|c| c.name == row[2]) {
                        c.primary_key = true;
                        c.nullable = false;
                    }
                }
            }
        }

        for row in &index_rows {
            if row.len() < 6 {
                continue;
            }
            let unique = pg_bool(&row[3]);
            let primary = pg_bool(&row[4]);
            if let Some(schema) = out.schemas.iter_mut().find(|s| s.name == row[0]) {
                if let Some(t) = schema.tables.iter_mut().find(|t| t.name == row[1]) {
                    t.indexes.push(Index {
                        name: row[2].clone(),
                        columns: split_list(&row[5]),
                        unique,
                        primary,
                        kind: if primary {
                            "primary key".into()
                        } else if unique {
                            "unique".into()
                        } else {
                            "index".into()
                        },
                    });
                }
            }
        }

        for row in &fk_rows {
            if row.len() < 9 {
                continue;
            }
            if let Some(schema) = out.schemas.iter_mut().find(|s| s.name == row[1]) {
                if let Some(t) = schema.tables.iter_mut().find(|t| t.name == row[2]) {
                    match t.foreign_keys.iter_mut().find(|f| f.name == row[0]) {
                        Some(fk) => {
                            fk.columns.push(row[3].clone());
                            fk.ref_columns.push(row[6].clone());
                        }
                        None => t.foreign_keys.push(ForeignKey {
                            name: row[0].clone(),
                            columns: vec![row[3].clone()],
                            ref_schema: Some(row[4].clone()),
                            ref_table: row[5].clone(),
                            ref_columns: vec![row[6].clone()],
                            on_delete: row[7].clone(),
                            on_update: row[8].clone(),
                        }),
                    }
                }
            }
        }

        let trigger_rows = self.string_rows(&format!(
            "SELECT n.nspname, t.tgname, c.relname, pg_get_triggerdef(t.oid) \
             FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid \
             JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE NOT t.tgisinternal AND n.nspname NOT IN ({EXCLUDE_SCHEMAS}) \
             AND n.nspname NOT LIKE 'pg_temp%' ORDER BY n.nspname, t.tgname"
        ))?;

        let routine_rows = self.string_rows(&format!(
            "SELECT n.nspname, p.proname, p.prokind::text, \
             COALESCE(pg_get_function_result(p.oid), ''), \
             COALESCE(obj_description(p.oid, 'pg_proc'), ''), \
             COALESCE(pg_get_functiondef(p.oid), '') \
             FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace \
             WHERE p.prokind IN ('f', 'p') AND n.nspname NOT IN ({EXCLUDE_SCHEMAS}) \
             AND n.nspname NOT LIKE 'pg_temp%' ORDER BY n.nspname, p.proname"
        ))?;

        let sequence_rows = self.string_rows(&format!(
            "SELECT schemaname, sequencename, start_value::text, increment_by::text, \
             min_value::text, max_value::text, cycle::text, last_value::text \
             FROM pg_sequences WHERE schemaname NOT IN ({EXCLUDE_SCHEMAS}) \
             AND schemaname NOT LIKE 'pg_temp%' ORDER BY schemaname, sequencename"
        ))?;

        for row in &trigger_rows {
            if row.len() < 4 {
                continue;
            }
            if let Some(schema) = out.schemas.iter_mut().find(|s| s.name == row[0]) {
                schema.triggers.push(Trigger {
                    schema: Some(row[0].clone()),
                    name: row[1].clone(),
                    table: row[2].clone(),
                    // The full definition stays in `ddl`; timing/events stay empty
                    // rather than hand-parsing the server's own text.
                    ddl: row[3].clone(),
                    ..Default::default()
                });
            }
        }

        for row in &routine_rows {
            if row.len() < 6 {
                continue;
            }
            let kind = match row[2].as_str() {
                "p" => "procedure",
                _ => "function",
            };
            if let Some(schema) = out.schemas.iter_mut().find(|s| s.name == row[0]) {
                schema.routines.push(Routine {
                    schema: Some(row[0].clone()),
                    name: row[1].clone(),
                    kind: kind.into(),
                    returns: row[3].clone(),
                    comment: row[4].clone(),
                    ddl: row[5].clone(),
                });
            }
        }

        for row in &sequence_rows {
            if row.len() < 8 {
                continue;
            }
            if let Some(schema) = out.schemas.iter_mut().find(|s| s.name == row[0]) {
                schema.sequences.push(Sequence {
                    schema: Some(row[0].clone()),
                    name: row[1].clone(),
                    start: row[2].parse().unwrap_or(1),
                    increment: row[3].parse().unwrap_or(1),
                    min: row[4].parse().ok(),
                    max: row[5].parse().ok(),
                    cycle: row[6].eq_ignore_ascii_case("true") || row[6] == "t",
                    last_value: row[7].parse().ok(),
                });
            }
        }
        out.fetched_ms = started.elapsed().as_millis() as u64;
        Ok(out)
    }

    fn databases(&mut self) -> Result<Vec<String>> {
        Ok(self
            .string_rows(
                "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname",
            )?
            .into_iter()
            .filter_map(|r| r.into_iter().next())
            .collect())
    }

    fn use_database(&mut self, database: &str) -> Result<()> {
        if self.in_tx {
            return Err(Error::query(
                "finish or roll back the open transaction before switching database",
            ));
        }
        let mut cfg = self.cfg.clone();
        cfg.database = database.to_string();
        let fresh = connect(&cfg)?;
        // Replacing the client closes the old connection.
        self.client = fresh;
        self.cfg = cfg;
        self.database = database.to_string();
        Ok(())
    }

    fn processes(&mut self) -> Result<Vec<DbProcess>> {
        let rows = self.string_rows(
            "SELECT pid::text, COALESCE(usename, ''), COALESCE(datname, ''), COALESCE(client_addr::text, 'local'), \
             COALESCE(state, ''), COALESCE(query, ''), \
             COALESCE(EXTRACT(MILLISECONDS FROM (now() - query_start))::bigint::text, ''), \
             COALESCE(query_start::text, '') \
             FROM pg_stat_activity WHERE pid <> pg_backend_pid() ORDER BY query_start NULLS LAST LIMIT 1000",
        )?;
        Ok(rows
            .iter()
            .filter(|r| r.len() >= 8)
            .map(|r| DbProcess {
                id: r[0].clone(),
                user: r[1].clone(),
                database: r[2].clone(),
                host: r[3].clone(),
                state: r[4].clone(),
                query: r[5].clone(),
                elapsed_ms: r[6].parse().ok(),
                started: if r[7].is_empty() {
                    None
                } else {
                    Some(r[7].clone())
                },
            })
            .collect())
    }

    fn kill_process(&mut self, id: &str) -> Result<()> {
        let pid: i32 = id
            .trim()
            .parse()
            .map_err(|_| Error::config(format!("'{id}' is not a process id")))?;
        let rs = self.run(&format!("SELECT pg_terminate_backend({pid})"), 1)?;
        let ok = rs
            .rows
            .first()
            .and_then(|r| r.first())
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if ok {
            Ok(())
        } else {
            Err(Error::query(format!("session {pid} was already gone")))
        }
    }

    fn variables(&mut self) -> Result<Vec<(String, String)>> {
        Ok(self
            .string_rows("SELECT name, setting FROM pg_settings ORDER BY name LIMIT 2000")?
            .into_iter()
            .filter(|r| r.len() >= 2)
            .map(|r| (r[0].clone(), r[1].clone()))
            .collect())
    }

    fn status(&mut self) -> Result<Vec<(String, String)>> {
        let mut out = Vec::new();
        for (label, sql) in [
            ("version", "SELECT version()"),
            ("started", "SELECT pg_postmaster_start_time()::text"),
            (
                "database size",
                "SELECT pg_size_pretty(pg_database_size(current_database()))",
            ),
            ("sessions", "SELECT count(*)::text FROM pg_stat_activity"),
            (
                "max connections",
                "SELECT current_setting('max_connections')",
            ),
            (
                "commits",
                "SELECT sum(xact_commit)::text FROM pg_stat_database",
            ),
            (
                "rollbacks",
                "SELECT sum(xact_rollback)::text FROM pg_stat_database",
            ),
            (
                "cache hit ratio",
                "SELECT CASE WHEN sum(blks_hit) + sum(blks_read) = 0 THEN 'n/a' \
                 ELSE round(100.0 * sum(blks_hit) / (sum(blks_hit) + sum(blks_read)), 2)::text END \
                 FROM pg_stat_database",
            ),
        ] {
            if let Some(v) = self.one(sql) {
                out.push((label.to_string(), v));
            }
        }
        Ok(out)
    }

    fn users(&mut self) -> Result<Vec<DbUser>> {
        Ok(self
            .string_rows(
                "SELECT rolname, rolcanlogin::text, rolsuper::text, COALESCE(rolvaliduntil::text, '') \
                 FROM pg_roles ORDER BY rolname",
            )?
            .into_iter()
            .filter(|r| r.len() >= 4)
            .map(|r| DbUser {
                name: r[0].clone(),
                host: String::new(),
                can_login: pg_bool(&r[1]),
                is_super: pg_bool(&r[2]),
                expires: if r[3].is_empty() { None } else { Some(r[3].clone()) },
            })
            .collect())
    }

    fn grants(&mut self, user: &DbUser) -> Result<Vec<String>> {
        let who = user.name.replace('\'', "''");
        let mut out: Vec<String> = self
            .string_rows(&format!(
                "SELECT privilege_type || ' on ' || table_name \
                 FROM information_schema.role_table_grants WHERE grantee = '{who}' LIMIT 500"
            ))?
            .into_iter()
            .filter_map(|r| r.into_iter().next())
            .collect();
        out.extend(
            self.string_rows(&format!(
                "SELECT 'member of ' || r.rolname FROM pg_auth_members m \
                 JOIN pg_roles r ON r.oid = m.roleid JOIN pg_roles u ON u.oid = m.member \
                 WHERE u.rolname = '{who}'"
            ))?
            .into_iter()
            .filter_map(|r| r.into_iter().next()),
        );
        if user.is_super {
            out.insert(0, "SUPERUSER".into());
        }
        Ok(out)
    }

    fn table_ddl(&mut self, schema: Option<&str>, table: &str) -> Result<String> {
        // Generated from the catalog: Postgres has no SHOW CREATE TABLE.
        let catalog = self.catalog()?;
        let t = catalog
            .find_table(schema, table)
            .ok_or_else(|| Error::not_found(format!("no such table: {table}")))?
            .clone();
        let mut out = nook_model::ddl::create_table(&t, Dialect::Postgres);
        out.push('\n');
        for idx in nook_model::ddl::create_indexes(&t, Dialect::Postgres) {
            out.push_str(&idx);
            out.push('\n');
        }
        for fk in nook_model::ddl::create_foreign_keys(&t, Dialect::Postgres) {
            out.push_str(&fk);
            out.push('\n');
        }
        Ok(out.trim_end().to_string())
    }

    fn begin(&mut self) -> Result<()> {
        self.simple("BEGIN")?;
        self.in_tx = true;
        Ok(())
    }

    fn commit(&mut self) -> Result<()> {
        self.simple("COMMIT")?;
        self.in_tx = false;
        Ok(())
    }

    fn rollback(&mut self) -> Result<()> {
        self.simple("ROLLBACK")?;
        self.in_tx = false;
        Ok(())
    }

    fn in_transaction(&self) -> bool {
        self.in_tx
    }

    fn close(&mut self) {
        // Dropping the client closes the connection; nothing to do explicitly.
    }
}

/// Row estimate for the tree, cheap enough to run for every table.
pub fn fetch_estimates(db: &mut Postgres) -> Result<Vec<(String, i64)>> {
    Ok(db
        .string_rows(&format!(
            "SELECT n.nspname || '.' || c.relname, GREATEST(c.reltuples::bigint, 0)::text \
             FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE c.relkind = 'r' AND n.nspname NOT IN ({EXCLUDE_SCHEMAS})"
        ))?
        .into_iter()
        .filter(|r| r.len() >= 2)
        .filter_map(|r| r[1].parse::<i64>().ok().map(|n| (r[0].clone(), n)))
        .collect())
}
