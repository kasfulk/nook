//! SQLite driver over rusqlite (system libsqlite3).

use std::time::{Duration, Instant};

use nook_core::{quote_ident, ConnectionConfig, Dialect, DriverKind, Error, Result, Value};
use nook_model::{
    Catalog, Column, ForeignKey, Index, ResultSet, Schema, ServerInfo, Table, TableKind,
};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};

use crate::{Driver, MAX_ROWS};

pub struct Sqlite {
    conn: Connection,
    path: String,
    read_only: bool,
    in_tx: bool,
}

impl Sqlite {
    pub fn open(cfg: &ConnectionConfig) -> Result<Self> {
        let path = expand_path(&cfg.file);
        if path.trim().is_empty() {
            return Err(Error::config("SQLite connection needs a database file"));
        }
        let mut flags = if cfg.read_only {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
        };
        flags |= OpenFlags::SQLITE_OPEN_URI;
        let conn = Connection::open_with_flags(&path, flags)
            .map_err(|e| Error::connect(format!("open {path}: {e}")))?;
        conn.busy_timeout(Duration::from_secs(cfg.connect_timeout_s.clamp(1, 300)))
            .map_err(|e| Error::connect(e.to_string()))?;
        let sqlite = Sqlite {
            conn,
            path,
            read_only: cfg.read_only,
            in_tx: false,
        };
        sqlite.enforce_foreign_keys_like_system_sqlite()?;
        Ok(sqlite)
    }

    /// In-memory session, used by the tests and by `--sqlite :memory:`.
    pub fn memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| Error::connect(e.to_string()))?;
        let sqlite = Sqlite {
            conn,
            path: ":memory:".into(),
            read_only: false,
            in_tx: false,
        };
        sqlite.enforce_foreign_keys_like_system_sqlite()?;
        Ok(sqlite)
    }

    /// Bundled SQLite is compiled with `SQLITE_DEFAULT_FOREIGN_KEYS=1`, so a fresh connection
    /// enforces foreign keys; every system SQLite starts with them off. The product has always been
    /// the latter — a session turns enforcement on when it wants it — so the default is restored
    /// here rather than letting the linked-in build silently change what a query means.
    fn enforce_foreign_keys_like_system_sqlite(&self) -> Result<()> {
        self.conn
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .map_err(|e| Error::connect(format!("PRAGMA foreign_keys: {e}")))
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    fn guard_write(&self, sql: &str) -> Result<()> {
        if self.read_only && !is_read_statement(sql) {
            return Err(Error::query(format!(
                "connection is read-only; refused: {}",
                sql.lines().next().unwrap_or("").trim()
            )));
        }
        Ok(())
    }

    fn pragma_rows(&mut self, pragma: &str) -> Result<Vec<Vec<Value>>> {
        Ok(self.run(pragma, MAX_ROWS)?.rows)
    }

    fn text_at(rows: &[Vec<Value>], row: usize, col: usize) -> String {
        rows.get(row)
            .and_then(|r| r.get(col))
            .map(Value::as_text)
            .unwrap_or_default()
    }
}

fn expand_path(file: &str) -> String {
    if let Some(rest) = file.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return format!("{}/{}", home.to_string_lossy(), rest);
        }
    }
    file.to_string()
}

fn map_err(e: rusqlite::Error, sql: &str) -> Error {
    let head = sql.lines().next().unwrap_or("").trim();
    let head = if head.len() > 80 { &head[..80] } else { head };
    Error::query(format!("{e} [{head}]"))
}

fn is_read_statement(sql: &str) -> bool {
    nook_core::returns_rows(sql) && !sql.to_ascii_lowercase().contains(" into ")
}

/// A cell as rusqlite wants it, so a bound value keeps its type instead of arriving as text.
fn to_sql_value(v: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as S;
    match v {
        Value::Null => S::Null,
        Value::Int(i) => S::Integer(*i),
        Value::UInt(u) => S::Integer(*u as i64),
        Value::Float(f) => S::Real(*f),
        Value::Bool(b) => S::Integer(i64::from(*b)),
        Value::Bytes(b) => S::Blob(b.clone()),
        Value::Text(s) | Value::Json(s) | Value::Date(s) | Value::Time(s) | Value::Timestamp(s) => {
            S::Text(s.clone())
        }
    }
}

fn value_from(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::Int(i),
        ValueRef::Real(f) => Value::Float(f),
        ValueRef::Text(t) => Value::Text(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Value::Bytes(b.to_vec()),
    }
}

fn quote_pragma_arg(name: &str) -> String {
    format!("'{}'", name.replace('\'', "''"))
}

