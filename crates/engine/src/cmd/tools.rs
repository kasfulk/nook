//! Tools: data generation, CSV/JSON import, SQL import, export, the visual query builder and reports.

use std::collections::BTreeMap;

use nook_core::{quote_ident, quote_qualified, sql_literal, Dialect, Error, Result, Value};
use nook_model::ddl;
use serde_json::json;

use crate::cmd::{dialect_name, run_sql};
use crate::datagen::{self, GenSpec, Generator};
use crate::report::{self, ChartKind};
use crate::{CommandSpec, Level, Params, Session, TableRef};

pub fn commands() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("tools.datagen", "Generate data", "Tools", datagen_cmd)
            .params(r#"{"table":"customers","rows":1000,"seed":1,"overrides":{"status":"pick:new|closed"},"apply":false,"out":"~/customers.sql"}"#)
            .help("Fake-but-consistent rows. apply:true inserts them; \"out\" writes an INSERT script."),
        CommandSpec::new("tools.generators", "Available generators", "Tools", generators),
        CommandSpec::new("tools.importCsv", "Import CSV", "Tools", import_csv)
            .params(r#"{"path":"~/data/users.csv","table":"users","create_table":true}"#),
        CommandSpec::new("tools.importExcel", "Import Excel", "Tools", import_excel)
            .params(r#"{"path":"~/data/users.xlsx","table":"users","create_table":true}"#)
            .help("First sheet of an .xlsx file; every cell arrives as text, like the CSV import."),
        CommandSpec::new("tools.importJson", "Import JSON", "Tools", import_json)
            .params(r#"{"path":"~/data/users.json","table":"users","create_table":true}"#),
        CommandSpec::new("tools.importSql", "Run a .sql file", "Tools", import_sql)
            .params(r#"{"path":"~/data/seed.sql"}"#),
        CommandSpec::new("tools.exportExcel", "Export a query result to Excel", "Tools", export_excel)
            .params(r#"{"sql":"select * from users","path":"~/exports/users.xlsx"}"#),
        CommandSpec::new("quickOpen.search", "Quick-open search", "Tools", quick_open_search)
            .params(r#"{"query":"cust","limit":40}"#)
            .help("One query, ranked hits across every open catalog plus saved queries and open tabs. This is what Cmd+P runs."),
        CommandSpec::new("quickOpen.open", "Open a quick-open hit", "Tools", quick_open_open)
            .params(r#"{"kind":"table","conn":"local","schema":"main","name":"customers"}"#),
        CommandSpec::new("tools.serverSearch", "Search the server's objects", "Tools", server_search)
            .params(r#"{"pattern":"order"}"#)
            .help("The catalog-wide object search: tables and views whose name, columns, schema or connection match."),
        CommandSpec::new("tools.exportQuery", "Export a query result", "Tools", export_query)
            .params(r#"{"sql":"select * from users","path":"~/exports/users.csv","format":"csv","stream":false}"#)
            .help("stream:true pages the statement instead of holding the result; csv and jsonl only."),
        CommandSpec::new("tools.importWizard", "Import wizard (dry run)", "Tools", import_wizard)
            .params(r#"{"path":"~/data/users.csv","table":"users"}"#)
            .help("Reads the file, infers a type per column and returns the first 20 rows as they would land. Writes nothing."),
        CommandSpec::new("tools.importXml", "Import XML", "Tools", import_xml)
            .params(r#"{"path":"~/data/users.xml","table":"users","row":"record"}"#)
            .help("Flat XML: one element per row, one child per column. Nested XML is refused by name."),
        CommandSpec::new("tools.importDbf", "Import DBF", "Tools", import_dbf)
            .params(r#"{"path":"~/data/people.dbf","table":"people","create_table":true}"#)
            .help("dBASE III/IV: the header names the columns and their widths. Deleted records are skipped and counted; dBASE II and Visual FoxPro are refused by name."),
        CommandSpec::new("tools.textImporter", "Import fixed-width text", "Tools", text_importer)
            .params(r#"{"path":"~/data/codes.txt","table":"codes","widths":[3,12,8],"header":false}"#)
            .help("A delimiter-less file split on explicit column widths."),
        CommandSpec::new("export.stream", "Stream a table or query to a file", "Tools", export_stream)
            .params(r#"{"table":"orders","path":"~/exports/orders.csv","format":"csv","page":5000}"#)
            .help("Pages the rows out without holding the result: csv or jsonl. json is refused."),
        CommandSpec::new(
            "tools.dataDictionary",
            "Data dictionary",
            "Tools",
            data_dictionary,
        )
        .params(r#"{"schema":"public","format":"markdown","path":"~/dict.md"}"#)
        .help(
            "Every table, column, type, key, default and comment, as Markdown or self-contained \
             HTML; one pass over the cached catalog, no new queries.",
        ),
        CommandSpec::new("tools.queryBuilder", "Query builder", "Tools", query_builder)
            .params(r#"{"from":[{"table":"users","alias":"u","columns":["id","email"]}],"where":"u.email like '%@example.com'","order_by":"u.id","limit":100}"#)
            .help("Builds a SELECT from a table/column/join description. Returns the SQL."),
        CommandSpec::new("report.aggregate", "Aggregate report", "Report", aggregate)
            .params(r#"{"table":"orders","group":"status","agg":"sum","value":"amount","chart":"column"}"#),
        CommandSpec::new("report.chart", "Chart a query", "Report", chart)
            .params(r#"{"sql":"select status, count(*) from orders group by status","chart":"pie","label":"status"}"#),
        CommandSpec::new("report.pivot", "Pivot a result", "Report", pivot)
            .params(r#"{"sql":"select region, quarter, sum(total) from sales group by 1,2"}"#),
        CommandSpec::new("report.save", "Save a report", "Report", report_save)
            .params(r#"{"name":"revenue","sql":"select day, sum(total) from orders group by 1","chart":"line","label":"day"}"#)
            .help("A report is the statement, the chart kind and its label column, stored by name. Saving over a name replaces it."),
        CommandSpec::new("report.list", "Saved reports", "Report", report_list)
            .params("{}")
            .help("Every saved report, newest name first, with the chart kind and when it last changed."),
        CommandSpec::new("report.delete", "Delete a report", "Report", report_delete)
            .params(r#"{"name":"revenue"}"#)
            .help("Removes one saved report. A name that is not saved is a no-op, so a script can run it twice."),
        CommandSpec::new("report.designer", "Report designer", "Report", report_designer)
            .params(r#"{"name":"revenue","layout":{"w":6,"h":4}}"#)
            .help("The save/edit surface: stores the report and its tile layout, and returns the report as it was written so a designer can paint what the server holds."),
        CommandSpec::new("report.biDashboard", "BI dashboard", "Report", report_dashboard)
            .params(r#"{"name":"ops","tiles":[{"report":"revenue","w":6,"h":4,"x":0,"y":0}]}"#)
            .help("A board is a name and its tiles. Each tile names a saved report and carries its own place on a 12-column grid; the layout is stored, never recomputed."),
        CommandSpec::new("report.board", "Paint a board", "Report", report_board)
            .params(r#"{"name":"ops"}"#)
            .help("Runs every tile's statement and returns each one charted, plus any tile that failed — a board with one broken tile still shows the rest."),
        CommandSpec::new("report.export", "Export a report", "Report", report_export)
            .params(r#"{"name":"ops","format":"html","path":"~/ops.html"}"#)
            .help("A report or a whole board as one self-contained HTML file, or the board's rows as CSV."),
        CommandSpec::new("monitor.charts", "Metrics board", "Monitor", metrics_charts)
            .params(r#"{"board":"ops","every_s":30}"#)
            .help("A board refreshed on a cadence: returns the tiles' charts and the interval to wait before asking again. The UI's own tick asks, so there is no second timer thread."),
        CommandSpec::new("tools.dataProfiling", "Profile columns", "Tools", data_profiling)
            .params(r#"{"table":"users","columns":["email","status"]}"#)
            .help("Per column: null share, distinct count, min/max, the five commonest values, and a shape guess (email, uuid, date, phone). One aggregate query per table, bounded by max_rows."),
        CommandSpec::new("metrics.input", "Board input", "Report", metrics_input)
            .params(r#"{"name":"since","value":"2026-10-01"}"#)
            .help("Binds a value that every tile's SQL reads as a named parameter, so one field moves the whole board."),
    ]
}

fn generators(_session: &mut Session, _p: &Params) -> Result<serde_json::Value> {
    Ok(json!({
        "generators": [
            "auto", "sequence", "int:1-100", "float:0-1000", "bool",
            "firstname", "lastname", "name", "email", "phone", "city", "country", "company",
            "word", "sentence", "pattern:INV-####", "pick:a|b|c",
            "date:365", "timestamp:365", "uuid", "null"
        ],
        "notes": "Overrides are keyed by column name, for example {\"email\": \"pattern:user-####@example.com\"}",
    }))
}

fn datagen_cmd(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = session.resolve_conn(p)?;
    let spec_name = p.str_arg("table")?;
    let reference = TableRef::parse(&spec_name);
    let table = session.table_of(index, reference.schema.as_deref(), &reference.table)?;
    let rows = p.int_opt("rows", 100).max(0) as usize;
    let seed = p.int_opt("seed", 1).max(0) as u64;
    let mut spec = GenSpec::from_table(&table, rows, seed);
    if let Some(serde_json::Value::Object(map)) = p.get("overrides") {
        let overrides: BTreeMap<String, String> = map
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
            .collect();
        spec.apply_overrides(&overrides)?;
    }
    let generated = datagen::generate(&spec, &table)?;
    let dialect = session
        .conns
        .get(index)
        .and_then(|c| c.driver.as_ref())
        .map(|d| d.dialect())
        .ok_or_else(|| Error::config("the connection is not open"))?;
    let script = datagen::insert_script(&spec, &table, &generated, dialect)?;

    let mut out = json!({
        "table": table.key(),
        "rows": generated.len(),
        "seed": seed,
        "statements": script.lines().count(),
        "preview": generated.iter().take(5).map(|row| {
            spec.columns.iter().enumerate().map(|(i, c)| {
                (c.name.clone(), row.get(i).cloned().unwrap_or(Value::Null).to_json())
            }).collect::<serde_json::Map<String, serde_json::Value>>()
        }).collect::<Vec<_>>(),
    });

    if p.bool_opt("apply", false) {
        let limit = session.settings.max_rows;
        let statements: Vec<&str> = script.lines().take(limit).collect();
        session.driver_for(index)?.begin()?;
        for statement in &statements {
            if let Err(e) = run_sql(session, index, statement, Some(1)) {
                let _ = session.driver_for(index).and_then(|d| d.rollback());
                return Err(e);
            }
        }
        session.driver_for(index)?.commit()?;
        session.log(
            Level::Success,
            format!("inserted {} generated rows", statements.len()),
        );
        out["inserted"] = json!(statements.len());
    }

    if let Some(path) = p.str_opt("out") {
        let path = crate::cmd::grid::expand(&path, &session.home);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::io(format!("create {}: {e}", parent.display())))?;
        }
        std::fs::write(&path, &script)
            .map_err(|e| Error::io(format!("write {}: {e}", path.display())))?;
        out["path"] = json!(path.to_string_lossy());
    }
    Ok(out)
}

/// CSV per RFC 4180. One parser lives in `crate::csv`; this is the whole-file wrapper so callers
/// holding a `String` keep working.
pub fn parse_csv(text: &str, delimiter: char) -> Vec<Vec<String>> {
    crate::csv::parse_csv(text, delimiter)
}

/// Guess a value's type so an import does not turn every number into text.
pub fn infer_value(text: &str, target_type: Option<&str>) -> Value {
    let raw = text.trim();
    if let Some(ty) = target_type {
        if let Ok(v) = Value::coerce(raw, ty) {
            return v;
        }
    }
    if raw.is_empty() {
        return Value::Null;
    }
    if raw.eq_ignore_ascii_case("null") {
        return Value::Null;
    }
    if raw.eq_ignore_ascii_case("true") || raw.eq_ignore_ascii_case("false") {
        return Value::Bool(raw.eq_ignore_ascii_case("true"));
    }
    if raw.parse::<i64>().is_ok() {
        return Value::Int(raw.parse().unwrap_or(0));
    }
    if raw.parse::<f64>().is_ok() && raw.contains('.') {
        return Value::Float(raw.parse().unwrap_or(0.0));
    }
    Value::text(raw)
}

/// `["src=target", ...]` pairs from the `columnMapping` parameter; empty input means no mapping.
pub(crate) fn parse_mapping(entries: &[String]) -> Result<Option<Vec<(String, String)>>> {
    if entries.is_empty() {
        return Ok(None);
    }
    let mut map = Vec::with_capacity(entries.len());
    for entry in entries {
        let (source, target) = entry.split_once('=').ok_or_else(|| {
            Error::config(format!(
                "columnMapping entries are 'source=target', got '{entry}'"
            ))
        })?;
        let (source, target) = (source.trim(), target.trim());
        if source.is_empty() || target.is_empty() {
            return Err(Error::config(format!(
                "columnMapping entries are 'source=target', got '{entry}'"
            )));
        }
        map.push((source.to_string(), target.to_string()));
    }
    Ok(Some(map))
}

/// How an import treats rows that already exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportMode {
    /// Plain inserts; a duplicate key fails the batch.
    Append,
    /// Empty the table first, then insert.
    Truncate,
    /// Insert or update on the key's uniqueness.
    Upsert,
}

impl ImportMode {
    fn parse(text: Option<&str>) -> Result<ImportMode> {
        match text
            .unwrap_or("append")
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "append" | "insert" => Ok(ImportMode::Append),
            "truncate" | "replace" => Ok(ImportMode::Truncate),
            "upsert" | "merge" => Ok(ImportMode::Upsert),
            other => Err(Error::config(format!(
                "unknown mode '{other}'; try append, truncate or upsert"
            ))),
        }
    }
}

/// Everything the row loop needs, resolved once from the catalog and the parameters.
struct ImportPlan {
    target: String,
    columns: Vec<String>,
    types: Vec<Option<String>>,
    dialect: Dialect,
    mode: ImportMode,
    key: Vec<String>,
    batch: usize,
    mapping: Option<Vec<(String, String)>>,
    max_rows: usize,
    /// Column order of the file, once the header is known.
    file_columns: Vec<String>,
    created_table: bool,
}

/// The result of an import, including a partial one.
struct ImportOutcome {
    inserted: usize,
    /// Rows the upsert replaced rather than added, when the driver reports it.
    updated: usize,
    batches: usize,
    stopped_at: Option<usize>,
    stopped_because: Option<String>,
}

/// Turn a header row and a mapping into the plan's column list, and validate it.
fn plan_columns(
    session: &Session,
    index: usize,
    reference: &TableRef,
    file_columns: &[String],
    mapping: Option<&Vec<(String, String)>>,
    key: &[String],
) -> Result<(Vec<String>, Vec<Option<String>>)> {
    let table = session
        .conns
        .get(index)
        .and_then(|c| c.catalog.as_ref())
        .and_then(|cat| cat.find_table(reference.schema.as_deref(), &reference.table));
    let (targets, types) = if let Some(map) = mapping {
        let table = table.ok_or_else(|| {
            Error::config(format!(
                "columnMapping needs an existing table; {} was not found in the catalog",
                reference.key()
            ))
        })?;
        let mut targets = Vec::with_capacity(map.len());
        let mut types = Vec::with_capacity(map.len());
        for (source, target) in map {
            if !file_columns.iter().any(|c| c == source) {
                return Err(Error::config(format!(
                    "the file has no column '{source}'; it has: {}",
                    file_columns.join(", ")
                )));
            }
            let column = table.column(target).ok_or_else(|| {
                Error::config(format!(
                    "the table {} has no column '{target}'",
                    reference.key()
                ))
            })?;
            targets.push(target.clone());
            types.push(Some(column.type_name.clone()));
        }
        (targets, types)
    } else if let Some(table) = table {
        // Against an existing table the catalog decides which columns exist. A file column the
        // table does not have is a note, not a failure: extra columns are how people hand in a
        // dump with more fields than the target wants.
        let targets: Vec<String> = file_columns
            .iter()
            .filter(|c| table.column(c).is_some())
            .cloned()
            .collect();
        if targets.is_empty() {
            return Err(Error::config(format!(
                "none of the file's columns ({}) exist in {}; pass columnMapping",
                file_columns.join(", "),
                reference.key()
            )));
        }
        let types = targets
            .iter()
            .map(|name| table.column(name).map(|c| c.type_name.clone()))
            .collect();
        (targets, types)
    } else {
        (file_columns.to_vec(), vec![None; file_columns.len()])
    };
    if !key.is_empty() {
        for k in key {
            if !targets.iter().any(|t| t == k) {
                return Err(Error::config(format!(
                    "the key column '{k}' is not part of the import; it has: {}",
                    targets.join(", ")
                )));
            }
        }
    }
    Ok((targets, types))
}

/// Refuse an upsert whose key cannot decide a collision.
///
/// A key that is not a primary key or a unique index makes the database accept the statement and
/// then silently insert a duplicate on the next run, which is the exact failure `mode: upsert` is
/// meant to prevent. Checking the catalog is cheap; discovering the duplicate at row 900,000 is not.
fn check_upsert_key(
    session: &Session,
    index: usize,
    reference: &TableRef,
    key: &[String],
) -> Result<()> {
    if key.is_empty() {
        return Err(Error::config(
            "mode: upsert needs \"key\": the column (or columns) whose uniqueness decides a collision",
        ));
    }
    let table = session
        .conns
        .get(index)
        .and_then(|c| c.catalog.as_ref())
        .and_then(|cat| cat.find_table(reference.schema.as_deref(), &reference.table))
        .ok_or_else(|| {
            Error::config(format!(
                "mode: upsert needs an existing table; {} is not in the catalog yet",
                reference.key()
            ))
        })?;
    let pk = table.primary_key();
    if pk == key {
        return Ok(());
    }
    let unique = table.indexes.iter().any(|i| i.unique && i.columns == key);
    if unique {
        return Ok(());
    }
    Err(Error::config(format!(
        "the key ({}) is not unique on {}: its primary key is ({}); an upsert on it would insert \
         duplicates. Add a unique index or name the primary key",
        key.join(", "),
        reference.key(),
        if pk.is_empty() {
            "none".to_string()
        } else {
            pk.join(", ")
        }
    )))
}

/// Run an import over a stream of rows.
///
/// The stream is pulled one row at a time and committed every `batch` rows. Peak memory is one
/// batch, not the file, and a failure costs the current batch rather than every row before it.
fn import_stream<I>(
    session: &mut Session,
    index: usize,
    reference: &TableRef,
    plan: ImportPlan,
    rows: I,
) -> Result<serde_json::Value>
where
    I: Iterator<Item = Result<Vec<String>>>,
{
    let ImportPlan {
        target,
        columns,
        types,
        dialect,
        mode,
        key,
        batch,
        mapping,
        max_rows,
        file_columns,
        created_table,
    } = plan;
    // Where each target column lives in the file's row.
    let source_index: Vec<usize> = columns
        .iter()
        .map(|target| {
            let source = mapping
                .as_ref()
                .and_then(|m| m.iter().find(|(_, t)| t == target).map(|(s, _)| s.clone()))
                .unwrap_or_else(|| target.clone());
            file_columns.iter().position(|c| *c == source).unwrap_or(0)
        })
        .collect();

    if mode == ImportMode::Truncate {
        // Only after the header parsed: a truncate that runs before the file is validated is how a
        // typo in the path eats a table.
        let sql = ddl::truncate_table(reference.schema.as_deref(), &reference.table, dialect);
        session.driver_for(index)?.run(&sql, 0)?;
    }

    let mut outcome = ImportOutcome {
        inserted: 0,
        updated: 0,
        batches: 0,
        stopped_at: None,
        stopped_because: None,
    };
    let mut pending: Vec<String> = Vec::with_capacity(batch);
    let mut seen = 0usize;
    // An upsert cannot know per row whether it inserted or updated from the statement alone, so
    // the count comes from the table itself: a delta of the rows before and after is the only
    // number that cannot be wrong. The other modes count statements, which is exact for them.
    let counted_before = if mode == ImportMode::Upsert {
        count_rows(
            session,
            index,
            reference.schema.as_deref(),
            &reference.table,
        )?
    } else {
        0
    };

    for row in rows {
        if seen >= max_rows {
            outcome.stopped_at = Some(seen + 1);
            outcome.stopped_because = Some(format!(
                "stopped at the max_rows limit of {max_rows}; pass a larger max_rows to take the rest"
            ));
            break;
        }
        let row = row?;
        seen += 1;
        let values: Vec<Value> = columns
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let raw = row.get(source_index[i]).map(String::as_str).unwrap_or("");
                infer_value(raw, types.get(i).and_then(|t| t.as_deref()))
            })
            .collect();
        let sql = match mode {
            ImportMode::Upsert => ddl::upsert_statement(
                reference.schema.as_deref(),
                &reference.table,
                &columns,
                &key,
                &values,
                dialect,
            ),
            _ => {
                let column_list = columns
                    .iter()
                    .map(|c| quote_ident(c, dialect))
                    .collect::<Vec<_>>()
                    .join(", ");
                let literals = values
                    .iter()
                    .map(|v| sql_literal(v, dialect))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("INSERT INTO {target} ({column_list}) VALUES ({literals});")
            }
        };
        pending.push(sql);
        if pending.len() >= batch {
            match run_batch(session, index, &pending) {
                Ok(()) => {
                    outcome.inserted += pending.len();
                    outcome.batches += 1;
                    pending.clear();
                }
                Err(e) => {
                    outcome.stopped_at = Some(seen);
                    outcome.stopped_because = Some(e.message().to_string());
                    pending.clear();
                    break;
                }
            }
        }
    }
    if outcome.stopped_at.is_none() && !pending.is_empty() {
        match run_batch(session, index, &pending) {
            Ok(()) => {
                outcome.inserted += pending.len();
                outcome.batches += 1;
            }
            Err(e) => {
                outcome.stopped_at = Some(seen);
                outcome.stopped_because = Some(e.message().to_string());
            }
        }
    }

    if mode == ImportMode::Upsert {
        let after = count_rows(
            session,
            index,
            reference.schema.as_deref(),
            &reference.table,
        )?;
        // `inserted` is every statement that ran; the table only grew by the difference. Reporting
        // both is what makes "200000 inserted" on a table that did not grow impossible.
        outcome.updated = outcome
            .inserted
            .saturating_sub(after.saturating_sub(counted_before));
        outcome.inserted = after.saturating_sub(counted_before);
    }
    let _ = crate::cmd::refresh_catalog(session, index);
    let committed = outcome.inserted;
    if let Some(row) = outcome.stopped_at {
        // "import failed" after 800,000 committed rows must not read like nothing happened.
        session.log(
            Level::Error,
            format!(
                "import stopped at row {row}: {}. {committed} rows before it are committed",
                outcome
                    .stopped_because
                    .as_deref()
                    .unwrap_or("the statement failed")
            ),
        );
    } else {
        session.log(
            Level::Success,
            format!("imported {committed} rows into {}", reference.key()),
        );
    }
    Ok(json!({
        "table": reference.key(),
        "columns": columns,
        "inserted": outcome.inserted,
        "updated": outcome.updated,
        "batches": outcome.batches,
        "batch": batch,
        "mode": match mode {
            ImportMode::Append => "append",
            ImportMode::Truncate => "truncate",
            ImportMode::Upsert => "upsert",
        },
        "key": key,
        "created_table": created_table,
        "dialect": dialect_name(dialect),
        "failed_at_row": outcome.stopped_at,
        "failed_because": outcome.stopped_because,
        "notes": if outcome.stopped_at.is_some() {
            vec![format!(
                "the import stopped early; {committed} rows were committed before it and stay committed"
            )]
        } else {
            Vec::<String>::new()
        },
    }))
}

/// One transaction per batch: the lock is held for `batch` statements, not for the whole file.
/// `SELECT COUNT(*)` on a table, through the same connection the import writes to.
fn count_rows(
    session: &mut Session,
    index: usize,
    schema: Option<&str>,
    table: &str,
) -> Result<usize> {
    let dialect = session.driver_for(index)?.dialect();
    let target = match schema {
        Some(s) => format!(
            "{}.{}",
            quote_ident(s, dialect),
            quote_ident(table, dialect)
        ),
        None => quote_ident(table, dialect),
    };
    let rs = session
        .driver_for(index)?
        .run(&format!("SELECT COUNT(*) AS n FROM {target}"), 0)?;
    let n = rs
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.display().parse::<usize>().ok())
        .unwrap_or(0);
    Ok(n)
}

fn run_batch(session: &mut Session, index: usize, statements: &[String]) -> Result<()> {
    session.driver_for(index)?.begin()?;
    for sql in statements {
        if let Err(e) = session.driver_for(index)?.run(sql, 0) {
            let _ = session.driver_for(index).and_then(|d| d.rollback());
            return Err(e);
        }
    }
    session.driver_for(index)?.commit()
}

/// Build the plan for a stream whose header has already been read.
#[allow(clippy::too_many_arguments)]
fn prepare_import(
    session: &mut Session,
    index: usize,
    reference: &TableRef,
    file_columns: Vec<String>,
    p: &Params,
    create_table: bool,
) -> Result<ImportPlan> {
    let mode = ImportMode::parse(p.str_opt("mode").as_deref())?;
    let key = p.list("key");
    let mapping = parse_mapping(&p.list("columnMapping"))?;
    let batch = p.int_opt("batch", 2_000).clamp(1, 100_000) as usize;
    // An import is not a grid page. `settings.max_rows` caps what the grid paints; inheriting it
    // here would mean no file could ever import more than a screenful, which is the silent
    // truncation this phase removes. The import's own ceiling is explicit and can be raised.
    let max_rows = p.int_opt("max_rows", 5_000_000).clamp(1, 100_000_000) as usize;
    let dialect = session
        .conns
        .get(index)
        .and_then(|c| c.driver.as_ref())
        .map(|d| d.dialect())
        .ok_or_else(|| Error::config("the connection is not open"))?;

    let exists = session
        .conns
        .get(index)
        .and_then(|c| c.catalog.as_ref())
        .and_then(|cat| cat.find_table(reference.schema.as_deref(), &reference.table))
        .is_some();

    let mut created = false;
    if create_table && !exists {
        let mut table = nook_model::Table::new(reference.schema.clone(), reference.table.clone());
        for name in &file_columns {
            table
                .columns
                .push(nook_model::Column::new(name.clone(), "TEXT"));
        }
        run_sql(session, index, &ddl::create_table(&table, dialect), Some(1))?;
        created = true;
        let _ = crate::cmd::refresh_catalog(session, index);
    }

    if mode == ImportMode::Upsert {
        check_upsert_key(session, index, reference, &key)?;
    }
    let (columns, types) = plan_columns(
        session,
        index,
        reference,
        &file_columns,
        mapping.as_ref(),
        &key,
    )?;
    let target = quote_qualified(reference.schema.as_deref(), &reference.table, dialect);
    Ok(ImportPlan {
        target,
        columns,
        types,
        dialect,
        mode,
        key,
        batch,
        mapping,
        max_rows,
        file_columns,
        created_table: created,
    })
}

fn import_csv(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let path = crate::cmd::grid::expand(&p.str_arg("path")?, &session.home);
    let delimiter = p
        .str_opt("delimiter")
        .and_then(|d| d.chars().next())
        .unwrap_or(',');
    let latin1 = p
        .str_opt("encoding")
        .map(|e| e.eq_ignore_ascii_case("latin1") || e.eq_ignore_ascii_case("latin-1"))
        .unwrap_or(false);
    let index = session.resolve_conn(p)?;
    let reference = TableRef::parse(&p.str_arg("table")?);
    let header = p.bool_opt("header", true);
    let create_table = p.bool_opt("create_table", true);

    // The reader is opened here, not after the plan: `truncate` must run only once the header has
    // parsed, and the header is the first thing the reader yields.
    let reader = crate::csv::open(&path, latin1)?;
    let mut rows = crate::csv::rows(reader, delimiter);
    let first = match rows.next() {
        Some(Ok(row)) => row,
        Some(Err(e)) => return Err(e),
        None => return Err(Error::config(format!("{} has no rows", path.display()))),
    };
    // Lazy on purpose: collecting here would put the whole file back in memory, which is the
    // defect this path exists to remove. The header is consumed, the rest is pulled by the loop.
    let (file_columns, rest): (Vec<String>, RowIter) = if header {
        (first, Box::new(rows))
    } else {
        (
            (0..first.len())
                .map(|i| format!("column_{}", i + 1))
                .collect(),
            Box::new(std::iter::once(Ok(first)).chain(rows)),
        )
    };
    let plan = prepare_import(session, index, &reference, file_columns, p, create_table)?;
    let mut out = import_stream(session, index, &reference, plan, rest)?;
    out["path"] = json!(path.to_string_lossy());
    out["format"] = json!("csv");
    out["encoding"] = json!(if latin1 { "latin1" } else { "utf-8" });
    Ok(out)
}

fn import_excel(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let path = crate::cmd::grid::expand(&p.str_arg("path")?, &session.home);
    let rows = nook_drivers::xlsx::read_rows(&path)?;
    let index = session.resolve_conn(p)?;
    let reference = TableRef::parse(&p.str_arg("table")?);
    let header = p.bool_opt("header", true);
    let create_table = p.bool_opt("create_table", true);
    if rows.is_empty() {
        return Err(Error::config(format!("{} has no rows", path.display())));
    }
    let (file_columns, data): (Vec<String>, Vec<Vec<String>>) = if header {
        (rows[0].clone(), rows[1..].to_vec())
    } else {
        (
            (0..rows[0].len())
                .map(|i| format!("column_{}", i + 1))
                .collect(),
            rows.clone(),
        )
    };
    let plan = prepare_import(session, index, &reference, file_columns, p, create_table)?;
    let mut out = import_stream(session, index, &reference, plan, data.into_iter().map(Ok))?;
    out["path"] = json!(path.to_string_lossy());
    out["format"] = json!("xlsx");
    out["notes"] = json!([
        "an .xlsx is read whole: it is a zip of XML, and streaming it needs a reader crate. \
         The max_rows limit still applies"
    ]);
    Ok(out)
}

fn export_excel(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = session.resolve_conn(p)?;
    let sql = p
        .str_opt("sql")
        .ok_or_else(|| Error::config("pass \"sql\""))?;
    let result = run_sql(session, index, &sql, None)?;
    let mut rows: Vec<Vec<String>> = vec![result.columns.clone()];
    for row in &result.rows {
        // as_text is the round-trip method: NULL must export as an empty cell, not "NULL".
        rows.push(row.iter().map(|v| v.as_text()).collect());
    }
    let path = crate::cmd::grid::expand(
        &p.str_opt("path")
            .unwrap_or_else(|| format!("exports/query-{}.xlsx", crate::jobs::now())),
        &session.home,
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::io(format!("create {}: {e}", parent.display())))?;
    }
    nook_drivers::xlsx::write_rows(&path, &rows)?;
    session.log(Level::Success, format!("exported to {}", path.display()));
    Ok(json!({
        "path": path.to_string_lossy(),
        "rows": result.row_count(),
        "format": "xlsx",
    }))
}

/// JSON, two shapes: `.jsonl` streams a line at a time, `.json` is one array read whole.
///
/// A JSON array cannot be streamed without a streaming parser, and `serde_json` is not one. Saying
/// so in `notes` is better than pretending: the file the wizard points people at for big data is
/// `.jsonl`.
fn import_json(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let path = crate::cmd::grid::expand(&p.str_arg("path")?, &session.home);
    let index = session.resolve_conn(p)?;
    let reference = TableRef::parse(&p.str_arg("table")?);
    let create_table = p.bool_opt("create_table", true);
    let jsonl = path
        .extension()
        .map(|e| e.eq_ignore_ascii_case("jsonl") || e.eq_ignore_ascii_case("ndjson"))
        .unwrap_or(false)
        || p.bool_opt("jsonl", false);

    let file = std::fs::File::open(&path)
        .map_err(|e| Error::io(format!("read {}: {e}", path.display())))?;
    let mut reader = std::io::BufReader::new(file);
    let mut first_line = String::new();
    if jsonl {
        use std::io::BufRead;
        loop {
            first_line.clear();
            let n = reader
                .read_line(&mut first_line)
                .map_err(|e| Error::io(format!("read {}: {e}", path.display())))?;
            if n == 0 {
                return Err(Error::config(format!("{} has no rows", path.display())));
            }
            if !first_line.trim().is_empty() {
                break;
            }
        }
        let first: serde_json::Value = serde_json::from_str(first_line.trim())
            .map_err(|e| Error::config(format!("{}: {e}", path.display())))?;
        let columns = object_keys(&first)
            .ok_or_else(|| Error::config("every JSON line must be an object with named fields"))?;
        let plan = prepare_import(session, index, &reference, columns.clone(), p, create_table)?;
        // The first line taught us the columns; it is still a row.
        let first_row = row_from(&first, &columns);
        let rest = JsonLines {
            reader,
            columns: columns.clone(),
            path: path.clone(),
            line: 1,
            pending: Some(first_row),
        };
        let mut out = import_stream(session, index, &reference, plan, rest)?;
        out["path"] = json!(path.to_string_lossy());
        out["format"] = json!("jsonl");
        Ok(out)
    } else {
        use std::io::Read;
        let mut text = String::new();
        reader
            .read_to_string(&mut text)
            .map_err(|e| Error::io(format!("read {}: {e}", path.display())))?;
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| Error::config(format!("{}: {e}", path.display())))?;
        let items = match value {
            serde_json::Value::Array(items) => items,
            serde_json::Value::Object(map) => match map.get("rows") {
                Some(serde_json::Value::Array(items)) => items.clone(),
                _ => vec![serde_json::Value::Object(map)],
            },
            _ => return Err(Error::config("the JSON root must be an array or object")),
        };
        let mut columns: Vec<String> = Vec::new();
        for item in &items {
            if let Some(keys) = object_keys(item) {
                for key in keys {
                    if !columns.contains(&key) {
                        columns.push(key);
                    }
                }
            }
        }
        if columns.is_empty() {
            return Err(Error::config("the JSON has no object keys to import"));
        }
        let file_columns = columns.clone();
        let rows = items.iter().map(move |item| {
            object_keys(item)
                .map(|_| {
                    file_columns
                        .iter()
                        .map(|c| match item.get(c) {
                            Some(serde_json::Value::String(s)) => s.clone(),
                            Some(serde_json::Value::Null) | None => String::new(),
                            Some(other) => other.to_string(),
                        })
                        .collect::<Vec<String>>()
                })
                .ok_or_else(|| {
                    Error::config("every array entry must be an object with named fields")
                })
        });
        let plan = prepare_import(session, index, &reference, columns, p, create_table)?;
        let mut out = import_stream(session, index, &reference, plan, rows)?;
        out["path"] = json!(path.to_string_lossy());
        out["format"] = json!("json");
        out["notes"] = json!([
            "a .json array is read whole; name the file .jsonl to stream it a line at a time"
        ]);
        Ok(out)
    }
}

fn object_keys(value: &serde_json::Value) -> Option<Vec<String>> {
    value
        .as_object()
        .map(|map| map.keys().cloned().collect::<Vec<_>>())
}

/// `.jsonl`: one object per line, decoded as it is read.
/// Rows pulled from a file one at a time, so a 500 MB import never sits in memory.
type RowIter = Box<dyn Iterator<Item = Result<Vec<String>>>>;

struct JsonLines {
    reader: std::io::BufReader<std::fs::File>,
    columns: Vec<String>,
    path: std::path::PathBuf,
    line: usize,
    /// The line already consumed to learn the column names.
    pending: Option<Vec<String>>,
}

/// One JSON object as a row in `columns` order.
fn row_from(value: &serde_json::Value, columns: &[String]) -> Vec<String> {
    let map = value.as_object();
    columns
        .iter()
        .map(|c| match map.and_then(|m| m.get(c)) {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Null) | None => String::new(),
            Some(other) => other.to_string(),
        })
        .collect()
}

impl Iterator for JsonLines {
    type Item = Result<Vec<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        use std::io::BufRead;
        if let Some(row) = self.pending.take() {
            return Some(Ok(row));
        }
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => return None,
                Ok(_) => {}
                Err(e) => {
                    return Some(Err(Error::io(format!("read {}: {e}", self.path.display()))))
                }
            }
            self.line += 1;
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = match serde_json::from_str(line.trim()) {
                Ok(v) => v,
                Err(e) => {
                    return Some(Err(Error::config(format!(
                        "{} line {}: {e}",
                        self.path.display(),
                        self.line
                    ))))
                }
            };
            if value.as_object().is_none() {
                return Some(Err(Error::config(format!(
                    "{} line {}: every line must be an object",
                    self.path.display(),
                    self.line
                ))));
            }
            return Some(Ok(row_from(&value, &self.columns)));
        }
    }
}

fn import_sql(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = session.resolve_conn(p)?;
    let path = crate::cmd::grid::expand(&p.str_arg("path")?, &session.home);
    let script = std::fs::read_to_string(&path)
        .map_err(|e| Error::io(format!("read {}: {e}", path.display())))?;
    let statements = nook_drivers::split_script(&script);
    if statements.is_empty() {
        return Err(Error::config("the file has no statements"));
    }
    run_sql(session, index, &script, None)?;
    let _ = crate::cmd::refresh_catalog(session, index);
    session.log(
        Level::Success,
        format!(
            "ran {} statements from {}",
            statements.len(),
            path.display()
        ),
    );
    Ok(json!({
        "path": path.to_string_lossy(),
        "statements": statements.len(),
    }))
}

/// The dry run: the same reader, the same inference, no write transaction.
///
/// This exists so a person can see the mapping and the first rows before committing a 2 GB import
/// to the wrong table. It never opens a write path — the only database work is a catalog read.
fn import_wizard(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let path = crate::cmd::grid::expand(&p.str_arg("path")?, &session.home);
    let reference = TableRef::parse(&p.str_arg("table")?);
    let index = session.resolve_conn(p)?;
    let sample_limit = p.int_opt("sample", 500).clamp(1, 50_000) as usize;
    let preview_rows = p.int_opt("preview", 20).clamp(1, 200) as usize;
    let delimiter = p
        .str_opt("delimiter")
        .and_then(|d| d.chars().next())
        .unwrap_or(',');
    let header = p.bool_opt("header", true);
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();

    let (file_columns, samples, preview, encoding, notes) = match extension.as_str() {
        "xml" => {
            let file = std::fs::File::open(&path)
                .map_err(|e| Error::io(format!("open {}: {e}", path.display())))?;
            let (columns, rows) =
                crate::xml::rows(std::io::BufReader::new(file), p.str_opt("row").as_deref())?;
            let mut samples = Vec::new();
            let mut preview = Vec::new();
            for (i, row) in rows.into_iter().enumerate() {
                if i >= sample_limit + preview_rows {
                    break;
                }
                let row = row?;
                if i < preview_rows {
                    preview.push(row.clone());
                }
                samples.push(row);
            }
            (
                columns,
                samples,
                preview,
                "utf-8".to_string(),
                vec!["flat XML only: a nested element is refused by name".to_string()],
            )
        }
        "txt" | "text" | "fixed" => {
            let widths: Vec<usize> = p
                .get("widths")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_u64().map(|n| n as usize))
                        .collect()
                })
                .unwrap_or_default();
            if widths.is_empty() {
                return Err(Error::config(
                    "a fixed-width file needs \"widths\": [3, 12, 8]",
                ));
            }
            let rows: Vec<Vec<String>> = fixed_width_rows(&path, &widths)?
                .take(sample_limit + preview_rows)
                .filter_map(|r| r.ok())
                .collect();
            let columns: Vec<String> = if header {
                rows.first().cloned().unwrap_or_default()
            } else {
                (0..widths.len())
                    .map(|i| format!("column_{}", i + 1))
                    .collect()
            };
            let data: Vec<Vec<String>> = if header {
                rows.into_iter().skip(1).collect()
            } else {
                rows
            };
            let preview = data.iter().take(preview_rows).cloned().collect();
            (
                columns,
                data,
                preview,
                "utf-8".to_string(),
                vec![format!(
                    "split on the given widths: {}",
                    widths
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )],
            )
        }
        "dbf" => {
            let mut rows = crate::dbf::open(&path)?;
            let columns = rows.columns();
            let mut samples = Vec::new();
            let mut preview = Vec::new();
            for row in rows.by_ref().take(sample_limit + preview_rows) {
                let row = row?;
                if preview.len() < preview_rows {
                    preview.push(row.clone());
                }
                samples.push(row);
            }
            let mut notes = vec![
                "dBASE III/IV: the header declares the columns and their widths, so there is no \
                 header or delimiter to choose"
                    .to_string(),
            ];
            if rows.deleted() > 0 {
                notes.push(format!(
                    "{} deleted record(s) in the sample will be skipped",
                    rows.deleted()
                ));
            }
            (
                columns,
                samples,
                preview,
                "utf-8, latin-1 per field".to_string(),
                notes,
            )
        }
        _ => {
            let latin1 = p
                .str_opt("encoding")
                .map(|e| e.eq_ignore_ascii_case("latin1") || e.eq_ignore_ascii_case("latin-1"))
                .unwrap_or(false);
            let reader = crate::csv::open(&path, latin1)?;
            let mut rows = crate::csv::rows(reader, delimiter);
            let first = match rows.next() {
                Some(Ok(row)) => row,
                Some(Err(e)) => return Err(e),
                None => return Err(Error::config(format!("{} has no rows", path.display()))),
            };
            let columns: Vec<String> = if header {
                first.clone()
            } else {
                (0..first.len())
                    .map(|i| format!("column_{}", i + 1))
                    .collect()
            };
            let mut samples: Vec<Vec<String>> = Vec::new();
            if !header {
                samples.push(first);
            }
            let mut preview = Vec::new();
            for row in rows.take(sample_limit + preview_rows) {
                let row = row?;
                if samples.len() < preview_rows {
                    preview.push(row.clone());
                }
                samples.push(row);
            }
            let notes = vec![
                format!("delimiter: {:?}", delimiter),
                format!("header: {header}"),
            ];
            (
                columns,
                samples,
                preview,
                if latin1 { "latin1" } else { "utf-8" }.to_string(),
                notes,
            )
        }
    };

    if file_columns.is_empty() {
        return Err(Error::config(format!(
            "{} has no columns to map",
            path.display()
        )));
    }

    // What the target already has decides whether a mismatch is a note or a decision.
    let table = session
        .conns
        .get(index)
        .and_then(|c| c.catalog.as_ref())
        .and_then(|cat| cat.find_table(reference.schema.as_deref(), &reference.table))
        .cloned();
    let mut columns = crate::infer::report(&file_columns, &samples);
    let mut notes = notes;
    if let Some(table) = &table {
        for column in columns.iter_mut() {
            let name = column["source"].as_str().unwrap_or_default().to_string();
            match table.column(&name) {
                Some(existing) => {
                    column["target"] = json!(name);
                    column["target_type"] = json!(existing.type_name);
                    let inferred = column["inferred"].as_str().unwrap_or("text");
                    if !type_fits(inferred, &existing.type_name) {
                        notes.push(format!(
                            "column '{name}' infers as {inferred} but the table holds {}; the \
                             import coerces per the table's type",
                            existing.type_name
                        ));
                    }
                }
                None => {
                    column["target"] = serde_json::Value::Null;
                    notes.push(format!(
                        "column '{name}' is not in {}; it will be skipped unless you map it",
                        reference.key()
                    ));
                }
            }
        }
    } else {
        notes.push(format!(
            "{} does not exist yet; the import would create it with the inferred types",
            reference.key()
        ));
    }
    let always_empty: Vec<&str> = columns
        .iter()
        .filter(|c| c["always_empty"].as_bool().unwrap_or(false))
        .filter_map(|c| c["source"].as_str())
        .collect();
    if !always_empty.is_empty() {
        notes.push(format!(
            "empty in every sampled row: {}",
            always_empty.join(", ")
        ));
    }

    Ok(json!({
        "path": path.to_string_lossy(),
        "table": reference.key(),
        "delimiter": if extension.is_empty() || extension == "csv" { json!(delimiter.to_string()) } else { serde_json::Value::Null },
        "encoding": encoding,
        "header": header,
        "rows_sampled": samples.len(),
        "columns": columns,
        "target_exists": table.is_some(),
        "would_truncate": false,
        "would_create_table": table.is_none(),
        "preview": preview,
        "notes": notes,
        "writes": 0,
    }))
}

/// Whether an inferred type is compatible with the table's declared type, for the wizard's note.
fn type_fits(inferred: &str, declared: &str) -> bool {
    let declared = declared.to_ascii_lowercase();
    match inferred {
        "integer" => declared.contains("int"),
        "float" => {
            declared.contains("int")
                || declared.contains("real")
                || declared.contains("float")
                || declared.contains("double")
                || declared.contains("numeric")
                || declared.contains("decimal")
        }
        "boolean" => declared.contains("bool"),
        "date" | "timestamp" => {
            declared.contains("date")
                || declared.contains("time")
                || declared.contains("char")
                || declared.contains("text")
        }
        _ => true,
    }
}

/// Rows of a fixed-width file, split on `widths`. Short lines are padded rather than refused: a
/// trailing blank column is common, and a refused import over it helps nobody.
/// Split a fixed-width file into rows, one line at a time.
///
/// Lazy on purpose: `text_importer` used to collect the whole file first, which is the defect the
/// CSV path removed. A width is a count of characters, not bytes, so a non-ASCII name is never cut
/// mid-codepoint.
struct FixedWidth {
    lines: std::io::Lines<std::io::BufReader<std::fs::File>>,
    widths: Vec<usize>,
    path: std::path::PathBuf,
    done: bool,
}

impl Iterator for FixedWidth {
    type Item = Result<Vec<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.done {
                return None;
            }
            let line = match self.lines.next() {
                Some(Ok(line)) => line,
                Some(Err(e)) => {
                    self.done = true;
                    return Some(Err(Error::io(format!("read {}: {e}", self.path.display()))));
                }
                None => {
                    self.done = true;
                    return None;
                }
            };
            let chars: Vec<char> = line.chars().collect();
            let mut at = 0usize;
            let mut row = Vec::with_capacity(self.widths.len());
            for width in &self.widths {
                let end = (at + width).min(chars.len());
                row.push(chars[at..end].iter().collect::<String>().trim().to_string());
                at = end;
            }
            // A blank line is padding, not a row of empty strings.
            if row.iter().all(|c| c.is_empty()) {
                continue;
            }
            return Some(Ok(row));
        }
    }
}

fn fixed_width_rows(path: &std::path::Path, widths: &[usize]) -> Result<FixedWidth> {
    use std::io::BufRead;
    let file = std::fs::File::open(path)
        .map_err(|e| Error::io(format!("open {}: {e}", path.display())))?;
    Ok(FixedWidth {
        lines: std::io::BufReader::new(file).lines(),
        widths: widths.to_vec(),
        path: path.to_path_buf(),
        done: false,
    })
}

fn import_xml(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let path = crate::cmd::grid::expand(&p.str_arg("path")?, &session.home);
    let index = session.resolve_conn(p)?;
    let reference = TableRef::parse(&p.str_arg("table")?);
    let create_table = p.bool_opt("create_table", true);
    let file = std::fs::File::open(&path)
        .map_err(|e| Error::io(format!("open {}: {e}", path.display())))?;
    let (file_columns, rows) =
        crate::xml::rows(std::io::BufReader::new(file), p.str_opt("row").as_deref())?;
    let plan = prepare_import(session, index, &reference, file_columns, p, create_table)?;
    let mut out = import_stream(session, index, &reference, plan, rows.into_iter())?;
    out["path"] = json!(path.to_string_lossy());
    out["format"] = json!("xml");
    Ok(out)
}

/// dBASE III/IV. The header names the columns and their widths, so no `header` or `widths`
/// parameter: unlike a text file, the shape is declared in the file itself.
fn import_dbf(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let path = crate::cmd::grid::expand(&p.str_arg("path")?, &session.home);
    let index = session.resolve_conn(p)?;
    let reference = TableRef::parse(&p.str_arg("table")?);
    let create_table = p.bool_opt("create_table", true);
    // Opened before the plan: a Visual FoxPro table is refused here, so `truncate` cannot run on a
    // file that was never going to import.
    let mut rows = crate::dbf::open(&path)?;
    let file_columns = rows.columns();
    let plan = prepare_import(session, index, &reference, file_columns, p, create_table)?;
    let mut out = import_stream(session, index, &reference, plan, &mut rows)?;
    out["path"] = json!(path.to_string_lossy());
    out["format"] = json!("dbf");
    out["deleted"] = json!(rows.deleted());
    if rows.deleted() > 0 {
        out["notes"] = json!([format!(
            "{} record(s) carried the deletion flag and were skipped",
            rows.deleted()
        )]);
    }
    Ok(out)
}

fn text_importer(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let path = crate::cmd::grid::expand(&p.str_arg("path")?, &session.home);
    let index = session.resolve_conn(p)?;
    let reference = TableRef::parse(&p.str_arg("table")?);
    let create_table = p.bool_opt("create_table", true);
    let header = p.bool_opt("header", false);
    let widths: Vec<usize> = p
        .get("widths")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_u64().map(|n| n as usize))
                .collect()
        })
        .unwrap_or_default();
    if widths.is_empty() {
        return Err(Error::config(
            "pass \"widths\": [3, 12, 8] — the character count of each column",
        ));
    }
    // Lazy: the header is the first row, and the rest is pulled by the import loop.
    let mut rows = fixed_width_rows(&path, &widths)?;
    let first = match rows.next() {
        Some(Ok(row)) => row,
        Some(Err(e)) => return Err(e),
        None => return Err(Error::config(format!("{} has no rows", path.display()))),
    };
    let (file_columns, rest): (Vec<String>, RowIter) = if header {
        (first, Box::new(rows))
    } else {
        (
            (0..widths.len())
                .map(|i| format!("column_{}", i + 1))
                .collect(),
            Box::new(std::iter::once(Ok(first)).chain(rows)),
        )
    };
    let plan = prepare_import(session, index, &reference, file_columns, p, create_table)?;
    let mut out = import_stream(session, index, &reference, plan, rest)?;
    out["path"] = json!(path.to_string_lossy());
    out["format"] = json!("fixed-width");
    out["widths"] = json!(widths);
    Ok(out)
}

/// Page a table or a statement to a file without holding the result.
///
/// `csv` and `jsonl` stream. `json` is refused with the reason: an array needs a closing bracket,
/// and producing one incrementally means either seeking back or holding every row.
fn export_stream(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    use std::io::Write;
    let index = session.resolve_conn(p)?;
    let format = p.str_opt("format").unwrap_or_else(|| "csv".into());
    if format == "json" {
        return Err(Error::config(
            "json cannot be streamed: an array needs its closing bracket, which means holding every \
             row. Use \"format\": \"jsonl\" (one object per line) or drop stream:true",
        ));
    }
    if !matches!(format.as_str(), "csv" | "jsonl" | "ndjson") {
        return Err(Error::config(format!(
            "unknown stream format '{format}'; try csv or jsonl"
        )));
    }
    let page = p.int_opt("page", 5_000).clamp(1, 100_000) as usize;
    let max = p.int_opt("max_rows", 0).clamp(0, 1_000_000_000) as usize;
    let table = p.str_opt("table");
    let sql = p.str_opt("sql");
    if table.is_none() && sql.is_none() {
        return Err(Error::config("pass \"table\" or \"sql\""));
    }
    let mut notes: Vec<String> = Vec::new();
    let (schema, name) = match &table {
        Some(spec) => {
            let reference = TableRef::parse(spec);
            (reference.schema.clone(), reference.table.clone())
        }
        None => {
            let sql = sql.clone().unwrap_or_default();
            if sql.to_ascii_uppercase().contains(" LIMIT ") {
                notes.push(
                    "the statement has its own LIMIT; it is wrapped as a subquery and paged, so the \
                     LIMIT applies to the wrapped result, not to each page"
                        .to_string(),
                );
            }
            (None, format!("({sql}) AS nook_stream"))
        }
    };
    let path = crate::cmd::grid::expand(
        &p.str_opt("path").unwrap_or_else(|| {
            format!(
                "exports/stream-{}.{}",
                crate::jobs::now(),
                if format == "csv" { "csv" } else { "jsonl" }
            )
        }),
        &session.home,
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::io(format!("create {}: {e}", parent.display())))?;
    }
    let file = std::fs::File::create(&path)
        .map_err(|e| Error::io(format!("write {}: {e}", path.display())))?;
    let mut writer = std::io::BufWriter::with_capacity(256 * 1024, file);

    let mut offset = 0usize;
    let mut total = 0usize;
    let mut pages = 0usize;
    let mut columns: Vec<String> = Vec::new();
    loop {
        let limit = if max == 0 {
            page
        } else {
            page.min(max.saturating_sub(total))
        };
        if limit == 0 {
            break;
        }
        let rs = {
            let driver = session.driver_for(index)?;
            if table.is_some() {
                driver.fetch_table(schema.as_deref(), &name, limit, offset, None, None)?
            } else {
                // Wrapping the statement is the only way to page it without rewriting it. A
                // statement that cannot be wrapped (a DDL, a procedure call) fails here with the
                // server's own message, which is the honest outcome.
                let sql = format!("SELECT * FROM {name} LIMIT {limit} OFFSET {offset}");
                driver.run(&sql, limit)?
            }
        };
        if rs.columns.is_empty() || rs.rows.is_empty() {
            // An empty page is the end. Testing `rows < limit` alone costs one extra round trip
            // and makes `pages` report one more than it wrote.
            break;
        }
        if columns.is_empty() {
            columns = rs.columns.clone();
            if format == "csv" {
                let header = columns
                    .iter()
                    .map(|c| csv_cell(c))
                    .collect::<Vec<_>>()
                    .join(",");
                writeln!(writer, "{header}")
                    .map_err(|e| Error::io(format!("write {}: {e}", path.display())))?;
            }
        }
        let rows = rs.rows.len();
        for row in &rs.rows {
            if format == "csv" {
                let cells = (0..columns.len())
                    .map(|i| csv_cell(&row.get(i).cloned().unwrap_or(Value::Null).as_text()))
                    .collect::<Vec<_>>()
                    .join(",");
                writeln!(writer, "{cells}")
                    .map_err(|e| Error::io(format!("write {}: {e}", path.display())))?;
            } else {
                let mut object = serde_json::Map::new();
                for (i, column) in columns.iter().enumerate() {
                    object.insert(
                        column.clone(),
                        row.get(i).cloned().unwrap_or(Value::Null).to_json(),
                    );
                }
                let line = serde_json::to_string(&serde_json::Value::Object(object))
                    .map_err(|e| Error::internal(format!("json: {e}")))?;
                writeln!(writer, "{line}")
                    .map_err(|e| Error::io(format!("write {}: {e}", path.display())))?;
            }
        }
        // Flush per page: a crash mid-export leaves whole pages, and memory stays at one page.
        writer
            .flush()
            .map_err(|e| Error::io(format!("flush {}: {e}", path.display())))?;
        total += rows;
        pages += 1;
        offset += limit;
        if rows < limit {
            break;
        }
    }
    drop(writer);
    let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    session.log(
        Level::Success,
        format!("streamed {total} rows to {}", path.display()),
    );
    Ok(json!({
        "path": path.to_string_lossy(),
        "rows": total,
        "pages": pages,
        "page": page,
        "bytes": bytes,
        "format": if format == "csv" { "csv" } else { "jsonl" },
        "columns": columns,
        "notes": notes,
    }))
}

/// A CSV cell, quoted only when it has to be.
fn csv_cell(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

fn export_query(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    if p.bool_opt("stream", false) {
        // The streaming path lives in export.stream so there is one page loop, not two.
        let sql = p
            .str_opt("sql")
            .ok_or_else(|| Error::config("pass \"sql\""))?;
        let mut params = p.raw().clone();
        params.insert("sql".into(), json!(sql));
        params.remove("table");
        let streamed = export_stream(session, &Params(params))?;
        return Ok(streamed);
    }
    let index = session.resolve_conn(p)?;
    let sql = p
        .str_opt("sql")
        .ok_or_else(|| Error::config("pass \"sql\""))?;
    let format = p.str_opt("format").unwrap_or_else(|| "csv".into());
    let result = run_sql(session, index, &sql, None)?;
    let text = match format.as_str() {
        "csv" => result.to_csv(),
        "json" => serde_json::to_string_pretty(&result.to_json())
            .map_err(|e| Error::internal(format!("json: {e}")))?,
        "markdown" | "md" => result.to_markdown(),
        "sql" => {
            let reference =
                crate::TableRef::parse(&p.str_opt("table").unwrap_or_else(|| "export".into()));
            let dialect = session
                .conns
                .get(index)
                .and_then(|c| c.driver.as_ref())
                .map(|d| d.dialect())
                .unwrap_or(Dialect::Sqlite);
            result.to_sql_inserts(reference.schema.as_deref(), &reference.table, dialect)
        }
        other => {
            return Err(Error::config(format!(
                "unknown format '{other}'; try csv, json, sql or markdown"
            )))
        }
    };
    let path = crate::cmd::grid::expand(
        &p.str_opt("path").unwrap_or_else(|| {
            format!(
                "exports/query-{}.{}",
                crate::jobs::now(),
                if format == "json" { "json" } else { "csv" }
            )
        }),
        &session.home,
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::io(format!("create {}: {e}", parent.display())))?;
    }
    std::fs::write(&path, &text)
        .map_err(|e| Error::io(format!("write {}: {e}", path.display())))?;
    session.log(Level::Success, format!("exported to {}", path.display()));
    Ok(json!({
        "path": path.to_string_lossy(),
        "rows": result.row_count(),
        "bytes": text.len(),
        "format": format,
    }))
}

/// Visual query builder: tables, joins, columns, filters, grouping and ordering into one SELECT.
fn query_builder(_session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let from = p
        .get("from")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::config("pass \"from\": [{table, columns, join}]"))?;
    if from.is_empty() {
        return Err(Error::config("at least one table is required"));
    }
    let dialect = p
        .str_opt("dialect")
        .as_deref()
        .and_then(crate::cmd::parse_dialect)
        .unwrap_or(Dialect::Postgres);

    // First pass: read every entry, qualify the tables and collect the columns.
    struct Entry {
        target: String,
        prefix: String,
        columns: Vec<String>,
        join: Option<(String, String)>,
    }
    let mut entries: Vec<Entry> = Vec::new();
    for raw in from {
        let params = Params::from_json(raw.clone())?;
        let name = params
            .str_opt("table")
            .ok_or_else(|| Error::config("every \"from\" entry needs a table"))?;
        let reference = TableRef::parse(&name);
        let qualified = quote_qualified(reference.schema.as_deref(), &reference.table, dialect);
        let alias = params.str_opt("alias");
        let target = match &alias {
            Some(a) => format!("{qualified} AS {}", quote_ident(a, dialect)),
            None => qualified,
        };
        let prefix = alias.unwrap_or_else(|| reference.table.clone());
        let join = match params.str_opt("on") {
            Some(on) => {
                let mut kind = params
                    .str_opt("join")
                    .unwrap_or_else(|| "inner".into())
                    .to_ascii_uppercase();
                if !kind.ends_with("JOIN") {
                    kind.push_str(" JOIN");
                }
                Some((kind, on))
            }
            None => None,
        };
        entries.push(Entry {
            target,
            prefix,
            columns: params.list("columns"),
            join,
        });
    }
    if entries.first().map(|e| e.join.is_some()).unwrap_or(false) {
        return Err(Error::config(
            "the first table has no join condition; only later entries take \"on\"",
        ));
    }

    // Second pass: the select list needs every table's columns, so it is built after the loop.
    let mut selects: Vec<String> = Vec::new();
    for entry in &entries {
        if entry.columns.is_empty() {
            selects.push(format!("{}.*", quote_ident(&entry.prefix, dialect)));
        } else {
            for column in &entry.columns {
                selects.push(format!(
                    "{}.{}",
                    quote_ident(&entry.prefix, dialect),
                    quote_ident(column, dialect)
                ));
            }
        }
    }

    let mut sql = format!("SELECT {} FROM {}", selects.join(", "), entries[0].target);
    for entry in entries.iter().skip(1) {
        let (kind, on) = entry.join.clone().ok_or_else(|| {
            Error::config(format!("the join onto '{}' needs \"on\"", entry.prefix))
        })?;
        sql.push_str(&format!(" {kind} {} ON {on}", entry.target));
    }
    if let Some(clause) = p.str_opt("where") {
        if !clause.trim().is_empty() {
            sql.push_str(&format!(" WHERE {}", clause.trim()));
        }
    }
    if let Some(group) = p.str_opt("group_by") {
        if !group.trim().is_empty() {
            sql.push_str(&format!(" GROUP BY {}", group.trim()));
        }
    }
    if let Some(having) = p.str_opt("having") {
        if !having.trim().is_empty() {
            sql.push_str(&format!(" HAVING {}", having.trim()));
        }
    }
    if let Some(order) = p.str_opt("order_by") {
        if !order.trim().is_empty() {
            sql.push_str(&format!(" ORDER BY {}", order.trim()));
        }
    }
    let limit = p.int_opt("limit", 100).clamp(1, 1_000_000);
    sql.push_str(&format!(" LIMIT {limit};"));
    Ok(json!({ "sql": sql, "columns": selects.len() }))
}

fn chart(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = session.resolve_conn(p)?;
    let kind = p
        .str_opt("chart")
        .as_deref()
        .and_then(ChartKind::parse)
        .unwrap_or(ChartKind::Table);
    let title = p.str_opt("title").unwrap_or_default();
    let label = p.str_opt("label");
    let sql = p
        .str_opt("sql")
        .or_else(|| {
            let reference = p.str_opt("table")?;
            let table_ref = TableRef::parse(&reference);
            let table = session
                .table_of(index, table_ref.schema.as_deref(), &table_ref.table)
                .ok()?;
            let group = p.str_opt("group")?;
            let dialect = session.conns.get(index)?.driver.as_ref()?.dialect();
            report::aggregate_sql(
                &table,
                &group,
                &p.str_opt("agg").unwrap_or_else(|| "count".into()),
                p.str_opt("value").as_deref(),
                dialect,
                p.int_opt("limit", 50).clamp(1, 10_000) as usize,
            )
            .ok()
        })
        .ok_or_else(|| Error::config("pass \"sql\", or \"table\" with \"group\""))?;
    let result = run_sql(session, index, &sql, None)?;
    if kind == ChartKind::Table {
        return Ok(json!({
            "sql": sql,
            "chart": "table",
            "result": result.to_json(),
        }));
    }
    let labeled = report::chart_from_result(&result, kind, &title, label.as_deref())?;
    // An aggregate report is more readable with the label column named.
    let labeled = if labeled.labels.is_empty() && result.column_count() >= 2 {
        report::chart_from_result(
            &result,
            kind,
            &title,
            result.columns.first().map(String::as_str),
        )?
    } else {
        labeled
    };
    Ok(json!({
        "sql": sql,
        "chart": labeled.kind.label(),
        "title": labeled.title,
        "labels": labeled.labels,
        "series": labeled.series.iter().map(|s| json!({ "name": s.name, "values": s.values })).collect::<Vec<_>>(),
        "summary": labeled.summary(),
        "rows": result.row_count(),
    }))
}

fn aggregate(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = session.resolve_conn(p)?;
    let reference = TableRef::parse(&p.str_arg("table")?);
    let table = session.table_of(index, reference.schema.as_deref(), &reference.table)?;
    let group = p.str_arg("group")?;
    let agg = p.str_opt("agg").unwrap_or_else(|| "count".into());
    let dialect = session
        .conns
        .get(index)
        .and_then(|c| c.driver.as_ref())
        .map(|d| d.dialect())
        .ok_or_else(|| Error::config("the connection is not open"))?;
    let sql = report::aggregate_sql(
        &table,
        &group,
        &agg,
        p.str_opt("value").as_deref(),
        dialect,
        p.int_opt("limit", 50).clamp(1, 10_000) as usize,
    )?;
    let result = run_sql(session, index, &sql, None)?;
    let kind = p
        .str_opt("chart")
        .as_deref()
        .and_then(ChartKind::parse)
        .unwrap_or(ChartKind::Column);
    let chart =
        report::chart_from_result(&result, kind, format!("{agg} by {group}"), Some("label"))?;
    Ok(json!({
        "sql": sql,
        "rows": result.row_count(),
        "data": result.to_json()["rows"],
        "chart": chart.kind.label(),
        "labels": chart.labels,
        "series": chart.series.iter().map(|s| json!({ "name": s.name, "values": s.values })).collect::<Vec<_>>(),
    }))
}

fn pivot(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = session.resolve_conn(p)?;
    let sql = p.str_arg("sql")?;
    let result = run_sql(session, index, &sql, None)?;
    let (rows, columns, cells) = report::pivot(&result)?;
    Ok(json!({
        "rows": rows,
        "columns": columns,
        "cells": cells,
        "source_rows": result.row_count(),
    }))
}

// --- Saved reports and boards --------------------------------------------------

fn report_save(_session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let name = p.str_arg("name")?;
    let sql = p.str_arg("sql")?;
    let chart = p.str_opt("chart").unwrap_or_else(|| "table".into());
    // A chart kind that does not exist is refused here rather than stored and discovered when the
    // board paints: a saved report that cannot render is a saved mistake.
    if ChartKind::parse(&chart).is_none() {
        return Err(Error::config(format!(
            "unknown chart {chart:?}; use {}",
            ChartKind::all()
                .iter()
                .map(|k| k.label())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    let label = p.str_opt("label").unwrap_or_default();
    let options = p.get("options").cloned().unwrap_or_else(|| json!({}));
    crate::store::save_report(&name, &sql, &chart, &label, &options)?;
    Ok(json!({
        "name": name,
        "saved": true,
        "report": crate::store::load_report(&name)?,
    }))
}

fn report_list(_session: &mut Session, _p: &Params) -> Result<serde_json::Value> {
    let reports = crate::store::load_reports()?;
    Ok(json!({ "reports": reports, "count": reports.len() }))
}

fn report_delete(_session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let name = p.str_arg("name")?;
    crate::store::delete_report(&name)?;
    Ok(json!({ "name": name, "deleted": true }))
}

fn report_designer(_session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let name = p.str_arg("name")?;
    let existing = crate::store::load_report(&name)?;
    // The designer edits what is stored: a name it has never seen starts blank rather than failing,
    // because "new report" and "edit report" are the same surface.
    let exists = existing.is_some();
    let mut report = existing.unwrap_or_else(
        || json!({ "name": name, "sql": "", "chart": "table", "label": "", "options": {} }),
    );
    if let Some(sql) = p.str_opt("sql") {
        report["sql"] = json!(sql);
    }
    if let Some(chart) = p.str_opt("chart") {
        report["chart"] = json!(chart);
    }
    if let Some(label) = p.str_opt("label") {
        report["label"] = json!(label);
    }
    if let Some(layout) = p.get("layout") {
        let mut options = report["options"].clone();
        options["layout"] = layout.clone();
        report["options"] = options;
    }
    // A name on its own is a read: the designer opens a report it has not seen as a blank draft. Any
    // write parameter means the caller is editing, and the help promises the edit is stored — so it
    // is, or the command is a draft surface pretending to be a save surface.
    let writing = p.get("sql").is_some()
        || p.get("chart").is_some()
        || p.get("label").is_some()
        || p.get("layout").is_some();
    if writing {
        let chart = report["chart"].as_str().unwrap_or("table").to_string();
        // Refused before the write, not after: a stored report with an unknown chart kind is a board
        // that fails to paint on every reload.
        if ChartKind::parse(&chart).is_none() {
            return Err(Error::config(format!(
                "unknown chart {chart:?}; use one of {}",
                ChartKind::all()
                    .iter()
                    .map(|k| k.label())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        crate::store::save_report(
            &name,
            report["sql"].as_str().unwrap_or_default(),
            &chart,
            report["label"].as_str().unwrap_or_default(),
            &report["options"],
        )?;
    }
    Ok(json!({ "report": report, "exists": exists, "saved": writing }))
}

fn report_dashboard(_session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let name = p.str_arg("name")?;
    let tiles = p.get("tiles").cloned().unwrap_or_else(|| json!([]));
    let Some(list) = tiles.as_array() else {
        return Err(Error::config("\"tiles\" must be an array"));
    };
    // Every tile has to name a saved report: a board that points at nothing is a blank rectangle
    // discovered on a Monday.
    let saved: Vec<String> = crate::store::load_reports()?
        .iter()
        .filter_map(|r| r["name"].as_str().map(str::to_string))
        .collect();
    for tile in list {
        let Some(report) = tile.get("report").and_then(|v| v.as_str()) else {
            return Err(Error::config(
                "every tile needs \"report\": the name of a saved report",
            ));
        };
        if !saved.iter().any(|s| s == report) {
            return Err(Error::config(format!(
                "no saved report named {report:?}; save one with report.save first"
            )));
        }
    }
    crate::store::save_dashboard(&name, &tiles)?;
    Ok(json!({
        "name": name,
        "saved": true,
        "tiles": list.len(),
        "board": crate::store::load_dashboard(&name)?,
    }))
}

/// Run every tile of a board and return each one charted.
///
/// A tile whose statement fails is reported in `errors` and the rest of the board still paints: one
/// broken panel is not a reason to hide the other five.
fn report_board(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let name = p.str_arg("name")?;
    let board = crate::store::load_dashboard(&name)?
        .ok_or_else(|| Error::not_found(format!("no board named {name:?}")))?;
    let index = session.resolve_conn(p)?;
    // The board's input fields bind parameters in every tile's statement.
    let mut bindings: Vec<(String, String)> = Vec::new();
    for (key, value) in p.raw() {
        if let Some(field) = key.strip_prefix("input.") {
            bindings.push((
                field.to_string(),
                value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| value.to_string()),
            ));
        }
    }
    let mut tiles: Vec<serde_json::Value> = Vec::new();
    let mut errors: Vec<serde_json::Value> = Vec::new();
    for tile in board["tiles"].as_array().cloned().unwrap_or_default() {
        let report_name = tile["report"].as_str().unwrap_or_default();
        let Some(report) = crate::store::load_report(report_name)? else {
            errors.push(json!({ "report": report_name, "error": "the report is no longer saved" }));
            continue;
        };
        let sql = report["sql"].as_str().unwrap_or_default().to_string();
        let kind = report["chart"]
            .as_str()
            .and_then(ChartKind::parse)
            .unwrap_or(ChartKind::Table);
        let label = report["label"].as_str().filter(|s| !s.is_empty());
        match run_tile_sql(session, index, &sql, &bindings) {
            Ok(result) if kind == ChartKind::Table => tiles.push(json!({
                "report": report_name,
                "chart": "table",
                "layout": tile,
                "result": result.to_json(),
            })),
            Ok(result) => match report::chart_from_result(&result, kind, report_name, label) {
                Ok(charted) => tiles.push(json!({
                    "report": report_name,
                    "chart": charted.kind.label(),
                    "layout": tile,
                    "labels": charted.labels,
                    "series": charted.series.iter()
                        .map(|s| json!({ "name": s.name, "values": s.values }))
                        .collect::<Vec<_>>(),
                    "summary": charted.summary(),
                    "rows": result.row_count(),
                })),
                Err(e) => errors.push(json!({ "report": report_name, "error": e.to_string() })),
            },
            Err(e) => errors.push(json!({ "report": report_name, "error": e.to_string() })),
        }
    }
    Ok(json!({
        "board": name,
        "tiles": tiles,
        "errors": errors,
        "tile_count": tiles.len(),
        "error_count": errors.len(),
        // The fields the view paints: every `:name` the board's tiles read, with the value this call
        // was given. Derived from the tiles rather than stored, so a tile that stops using an input
        // stops asking for it.
        "inputs": inputs_of(&board, &bindings)?,
    }))
}

/// The named inputs a board's tiles read, with the values this call supplied.
fn inputs_of(
    board: &serde_json::Value,
    bindings: &[(String, String)],
) -> Result<Vec<serde_json::Value>> {
    let mut names: Vec<String> = Vec::new();
    for tile in board["tiles"].as_array().cloned().unwrap_or_default() {
        let report_name = tile["report"].as_str().unwrap_or_default();
        let Some(report) = crate::store::load_report(report_name)? else {
            continue;
        };
        let sql = report["sql"].as_str().unwrap_or_default();
        for (_, slots) in [crate::sqltext::bind_placeholders(
            sql,
            crate::sqltext::BindStyle::Question,
        )] {
            for slot in slots.into_iter().flatten() {
                if !names.contains(&slot) {
                    names.push(slot);
                }
            }
        }
    }
    names.sort();
    Ok(names
        .into_iter()
        .map(|name| {
            let value = bindings
                .iter()
                .find(|(field, _)| *field == name)
                .map(|(_, value)| value.clone())
                .unwrap_or_default();
            json!({ "name": name, "value": value })
        })
        .collect())
}

/// Run one tile's statement with the board's input fields bound by the server.
///
/// A `:field` the board supplied is a bound parameter, not a substituted literal: a literal breaks
/// `limit :n`, and on MySQL a backslash in the value can end the quoted string. A `:name` the board
/// did not supply is left to the driver, which refuses it and names the parameter — the same
/// contract `sql.run` has.
fn run_tile_sql(
    session: &mut Session,
    index: usize,
    sql: &str,
    bindings: &[(String, String)],
) -> Result<nook_model::ResultSet> {
    if bindings.is_empty() {
        return run_sql(session, index, sql, None);
    }
    let style = match session.driver_for(index)?.dialect() {
        Dialect::Postgres => crate::sqltext::BindStyle::Dollar,
        _ => crate::sqltext::BindStyle::Question,
    };
    let (prepared, slots) = crate::sqltext::bind_placeholders(sql, style);
    let mut values: Vec<Value> = Vec::with_capacity(slots.len());
    for (i, slot) in slots.iter().enumerate() {
        let value = match slot {
            Some(name) => bindings
                .iter()
                .find(|(field, _)| field == name)
                .map(|(_, value)| value.clone()),
            // A `?` slot is filled positionally when the board supplied exactly that many inputs.
            None => bindings.get(i).map(|(_, value)| value.clone()),
        };
        match value {
            Some(value) => values.push(Value::Text(value)),
            None => {
                return Err(Error::config(match slot {
                    Some(name) => format!("the board has no input named {name:?}"),
                    None => "the tile has more positional slots than the board has inputs".into(),
                }))
            }
        }
    }
    crate::cmd::run_prepared_sql(session, index, &prepared, &values, None)
}

fn report_export(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let name = p.str_arg("name")?;
    let format = p.str_opt("format").unwrap_or_else(|| "html".into());
    let path = p.str_arg("path")?;
    let board = crate::store::load_dashboard(&name)?;
    let index = session.resolve_conn(p)?;
    // A section is the result *and* the chart its report asked for, so an HTML export can draw the
    // chart rather than only its numbers.
    let mut sections: Vec<(String, nook_model::ResultSet, Option<report::Chart>)> = Vec::new();
    // A tile left out of the file is named in the result: an export that quietly drops a tile
    // because its query needs an input is a lie about what the board holds.
    // The board's input fields, so an export can paint the tiles that need one — the same
    // `input.<name>` keys `report.board` takes.
    let mut bindings: Vec<(String, String)> = Vec::new();
    for (key, value) in p.raw() {
        if let Some(field) = key.strip_prefix("input.") {
            bindings.push((
                field.to_string(),
                value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| value.to_string()),
            ));
        }
    }
    let mut skipped: Vec<serde_json::Value> = Vec::new();
    let mut collect = |report_name: &str, report: &serde_json::Value| -> Result<()> {
        let sql = report["sql"].as_str().unwrap_or_default();
        let result = match run_tile_sql(session, index, sql, &bindings) {
            Ok(result) => result,
            Err(e) => {
                skipped.push(json!({ "title": report_name, "reason": e.to_string() }));
                return Ok(());
            }
        };
        let kind = report["chart"]
            .as_str()
            .and_then(ChartKind::parse)
            .unwrap_or(ChartKind::Table);
        let label = report["label"].as_str().filter(|s| !s.is_empty());
        let chart = match kind {
            ChartKind::Table => None,
            _ => report::chart_from_result(&result, kind, report_name, label).ok(),
        };
        sections.push((report_name.to_string(), result, chart));
        Ok(())
    };
    if let Some(board) = &board {
        for tile in board["tiles"].as_array().cloned().unwrap_or_default() {
            let report_name = tile["report"].as_str().unwrap_or_default();
            let Some(report) = crate::store::load_report(report_name)? else {
                continue;
            };
            // A tile that fails is left out of the export rather than failing the whole file: the
            // board itself paints the rest, and an export should match the board.
            let _ = collect(report_name, &report);
        }
    } else {
        let report = crate::store::load_report(&name)?
            .ok_or_else(|| Error::not_found(format!("no report or board named {name:?}")))?;
        collect(&name, &report)?;
    }
    let written = match format.as_str() {
        "csv" => {
            let text = sections
                .iter()
                .map(|(title, result, _)| format!("# {title}\n{}", result.to_tsv(None, true)))
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(&path, text).map_err(|e| Error::io(format!("write {path}: {e}")))?;
            path.clone()
        }
        "html" => {
            let mut body = String::new();
            for (title, result, chart) in &sections {
                body.push_str(&format!("<h2>{}</h2>\n", html_escape(title)));
                // The chart first, then its numbers: a reader who wants the shape gets it, and one
                // who wants the figures scrolls one block further.
                if let Some(chart) = chart {
                    body.push_str(&report::chart_svg(chart));
                    body.push('\n');
                }
                body.push_str("<table>\n");
                body.push_str("<tr>");
                for column in &result.columns {
                    body.push_str(&format!("<th>{}</th>", html_escape(column)));
                }
                body.push_str("</tr>\n");
                for row in &result.rows {
                    body.push_str("<tr>");
                    for cell in row {
                        body.push_str(&format!("<td>{}</td>", html_escape(&cell.display())));
                    }
                    body.push_str("</tr>\n");
                }
                body.push_str("</table>\n");
            }
            let html = format!(
                "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>{}</title>\n\
                 <style>body{{font:14px system-ui;margin:2rem;color:#111}}\n\
                 table{{border-collapse:collapse;margin-bottom:2rem}}\n\
                 th,td{{border:1px solid #ccc;padding:4px 8px;text-align:left}}\n\
                 th{{background:#f4f4f4}}</style></head>\n<body>\n<h1>{}</h1>\n{}</body></html>\n",
                html_escape(&name),
                html_escape(&name),
                body
            );
            std::fs::write(&path, html).map_err(|e| Error::io(format!("write {path}: {e}")))?;
            path.clone()
        }
        other => {
            return Err(Error::config(format!(
                "unknown format {other:?}; use html or csv"
            )))
        }
    };
    Ok(json!({
        "path": written,
        "format": format,
        "sections": sections
            .iter()
            .map(|(t, r, chart)| json!({
                "title": t,
                "rows": r.row_count(),
                "chart": chart.as_ref().map(|c| c.kind.label()),
            }))
            .collect::<Vec<_>>(),
        "skipped": skipped,
        "summary": if skipped.is_empty() {
            format!("wrote {written}")
        } else {
            format!(
                "wrote {written}; {} of {} sections left out (see \"skipped\")",
                skipped.len(),
                skipped.len() + sections.len()
            )
        },
    }))
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn metrics_charts(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let board = p
        .str_opt("board")
        .or_else(|| p.str_opt("name"))
        .ok_or_else(|| Error::config("pass \"board\": the name of a saved board"))?;
    let every_s = p.int_opt("every_s", 30).clamp(1, 86_400) as u64;
    let mut params = p.raw().clone();
    params.insert("name".into(), json!(board));
    let painted = report_board(session, &Params::from_json(json!(params))?)?;
    Ok(json!({
        "board": board,
        "every_s": every_s,
        "refresh_at": crate::jobs::now() + every_s,
        "tiles": painted["tiles"],
        "errors": painted["errors"],
        "tile_count": painted["tile_count"],
    }))
}

/// Per-column profile: null share, distinct count, min/max, the five commonest values, and a guess at
/// what the column holds.
///
/// One aggregate query per table rather than one per column, because a table with forty columns
/// should cost one round trip, not forty.
fn data_profiling(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = session.resolve_conn(p)?;
    let reference = p.str_arg("table")?;
    let table_ref = TableRef::parse(&reference);
    let table = session.table_of(index, table_ref.schema.as_deref(), &table_ref.table)?;
    let dialect = session
        .conns
        .get(index)
        .and_then(|c| c.driver.as_ref())
        .map(|d| d.dialect())
        .unwrap_or(nook_core::Dialect::Sqlite);
    let wanted: Vec<String> = match p.get("columns").and_then(|v| v.as_array()) {
        Some(list) => list
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        None => table.columns.iter().map(|c| c.name.clone()).collect(),
    };
    if wanted.is_empty() {
        return Err(Error::config(
            "pass \"columns\": the names to profile, or leave it out to profile every column",
        ));
    }
    // A column that is not in the table is a refusal, not a silently empty result: a typo that
    // profiles nothing looks exactly like a column that is entirely NULL.
    let known: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
    for column in &wanted {
        if !known.contains(&column.as_str()) {
            return Err(Error::config(format!(
                "no column {column:?} in {}; it has {}",
                table.name,
                known.join(", ")
            )));
        }
    }
    let qualified = nook_core::quote_qualified(
        table_ref.schema.as_deref().or(table.schema.as_deref()),
        &table.name,
        dialect,
    );
    let total = {
        let count = run_sql(
            session,
            index,
            &format!("SELECT COUNT(*) AS n FROM {qualified}"),
            Some(1),
        )?;
        count
            .rows
            .first()
            .and_then(|r| r.first())
            .map(|v| v.display().parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0)
    };
    let mut profiles: Vec<serde_json::Value> = Vec::new();
    for column in &wanted {
        let ident = nook_core::quote_ident(column, dialect);
        let aggregate = format!(
            "SELECT COUNT(*) AS total, COUNT({ident}) AS present, \
                    COUNT(DISTINCT {ident}) AS distinct_count, \
                    MIN({ident}) AS min_value, MAX({ident}) AS max_value \
             FROM {qualified}"
        );
        let stats = run_sql(session, index, &aggregate, Some(1))?;
        let row = stats.rows.first().cloned().unwrap_or_default();
        let present = row
            .get(1)
            .map(|v| v.display().parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);
        let distinct = row
            .get(2)
            .map(|v| v.display().parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);
        let min = row
            .get(3)
            .map(|v| v.display())
            .unwrap_or_else(|| "NULL".into());
        let max = row
            .get(4)
            .map(|v| v.display())
            .unwrap_or_else(|| "NULL".into());
        // The commonest five: ordered by count, so the answer is what the column mostly holds.
        let common = format!(
            "SELECT {ident} AS value, COUNT(*) AS n FROM {qualified} \
             GROUP BY {ident} ORDER BY n DESC"
        );
        let commonest = run_sql(session, index, &common, Some(5))?;
        let values: Vec<serde_json::Value> = commonest
            .rows
            .iter()
            .map(|r| {
                json!({
                    "value": r.first().map(|v| v.display()).unwrap_or_default(),
                    "count": r.get(1)
                        .map(|v| v.display().parse::<u64>().unwrap_or(0))
                        .unwrap_or(0),
                })
            })
            .collect();
        let sample: Vec<String> = values
            .iter()
            .filter_map(|v| v["value"].as_str().map(str::to_string))
            .filter(|v| v != "NULL")
            .collect();
        profiles.push(json!({
            "column": column,
            "null_share": if total > 0.0 { (total - present) / total } else { 0.0 },
            "distinct": distinct as u64,
            "min": min,
            "max": max,
            "commonest": values,
            "shape": guess_shape(&sample),
        }));
    }
    Ok(json!({
        "table": table.name,
        "schema": table.schema,
        "rows": total as u64,
        "columns": profiles,
        "count": profiles.len(),
    }))
}

/// A guess at what a column holds, from a handful of real values. It is a guess and the key says so:
/// `shape` is a hint for a reader, never something to build a constraint on.
fn guess_shape(values: &[String]) -> &'static str {
    if values.is_empty() {
        return "empty";
    }
    let looks = |f: fn(&str) -> bool| values.iter().all(|v| f(v));
    if looks(|v| {
        let (name, domain) = match v.split_once('@') {
            Some(parts) => parts,
            None => return false,
        };
        !name.is_empty() && domain.contains('.') && !v.contains(' ')
    }) {
        return "email";
    }
    if looks(|v| {
        v.len() == 36
            && v.chars().enumerate().all(|(i, c)| match i {
                8 | 13 | 18 | 23 => c == '-',
                _ => c.is_ascii_hexdigit(),
            })
    }) {
        return "uuid";
    }
    if looks(|v| {
        let digits: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
        let others: String = v.chars().filter(|c| !c.is_ascii_digit()).collect();
        digits.len() >= 7
            && digits.len() <= 15
            && others
                .chars()
                .all(|c| matches!(c, '+' | '-' | ' ' | '(' | ')' | '.'))
    }) {
        return "phone";
    }
    if looks(|v| {
        v.len() >= 8
            && v.chars().filter(|c| c.is_ascii_digit()).count() >= 6
            && matches!(v.as_bytes().get(4), Some(b'-') | Some(b'/'))
    }) {
        return "date";
    }
    "text"
}

fn metrics_input(_session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let name = p.str_arg("name")?;
    let value = p
        .get("value")
        .cloned()
        .ok_or_else(|| Error::config("pass \"value\": what the field holds"))?;
    Ok(json!({
        "name": name,
        "value": value,
        "binding": format!("input.{name}"),
        "note": "pass it back as \"input.{name}\" to report.board or monitor.charts and every tile's SQL reads it as :{name}",
    }))
}

/// Helper for the UI: what a generator spec is, for the override editor's placeholder text.
pub fn generator_hint(_generator: &Generator) -> &'static str {
    "auto|int:1-100|pick:a|b|pattern:INV-####|email|sentence|uuid|date:365|null"
}

// --- Open Anything (Cmd+P) ----------------------------------------------------

/// One searchable thing: a table or view, a saved query, or an open tab.
#[derive(Debug, Clone, PartialEq)]
struct IndexEntry {
    kind: &'static str,
    conn: String,
    schema: Option<String>,
    name: String,
    /// `schema.name` plus the table's column names, lowercased — one pass at build time so the
    /// scorer only does substring/`starts_with` checks.
    haystack: String,
}

/// The index, built per query from what is already in memory. No cache, no invalidation: a cache
/// needs a hook in every `conn.refresh`, and this costs one pass over parsed catalogs.
/// ponytail: rebuilt per query; move to a cached index when a user's catalog is big enough to
/// notice, and add the invalidation hook then.
fn build_index(catalogs: &[(&str, &nook_model::Catalog)]) -> Vec<IndexEntry> {
    let mut out = Vec::new();
    for (conn, catalog) in catalogs {
        for schema in &catalog.schemas {
            for t in &schema.tables {
                let columns: Vec<&str> = t.columns.iter().map(|c| c.name.as_str()).collect();
                out.push(IndexEntry {
                    kind: match t.kind {
                        nook_model::TableKind::View => "view",
                        _ => "table",
                    },
                    conn: (*conn).to_string(),
                    schema: Some(schema.name.clone()),
                    name: t.name.clone(),
                    haystack: format!("{}.{} {}", schema.name, t.name, columns.join(" "))
                        .to_ascii_lowercase(),
                });
            }
        }
    }
    out
}

/// Score one entry against one query. Substring-only matching buries `customers` under
/// `customer_audit_log`; the ladder below is the ranking. No fuzzy crate: every rung is a
/// `to_ascii_lowercase` and a comparison, and a real fuzzy matcher is the upgrade path.
fn entry_score(entry: &IndexEntry, query: &str) -> Option<i64> {
    let name = entry.name.to_ascii_lowercase();
    let schema_conn = format!(
        "{} {}",
        entry.schema.as_deref().unwrap_or_default(),
        entry.conn
    )
    .to_ascii_lowercase();
    let mut score: i64 = 0;
    let mut matched = false;
    if name == query {
        return Some(100);
    }
    if name.starts_with(query) {
        score = score.max(60);
        matched = true;
    }
    if !matched && name.contains(query) {
        score = score.max(40);
        matched = true;
    }
    if !matched
        && name
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word.starts_with(query))
    {
        score = score.max(30);
        matched = true;
    }
    if !matched && entry.haystack.contains(&format!(" {query}")) {
        score = score.max(15);
        matched = true;
    }
    if !matched && schema_conn.contains(query) {
        score = score.max(10);
        matched = true;
    }
    matched.then_some(score)
}

/// The ranked hits for a query, best first, at most `limit`. An empty query returns nothing: the
/// list is a search result, not a browser.
fn score(entries: &[IndexEntry], query: &str, limit: usize) -> Vec<IndexEntry> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(i64, &IndexEntry)> = entries
        .iter()
        .filter_map(|e| entry_score(e, query).map(|s| (s, e)))
        .collect();
    hits.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
    hits.into_iter()
        .take(limit)
        .map(|(_, e)| e.clone())
        .collect()
}

/// The session's quick-open index: every open connection's catalog, the saved snippets, the open
/// tabs. A closed connection contributes nothing: `close_conn` clears the driver but keeps the
/// last catalog, so the gate is "is the connection live", not "does it have a catalog".
fn session_index(session: &Session) -> Vec<IndexEntry> {
    let catalogs: Vec<(&str, &nook_model::Catalog)> = session
        .conns
        .iter()
        .filter(|c| c.driver.is_some())
        .filter_map(|c| c.catalog.as_ref().map(|cat| (c.cfg.id.as_str(), cat)))
        .collect();
    let mut entries = build_index(&catalogs);
    for s in &session.snippets {
        entries.push(IndexEntry {
            kind: "saved",
            conn: String::new(),
            schema: None,
            name: s.name.clone(),
            haystack: s.name.to_ascii_lowercase(),
        });
    }
    for (i, tab) in session.tabs.iter().enumerate() {
        entries.push(IndexEntry {
            kind: "tab",
            conn: tab
                .conn
                .and_then(|c| session.conns.get(c))
                .map(|c| c.cfg.id.clone())
                .unwrap_or_default(),
            schema: None,
            name: tab.title.clone(),
            haystack: format!("{} {}", tab.title, tab.sql.text).to_ascii_lowercase(),
        });
        let _ = i;
    }
    entries
}

/// One query, ranked hits across every open catalog plus saved queries and open tabs.
fn quick_open_search(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let query = p.str_arg("query")?.to_ascii_lowercase();
    let limit = p.int_opt("limit", 40).clamp(1, 200) as usize;
    let hits = score(&session_index(session), &query, limit);
    Ok(json!({
        "hits": hits.iter().map(|h| json!({
            "kind": h.kind, "conn": h.conn, "schema": h.schema, "name": h.name
        })).collect::<Vec<_>>(),
        "total": hits.len(),
    }))
}

/// The Navicat catalog id: the same index, server-wide, under a `pattern` param — kept for parity
/// and for scripts.
fn server_search(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    // `pattern` is optional but typed: a junk value must be a named error, not a silent empty
    // search.
    let query = match p.0.get("pattern").or_else(|| p.0.get("query")) {
        Some(serde_json::Value::String(s)) => s.to_ascii_lowercase(),
        Some(other) => {
            return Err(Error::config(format!(
                "parameter 'pattern' must be a string, got {}",
                crate::json_kind(other)
            )))
        }
        None => String::new(),
    };
    let limit = p.int_opt("limit", 100).clamp(1, 500) as usize;
    let entries = session_index(session);
    // Objects only: server search is a catalog search, not a tab search.
    let objects: Vec<IndexEntry> = entries
        .into_iter()
        .filter(|e| e.kind == "table" || e.kind == "view")
        .collect();
    let hits = score(&objects, &query, limit);
    Ok(json!({
        "hits": hits.iter().map(|h| json!({
            "kind": h.kind, "conn": h.conn, "schema": h.schema, "name": h.name
        })).collect::<Vec<_>>(),
        "total": hits.len(),
    }))
}

/// Open one hit: a table/view becomes a Data tab, a saved query a Query tab with its SQL, a tab
/// becomes active.
fn quick_open_open(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let kind = p.str_arg("kind")?;
    let name = p.str_arg("name")?;
    match kind.as_str() {
        "table" | "view" => {
            let conn = p.str_arg("conn")?;
            let index = session.conn_index(&conn)?;
            let schema = p.str_opt("schema");
            // The lookup is the guard: a name that is not in the catalog is a named error, and a
            // real Table is bound the same way the tree binds it.
            session.table_of(index, schema.as_deref(), &name)?;
            let reference = TableRef {
                schema: schema.clone(),
                table: name.clone(),
            };
            let tab = session.add_tab(crate::TabKind::Data, name.clone(), Some(index));
            crate::cmd::bind_grid_table(session, tab, reference)?;
            crate::cmd::load_grid(session, tab)?;
            Ok(json!({ "opened": name, "kind": kind, "conn": conn }))
        }
        "saved" => {
            let snippet = session
                .snippets
                .iter()
                .find(|s| s.name == name)
                .ok_or_else(|| Error::not_found(format!("no saved query named '{name}'")))?;
            let sql = snippet.sql.clone();
            let id = session.next_tab_id;
            session.next_tab_id += 1;
            let mut tab = crate::Tab::new(id, crate::TabKind::Query, name.clone());
            tab.sql.text = sql;
            session.tabs.push(tab);
            session.active_tab = Some(session.tabs.len() - 1);
            Ok(json!({ "opened": name, "kind": kind }))
        }
        "tab" => {
            let index = session
                .tabs
                .iter()
                .position(|t| t.title == name)
                .ok_or_else(|| Error::not_found(format!("no open tab named '{name}'")))?;
            session.active_tab = Some(index);
            Ok(json!({ "opened": name, "kind": kind, "index": index }))
        }
        other => Err(Error::config(format!("unknown quick-open kind '{other}'"))),
    }
}

/// The catalog projected as a document: the same facts `schema.ddl` and the diagram use, written for
/// a reader rather than a server. One pass, no queries — a dictionary that ran one query per table
/// would take minutes on a real schema.
fn data_dictionary(session: &mut crate::Session, p: &crate::Params) -> Result<serde_json::Value> {
    let index = session.resolve_conn(p)?;
    let schema_filter = p.str_opt("schema");
    let format = p.str_opt("format").unwrap_or_else(|| "markdown".into());
    let (body, tables, columns) = {
        let catalog = session.catalog_of(index)?;
        let rendered = match format.as_str() {
            "markdown" => dictionary_markdown(catalog, schema_filter.as_deref()),
            "html" => dictionary_html(catalog, schema_filter.as_deref()),
            other => {
                return Err(Error::config(format!(
                    "unknown format '{other}'; use markdown or html"
                )))
            }
        };
        let tables = catalog
            .schemas
            .iter()
            .filter(|s| schema_filter.as_deref().is_none_or(|want| s.name == want))
            .map(|s| s.tables.len())
            .sum::<usize>();
        let columns = catalog
            .schemas
            .iter()
            .filter(|s| schema_filter.as_deref().is_none_or(|want| s.name == want))
            .flat_map(|s| &s.tables)
            .map(|t| t.columns.len())
            .sum::<usize>();
        (rendered, tables, columns)
    };
    if tables == 0 {
        return Err(Error::not_found(
            "this connection has no tables to describe",
        ));
    }
    match p.str_opt("path") {
        Some(path) => {
            let path = crate::cmd::grid::expand(&path, &session.home);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| Error::io(format!("create {}: {e}", parent.display())))?;
            }
            std::fs::write(&path, &body)
                .map_err(|e| Error::io(format!("write {}: {e}", path.display())))?;
            session.log(
                crate::Level::Success,
                format!("dictionary written: {}", path.display()),
            );
            Ok(json!({
                "path": path.to_string_lossy(),
                "bytes": body.len(),
                "tables": tables,
                "columns": columns,
                "format": format,
            }))
        }
        None => Ok(json!({
            "text": body,
            "tables": tables,
            "columns": columns,
            "format": format,
        })),
    }
}

fn dictionary_markdown(catalog: &nook_model::Catalog, schema_filter: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", catalog.driver));
    for schema in &catalog.schemas {
        if schema_filter.is_some_and(|want| schema.name != want) {
            continue;
        }
        out.push_str(&format!("## {}\n\n", schema.name));
        for table in &schema.tables {
            out.push_str(&format!("### {}\n\n", table.key()));
            if !table.comment.is_empty() {
                out.push_str(&format!("{}\n\n", table.comment));
            }
            if let Some(rows) = table.row_estimate {
                out.push_str(&format!("~{rows} rows\n\n"));
            }
            out.push_str("| Column | Type | Null | Key | Default | Comment |\n");
            out.push_str("|---|---|---|---|---|---|\n");
            for column in &table.columns {
                out.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} |\n",
                    column.name,
                    column.type_name,
                    if column.nullable { "yes" } else { "no" },
                    if column.primary_key { "PK" } else { "" },
                    column.default.as_deref().unwrap_or(""),
                    column.comment,
                ));
            }
            out.push('\n');
            if !table.indexes.is_empty() {
                out.push_str("Indexes: ");
                let names: Vec<String> = table
                    .indexes
                    .iter()
                    .map(|i| {
                        format!(
                            "{}({}){}",
                            i.name,
                            i.columns.join(", "),
                            if i.unique { " unique" } else { "" }
                        )
                    })
                    .collect();
                out.push_str(&names.join("; "));
                out.push_str("\n\n");
            }
            if !table.foreign_keys.is_empty() {
                out.push_str("Foreign keys: ");
                let names: Vec<String> = table
                    .foreign_keys
                    .iter()
                    .map(|f| {
                        format!(
                            "({}) → {}.{}({})",
                            f.columns.join(", "),
                            f.ref_table,
                            "",
                            f.ref_columns.join(", ")
                        )
                    })
                    .collect();
                out.push_str(&names.join("; "));
                out.push_str("\n\n");
            }
        }
    }
    out
}

fn dictionary_html(catalog: &nook_model::Catalog, schema_filter: Option<&str>) -> String {
    let escape = |s: &str| -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    let mut out = String::from(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\">\n\
         <title>Data dictionary</title>\n\
         <style>body{font:14px/1.5 system-ui,sans-serif;margin:2rem;max-width:60rem}\
         table{border-collapse:collapse;width:100%;margin:0 0 1.5rem}\
         th,td{border:1px solid #ccc;padding:.25rem .5rem;text-align:left;vertical-align:top}\
         th{background:#f4f4f4}h2{margin-top:2.5rem}</style>\n</head><body>\n",
    );
    out.push_str(&format!("<h1>{}</h1>\n", escape(&catalog.driver)));
    for schema in &catalog.schemas {
        if schema_filter.is_some_and(|want| schema.name != want) {
            continue;
        }
        out.push_str(&format!("<h2>{}</h2>\n", escape(&schema.name)));
        for table in &schema.tables {
            out.push_str(&format!("<h3>{}</h3>\n", escape(&table.key())));
            if !table.comment.is_empty() {
                out.push_str(&format!("<p>{}</p>\n", escape(&table.comment)));
            }
            out.push_str(
                "<table><thead><tr><th>Column</th><th>Type</th><th>Null</th><th>Key</th>\
                 <th>Default</th><th>Comment</th></tr></thead><tbody>\n",
            );
            for column in &table.columns {
                out.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                    escape(&column.name),
                    escape(&column.type_name),
                    if column.nullable { "yes" } else { "no" },
                    if column.primary_key { "PK" } else { "" },
                    escape(column.default.as_deref().unwrap_or("")),
                    escape(&column.comment),
                ));
            }
            out.push_str("</tbody></table>\n");
            if !table.indexes.is_empty() || !table.foreign_keys.is_empty() {
                out.push_str("<ul>\n");
                for index in &table.indexes {
                    out.push_str(&format!(
                        "<li>index {} ({}){}</li>\n",
                        escape(&index.name),
                        escape(&index.columns.join(", ")),
                        if index.unique { " unique" } else { "" }
                    ));
                }
                for fk in &table.foreign_keys {
                    out.push_str(&format!(
                        "<li>foreign key ({}) → {}({})</li>\n",
                        escape(&fk.columns.join(", ")),
                        escape(&fk.ref_table),
                        escape(&fk.ref_columns.join(", "))
                    ));
                }
                out.push_str("</ul>\n");
            }
        }
    }
    out.push_str("</body></html>\n");
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    fn table(name: &str, columns: Vec<nook_model::Column>) -> nook_model::Table {
        nook_model::Table {
            schema: Some("main".into()),
            name: name.into(),
            kind: nook_model::TableKind::Table,
            columns,
            indexes: vec![],
            foreign_keys: vec![],
            row_estimate: None,
            size_bytes: None,
            comment: String::new(),
        }
    }

    fn catalog(tables: Vec<nook_model::Table>) -> nook_model::Catalog {
        nook_model::Catalog {
            driver: "SQLite 3".into(),
            database: None,
            databases: vec![],
            schemas: vec![nook_model::Schema {
                name: "main".into(),
                tables,
                triggers: vec![],
                routines: vec![],
                sequences: vec![],
            }],
            fetched_ms: 0,
        }
    }

    #[test]
    fn exact_name_outranks_prefix_outranks_contains() {
        let entries = build_index(&[(
            "local",
            &catalog(vec![
                table("customer_audit_log", vec![]),
                table("customers", vec![]),
                table("customer_standards", vec![]),
            ]),
        )]);
        // Exact beats both: `customers` must be first even though it was indexed second.
        let hits = score(&entries, "customers", 40);
        assert_eq!(hits[0].name, "customers", "exact must be first: {hits:?}");
        // `customer_audit_log` does not contain `customers`; it must be absent.
        assert!(
            hits.iter().all(|h| h.name != "customer_audit_log"),
            "substring must not match: {hits:?}"
        );
        // Prefix ties break alphabetically — all three start with `customer`.
        let hits2 = score(&entries, "customer", 40);
        assert_eq!(
            hits2.iter().map(|h| h.name.as_str()).collect::<Vec<_>>(),
            vec!["customer_audit_log", "customer_standards", "customers"],
            "all prefix, alphabetical tiebreak: {hits2:?}"
        );
    }

    #[test]
    fn contains_ranks_below_prefix() {
        let entries = build_index(&[(
            "local",
            &catalog(vec![
                table("x_customers", vec![]), // contains only
                table("customers_x", vec![]), // prefix
            ]),
        )]);
        // `customers_x` starts with the query (60); `x_customers` only contains it (40).
        let hits = score(&entries, "customers", 40);
        assert_eq!(
            hits.iter().map(|h| h.name.as_str()).collect::<Vec<_>>(),
            vec!["customers_x", "x_customers"],
            "prefix must outrank contains: {hits:?}"
        );
    }

    #[test]
    fn a_closed_connection_contributes_nothing_even_though_its_catalog_remains() {
        let cat = catalog(vec![table("customers", vec![])]);
        let mut session = Session::new();
        // A connection that was opened (catalog filled) and then closed: `close_conn` keeps the
        // catalog but drops the driver — the gate must be the driver, not the catalog.
        let cfg = nook_core::ConnectionConfig::parse("sqlite::memory:closed").expect("parses");
        let mut conn = crate::Conn::new(cfg);
        conn.catalog = Some(cat);
        session.conns.push(conn);
        let entries = session_index(&session);
        assert!(
            entries.iter().all(|e| e.kind != "table"),
            "a closed connection's catalog must not be searchable: {entries:?}"
        );
    }

    #[test]
    fn a_column_name_finds_its_table() {
        let entries = build_index(&[(
            "local",
            &catalog(vec![table(
                "users",
                vec![nook_model::Column {
                    name: "email".into(),
                    type_name: "text".into(),
                    nullable: true,
                    primary_key: false,
                    auto_increment: false,
                    default: None,
                    comment: String::new(),
                    ordinal: 1,
                    max_len: None,
                    numeric_precision: None,
                    numeric_scale: None,
                    enum_values: vec![],
                }],
            )]),
        )]);
        let hits = score(&entries, "email", 40);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "users");
    }

    #[test]
    fn an_empty_query_returns_nothing() {
        let entries = build_index(&[]);
        assert!(score(&entries, "", 40).is_empty());
    }

    #[test]
    fn csv_parsing_handles_quotes_and_newlines() {
        let text = "id,name\n1,\"Smith, John\"\n2,\"line\nbreak\"\n";
        let rows = parse_csv(text, ',');
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1][1], "Smith, John");
        assert_eq!(rows[2][1], "line\nbreak");
        // Escaped quotes survive.
        let rows = parse_csv("a\n\"say \"\"hi\"\"\"\n", ',');
        assert_eq!(rows[1][0], "say \"hi\"");
        // A trailing blank line does not become a row.
        assert_eq!(parse_csv("a,b\n1,2\n\n", ',').len(), 2);
    }

    #[test]
    fn inference_follows_the_target_type_first() {
        assert_eq!(infer_value("42", Some("TEXT")), Value::text("42"));
        assert_eq!(infer_value("42", Some("INTEGER")), Value::Int(42));
        assert_eq!(infer_value("42", None), Value::Int(42));
        assert_eq!(infer_value("4.5", None), Value::Float(4.5));
        assert_eq!(infer_value("true", None), Value::Bool(true));
        assert_eq!(infer_value("", None), Value::Null);
        assert_eq!(infer_value("null", None), Value::Null);
        assert_eq!(infer_value("hello", None), Value::text("hello"));
    }

    #[test]
    fn query_builder_joins_and_filters() {
        let mut session = Session::new();
        let p = Params::from_json(json!({
            "from": [
                {"table": "public.users", "alias": "u", "columns": ["id", "email"]},
                {"table": "public.orders", "alias": "o", "columns": ["total"], "join": "left", "on": "o.user_id = u.id"}
            ],
            "where": "u.email like '%@example.com'",
            "order_by": "u.id desc",
            "limit": 25,
            "dialect": "postgres"
        }))
        .expect("params");
        let out = query_builder(&mut session, &p).expect("build");
        let sql = out["sql"].as_str().unwrap_or_default();
        assert!(sql.contains("SELECT \"u\".\"id\", \"u\".\"email\", \"o\".\"total\""));
        assert!(sql.contains("FROM \"public\".\"users\" AS \"u\""));
        assert!(sql.contains("LEFT JOIN \"public\".\"orders\" AS \"o\" ON o.user_id = u.id"));
        assert!(sql.contains("WHERE u.email like"));
        assert!(sql.ends_with("LIMIT 25;"));
    }

    #[test]
    fn query_builder_needs_tables_and_join_conditions() {
        let mut session = Session::new();
        assert!(query_builder(&mut session, &Params::none()).is_err());
        let p =
            Params::from_json(json!({"from": [{"table": "a"}, {"table": "b"}]})).expect("params");
        assert!(query_builder(&mut session, &p).is_err());
    }

    #[test]
    fn importing_a_missing_file_is_an_error() {
        let mut session = Session::new();
        let p =
            Params::from_json(json!({"path": "/tmp/nope-nook.csv", "table": "t"})).expect("params");
        assert!(import_csv(&mut session, &p).is_err());
        let p = Params::from_json(json!({"path": "/tmp/nope-nook.json", "table": "t"}))
            .expect("params");
        assert!(import_json(&mut session, &p).is_err());
    }

    #[test]
    fn generator_hint_is_documented() {
        assert!(generator_hint(&Generator::Auto).contains("pick:"));
        let mut session = Session::new();
        let out = generators(&mut session, &Params::none()).expect("list");
        assert!(out["generators"].as_array().map(Vec::len).unwrap_or(0) > 10);
    }
}