impl Driver for Sqlite {
    fn kind(&self) -> DriverKind {
        DriverKind::Sqlite
    }

    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }

    fn current_database(&self) -> Option<String> {
        Some(self.path.clone())
    }

    fn server_info(&mut self) -> Result<ServerInfo> {
        let version = self
            .run("SELECT sqlite_version()", 1)?
            .rows
            .first()
            .and_then(|r| r.first())
            .map(Value::as_text)
            .unwrap_or_else(|| "unknown".into());
        let size = std::fs::metadata(&self.path).map(|m| m.len() as i64).ok();
        Ok(ServerInfo {
            version: format!("SQLite {version}"),
            driver: "SQLite".into(),
            database: Some(self.path.clone()),
            host: "file".into(),
            uptime_s: None,
            size_bytes: size,
            read_only: self.read_only,
        })
    }

    fn run(&mut self, sql: &str, limit: usize) -> Result<ResultSet> {
        if sql.trim().is_empty() {
            return Err(Error::query("empty statement"));
        }
        self.guard_write(sql)?;
        let started = Instant::now();
        let limit = limit.max(1);
        let mut stmt = self.conn.prepare(sql).map_err(|e| map_err(e, sql))?;
        if stmt.column_count() == 0 {
            let affected = stmt.execute([]).map_err(|e| map_err(e, sql))? as u64;
            let last = self.conn.last_insert_rowid();
            return Ok(ResultSet {
                affected,
                last_insert_id: if last > 0 { Some(last as u64) } else { None },
                elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
                ..ResultSet::default()
            });
        }
        let declared: Vec<(String, Option<String>)> = stmt
            .columns()
            .map(|c| (c.name().to_string(), c.decl_type().map(|s| s.to_string())))
            .collect();
        let columns: Vec<String> = declared.iter().map(|(n, _)| n.clone()).collect();
        let mut rows_out: Vec<Vec<Value>> = Vec::new();
        let mut truncated = false;
        let mut types: Vec<String> = declared
            .iter()
            .map(|(_, d)| d.clone().unwrap_or_default())
            .collect();
        {
            let mut rows = stmt.query([]).map_err(|e| map_err(e, sql))?;
            while let Some(row) = rows.next().map_err(|e| map_err(e, sql))? {
                if rows_out.len() >= limit {
                    truncated = true;
                    break;
                }
                let mut vals = Vec::with_capacity(columns.len());
                for (i, declared_type) in types.iter_mut().enumerate() {
                    let v = value_from(row.get_ref(i).map_err(|e| map_err(e, sql))?);
                    if declared_type.is_empty() && rows_out.is_empty() {
                        *declared_type = v.type_name().to_ascii_uppercase();
                    }
                    vals.push(v);
                }
                rows_out.push(vals);
            }
        }
        Ok(ResultSet {
            columns,
            column_types: types,
            rows: rows_out,
            affected: 0,
            last_insert_id: None,
            elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
            notices: vec![],
            truncated,
        })
    }

    fn run_stream(
        &mut self,
        sql: &str,
        limit: usize,
        page_rows: usize,
        sink: &mut dyn FnMut(ResultSet) -> bool,
    ) -> Result<ResultSet> {
        if sql.trim().is_empty() {
            return Err(Error::query("empty statement"));
        }
        self.guard_write(sql)?;
        let started = Instant::now();
        let limit = limit.max(1);
        let page_rows = page_rows.clamp(1, limit);
        let mut stmt = self.conn.prepare(sql).map_err(|e| map_err(e, sql))?;
        if stmt.column_count() == 0 {
            let affected = stmt.execute([]).map_err(|e| map_err(e, sql))? as u64;
            let last = self.conn.last_insert_rowid();
            return Ok(ResultSet {
                affected,
                last_insert_id: if last > 0 { Some(last as u64) } else { None },
                elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
                ..ResultSet::default()
            });
        }
        let declared: Vec<(String, Option<String>)> = stmt
            .columns()
            .map(|c| (c.name().to_string(), c.decl_type().map(|s| s.to_string())))
            .collect();
        let columns: Vec<String> = declared.iter().map(|(n, _)| n.clone()).collect();
        let mut types: Vec<String> = declared
            .iter()
            .map(|(_, d)| d.clone().unwrap_or_default())
            .collect();
        let mut total = 0usize;
        let mut truncated = false;
        let mut page: Vec<Vec<Value>> = Vec::with_capacity(page_rows.min(1024));
        {
            let mut rows = stmt.query([]).map_err(|e| map_err(e, sql))?;
            while let Some(row) = rows.next().map_err(|e| map_err(e, sql))? {
                if total >= limit {
                    truncated = true;
                    break;
                }
                let mut vals = Vec::with_capacity(columns.len());
                for (i, declared_type) in types.iter_mut().enumerate() {
                    let v = value_from(row.get_ref(i).map_err(|e| map_err(e, sql))?);
                    if declared_type.is_empty() && total == 0 {
                        *declared_type = v.type_name().to_ascii_uppercase();
                    }
                    vals.push(v);
                }
                page.push(vals);
                total += 1;
                if page.len() >= page_rows {
                    // The page boundary is the only place SQLite can stop: it has no out-of-band
                    // cancel, and the reader is inside one `sqlite3_step`.
                    let batch = ResultSet {
                        columns: columns.clone(),
                        column_types: types.clone(),
                        rows: std::mem::take(&mut page),
                        ..ResultSet::default()
                    };
                    if !sink(batch) {
                        break;
                    }
                }
            }
        }
        if !page.is_empty() {
            let batch = ResultSet {
                columns: columns.clone(),
                column_types: types.clone(),
                rows: page,
                ..ResultSet::default()
            };
            sink(batch);
        }
        Ok(ResultSet {
            columns,
            column_types: types,
            rows: Vec::new(),
            affected: 0,
            last_insert_id: None,
            elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
            notices: vec![],
            truncated,
        })
    }

    fn run_prepared(&mut self, sql: &str, values: &[Value], limit: usize) -> Result<ResultSet> {
        if sql.trim().is_empty() {
            return Err(Error::query("empty statement"));
        }
        self.guard_write(sql)?;
        let started = Instant::now();
        let limit = limit.max(1);
        let bound: Vec<rusqlite::types::Value> = values.iter().map(to_sql_value).collect();
        let mut stmt = self.conn.prepare(sql).map_err(|e| map_err(e, sql))?;
        if stmt.column_count() == 0 {
            let affected = stmt
                .execute(rusqlite::params_from_iter(bound.iter()))
                .map_err(|e| map_err(e, sql))? as u64;
            let last = self.conn.last_insert_rowid();
            return Ok(ResultSet {
                affected,
                last_insert_id: if last > 0 { Some(last as u64) } else { None },
                elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
                ..ResultSet::default()
            });
        }
        let declared: Vec<(String, Option<String>)> = stmt
            .columns()
            .map(|c| (c.name().to_string(), c.decl_type().map(|s| s.to_string())))
            .collect();
        let columns: Vec<String> = declared.iter().map(|(n, _)| n.clone()).collect();
        let mut types: Vec<String> = declared
            .iter()
            .map(|(_, d)| d.clone().unwrap_or_default())
            .collect();
        let mut rows_out: Vec<Vec<Value>> = Vec::new();
        let mut truncated = false;
        {
            let mut rows = stmt
                .query(rusqlite::params_from_iter(bound.iter()))
                .map_err(|e| map_err(e, sql))?;
            while let Some(row) = rows.next().map_err(|e| map_err(e, sql))? {
                if rows_out.len() >= limit {
                    truncated = true;
                    break;
                }
                let mut vals = Vec::with_capacity(columns.len());
                for (i, declared_type) in types.iter_mut().enumerate() {
                    let v = value_from(row.get_ref(i).map_err(|e| map_err(e, sql))?);
                    if declared_type.is_empty() && rows_out.is_empty() {
                        *declared_type = v.type_name().to_ascii_uppercase();
                    }
                    vals.push(v);
                }
                rows_out.push(vals);
            }
        }
        Ok(ResultSet {
            columns,
            column_types: types,
            rows: rows_out,
            truncated,
            elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
            ..ResultSet::default()
        })
    }

    fn catalog(&mut self) -> Result<Catalog> {
        let started = Instant::now();
        let objects = self.run(
            "SELECT name, type, sql FROM sqlite_master \
             WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' ORDER BY name",
            MAX_ROWS,
        )?;
        let mut tables = Vec::with_capacity(objects.rows.len());
        for row in &objects.rows {
            let name = row.first().map(Value::as_text).unwrap_or_default();
            let kind = match row.get(1).map(Value::as_text).unwrap_or_default().as_str() {
                "view" => TableKind::View,
                _ => TableKind::Table,
            };
            let create_sql = row.get(2).map(Value::as_text).unwrap_or_default();
            let mut table = Table::new(None, name.clone());
            table.kind = kind;
            if kind == TableKind::Table {
                table.columns = self.columns_of(&name)?;
                table.indexes = self.indexes_of(&name)?;
                table.foreign_keys = self.foreign_keys_of(&name)?;
                if create_sql.to_ascii_uppercase().contains("AUTOINCREMENT") {
                    for c in &mut table.columns {
                        if c.primary_key && c.type_name.eq_ignore_ascii_case("integer") {
                            c.auto_increment = true;
                        }
                    }
                }
            }
            tables.push(table);
        }
        let trigger_rows = self.run(
            "SELECT name, tbl_name, sql FROM sqlite_master WHERE type = 'trigger' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            MAX_ROWS,
        )?;
        let triggers: Vec<nook_model::Trigger> = trigger_rows
            .rows
            .iter()
            .map(|row| nook_model::Trigger {
                name: row.first().map(Value::as_text).unwrap_or_default(),
                table: row.get(1).map(Value::as_text).unwrap_or_default(),
                // `ddl` carries the whole statement; timing and events stay empty rather
                // than a hand parser — consumers show the DDL for SQLite anyway.
                enabled: true,
                ddl: row.get(2).map(Value::as_text).unwrap_or_default(),
                ..Default::default()
            })
            .collect();
        Ok(Catalog {
            driver: self
                .server_info()
                .map(|i| i.version)
                .unwrap_or_else(|_| "SQLite".into()),
            database: Some(self.path.clone()),
            databases: vec![self.path.clone()],
            schemas: vec![Schema {
                name: "main".into(),
                tables,
                triggers,
                ..Default::default()
            }],
            fetched_ms: started.elapsed().as_millis() as u64,
        })
    }

    fn table_ddl(&mut self, _schema: Option<&str>, table: &str) -> Result<String> {
        let sql = format!(
            "SELECT sql FROM sqlite_master WHERE name = {}",
            quote_pragma_arg(table)
        );
        let rs = self.run(&sql, 1)?;
        match rs.rows.first().and_then(|r| r.first()) {
            Some(v) => {
                let text = v.as_text();
                if text.trim().is_empty() {
                    Err(Error::not_found(format!("{table} is a built-in object")))
                } else {
                    Ok(format!("{};", text.trim_end_matches(';')))
                }
            }
            None => Err(Error::not_found(format!("no such table: {table}"))),
        }
    }

    fn rebuild_table(
        &mut self,
        schema: Option<&str>,
        table: &Table,
        new_columns: &[Column],
    ) -> Result<Vec<String>> {
        self.rebuild_table_plan(schema, table, new_columns)
    }

    fn variables(&mut self) -> Result<Vec<(String, String)>> {
        let rows = self.pragma_rows("PRAGMA compile_options")?;
        Ok(rows
            .iter()
            .filter_map(|r| {
                r.first()
                    .map(|v| ("compile_option".to_string(), v.as_text()))
            })
            .take(200)
            .collect())
    }

    fn status(&mut self) -> Result<Vec<(String, String)>> {
        let mut out = Vec::new();
        for pragma in [
            "PRAGMA page_count",
            "PRAGMA page_size",
            "PRAGMA freelist_count",
            "PRAGMA journal_mode",
            "PRAGMA cache_size",
            "PRAGMA encoding",
            "PRAGMA foreign_keys",
            "PRAGMA user_version",
        ] {
            if let Ok(rows) = self.pragma_rows(pragma) {
                if let Some(v) = rows.first().and_then(|r| r.first()) {
                    out.push((
                        pragma.trim_start_matches("PRAGMA ").to_string(),
                        v.as_text(),
                    ));
                }
            }
        }
        Ok(out)
    }

    fn explain(&mut self, sql: &str) -> Result<ResultSet> {
        self.run(&format!("EXPLAIN QUERY PLAN {sql}"), 1000)
    }

    fn explain_plan(&mut self, sql: &str, _analyze: bool) -> Result<nook_model::plan::Plan> {
        // `EXPLAIN QUERY PLAN` returns `(id, parent, notused, detail)`: a tree already, with the
        // server's own ids. SQLite has no ANALYZE form of this, so `analyze` is not offered rather
        // than silently ignored by the caller.
        let rs = self.run(&format!("EXPLAIN QUERY PLAN {sql}"), 1000)?;
        let rows: Vec<(i64, i64, String)> = rs
            .rows
            .iter()
            .map(|r| {
                (
                    r.first().and_then(|v| v.as_i64()).unwrap_or(0),
                    r.get(1).and_then(|v| v.as_i64()).unwrap_or(0),
                    r.get(3).map(|v| v.display()).unwrap_or_default(),
                )
            })
            .collect();
        Ok(nook_model::plan::from_sqlite(&rows))
    }

    fn begin(&mut self) -> Result<()> {
        self.run("BEGIN IMMEDIATE", 0)?;
        self.in_tx = true;
        Ok(())
    }

    fn commit(&mut self) -> Result<()> {
        self.run("COMMIT", 0)?;
        self.in_tx = false;
        Ok(())
    }

    fn rollback(&mut self) -> Result<()> {
        self.run("ROLLBACK", 0)?;
        self.in_tx = false;
        Ok(())
    }

    fn in_transaction(&self) -> bool {
        // The flag only knows about transactions this driver opened; `BEGIN` typed into the editor
        // starts one too, and rusqlite can always tell.
        self.in_tx || !self.conn.is_autocommit()
    }
}

impl Sqlite {
    fn columns_of(&mut self, table: &str) -> Result<Vec<Column>> {
        let rows = self.pragma_rows(&format!("PRAGMA table_info({})", quote_pragma_arg(table)))?;
        Ok(rows
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let type_name = Self::text_at(&rows, i, 2);
                let primary_key = Self::text_at(&rows, i, 5) != "0";
                let rowid_alias = primary_key && type_name.trim().eq_ignore_ascii_case("integer");
                Column {
                    name: Self::text_at(&rows, i, 1),
                    type_name,
                    nullable: Self::text_at(&rows, i, 3) == "0" && !rowid_alias,
                    default: match r.get(4) {
                        Some(Value::Null) | None => None,
                        Some(v) => Some(v.as_text()),
                    },
                    primary_key,
                    ordinal: Self::text_at(&rows, i, 0).parse().unwrap_or(0),
                    ..Column::default()
                }
            })
            .collect())
    }

    fn indexes_of(&mut self, table: &str) -> Result<Vec<Index>> {
        let list = self.pragma_rows(&format!("PRAGMA index_list({})", quote_pragma_arg(table)))?;
        let mut out = Vec::new();
        for i in 0..list.len() {
            let name = Self::text_at(&list, i, 1);
            if name.is_empty() {
                continue;
            }
            let unique = Self::text_at(&list, i, 2) != "0";
            let origin = Self::text_at(&list, i, 3);
            let info =
                self.pragma_rows(&format!("PRAGMA index_info({})", quote_pragma_arg(&name)))?;
            let columns: Vec<String> = (0..info.len())
                .map(|r| Self::text_at(&info, r, 2))
                .filter(|c| !c.is_empty())
                .collect();
            out.push(Index {
                name,
                columns,
                unique,
                primary: origin == "pk",
                kind: if origin == "pk" {
                    "primary key".into()
                } else if unique {
                    "unique".into()
                } else {
                    "index".into()
                },
            });
        }
        Ok(out)
    }

    fn foreign_keys_of(&mut self, table: &str) -> Result<Vec<ForeignKey>> {
        let rows = self.pragma_rows(&format!(
            "PRAGMA foreign_key_list({})",
            quote_pragma_arg(table)
        ))?;
        let mut grouped: Vec<(String, ForeignKey)> = Vec::new();
        for i in 0..rows.len() {
            let id = Self::text_at(&rows, i, 0);
            let ref_table = Self::text_at(&rows, i, 2);
            let from = Self::text_at(&rows, i, 3);
            let to = Self::text_at(&rows, i, 4);
            let on_update = Self::text_at(&rows, i, 5);
            let on_delete = Self::text_at(&rows, i, 6);
            match grouped.iter_mut().find(|(gid, _)| *gid == id) {
                Some((_, fk)) => {
                    fk.columns.push(from);
                    fk.ref_columns.push(to);
                }
                None => grouped.push((
                    id.clone(),
                    ForeignKey {
                        name: format!("fk_{table}_{id}"),
                        columns: vec![from],
                        ref_schema: None,
                        ref_table,
                        ref_columns: vec![to],
                        on_delete,
                        on_update,
                    },
                )),
            }
        }
        let mut out: Vec<ForeignKey> = grouped.into_iter().map(|(_, fk)| fk).collect();
        // `REFERENCES customers` names no column: SQLite leaves `to` empty and means the parent's
        // primary key. An empty identifier would be quoted as `""` and match nothing, so the
        // parent's key is filled in from its own table info.
        for fk in &mut out {
            if fk.ref_columns.iter().any(|c| c.is_empty()) {
                if let Ok(info) = self.pragma_rows(&format!(
                    "PRAGMA table_info({})",
                    quote_pragma_arg(&fk.ref_table)
                )) {
                    let keys: Vec<String> = (0..info.len())
                        .filter(|i| Self::text_at(&info, *i, 5) != "0")
                        .map(|i| Self::text_at(&info, i, 1))
                        .collect();
                    if keys.len() == fk.ref_columns.len() {
                        fk.ref_columns = keys;
                    }
                }
            }
        }
        Ok(out)
    }

    /// Table rebuild used by the table designer when SQLite cannot ALTER in place.
    /// Returns the statements to run in order, inside one transaction.
    pub fn rebuild_table_plan(
        &mut self,
        _schema: Option<&str>,
        table: &Table,
        new_columns: &[Column],
    ) -> Result<Vec<String>> {
        let tmp = format!("{}_dc_new", table.name);
        let mut rebuilt = table.clone();
        rebuilt.columns = new_columns.to_vec();
        // The copy list comes from the *live* table, not from `table`: callers hand over a table they
        // have already edited, so a renamed column would not be found there and the SELECT would
        // silently copy nothing (or, worse, a bare identifier would read as a string literal).
        //
        // Columns are paired by ordinal, which is the mapping an in-place edit preserves: a rename
        // keeps its position, a retype keeps it, a drop leaves the rest untouched, and an added
        // column takes a fresh ordinal past every existing one. Pairing by name would lose a rename.
        let live = self.columns_of(&table.name)?;
        let mut pairs: Vec<(String, String)> = Vec::new();
        for column in new_columns {
            if let Some(old) = live.iter().find(|l| l.ordinal == column.ordinal) {
                pairs.push((column.name.clone(), old.name.clone()));
            }
        }
        let (dest, src): (Vec<String>, Vec<String>) = pairs.into_iter().unzip();
        let col_list = dest
            .iter()
            .map(|c| quote_ident(c, Dialect::Sqlite))
            .collect::<Vec<_>>()
            .join(", ");
        let src_list = src
            .iter()
            .map(|c| quote_ident(c, Dialect::Sqlite))
            .collect::<Vec<_>>()
            .join(", ");
        let scratch = Table {
            schema: None,
            name: tmp.clone(),
            ..rebuilt.clone()
        };
        // Enforcement is per-connection state: restore whatever the session had, not a hardcoded
        // ON that would flip unrelated later writes into failing on pre-existing orphans.
        let fk_before = self
            .pragma_rows("PRAGMA foreign_keys")?
            .first()
            .and_then(|r| r.first())
            .map(|v| v.as_text())
            .unwrap_or_else(|| "0".into());
        let restore_pragma = format!(
            "PRAGMA foreign_keys = {};",
            if fk_before != "0" { "ON" } else { "OFF" }
        );
        // Triggers die with DROP TABLE and cannot be ALTERed back — re-run their CREATE after
        // the rename. The DDL comes from sqlite_master, the server's own record of the trigger.
        let trigger_rows = self.pragma_rows(&format!(
            "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND tbl_name = {}",
            quote_pragma_arg(&table.name)
        ))?;
        let trigger_ddl: Vec<String> = (0..trigger_rows.len())
            .filter_map(|i| {
                let sql = Self::text_at(&trigger_rows, i, 0);
                (!sql.is_empty()).then_some(sql)
            })
            .collect();
        let mut script = vec![
            "PRAGMA foreign_keys = OFF;".to_string(),
            // FKs ride inside the CREATE: SQLite cannot ALTER a constraint onto a table, so a
            // rebuild that dropped them would silently delete every reference the table has.
            nook_model::ddl::create_table_with(&scratch, Dialect::Sqlite, true),
            nook_core::quote_qualified(None, &tmp, Dialect::Sqlite),
        ];
        script[2] = format!(
            "INSERT INTO {tmp_q} ({col_list}) SELECT {src_list} FROM {orig};",
            tmp_q = script[2],
            orig = quote_ident(&table.name, Dialect::Sqlite)
        );
        script.push(nook_model::ddl::drop_table(
            None,
            &table.name,
            Dialect::Sqlite,
            false,
        ));
        script.push(nook_model::ddl::rename_table(
            None,
            &tmp,
            &table.name,
            Dialect::Sqlite,
        ));
        script.extend(nook_model::ddl::create_indexes(&rebuilt, Dialect::Sqlite));
        script.extend(trigger_ddl);
        script.push(restore_pragma);
        Ok(script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nook_model::{ddl, plan_edits, EditKind, RowEdit};

    fn db() -> Sqlite {
        let mut db = Sqlite::memory().expect("memory db");
        db.run_script(
            "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, email TEXT NOT NULL, note TEXT DEFAULT 'x');\
             CREATE TABLE posts (id INTEGER PRIMARY KEY, author_id INTEGER REFERENCES users(id) ON DELETE CASCADE, title TEXT);\
             CREATE INDEX ix_posts_author ON posts(author_id);\
             INSERT INTO users (email) VALUES ('a@b.c'), ('d@e.f');",
            10,
        )
        .expect("setup");
        db
    }

    #[test]
    fn create_insert_and_query_round_trip() {
        let mut db = db();
        let rs = db
            .run("SELECT id, email FROM users ORDER BY id", 100)
            .expect("query");
        assert_eq!(rs.columns, vec!["id", "email"]);
        assert_eq!(rs.row_count(), 2);
        assert_eq!(rs.rows[0][0], Value::Int(1));
        assert_eq!(rs.rows[1][1], Value::text("d@e.f"));
        assert!(rs.elapsed_ms >= 0.0);
    }

    #[test]
    fn dml_reports_affected_rows() {
        let mut db = db();
        let rs = db.run("UPDATE users SET note = 'y'", 10).expect("update");
        assert_eq!(rs.affected, 2);
        assert!(rs.columns.is_empty());
        let err = db.run("", 10);
        assert!(err.is_err());
    }

    #[test]
    fn script_with_several_statements_runs_in_order() {
        let mut db = Sqlite::memory().expect("db");
        let rs = db
            .run_script(
                "CREATE TABLE t (a int);\nINSERT INTO t VALUES (1);\nINSERT INTO t VALUES (2);\nSELECT * FROM t;",
                10,
            )
            .expect("script");
        assert_eq!(rs.row_count(), 2);
    }

    #[test]
    fn read_only_connection_refuses_writes() {
        let path = std::env::temp_dir().join("nook_ro_test.db");
        let _ = std::fs::remove_file(&path);
        let mut rw = Sqlite::open(&ConnectionConfig::sqlite(path.to_string_lossy())).expect("open");
        rw.run("CREATE TABLE t (a int)", 1).expect("create");
        let mut ro_cfg = ConnectionConfig::sqlite(path.to_string_lossy());
        ro_cfg.read_only = true;
        let mut ro = Sqlite::open(&ro_cfg).expect("open ro");
        assert!(ro.run("INSERT INTO t VALUES (1)", 1).is_err());
        assert!(ro.run("SELECT * FROM t", 1).is_ok());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn catalog_reads_columns_indexes_and_foreign_keys() {
        let mut db = db();
        let cat = db.catalog().expect("catalog");
        assert_eq!(cat.table_count(), 2);
        let users = cat.find_table(None, "users").expect("users");
        assert_eq!(users.primary_key(), vec!["id".to_string()]);
        assert!(users.column("id").expect("id").auto_increment);
        assert!(!users.column("email").expect("email").nullable);
        assert_eq!(
            users.column("note").expect("note").default.as_deref(),
            Some("'x'")
        );
        let posts = cat.find_table(None, "posts").expect("posts");
        assert_eq!(posts.foreign_keys.len(), 1);
        assert_eq!(posts.foreign_keys[0].ref_table, "users");
        assert!(posts.foreign_keys[0]
            .on_delete
            .eq_ignore_ascii_case("CASCADE"));
        assert!(posts.indexes.iter().any(|i| i.name == "ix_posts_author"));
    }

    #[test]
    fn ddl_and_paging_and_count() {
        let mut db = db();
        let ddl = db.table_ddl(None, "users").expect("ddl");
        assert!(ddl.to_ascii_uppercase().starts_with("CREATE TABLE"));
        assert!(db.table_ddl(None, "missing").is_err());
        let page = db
            .fetch_table(None, "users", 1, 1, Some("\"id\" DESC"), None)
            .expect("page");
        assert_eq!(page.row_count(), 1);
        assert_eq!(page.rows[0][0], Value::Int(1));
        let filtered = db
            .fetch_table(None, "users", 10, 0, None, Some("note = 'x'"))
            .expect("filter");
        assert_eq!(filtered.row_count(), 2);
        assert_eq!(db.count_rows(None, "users").expect("count"), 2);
    }

    #[test]
    fn grid_edit_plan_applies_to_the_real_database() {
        let mut db = db();
        let cat = db.catalog().expect("catalog");
        let users = cat.find_table(None, "users").expect("users").clone();
        let edit = RowEdit {
            kind: EditKind::SetCell,
            row_index: 0,
            schema: None,
            table: "users".into(),
            pk: vec![("id".into(), Value::Int(1))],
            changes: vec![("email".into(), Value::text("new@b.c"))],
        };
        let sql = plan_edits(&[edit], Dialect::Sqlite).expect("plan");
        for s in sql {
            db.run(&s, 1).expect("apply");
        }
        let rs = db
            .run("SELECT email FROM users WHERE id = 1", 1)
            .expect("read");
        assert_eq!(rs.rows[0][0], Value::text("new@b.c"));
        assert_eq!(users.name, "users");
    }

    #[test]
    fn table_designer_rebuild_plan_is_runnable() {
        let mut db = db();
        let cat = db.catalog().expect("catalog");
        let users = cat.find_table(None, "users").expect("users").clone();
        let mut cols = users.columns.clone();
        let mut added = Column::new("created_at", "TEXT");
        added.nullable = true;
        added.default = Some("'1970-01-01'".into());
        cols.insert(0, added);
        let plan = db.rebuild_table_plan(None, &users, &cols).expect("plan");
        for stmt in &plan {
            if stmt.starts_with("/*") {
                continue;
            }
            db.run(stmt, 10).expect("run rebuild step");
        }
        let cat2 = db.catalog().expect("catalog again");
        let users2 = cat2.find_table(None, "users").expect("users again");
        assert!(users2.column("created_at").is_some());
        assert_eq!(users2.primary_key(), vec!["id".to_string()]);
    }

    /// A rebuild of a child table must not drop its foreign key: SQLite cannot ALTER a constraint
    /// back on, so the FK has to ride inside the scratch CREATE. Regression for the rebuild that
    /// silently deleted `posts.author_id REFERENCES users(id)`.
    #[test]
    fn rebuild_plan_keeps_foreign_keys() {
        let mut db = db();
        let cat = db.catalog().expect("catalog");
        let posts = cat.find_table(None, "posts").expect("posts").clone();
        assert!(
            !posts.foreign_keys.is_empty(),
            "the fixture has a child table"
        );
        // Give the child table a key the way the designer would: same columns, marked.
        let mut cols = posts.columns.clone();
        for column in cols.iter_mut() {
            if column.name == "id" {
                column.primary_key = true;
            }
        }
        let plan = db.rebuild_table_plan(None, &posts, &cols).expect("plan");
        for stmt in &plan {
            if stmt.starts_with("/*") {
                continue;
            }
            db.run(stmt, 10).expect("run rebuild step");
        }
        let cat2 = db.catalog().expect("catalog again");
        let posts2 = cat2.find_table(None, "posts").expect("posts again");
        assert_eq!(posts2.primary_key(), vec!["id".to_string()]);
        assert!(
            !posts2.foreign_keys.is_empty(),
            "the foreign key must survive the rebuild"
        );
        assert_eq!(posts2.foreign_keys[0].ref_table, "users");
        // And the constraint really is enforced by the server, not just in the catalog. The plan
        // restores the session's pragma (possibly OFF), so the test opts in itself — the catalog
        // assert above is the one that proves the FK survived.
        db.run("PRAGMA foreign_keys = ON", 1).expect("enforce");
        let err = crate::Driver::run_script(
            &mut db,
            "INSERT INTO posts (id, author_id, title) VALUES (1, 99, 'orphan');",
            1,
        );
        assert!(err.is_err(), "the restored FK must reject an orphan row");
    }

    /// A rebuild must carry the table's triggers across: DROP TABLE destroys them and SQLite
    /// cannot ALTER them back, so the plan has to re-run their CREATE after the rename.
    #[test]
    fn rebuild_plan_keeps_triggers() {
        let mut db = db();
        db.run_script(
            "CREATE TRIGGER posts_touch AFTER UPDATE ON posts BEGIN UPDATE posts SET title = title WHERE id = NEW.id; END;",
            1,
        )
        .expect("trigger");
        let cat = db.catalog().expect("catalog");
        let posts = cat.find_table(None, "posts").expect("posts").clone();
        let plan = db
            .rebuild_table_plan(None, &posts, &posts.columns.clone())
            .expect("plan");
        let joined = plan.join("\n");
        assert!(
            joined.contains("CREATE TRIGGER"),
            "the plan must re-create the trigger: {joined}"
        );
        for stmt in &plan {
            if stmt.starts_with("/*") {
                continue;
            }
            db.run(stmt, 10).expect("run rebuild step");
        }
        let cat2 = db.catalog().expect("catalog again");
        let triggers: Vec<&nook_model::Trigger> = cat2
            .schemas
            .iter()
            .flat_map(|s| s.triggers.iter())
            .filter(|t| t.table == "posts")
            .collect();
        assert_eq!(triggers.len(), 1, "the trigger must survive the rebuild");
    }

    /// A fresh connection enforces foreign keys only when the session asks it to. Bundled SQLite is
    /// compiled with `SQLITE_DEFAULT_FOREIGN_KEYS=1`, so this is the flag the driver has to undo: with
    /// it left alone, an `INSERT` into a child table starts failing on connections that never enabled
    /// enforcement.
    #[test]
    fn a_fresh_connection_does_not_enforce_foreign_keys() {
        let path = std::env::temp_dir().join("nook_fk_default_test.db");
        let _ = std::fs::remove_file(&path);
        for mut db in [
            Sqlite::memory().expect("memory"),
            Sqlite::open(&ConnectionConfig::sqlite(path.to_string_lossy())).expect("file"),
        ] {
            let state = db.pragma_rows("PRAGMA foreign_keys").expect("read pragma")[0][0].clone();
            assert_eq!(state, Value::Int(0), "the documented default is off");
        }
        let _ = std::fs::remove_file(&path);
    }

    /// The plan must restore the connection's foreign_keys pragma to what it was, not force it
    /// on: a rebuild on a session that started with enforcement OFF must not switch unrelated
    /// later writes into failing on pre-existing orphan rows.
    #[test]
    fn rebuild_plan_restores_the_foreign_keys_pragma() {
        let mut db = db();
        db.run("PRAGMA foreign_keys = OFF", 1).expect("off");
        let cat = db.catalog().expect("catalog");
        let posts = cat.find_table(None, "posts").expect("posts").clone();
        let plan = db
            .rebuild_table_plan(None, &posts, &posts.columns.clone())
            .expect("plan");
        let restore = plan.last().expect("restore step");
        assert_eq!(
            restore.trim().to_ascii_lowercase(),
            "pragma foreign_keys = off;",
            "the plan ends by restoring the pragma it changed"
        );
        for stmt in &plan {
            if stmt.starts_with("/*") {
                continue;
            }
            db.run(stmt, 10).expect("run rebuild step");
        }
        let state = db.pragma_rows("PRAGMA foreign_keys").expect("read pragma")[0][0].clone();
        assert_eq!(
            state,
            Value::Int(0),
            "enforcement stays off after the rebuild"
        );
    }

    #[test]
    fn transactions_roll_back() {
        let mut db = db();
        db.begin().expect("begin");
        assert!(db.in_transaction());
        db.run("INSERT INTO users (email) VALUES ('x@y.z')", 1)
            .expect("insert");
        db.rollback().expect("rollback");
        assert!(!db.in_transaction());
        assert_eq!(db.count_rows(None, "users").expect("count"), 2);
    }

    #[test]
    fn hostile_input_is_an_error_not_a_panic() {
        let mut db = Sqlite::memory().expect("db");
        for sql in [
            "SELECT * FROM nope",
            "CREATE TABLE (",
            "SELECT 1; SELECT 2",
            "DROP TABLE",
            "\u{0}",
        ] {
            let _ = db.run(sql, 1);
        }
        let _ = db.table_ddl(None, "a'b'c");
        let _ = db.count_rows(None, "no such table");
        let cat = db.catalog().expect("empty catalog is fine");
        assert_eq!(cat.table_count(), 0);
    }

    #[test]
    fn structure_diff_against_live_catalog() {
        let mut db = db();
        let live = db.catalog().expect("catalog");
        let mut wanted = live.clone();
        {
            let users = wanted
                .schemas
                .first_mut()
                .and_then(|s| s.tables.iter_mut().find(|t| t.name == "users"))
                .expect("users");
            users.columns.push(Column::new("nickname", "TEXT"));
        }
        let changes = nook_model::diff_catalogs(&wanted, &live, Dialect::Sqlite);
        assert_eq!(changes.len(), 1);
        db.run(&changes[0].sql, 1).expect("apply add column");
        let after = db.catalog().expect("catalog");
        assert!(after
            .find_table(None, "users")
            .expect("users")
            .column("nickname")
            .is_some());
    }

    #[test]
    fn ddl_helpers_match_this_engine() {
        let mut db = db();
        let cat = db.catalog().expect("catalog");
        let posts = cat.find_table(None, "posts").expect("posts").clone();
        let create = ddl::create_table(&posts, Dialect::Sqlite);
        let target = create.replace("\"posts\"", "\"posts_copy\"");
        db.run(&target, 1).expect("create copy");
        assert_eq!(db.count_rows(None, "posts_copy").expect("count"), 0);
    }
}
