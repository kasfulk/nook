//! Grid commands: paging, sorting, filtering, inline edits, export.

use std::path::PathBuf;

use nook_core::{Error, Result, Value};
use nook_model::{plan_edits, EditKind, RowEdit};
use serde_json::json;

use crate::cmd::{bind_grid_table, load_grid, pk_values, run_sql, sync_grid_table};
use crate::{CommandSpec, Level, Params, Session, TabKind, TableRef};

pub fn commands() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("grid.open", "Open table data", "Data", open)
            .params(r#"{"table":"public.users"}"#)
            .help("Open a data tab bound to a table, with the primary key known so edits work."),
        CommandSpec::new("grid.fetchPage", "Go to page", "Data", fetch_page)
            .params(r#"{"page":2}"#),
        CommandSpec::new("grid.nextPage", "Next page", "Data", next_page),
        CommandSpec::new("grid.prevPage", "Previous page", "Data", prev_page),
        CommandSpec::new("grid.firstPage", "First page", "Data", first_page),
        CommandSpec::new("grid.lastPage", "Last page", "Data", last_page),
        CommandSpec::new("grid.setPageSize", "Rows per page", "Data", set_page_size)
            .params(r#"{"page_size":500}"#),
        CommandSpec::new("grid.sort", "Sort by column", "Data", sort)
            .params(r#"{"column":"created_at","desc":true}"#),
        CommandSpec::new("grid.filter", "Filter rows", "Data", filter)
            .params(r#"{"where":"status = 'live'"}"#)
            .help("A raw WHERE clause, shown in the grid's filter bar."),
        CommandSpec::new("grid.filters", "Filter rows with conditions", "Data", filters)
            .params(r#"{"filters":[{"column":"status","op":"=","value":"live"},{"column":"total","op":">","value":100}]}"#)
            .help("A list of conditions compiled to the same WHERE clause `grid.filter` takes: column and value are quoted, the operator is a closed list (=, !=, <, <=, >, >=, like, ilike, in, is null, is not null, between). An empty list clears the filter."),
        CommandSpec::new("grid.filterPreview", "Preview the filter SQL", "Data", filter_preview)
            .params(r#"{"filters":[{"column":"status","op":"=","value":"live"}]}"#)
            .help("The WHERE clause the conditions compile to, and the COUNT(*) that would run with it, without touching the grid. Refuses exactly what `grid.filters` would refuse."),
        CommandSpec::new("grid.clearFilter", "Clear the filter", "Data", clear_filter),
        CommandSpec::new("grid.countRows", "Count rows", "Data", count_rows),
        CommandSpec::new("grid.editCell", "Edit a cell", "Data", edit_cell)
            .params(r#"{"row":0,"column":"email","value":"new@example.com"}"#)
            .help("Records an edit; nothing is sent until grid.apply."),
        CommandSpec::new("grid.setNull", "Set a cell to NULL", "Data", set_null),
        CommandSpec::new("grid.insertRow", "Insert a row", "Data", insert_row)
            .params(r#"{"values":{"email":"new@example.com"}}"#),
        CommandSpec::new("grid.deleteRow", "Delete a row", "Data", delete_row)
            .params(r#"{"row":0}"#),
        CommandSpec::new(
            "grid.duplicateRow",
            "Duplicate a row",
            "Data",
            duplicate_row,
        )
        .params(r#"{"row":0}"#),
        CommandSpec::new("grid.pending", "Show pending edits", "Data", pending),
        CommandSpec::new("grid.revert", "Discard pending edits", "Data", revert),
        CommandSpec::new("grid.bulkEdit", "Set a column across rows", "Data", bulk_edit)
            .params(r#"{"column":"status","value":"archived","rows":[3,4,9]}"#)
            .help("Stages one cell edit per row, exactly as the inline editor would: the same pending list, the same one-transaction grid.apply. `rows` are page rows; a row outside the page is a named error, and the batch is capped at max_rows with `skipped` saying how many were left."),
        CommandSpec::new(
            "grid.editQueryResult",
            "Make a query result editable",
            "Data",
            edit_query_result,
        )
        .params(r#"{"statement":0}"#)
        .help("Resolves the source table and primary key of one statement of the last run, so its rows can be edited like a data tab. Refused — with the reason — for anything whose rows are not one-to-one with a table row: GROUP BY, DISTINCT, JOIN, UNION, an aggregate, a CTE, a subquery, or a projection that leaves out a key column."),
        CommandSpec::new("grid.rawMode", "The SQL a pending edit will run", "Data", raw_mode)
            .params(r#"{"index":0,"sql":"UPDATE users SET status = 'x' WHERE id = 3"}"#)
            .help("Without \"sql\" it lists every pending edit with the statement it will run. With it, the statement is mapped back to exactly one pending edit by its WHERE clause and that edit's value is replaced. A statement that does not map to one pending edit is refused and the list is left alone — arbitrary SQL belongs in the query editor."),
        CommandSpec::new("grid.apply", "Apply changes", "Data", apply)
            .shortcut("cmd+s")
            .help("Runs the pending statements on the server, then reloads the page."),
        CommandSpec::new("grid.export", "Export the grid", "Data", export)
            .params(r#"{"format":"csv","path":"/tmp/rows.csv"}"#)
            .help("format: csv, json, sql, markdown. Without a path the text is returned."),
        CommandSpec::new(
            "grid.exportTable",
            "Export a whole table",
            "Data",
            export_table,
        )
        .params(r#"{"table":"users","format":"csv","path":"/tmp/users.csv"}"#),
        CommandSpec::new("grid.find", "Find in the grid", "Data", find).params(r#"{"text":"a@b"}"#),
        CommandSpec::new("grid.cellValue", "Read one cell", "Data", cell_value)
            .params(r#"{"row":0,"column":"id"}"#),
        CommandSpec::new("grid.formView", "Form view of one record", "Data", form_view)
            .params(r#"{"row":0}"#)
            .help("The record at `row` as a vertical form: one field per column with its type, value and whether it is a key. Read-only over the page the grid already holds."),
        CommandSpec::new("grid.copyAsSql", "Copy rows as INSERT", "Data", copy_as_sql),
        CommandSpec::new("grid.copyTsv", "Copy as spreadsheet TSV", "Data", copy_tsv)
            .params(r#"{"header":true,"anchor":[0,0],"cursor":[5,3]}"#)
            .help("The selection rectangle (or the whole page) as tab-separated text: a paste into Excel or Sheets lands as a table. Embedded newlines are escaped and wrapped; NULL is an empty cell."),
        CommandSpec::new("grid.selection", "Read or set the selection rectangle", "Data", grid_selection)
            .params(r#"{"anchor":[0,0],"cursor":[5,3]}"#)
            .help("anchor is where the drag started, cursor is the live corner. Without params it reads the current rectangle; both corners are clamped to the page and returned normalised (min, max)."),
        CommandSpec::new("grid.hideColumns", "Hide columns of this table", "Data", hide_columns)
            .params(r#"{"hidden":["password_hash"]}"#)
            .help("Hiding is a view concern: the query and `grid.fetchPage` still return every column. The list persists per connection and table, and an empty list clears it."),
        CommandSpec::new("grid.columnWidths", "Column widths of this table", "Data", column_widths_cmd)
            .params(r#"{"widths":{"email":300,"name":200}}"#)
            .help("Width overrides are a view concern: the query and `grid.fetchPage` still return every column. The map persists per connection and table, and an empty map clears it."),
        CommandSpec::new("grid.hints", "Display hints from column names", "Data", hints)
            .help("Reads the `__format__` marker in a column name: money, check or link. The header shows the name without the marker, and the query is untouched — a hint is a view concern."),
        CommandSpec::new("grid.blobEditor", "Edit a BLOB from hex", "Data", blob_editor)
            .params(r#"{"row":0,"column":"body","hex":"deadbeef"}"#)
            .help("Decodes hex (whitespace ignored) into the same pending edit the inline editor stages; nothing is written until grid.apply. Junk hex and a blob over max_blob_bytes are named errors."),
        CommandSpec::new("grid.quickLook", "Inspect one cell", "Data", quick_look)
            .params(r#"{"row":0,"column":"payload"}"#)
            .help("One cell as readable text: JSON is re-indented (invalid JSON stays verbatim with a note), bytes are hex with an offset column and an ASCII gutter, NULL says so. Never an error for bad content — a json column holding bad text is exactly when you need to look at it."),
        CommandSpec::new("grid.rowJson", "Row as JSON, one FK level", "Data", row_json)
            .params(r#"{"row":0,"expand":["customer_id"]}"#)
            .help("The row as a document: every column as a scalar, except the foreign keys in `expand`, which become {\"__ref__\": table, \"row\": {...}} with their target row nested one level. Omit `expand` to expand the table's own single-column keys. A dangling key is {\"__error__\": ...} inside the document, never a failed command."),
        CommandSpec::new(
            "grid.followForeignKey",
            "Where a foreign key points",
            "Data",
            follow_foreign_key,
        )
        .params(r#"{"row":0,"column":"customer_id"}"#)
        .help("Resolves a foreign-key cell to {conn, schema, table, where}: the target's primary key as a WHERE clause, or every column when the target has none (with a note saying so). A column that is not a foreign key, or a NULL value, is a named error."),
    ]
}

fn active_grid_tab(session: &Session) -> Result<usize> {
    let index = session
        .active_tab
        .ok_or_else(|| Error::internal("no active tab"))?;
    let tab = session
        .tabs
        .get(index)
        .ok_or_else(|| Error::internal("active tab is gone"))?;
    // A data tab always qualifies; a query tab does once `grid.editQueryResult` resolved its source
    // and bound the grid to it, which is what lets the same pending list, the same planner and the
    // same one-transaction apply serve both.
    if tab.kind != TabKind::Data && tab.grid.table.is_none() {
        return Err(Error::config(no_grid_reason()));
    }
    Ok(index)
}

/// Refuse a command that would change the *page query* on a grid whose rows are a statement's own
/// result. `load_grid` re-runs that statement, so an order, filter or page set here would be stored,
/// reported back by `grid_summary`, and never applied — the silent lie this phase exists to prevent.
fn refuse_on_statement_rows(session: &Session, index: usize, what: &str) -> Result<()> {
    let editable = session.tabs[index].sql.editable.as_ref();
    if editable.is_some() {
        return Err(Error::config(format!(
            "this result is the statement's own rows: {what} would not reach the query. Change the \
             statement and run it again"
        )));
    }
    Ok(())
}

/// Why the grid commands refuse the active tab. One definition, so a surface that cannot run one —
/// the results register's Export sits on a query tab, which has no data grid to hand over — can say
/// the engine's own words rather than its own.
pub fn no_grid_reason() -> &'static str {
    "the active tab has no data grid; open a table with grid.open"
}

fn grid_summary(session: &Session, index: usize) -> serde_json::Value {
    let tab = match session.tabs.get(index) {
        Some(t) => t,
        None => return json!({ "error": "tab is gone" }),
    };
    json!({
        "table": tab.grid.table.as_ref().map(TableRef::key),
        "page": tab.grid.page,
        "page_size": tab.grid.page_size,
        "rows": tab.grid.result.row_count(),
        "columns": tab.grid.result.columns,
        "types": tab.grid.result.column_types,
        "primary_key": tab.grid.primary_key,
        "order": tab.grid.order,
        "filter": tab.grid.filter,
        "total_rows": tab.grid.total_rows,
        "pending": tab.grid.pending.len(),
        "hidden": tab.grid.hidden,
        "truncated": tab.grid.result.truncated,
        "elapsed_ms": tab.grid.elapsed_ms,
        "error": tab.grid.error,
    })
}

fn open(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let conn = session.resolve_conn(p)?;
    let spec = p
        .str_opt("table")
        .ok_or_else(|| Error::config("pass \"table\""))?;
    let reference = TableRef::parse(&spec);
    let title = reference.key();
    let index = session.add_tab(TabKind::Data, title, Some(conn));
    bind_grid_table(session, index, reference)?;
    load_grid(session, index)?;
    if p.has("page_size") {
        let size = p.int_opt("page_size", 200).clamp(1, 100_000) as usize;
        if let Some(tab) = session.tabs.get_mut(index) {
            tab.grid.page_size = size;
        }
        load_grid(session, index)?;
    }
    Ok(grid_summary(session, index))
}

fn fetch_page(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    refuse_on_statement_rows(session, index, "paging")?;
    let page = p.int_opt("page", 0).max(0) as usize;
    session
        .tabs
        .get_mut(index)
        .ok_or_else(|| Error::internal("tab is gone"))?
        .grid
        .page = page;
    load_grid(session, index)?;
    Ok(grid_summary(session, index))
}

fn next_page(session: &mut Session, _p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    {
        let tab = session
            .tabs
            .get_mut(index)
            .ok_or_else(|| Error::internal("tab is gone"))?;
        if tab.grid.result.row_count() < tab.grid.page_size {
            return Ok(grid_summary(session, index));
        }
        tab.grid.page += 1;
    }
    load_grid(session, index)?;
    Ok(grid_summary(session, index))
}

fn prev_page(session: &mut Session, _p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    {
        let tab = session
            .tabs
            .get_mut(index)
            .ok_or_else(|| Error::internal("tab is gone"))?;
        tab.grid.page = tab.grid.page.saturating_sub(1);
    }
    load_grid(session, index)?;
    Ok(grid_summary(session, index))
}

fn first_page(session: &mut Session, _p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    session
        .tabs
        .get_mut(index)
        .ok_or_else(|| Error::internal("tab is gone"))?
        .grid
        .page = 0;
    load_grid(session, index)?;
    Ok(grid_summary(session, index))
}

fn last_page(session: &mut Session, _p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let total = count_rows(session, &Params::none())?;
    let rows = total["rows"].as_i64().unwrap_or(0).max(0) as usize;
    let size = session.tabs[index].grid.page_size.max(1);
    session
        .tabs
        .get_mut(index)
        .ok_or_else(|| Error::internal("tab is gone"))?
        .grid
        .page = rows.saturating_sub(1) / size;
    load_grid(session, index)?;
    Ok(grid_summary(session, index))
}

fn set_page_size(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    refuse_on_statement_rows(session, index, "a page size")?;
    let size = p.int_opt("page_size", 200).clamp(1, 100_000) as usize;
    session
        .tabs
        .get_mut(index)
        .ok_or_else(|| Error::internal("tab is gone"))?
        .grid
        .page_size = size;
    session.settings.page_size = size;
    load_grid(session, index)?;
    Ok(grid_summary(session, index))
}

fn sort(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    refuse_on_statement_rows(session, index, "sorting")?;
    let column = p.str_arg("column")?;
    let desc = p.bool_opt("desc", false);
    {
        let tab = session
            .tabs
            .get_mut(index)
            .ok_or_else(|| Error::internal("tab is gone"))?;
        if !tab.grid.result.columns.iter().any(|c| c == &column) {
            return Err(Error::not_found(format!(
                "no column '{column}' in this grid"
            )));
        }
        let dialect = tab
            .payload
            .get("dialect")
            .and_then(|v| v.as_str())
            .and_then(super::parse_dialect)
            .unwrap_or(nook_core::Dialect::Sqlite);
        // Clicking the same column again flips the direction.
        let current = tab.grid.order.clone();
        let wanted = format!(
            "{} {}",
            nook_core::quote_ident(&column, dialect),
            if desc { "DESC" } else { "ASC" }
        );
        tab.grid.order = if current == wanted {
            format!(
                "{} {}",
                nook_core::quote_ident(&column, dialect),
                if desc { "ASC" } else { "DESC" }
            )
        } else {
            wanted
        };
        tab.grid.page = 0;
        // The count described the request in hand, and this is a new one; `grid.countRows` refills it.
        tab.grid.total_rows = None;
    }
    load_grid(session, index)?;
    Ok(grid_summary(session, index))
}

fn filter(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    refuse_on_statement_rows(session, index, "a WHERE clause")?;
    let where_clause = p.str_arg("where")?;
    {
        let tab = session
            .tabs
            .get_mut(index)
            .ok_or_else(|| Error::internal("tab is gone"))?;
        tab.grid.filter = where_clause;
        // A hand-typed WHERE replaces whatever the chips described; keeping them would paint a
        // condition the grid is no longer filtering by.
        tab.grid.filters.clear();
        tab.grid.page = 0;
        // The count belongs to the rows the grid was asking for; this asks for a different set.
        tab.grid.total_rows = None;
    }
    load_grid(session, index)?;
    Ok(grid_summary(session, index))
}

/// The conditions a request carries, parsed by the one compiler. A request with no `filters` key
/// is an empty list — clearing — while a `filters` that is not a list is refused rather than
/// silently read as empty, which would drop a filter the caller meant to set.
fn filters_arg(p: &Params) -> Result<Vec<crate::filter::Filter>> {
    match p.raw().get("filters") {
        None => Ok(Vec::new()),
        Some(serde_json::Value::Array(items)) => {
            items.iter().map(crate::filter::Filter::from_json).collect()
        }
        Some(other) => Err(Error::config(format!(
            "parameter \"filters\" must be a list, got {}",
            crate::json_kind(other)
        ))),
    }
}

/// Refuse a condition whose column the page does not have. SQLite reads a double-quoted unknown
/// name as a string literal, so without this a typo'd chip compiles to `WHERE "ctiy" = 'London'`,
/// which is `'ctiy' = 'London'` — a filter that silently matches nothing instead of reporting a
/// mistake.
fn check_filter_columns(
    session: &Session,
    index: usize,
    filters: &[crate::filter::Filter],
) -> Result<()> {
    let known: Vec<String> = session.tabs[index]
        .grid
        .result
        .columns
        .iter()
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if known.is_empty() {
        // Nothing loaded: there is no column list to check against, and refusing every condition
        // would make the command unusable on a tab that has not fetched yet.
        return Ok(());
    }
    for filter in filters {
        if !known
            .iter()
            .any(|c| *c == filter.column.to_ascii_lowercase())
        {
            return Err(Error::not_found(format!(
                "no column '{}' in this result; it has: {}",
                filter.column,
                session.tabs[index].grid.result.columns.join(", ")
            )));
        }
    }
    Ok(())
}

fn dialect_of(session: &Session, index: usize) -> nook_core::Dialect {
    session
        .tabs
        .get(index)
        .and_then(|tab| tab.payload.get("dialect"))
        .and_then(|v| v.as_str())
        .and_then(super::parse_dialect)
        .unwrap_or(nook_core::Dialect::Sqlite)
}

fn filters(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    refuse_on_statement_rows(session, index, "a filter")?;
    let parsed = filters_arg(p)?;
    check_filter_columns(session, index, &parsed)?;
    let where_clause = crate::filter::compile(&parsed, dialect_of(session, index))?;
    {
        let tab = session
            .tabs
            .get_mut(index)
            .ok_or_else(|| Error::internal("tab is gone"))?;
        tab.grid.filter = where_clause.clone();
        tab.grid.filters = parsed.clone();
        tab.grid.page = 0;
        // The count described the rows the grid was asking for; this asks for a different set.
        tab.grid.total_rows = None;
    }
    load_grid(session, index)?;
    let mut summary = grid_summary(session, index);
    summary["where"] = json!(where_clause);
    summary["conditions"] = json!(parsed.len());
    Ok(summary)
}

fn filter_preview(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let parsed = filters_arg(p)?;
    check_filter_columns(session, index, &parsed)?;
    let dialect = dialect_of(session, index);
    let where_clause = crate::filter::compile(&parsed, dialect)?;
    let table = session.tabs[index]
        .grid
        .table
        .clone()
        .ok_or_else(|| Error::config("this data tab is not bound to a table"))?;
    // The count is the point of the preview: it is what tells a user their filter selects 3 rows
    // rather than 30,000, before the grid spends a round trip showing them.
    let count_sql = if where_clause.is_empty() {
        format!(
            "SELECT count(*) FROM {}",
            nook_core::quote_qualified(table.schema.as_deref(), &table.table, dialect)
        )
    } else {
        format!(
            "SELECT count(*) FROM {} WHERE {}",
            nook_core::quote_qualified(table.schema.as_deref(), &table.table, dialect),
            where_clause
        )
    };
    let conn = session.tabs[index]
        .conn
        .ok_or_else(|| Error::config("this tab has no connection"))?;
    let rows = session
        .driver_for(conn)?
        .run(&count_sql, 1)?
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    Ok(json!({
        "where": where_clause,
        "conditions": parsed.len(),
        "count_sql": count_sql,
        "rows": rows,
        "applied": false,
    }))
}

fn clear_filter(session: &mut Session, _p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    refuse_on_statement_rows(session, index, "clearing the filter")?;
    {
        let tab = session
            .tabs
            .get_mut(index)
            .ok_or_else(|| Error::internal("tab is gone"))?;
        tab.grid.filter.clear();
        tab.grid.filters.clear();
        tab.grid.page = 0;
        tab.grid.total_rows = None;
    }
    load_grid(session, index)?;
    Ok(grid_summary(session, index))
}

fn count_rows(session: &mut Session, _p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let (conn, schema, table, filter) = {
        let tab = &session.tabs[index];
        let table = tab
            .grid
            .table
            .clone()
            .ok_or_else(|| Error::config("this data tab is not bound to a table"))?;
        (
            tab.conn
                .ok_or_else(|| Error::config("this tab has no connection"))?,
            table.schema.clone(),
            table.table.clone(),
            tab.grid.filter.clone(),
        )
    };
    let total = {
        let driver = session.driver_for(conn)?;
        if filter.trim().is_empty() {
            driver.count_rows(schema.as_deref(), &table)?
        } else {
            let dialect = driver.dialect();
            let sql = format!(
                "SELECT count(*) FROM {} WHERE {}",
                nook_core::quote_qualified(schema.as_deref(), &table, dialect),
                filter.trim()
            );
            let rs = driver.run(&sql, 1)?;
            rs.rows
                .first()
                .and_then(|r| r.first())
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0) as u64
        }
    };
    if let Some(tab) = session.tabs.get_mut(index) {
        tab.grid.total_rows = Some(total);
    }
    Ok(json!({ "rows": total, "table": table }))
}

/// Plan one edit without touching the buffer. The split exists so a batch can plan every row
/// before it pushes any: a refusal halfway through must leave the pending list as it was.
fn plan_one(
    session: &Session,
    tab_index: usize,
    kind: EditKind,
    row: usize,
    column: Option<String>,
    value: Option<Value>,
    insert_values: Vec<(String, Value)>,
) -> Result<crate::PendingEdit> {
    let (sql, key) = plan_edit(
        session,
        tab_index,
        &EditRequest {
            kind,
            row,
            column: column.as_deref(),
            value: value.as_ref(),
            insert: &insert_values,
            key: None,
        },
    )?;
    Ok(crate::PendingEdit {
        kind,
        row,
        column: column.unwrap_or_default(),
        value: value.unwrap_or(Value::Null),
        insert: insert_values,
        key,
        sql,
    })
}

fn record_edit(
    session: &mut Session,
    tab_index: usize,
    kind: EditKind,
    row: usize,
    column: Option<String>,
    value: Option<Value>,
    insert_values: Vec<(String, Value)>,
) -> Result<serde_json::Value> {
    let edit = plan_one(session, tab_index, kind, row, column, value, insert_values)?;
    let sql = edit.sql.clone();
    let tab = session
        .tabs
        .get_mut(tab_index)
        .ok_or_else(|| Error::internal("tab is gone"))?;
    tab.grid.pending.push(edit);
    Ok(json!({
        "pending": tab.grid.pending.len(),
        "sql": sql,
        "kind": format!("{kind:?}").to_lowercase(),
    }))
}

/// The statement one grid edit becomes, planned against the table and the page as they are now: the
/// primary key values come from the page, the names and the dialect from the binding.
/// An edit waiting to be turned into a statement: what the grid recorded, and the key values it was
/// made with. Six loose parameters became a struct here because the key is the one that must not be
/// left out of a call.
struct EditRequest<'a> {
    kind: EditKind,
    /// Where the edit sits in the page the grid is showing *now* — what the pending drawer and the
    /// status bar count by. The statement is keyed by `key`, not by this.
    row: usize,
    column: Option<&'a str>,
    value: Option<&'a Value>,
    /// The columns an insert writes.
    insert: &'a [(String, Value)],
    /// The key values the edit was made with, in the key columns' order, or `None` to read them from
    /// `row` as the page stands now.
    key: Option<&'a [Value]>,
}

fn plan_edit(
    session: &Session,
    tab_index: usize,
    request: &EditRequest<'_>,
) -> Result<(String, Vec<Value>)> {
    let EditRequest {
        kind,
        row,
        column,
        value,
        insert: insert_values,
        key,
    } = *request;
    let dialect = session.tabs[tab_index]
        .payload
        .get("dialect")
        .and_then(|v| v.as_str())
        .and_then(super::parse_dialect)
        .unwrap_or(nook_core::Dialect::Sqlite);
    let tab = session
        .tabs
        .get(tab_index)
        .ok_or_else(|| Error::internal("tab is gone"))?;
    let table_ref = tab
        .grid
        .table
        .clone()
        .ok_or_else(|| Error::config("this data tab is not bound to a table"))?;
    // The values the edit captured win over the row index, which the page may have moved under: the
    // edit belongs to the record it was made on. Their *names* come from the table as it is now, so a
    // key column renamed since the edit was made still resolves.
    let pk = match (key, kind) {
        (_, EditKind::InsertRow) => Vec::new(),
        (Some(values), _) => {
            if values.len() != tab.grid.primary_key.len() {
                return Err(Error::config(
                    "the primary key of this table changed since the edit was made — \
                     reload the tab and edit again",
                ));
            }
            tab.grid
                .primary_key
                .iter()
                .cloned()
                .zip(values.iter().cloned())
                .collect()
        }
        (None, _) => pk_values(tab, row)?,
    };
    let changes = match kind {
        EditKind::InsertRow => insert_values.to_vec(),
        EditKind::SetCell => vec![(
            column
                .ok_or_else(|| Error::config("pass \"column\""))?
                .to_string(),
            value.cloned().unwrap_or(Value::Null),
        )],
        EditKind::DeleteRow => Vec::new(),
    };
    let edit = RowEdit {
        kind,
        row_index: row,
        schema: table_ref.schema.clone(),
        table: table_ref.table.clone(),
        pk: pk.clone(),
        changes,
    };
    let sql = plan_edits(std::slice::from_ref(&edit), dialect)?
        .first()
        .cloned()
        .unwrap_or_default();
    Ok((sql, pk.into_iter().map(|(_, value)| value).collect()))
}

/// A value that satisfies a NOT NULL column the server would otherwise reject.
fn fallback(column: &nook_model::Column) -> Value {
    let ty = column.type_name.to_ascii_uppercase();
    if ty.contains("INT") || ty.contains("SERIAL") {
        Value::Int(0)
    } else if ty.contains("BOOL") {
        Value::Bool(false)
    } else if ty.contains("REAL")
        || ty.contains("FLOAT")
        || ty.contains("NUMERIC")
        || ty.contains("DECIMAL")
    {
        Value::Float(0.0)
    } else {
        Value::text("")
    }
}

fn edit_cell(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    // Before the column's type is read: a schema edit since the tab was bound would coerce the new
    // value with the old type, and a stale page would plan the edit behind it.
    sync_grid_table(session, index)?;
    let row = p.int_arg("row").unwrap_or(0).max(0) as usize;
    let column = p.str_arg("column")?;
    let raw = p.cell("value").unwrap_or(Value::Null);
    // Coerce to the declared column type, so '42' becomes 42 for an int column.
    let value = {
        let tab = &session.tabs[index];
        match tab.grid.columns.iter().find(|c| c.name == column) {
            Some(column_info) => {
                Value::coerce(&raw.as_text(), &column_info.type_name).map_err(Error::config)?
            }
            None => raw,
        }
    };
    record_edit(
        session,
        index,
        EditKind::SetCell,
        row,
        Some(column),
        Some(value),
        vec![],
    )
}

fn set_null(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    sync_grid_table(session, index)?;
    let row = p.int_arg("row").unwrap_or(0).max(0) as usize;
    let column = p.str_arg("column")?;
    record_edit(
        session,
        index,
        EditKind::SetCell,
        row,
        Some(column),
        Some(Value::Null),
        vec![],
    )
}

fn insert_row(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    // The NOT NULL columns the empty row is filled from come from the binding.
    sync_grid_table(session, index)?;
    let values = match p.get("values") {
        Some(serde_json::Value::Object(map)) => map
            .iter()
            .map(|(k, v)| (k.clone(), Value::from_json(v)))
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    // An empty row still needs a statement: fill the columns the server would reject when missing.
    let values = if values.is_empty() {
        let tab = &session.tabs[index];
        tab.grid
            .columns
            .iter()
            .filter(|c| !c.auto_increment && !c.nullable && c.default.is_none())
            .map(|c| (c.name.clone(), fallback(c)))
            .collect()
    } else {
        values
    };
    record_edit(session, index, EditKind::InsertRow, 0, None, None, values)
}

fn delete_row(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    sync_grid_table(session, index)?;
    let row = p.int_arg("row").unwrap_or(0).max(0) as usize;
    {
        let tab = &session.tabs[index];
        if row >= tab.grid.result.row_count() {
            return Err(Error::not_found(format!("no row {row} on this page")));
        }
    }
    record_edit(session, index, EditKind::DeleteRow, row, None, None, vec![])
}

fn duplicate_row(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    sync_grid_table(session, index)?;
    let row = p.int_arg("row").unwrap_or(0).max(0) as usize;
    let values: Vec<(String, Value)> = {
        let tab = &session.tabs[index];
        if row >= tab.grid.result.row_count() {
            return Err(Error::not_found(format!("no row {row} on this page")));
        }
        tab.grid
            .result
            .columns
            .iter()
            .enumerate()
            .filter(|(_, name)| {
                // Generated keys are left out so the copy gets its own.
                !tab.grid
                    .columns
                    .iter()
                    .find(|c| &c.name == *name)
                    .map(|c| c.auto_increment)
                    .unwrap_or(false)
            })
            .map(|(i, name)| {
                (
                    name.clone(),
                    tab.grid.result.rows[row]
                        .get(i)
                        .cloned()
                        .unwrap_or(Value::Null),
                )
            })
            .collect()
    };
    record_edit(session, index, EditKind::InsertRow, 0, None, None, values)
}

/// The most rows one bulk edit will stage. A batch bigger than this is a sign the caller meant a
/// table-wide `UPDATE`, which belongs in the SQL editor where the row count is visible as SQL.
const BULK_LIMIT: usize = 200;

fn bulk_edit(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    // Before the type is read: a schema edit since the tab was bound would coerce with the old
    // type, and a stale page would plan the edit behind it.
    sync_grid_table(session, index)?;
    let column = p.str_arg("column")?;
    let rows: Vec<usize> = match p.raw().get("rows") {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .map(|v| {
                v.as_u64()
                    .map(|n| n as usize)
                    .ok_or_else(|| Error::config("every entry of \"rows\" must be a row number"))
            })
            .collect::<Result<Vec<_>>>()?,
        Some(other) => {
            return Err(Error::config(format!(
                "parameter \"rows\" must be a list, got {}",
                crate::json_kind(other)
            )))
        }
        None => return Err(Error::config("pass \"rows\": the page rows to set")),
    };
    // Coerce to the declared column type, the same rule `grid.editCell` follows: '42' becomes 42
    // for an int column, and a value of `null` is a real NULL rather than the string.
    let raw = p.cell("value").unwrap_or(Value::Null);
    let value = {
        let tab = &session.tabs[index];
        let info = tab
            .grid
            .columns
            .iter()
            .find(|c| c.name == column)
            .ok_or_else(|| Error::not_found(format!("no column '{column}' in this table")))?;
        if matches!(raw, Value::Null) {
            Value::Null
        } else {
            Value::coerce(&raw.as_text(), &info.type_name).map_err(Error::config)?
        }
    };
    // The cap comes first, and the bounds check covers only the rows that will be staged: a row
    // beyond the cap is not going to be touched, so refusing the batch for it would report a
    // problem with a row the command was never going to look at.
    let staged_rows: Vec<usize> = rows.iter().copied().take(BULK_LIMIT).collect();
    let skipped = rows.len() - staged_rows.len();
    let page_rows = session.tabs[index].grid.result.row_count();
    if let Some(bad) = staged_rows.iter().find(|r| **r >= page_rows) {
        return Err(Error::config(format!(
            "row {bad} is not on this page (it holds {page_rows}); fetch the page that has it"
        )));
    }
    // A key column set to one value across several rows is a uniqueness collision the server would
    // refuse on the second statement — after the first had already been staged. Say it up front.
    let is_key = session.tabs[index]
        .grid
        .primary_key
        .iter()
        .any(|k| k == &column);
    if is_key && rows.len() > 1 {
        return Err(Error::config(format!(
            "'{column}' is part of the primary key: one value across {} rows would collide.              Set a key on one row at a time.",
            rows.len()
        )));
    }
    // Plan every row first, then push. A refusal on row 7 must not leave rows 0..6 staged: the
    // caller asked for one batch and would have no way to tell a partial one from a whole one.
    let mut planned = Vec::with_capacity(staged_rows.len());
    for row in &staged_rows {
        planned.push(plan_one(
            session,
            index,
            EditKind::SetCell,
            *row,
            Some(column.clone()),
            Some(value.clone()),
            vec![],
        )?);
    }
    {
        let tab = session
            .tabs
            .get_mut(index)
            .ok_or_else(|| Error::internal("tab is gone"))?;
        tab.grid.pending.extend(planned);
    }
    Ok(json!({
        "staged": staged_rows.len(),
        "skipped": skipped,
        "pending": session.tabs[index].grid.pending.len(),
        "column": column,
    }))
}

/// The statements a pending edit will run, and the one door into changing one of them. The review
/// surface never accepts a statement it cannot map back to a pending edit: appending arbitrary SQL to
/// the pending list is how a review pane becomes a way to run SQL without saying so.
fn raw_mode(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let list = |session: &Session| {
        session.tabs[index]
            .grid
            .pending
            .iter()
            .enumerate()
            .map(|(i, e)| {
                json!({
                    "index": i,
                    "kind": format!("{:?}", e.kind).to_lowercase(),
                    "column": e.column,
                    "sql": e.sql,
                })
            })
            .collect::<Vec<_>>()
    };
    let Some(sql) = p.str_opt("sql") else {
        return Ok(json!({ "edits": list(session) }));
    };
    let at = p.int_opt("index", -1);
    let (new_set, new_where) = split_statement(&sql).ok_or_else(|| {
        Error::config("raw mode takes the UPDATE the planner emits, with a WHERE clause")
    })?;
    // The match is by WHERE clause: it is the only part of the statement that identifies the record,
    // and the planner writes it from the primary key alone, so two pending edits never share one.
    let matches: Vec<usize> = session.tabs[index]
        .grid
        .pending
        .iter()
        .enumerate()
        // Exact, not case-folded: the WHERE is the primary key the planner wrote, and folding the
        // case would make `'x'` and `'X'` the same record.
        .filter(|(_, e)| {
            split_statement(&e.sql).map(|(_, w)| w).as_deref() == Some(new_where.as_str())
        })
        .map(|(i, _)| i)
        .collect();
    let target = match (at, matches.as_slice()) {
        (n, _) if n >= 0 => {
            let n = n as usize;
            if n >= session.tabs[index].grid.pending.len() {
                return Err(Error::not_found(format!("no pending edit {n}")));
            }
            n
        }
        (_, [only]) => *only,
        (_, []) => {
            return Err(Error::config(
                "no pending edit has that WHERE clause; nothing was changed",
            ))
        }
        (_, many) => {
            return Err(Error::config(format!(
                "{} pending edits share that WHERE clause; pass \"index\" to say which",
                many.len()
            )))
        }
    };
    let edit = &session.tabs[index].grid.pending[target];
    let value = match edit.kind {
        EditKind::SetCell => parse_set_value(&new_set, &edit.column)?,
        _ => {
            return Err(Error::config(
                "raw mode edits one cell: an insert or a delete has no single value to change",
            ))
        }
    };
    // Re-plan against the table as it is now, so the statement the review shows is the statement the
    // apply will run — and a column that changed type rejects the literal here rather than at commit.
    let key = session.tabs[index].grid.pending[target].key.clone();
    let column = edit.column.clone();
    let (planned, _) = plan_edit(
        session,
        index,
        &EditRequest {
            kind: EditKind::SetCell,
            row: edit.row,
            column: Some(&column),
            value: Some(&value),
            insert: &[],
            key: Some(&key),
        },
    )?;
    let tab = session
        .tabs
        .get_mut(index)
        .ok_or_else(|| Error::internal("tab is gone"))?;
    let edit = &mut tab.grid.pending[target];
    edit.value = value;
    edit.sql = planned.clone();
    Ok(json!({ "index": target, "sql": planned, "edits": list(session) }))
}

/// A planner statement split at its `WHERE`: `(before, where)`, both in the statement's own case —
/// the literal keeps its case because it is going to be read back into a value. The planner is the
/// only producer, and it writes the keyword in upper case, so the split is on the last ` WHERE ` it
/// wrote; a `WHERE` inside a literal is inside quotes and cannot match that shape.
fn split_statement(sql: &str) -> Option<(String, String)> {
    let at = sql.rfind(" WHERE ")?;
    Some((
        sql[..at].trim().to_string(),
        sql[at + 7..]
            .trim()
            .trim_end_matches(';')
            .trim()
            .to_string(),
    ))
}

/// The literal after `SET <column> =` in a planner statement, parsed back into a value. Only the
/// shapes `sql_literal` writes are accepted: a single-quoted string (with `''` for a quote), a
/// number, `NULL`, or `X'hex'`.
fn parse_set_value(set: &str, column: &str) -> Result<Value> {
    let upper = set.to_ascii_uppercase();
    let at = upper.find(" SET ").ok_or_else(|| {
        Error::config("raw mode takes an UPDATE: the statement has no SET clause")
    })?;
    let assignments = &set[at + 5..];
    let column_upper = column.to_ascii_uppercase();
    // The planner writes one assignment per UPDATE; a hand-written multi-column SET is not a shape
    // this maps back to a single pending edit, so it is refused rather than half-applied.
    let (name, literal) = assignments
        .split_once('=')
        .ok_or_else(|| Error::config("the SET clause is not `column = value`"))?;
    let name = name
        .trim()
        .trim_matches('"')
        .trim_matches('`')
        .trim_matches('\'');
    if !name.eq_ignore_ascii_case(&column_upper) {
        return Err(Error::config(format!(
            "this edit changes '{column}'; the statement sets '{name}'"
        )));
    }
    let literal = literal.trim().trim_end_matches(';').trim();
    if literal.eq_ignore_ascii_case("null") {
        return Ok(Value::Null);
    }
    if let Some(hex) = literal
        .strip_prefix("X'")
        .or_else(|| literal.strip_prefix("x'"))
        .and_then(|rest| rest.strip_suffix('\''))
    {
        let bytes = (0..hex.len() / 2)
            .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16))
            .collect::<std::result::Result<Vec<u8>, _>>()
            .map_err(|_| Error::config("that hex literal is not hex"))?;
        return Ok(Value::Bytes(bytes));
    }
    if let Some(text) = literal
        .strip_prefix('\'')
        .and_then(|r| r.strip_suffix('\''))
    {
        return Ok(Value::text(text.replace("''", "'")));
    }
    if let Ok(n) = literal.parse::<i64>() {
        return Ok(Value::Int(n));
    }
    if let Ok(f) = literal.parse::<f64>() {
        return Ok(Value::Float(f));
    }
    Err(Error::config(
        "that SET value is not a literal the planner writes (string, number, NULL or X'hex')",
    ))
}

fn pending(session: &mut Session, _p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let tab = &session.tabs[index];
    Ok(json!({
        "pending": tab.grid.pending.iter().map(|e| json!({
            "kind": format!("{:?}", e.kind).to_lowercase(),
            "row": e.row, "column": e.column, "sql": e.sql,
        })).collect::<Vec<_>>(),
        "dirty": tab.grid.dirty(),
    }))
}

fn revert(session: &mut Session, _p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let discarded = {
        let tab = session
            .tabs
            .get_mut(index)
            .ok_or_else(|| Error::internal("tab is gone"))?;
        let n = tab.grid.pending.len();
        tab.grid.pending.clear();
        n
    };
    Ok(json!({ "discarded": discarded }))
}

fn apply(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    // The commit's own gate, before anything is planned: safe mode wants the token of the connection
    // this write will land in — the tab's, not whichever one the params name — and a caller that
    // cannot present it never reaches the statements.
    let conn_index = session.tabs[index]
        .conn
        .ok_or_else(|| Error::config("this tab has no connection"))?;
    super::require_safe_mode(session, p, conn_index)?;
    // Plan every pending edit again, against the table as it is now. A statement planned before the
    // table changed cannot be trusted: the columns it names may be gone, and a stale `WHERE` matches
    // no row while reporting the edit applied — the SQL that runs here is the SQL the grid shows
    // afterwards, and an edit that cannot be planned fails before anything runs. The *row* is not
    // re-read: it comes from the key the edit captured, so a page sorted, filtered or paged since the
    // edit was made cannot move the statement onto another record.
    sync_grid_table(session, index)?;
    let pending = session.tabs[index].grid.pending.clone();
    let mut statements: Vec<String> = Vec::with_capacity(pending.len());
    for edit in &pending {
        let column = (!edit.column.is_empty()).then_some(edit.column.as_str());
        let (sql, _) = plan_edit(
            session,
            index,
            &EditRequest {
                kind: edit.kind,
                row: edit.row,
                column,
                value: Some(&edit.value),
                insert: &edit.insert,
                key: Some(&edit.key),
            },
        )?;
        if let Some(slot) = session.tabs[index].grid.pending.get_mut(statements.len()) {
            slot.sql = sql.clone();
        }
        statements.push(sql);
    }
    let conn = session.tabs[index]
        .conn
        .ok_or_else(|| Error::config("this tab has no connection"))?;
    if statements.is_empty() {
        return Ok(json!({ "applied": 0, "message": "nothing to apply" }));
    }
    let in_transaction = !session.settings.auto_commit_edits;
    if in_transaction {
        session.driver_for(conn)?.begin()?;
    }
    let mut applied = 0usize;
    for (edit, sql) in pending.iter().zip(&statements) {
        // An `UPDATE` or a `DELETE` keyed by a primary key changes exactly one row, so a driver that
        // reports none changed nothing, and saying "applied" would be a lie the data disagrees with.
        // Without this, a key that moved under the tab makes the edit a silent no-op: SQLite reads a
        // double-quoted name that matches no column as a string literal, and the statement then
        // matches no row at all. A row that is gone (deleted elsewhere) lands here too, and the user
        // should hear about it rather than be told their edit landed.
        let one_row = matches!(edit.kind, EditKind::SetCell | EditKind::DeleteRow);
        let failure = match run_sql(session, conn, sql, Some(1)) {
            Ok(result) if one_row && result.affected == 0 => Some(Error::config(format!(
                "the edit changed no row, so nothing was applied: {sql}\n\
                 The row or the table may have moved under this tab — reload it and edit again."
            ))),
            Ok(_) => None,
            Err(e) => Some(e),
        };
        if let Some(e) = failure {
            if in_transaction {
                let _ = session.driver_for(conn).and_then(|d| d.rollback());
            }
            session.log(
                Level::Error,
                format!("edit {applied} of {} failed; rolled back", statements.len()),
            );
            return Err(e);
        }
        applied += 1;
    }
    if in_transaction {
        session.driver_for(conn)?.commit()?;
    }
    if let Some(tab) = session.tabs.get_mut(index) {
        tab.grid.pending.clear();
        // Rows the grid had counted have just been inserted, updated or deleted; the count described
        // the request before them, so it goes the way `filter`, `clear_filter` and `sort` send theirs.
        tab.grid.total_rows = None;
    }
    load_grid(session, index)?;
    session.log(Level::Success, format!("{applied} changes applied"));
    Ok(json!({ "applied": applied, "grid": grid_summary(session, index) }))
}

/// Make one statement's result editable, honestly: the source table must be exactly one table, it
/// must have a primary key, and the projection must carry every key column. Each of those is a
/// refusal that names itself, because a result made editable without its key is a result whose
/// writes would hit whichever row happens to sit at that index.
fn edit_query_result(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = session
        .active_tab
        .ok_or_else(|| Error::internal("no active tab"))?;
    let statement = p.int_opt("statement", 0).max(0) as usize;
    // A data tab's grid already edits its table; this command exists for the query tab, and saying
    // so beats "the last run has 0 statements" to someone looking at a full grid.
    if session.tabs[index].kind != crate::TabKind::Query {
        return Err(Error::config(
            "grid.editQueryResult works on a query tab: open one and run the statement first",
        ));
    }
    let sql = session
        .tabs
        .get(index)
        .and_then(|tab| tab.sql.statements.get(statement))
        .cloned()
        .ok_or_else(|| {
            Error::not_found(format!(
                "no statement {statement}; the last run has {}",
                session.tabs[index].sql.statements.len()
            ))
        })?;
    let source = crate::sqltext::single_select_source(&sql).map_err(Error::config)?;
    let conn_index = session.tabs[index]
        .conn
        .ok_or_else(|| Error::config("this tab has no connection"))?;
    let table = TableRef {
        schema: source.schema().map(str::to_string),
        table: source.name().to_string(),
    };
    let info = session
        .conns
        .get(conn_index)
        .and_then(|c| c.catalog.as_ref())
        .and_then(|cat| cat.find_table(table.schema.as_deref(), &table.table))
        .cloned()
        .ok_or_else(|| {
            Error::not_found(format!(
                "{} is not in the catalog; refresh the connection and try again",
                table.key()
            ))
        })?;
    let key = info.primary_key();
    if key.is_empty() {
        return Err(Error::unsupported(format!(
            "{} has no primary key, so its rows cannot be told apart; add one in the designer              (schema.setPrimaryKey) and run the statement again",
            table.key()
        )));
    }
    // Without the key in the projection there is nothing to write the WHERE from. Guessing which
    // row is meant is how a client updates the wrong record.
    if let Some(columns) = &source.columns {
        let missing: Vec<&str> = key
            .iter()
            .map(String::as_str)
            .filter(|k| !columns.iter().any(|c| c == k))
            .collect();
        if !missing.is_empty() {
            return Err(Error::config(format!(
                "the result does not select the primary key column{} {}; add {} to the SELECT",
                if missing.len() == 1 { "" } else { "s" },
                missing.join(", "),
                missing.join(", ")
            )));
        }
    }
    let editable = crate::EditableSource {
        statement,
        table: table.clone(),
        key: key.clone(),
    };
    // The rows the statement returned become the tab's grid page, and the table becomes its binding:
    // from here the edit, pending and apply commands run unchanged, because they read the same
    // `GridState` a data tab fills. `load_grid` re-runs the statement rather than selecting the
    // table, so an apply or a refresh cannot replace these rows with a different question's answer.
    let page = session.tabs[index]
        .sql
        .results
        .get(statement)
        .and_then(|run| run.result.clone())
        .ok_or_else(|| Error::config(format!("statement {statement} produced no rows to edit")))?;
    bind_grid_table(session, index, table.clone())?;
    if let Some(tab) = session.tabs.get_mut(index) {
        tab.sql.editable = Some(editable.clone());
        tab.grid.result = page;
        tab.grid.page = 0;
        tab.grid.filter.clear();
        tab.grid.filters.clear();
        tab.grid.order.clear();
        tab.grid.total_rows = None;
        tab.grid.error = None;
    }
    Ok(json!({
        "statement": statement,
        "table": table.key(),
        "key": key,
        "read_only": false,
    }))
}

fn export(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let format = p.str_opt("format").unwrap_or_else(|| "csv".into());
    let text = render(&session.tabs[index].grid.result, &format)?;
    write_or_return(session, p, &format, &text)
}

fn export_table(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let conn = session.resolve_conn(p)?;
    let spec = p
        .str_opt("table")
        .ok_or_else(|| Error::config("pass \"table\""))?;
    let reference = TableRef::parse(&spec);
    let format = p.str_opt("format").unwrap_or_else(|| "csv".into());
    let limit = p
        .int_opt("limit", session.settings.max_rows as i64)
        .clamp(1, 5_000_000) as usize;
    let (dialect, result) = {
        let driver = session.driver_for(conn)?;
        let dialect = driver.dialect();
        let rs = driver.fetch_table(
            reference.schema.as_deref(),
            &reference.table,
            limit,
            0,
            None,
            None,
        )?;
        (dialect, rs)
    };
    let text = match format.as_str() {
        "sql" => result.to_sql_inserts(reference.schema.as_deref(), &reference.table, dialect),
        other => render(&result, other)?,
    };
    write_or_return(session, p, &format, &text)
}

fn render(result: &nook_model::ResultSet, format: &str) -> Result<String> {
    match format {
        "csv" => Ok(result.to_csv()),
        "json" => serde_json::to_string_pretty(&result.to_json())
            .map_err(|e| Error::internal(format!("json: {e}"))),
        "sql" => Err(Error::config(
            "exporting SQL needs a table; use grid.exportTable or dump_script",
        )),
        "markdown" | "md" => Ok(result.to_markdown()),
        other => Err(Error::config(format!(
            "unknown format '{other}'; try csv, json, sql or markdown"
        ))),
    }
}

fn write_or_return(
    session: &mut Session,
    p: &Params,
    format: &str,
    text: &str,
) -> Result<serde_json::Value> {
    match p.str_opt("path") {
        Some(path) => {
            let path = expand(&path, &session.home);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| Error::io(format!("create {}: {e}", parent.display())))?;
            }
            std::fs::write(&path, text)
                .map_err(|e| Error::io(format!("write {}: {e}", path.display())))?;
            session.log(Level::Success, format!("exported to {}", path.display()));
            Ok(json!({
                "path": path.to_string_lossy(),
                "bytes": text.len(),
                "lines": text.lines().count(),
                "format": format,
            }))
        }
        None => Ok(json!({
            "format": format,
            "bytes": text.len(),
            "lines": text.lines().count(),
            "text": text,
        })),
    }
}

pub(crate) fn expand(path: &str, home: &std::path::Path) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(base) = std::env::var_os("HOME") {
            return PathBuf::from(base).join(rest);
        }
    }
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        home.join(candidate)
    }
}

fn find(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let needle = p.str_arg("text")?.to_ascii_lowercase();
    let column = p.str_opt("column");
    let tab = &session.tabs[index];
    let mut hits = Vec::new();
    for (row, values) in tab.grid.result.rows.iter().enumerate() {
        for (col, value) in values.iter().enumerate() {
            let name = tab.grid.result.columns.get(col);
            if let Some(want) = &column {
                if name.map(|n| n != want).unwrap_or(true) {
                    continue;
                }
            }
            if value.as_text().to_ascii_lowercase().contains(&needle) {
                hits.push(json!({
                    "row": row,
                    "column": name,
                    "value": value.display(),
                }));
            }
        }
    }
    Ok(json!({ "text": needle, "matches": hits }))
}

fn cell_value(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let row = p.int_arg("row").unwrap_or(0).max(0) as usize;
    let column = p.str_arg("column")?;
    let tab = &session.tabs[index];
    let value = tab
        .grid
        .result
        .value_at(row, &column)
        .cloned()
        .ok_or_else(|| Error::not_found(format!("no cell at row {row}, column {column}")))?;
    Ok(json!({ "value": value.to_json(), "display": value.display(), "type": value.type_name() }))
}
/// One record as a vertical form: every column with its declared type, its value and whether it is
/// a key. Read-only over the page the grid already holds, so it asks the server nothing — the same
/// row the grid painted, arranged for a table too wide to read across.
fn form_view(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    // The form offers a field per column, so it reads the same binding an edit plans against: without
    // this it lists the names and types the tab was opened with, and an edit made from a field the
    // table no longer has dies on the server instead of being refused here.
    sync_grid_table(session, index)?;
    let tab = &session.tabs[index];
    let grid = &tab.grid;
    // `row` is explicit. The engine's cursor is not kept in step with the grid's selection, so
    // guessing here would show a different record than the one the user clicked — the UI passes
    // the selected row instead.
    let row = p.int_opt("row", 0).max(0) as usize;
    let row_count = grid.result.rows.len();
    if row >= row_count {
        return Err(Error::not_found(format!(
            "no row {row} on this page (it holds {row_count} rows)"
        )));
    }
    // The form shows what `grid.apply` will send, not the page as it was fetched: a pending edit
    // to this row overlays the cell the form paints. Pending edits apply in order, so the last
    // one to a cell wins — the same rule the apply loop follows.
    //
    // "This row" is the record, not the index: a sort or a page turn between the edit and this call
    // moves the record, and `PendingEdit::row` would then overlay the edit onto whichever record took
    // that row. The key is what says they belong together, and it is the same key the apply uses.
    let belongs_to_this_record = |edit: &crate::PendingEdit| {
        if edit.key.is_empty() {
            edit.row == row
        } else {
            super::row_of_key(&grid.result, &grid.primary_key, &edit.key) == Some(row)
        }
    };
    let overlaid: std::collections::BTreeMap<&str, &Value> = grid
        .pending
        .iter()
        .filter(|e| belongs_to_this_record(e))
        .filter_map(|e| match e.kind {
            nook_model::EditKind::SetCell => Some((e.column.as_str(), &e.value)),
            _ => None,
        })
        .collect();
    let fields: Vec<serde_json::Value> = grid
        .columns
        .iter()
        .map(|column| {
            let value = overlaid
                .get(column.name.as_str())
                .copied()
                .or_else(|| grid.result.value_at(row, &column.name));
            json!({
                "name": column.name,
                "type": column.type_name,
                "value": value.map(|v| v.to_json()),
                "display": value.map(|v| v.display()).unwrap_or_default(),
                "primary_key": grid.primary_key.contains(&column.name),
            })
        })
        .collect();
    Ok(json!({
        "row": row,
        "rows": row_count,
        // The grid's own gate, so the form never offers a field the engine would refuse.
        "editable": grid.editable(),
        "fields": fields,
    }))
}

fn copy_as_sql(session: &mut Session, _p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let tab = &session.tabs[index];
    let table = tab
        .grid
        .table
        .clone()
        .ok_or_else(|| Error::config("this data tab is not bound to a table"))?;
    let dialect = tab
        .payload
        .get("dialect")
        .and_then(|v| v.as_str())
        .and_then(super::parse_dialect)
        .unwrap_or(nook_core::Dialect::Sqlite);
    let sql = tab
        .grid
        .result
        .to_sql_inserts(table.schema.as_deref(), &table.table, dialect);
    Ok(json!({
        "statements": sql.lines().count(),
        "sql": sql,
    }))
}

/// Read a `[row, column]` pair from the params, validating it against the page.
fn pair_arg(p: &Params, key: &str, rows: usize, columns: usize) -> Result<(usize, usize)> {
    let raw = p
        .raw()
        .get(key)
        .ok_or_else(|| Error::config(format!("missing parameter '{key}'")))?;
    let serde_json::Value::Array(items) = raw else {
        return Err(Error::config(format!(
            "parameter '{key}' must be [row, column], got {}",
            crate::json_kind(raw)
        )));
    };
    if items.len() != 2 {
        return Err(Error::config(format!(
            "parameter '{key}' must be [row, column], got {} items",
            items.len()
        )));
    }
    let (Some(row), Some(col)) = (items[0].as_i64(), items[1].as_i64()) else {
        return Err(Error::config(format!(
            "parameter '{key}' must hold two integers"
        )));
    };
    if row < 0 || row as usize >= rows {
        return Err(Error::not_found(format!(
            "row {row} is outside the page (it holds {rows} rows)"
        )));
    }
    if col < 0 || col as usize >= columns {
        return Err(Error::not_found(format!(
            "column {col} is outside the page (it holds {columns} columns)"
        )));
    }
    Ok((row as usize, col as usize))
}

/// The rectangle as normalised inclusive ranges: (min_row..=max_row, min_col..=max_col).
fn rect_of(
    anchor: (usize, usize),
    cursor: (usize, usize),
) -> (
    std::ops::RangeInclusive<usize>,
    std::ops::RangeInclusive<usize>,
) {
    (
        anchor.0.min(cursor.0)..=anchor.0.max(cursor.0),
        anchor.1.min(cursor.1)..=anchor.1.max(cursor.1),
    )
}

/// The grid's copy: TSV over the selection rectangle (or the whole page). The formatting lives in
/// `ResultSet::to_tsv`, so the engine and the UI cannot drift apart.
fn copy_tsv(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let header = p.strict_bool_opt("header", true)?;
    let tab = &session.tabs[index];
    let rows = tab.grid.result.rows.len();
    let columns = tab.grid.result.columns.len();
    let rect = if p.raw().contains_key("anchor") || p.raw().contains_key("cursor") {
        let anchor = pair_arg(p, "anchor", rows, columns)?;
        let cursor = pair_arg(p, "cursor", rows, columns)?;
        Some(rect_of(anchor, cursor))
    } else if let (Some(a), Some(c)) = (tab.grid.anchor, tab.grid.cursor) {
        // A copy with no rectangle copies what the user selected: the stored anchor/cursor
        // pair, so a Cmd+C after a drag does not re-send the corners. Stored corners are
        // clamped against the page in case it shrank underneath them.
        let a = (
            a.0.min(rows.saturating_sub(1)),
            a.1.min(columns.saturating_sub(1)),
        );
        let c = (
            c.0.min(rows.saturating_sub(1)),
            c.1.min(columns.saturating_sub(1)),
        );
        Some(rect_of(a, c))
    } else {
        None
    };
    let tsv = tab.grid.result.to_tsv(rect, header);
    Ok(json!({ "tsv": tsv }))
}

/// Read or set the selection rectangle. Setting normalises the corners (min, max) so a caller
/// never has to guess which corner the drag started from.
fn grid_selection(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let rows = session.tabs[index].grid.result.rows.len();
    let columns = session.tabs[index].grid.result.columns.len();
    if p.raw().contains_key("anchor") || p.raw().contains_key("cursor") {
        let anchor = pair_arg(p, "anchor", rows, columns)?;
        let cursor = pair_arg(p, "cursor", rows, columns)?;
        let (a, c) = (
            (anchor.0.min(cursor.0), anchor.1.min(cursor.1)),
            (anchor.0.max(cursor.0), anchor.1.max(cursor.1)),
        );
        let tab = &mut session.tabs[index];
        tab.grid.anchor = Some(a);
        tab.grid.cursor = Some(c);
    }
    let tab = &session.tabs[index];
    let (anchor, cursor) = match (tab.grid.anchor, tab.grid.cursor) {
        (Some(a), Some(c)) => (a, c),
        // No selection yet: a degenerate rectangle at the first cell, so callers reading before
        // the first click get a shape, not a null to special-case.
        _ => ((0, 0), (0, 0)),
    };
    let (rows_range, cols_range) = rect_of(anchor, cursor);
    Ok(json!({
        "anchor": [anchor.0, anchor.1],
        "cursor": [cursor.0, cursor.1],
        "rows": rows_range.count(),
        "columns": cols_range.count(),
    }))
}

/// Hidden columns, persisted per `conn + table` through the store's `ui_state` — one writer per
/// fact. Hiding never touches the query: `grid.fetchPage` keeps returning every column, and the
/// UI decides what to paint. Without params, the current list reads back.
fn hide_columns(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let tab = &session.tabs[index];
    let table = tab
        .grid
        .table
        .clone()
        .ok_or_else(|| Error::config("this data tab is not bound to a table"))?;
    let conn_id = tab
        .conn
        .and_then(|i| session.conns.get(i))
        .map(|c| c.cfg.id.clone())
        .unwrap_or_default();
    let key = format!("hidden:{}:{}", conn_id, table.key());
    let current: Vec<String> = tab.grid.hidden.clone();

    if p.raw().contains_key("hidden") {
        // `hidden` must be an array of names (a comma-separated string is accepted as the
        // repo's list convention); a bare scalar is junk and stays junk.
        match p.raw().get("hidden") {
            Some(serde_json::Value::Array(_)) | Some(serde_json::Value::String(_)) => {}
            Some(other) => {
                return Err(Error::config(format!(
                    "parameter 'hidden' must be a list of column names, got {}",
                    crate::json_kind(other)
                )))
            }
            None => {}
        }
        let wanted = p.list("hidden");
        // Every hidden name must be a column of the table the grid shows: a typo would
        // silently hide nothing, and the caller has no other way to learn the real names.
        for name in &wanted {
            if !tab.grid.result.columns.iter().any(|c| c == name) {
                return Err(Error::not_found(format!(
                    "no column '{name}': this table holds {}",
                    tab.grid.result.columns.join(", ")
                )));
            }
        }
        let value = serde_json::to_string(&wanted).map_err(|e| Error::internal(e.to_string()))?;
        crate::store::save_ui_state(&key, &value)?;
        // The cache and the store move together: the paint path reads the cache, and a stale one
        // would keep painting columns the store already says are hidden.
        if let Some(tab) = session.tabs.get_mut(index) {
            tab.grid.hidden = wanted.clone();
        }
        return Ok(json!({ "hidden": wanted }));
    }
    Ok(json!({ "hidden": current }))
}

/// Column width overrides, persisted per `conn + table` through the store's `ui_state` — one
/// writer per fact. Widths never touch the query: `grid.fetchPage` keeps returning every column,
/// and the UI decides how wide to paint each one. Without params, the current map reads back.
fn column_widths_cmd(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let tab = &session.tabs[index];
    let table = tab
        .grid
        .table
        .clone()
        .ok_or_else(|| Error::config("this data tab is not bound to a table"))?;
    let conn_id = tab
        .conn
        .and_then(|i| session.conns.get(i))
        .map(|c| c.cfg.id.clone())
        .unwrap_or_default();
    let key = format!("widths:{}:{}", conn_id, table.key());
    let current: Vec<(String, f32)> = tab.grid.widths.clone();

    if let Some(raw) = p.raw().get("widths") {
        // `widths` must be an object mapping column names to numbers; anything else is junk.
        let widths_obj = match raw {
            serde_json::Value::Object(obj) => obj,
            other => {
                return Err(Error::config(format!(
                    "parameter 'widths' must be an object mapping column names to numbers, got {}",
                    crate::json_kind(other)
                )))
            }
        };
        // Every width must be a finite number > 0 and <= 4000; every name must be a column of
        // the table the grid shows. A typo would silently set nothing, and the caller has no
        // other way to learn the real names.
        let mut validated: Vec<(String, f32)> = Vec::new();
        for (name, value) in widths_obj {
            if !tab.grid.result.columns.iter().any(|c| c == name) {
                return Err(Error::not_found(format!(
                    "no column '{name}': this table holds {}",
                    tab.grid.result.columns.join(", ")
                )));
            }
            let w = value.as_f64().ok_or_else(|| {
                Error::config(format!(
                    "width for column '{name}' must be a number, got {}",
                    crate::json_kind(value)
                ))
            })?;
            if !w.is_finite() || w <= 0.0 || w > 4000.0 {
                return Err(Error::config(format!(
                    "width for column '{name}' must be a finite number > 0 and <= 4000, got {w}"
                )));
            }
            validated.push((name.clone(), w as f32));
        }
        // Stored as the bare `{name: width}` object, the same shape `hidden` uses (a bare array)
        // and the shape `column_widths` parses. A `{"widths": {...}}` wrapper here would read back
        // as one entry named "widths" whose value is an object — dropped, so every width would
        // silently vanish on reopen.
        let value = serde_json::to_string(
            &validated
                .iter()
                .map(|(n, w)| (n.clone(), serde_json::json!(w)))
                .collect::<serde_json::Map<_, _>>(),
        )
        .map_err(|e| Error::internal(e.to_string()))?;
        crate::store::save_ui_state(&key, &value)?;
        // The cache and the store move together: the paint path reads the cache, and a stale one
        // would keep painting columns the store already says are a different width.
        if let Some(tab) = session.tabs.get_mut(index) {
            tab.grid.widths = validated.clone();
        }
        let map: serde_json::Map<String, serde_json::Value> = validated
            .into_iter()
            .map(|(n, w)| (n, serde_json::json!(w)))
            .collect();
        return Ok(json!({ "widths": map }));
    }
    let map: serde_json::Map<String, serde_json::Value> = current
        .into_iter()
        .map(|(n, w)| (n, serde_json::json!(w)))
        .collect();
    Ok(json!({ "widths": map }))
}

/// Display hints read from the column names: `name__format__money` renders as money, and the
/// header shows `name`. A hint is a view concern — the query still returns every column — so this
/// reads the page the grid already holds and never touches the SQL.
fn hints(session: &mut Session, _p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let tab = &session.tabs[index];
    let hints: Vec<serde_json::Value> = tab
        .grid
        .result
        .columns
        .iter()
        .filter_map(|name| {
            let (label, kind) = nook_model::format_hint(name)?;
            Some(json!({ "column": name, "label": label, "kind": kind }))
        })
        .collect();
    Ok(json!({ "hints": hints }))
}

/// Stage a BLOB edit from hex text.
///
/// The write path is the same `record_edit` the inline editor uses: the blob editor only decodes
/// the text and hands over a `Value::Bytes`, so `plan_edits` and `grid.apply` need no new case.
/// The cap exists because a 2 GB `bytea` painted as hex is how a database client eats all the RAM.
fn blob_editor(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    sync_grid_table(session, index)?;
    let row = p.int_arg("row").unwrap_or(0).max(0) as usize;
    let column = p.str_arg("column")?;
    let hex = p.str_arg("hex")?;
    let compact: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    // Byte-slicing a `String` panics mid-character, so the alphabet is checked before the pairs
    // are cut: a caller sending "é0" gets a named error, not a crashed app.
    if !compact.is_ascii() {
        return Err(Error::config("hex must be ASCII: only 0-9 and a-f"));
    }
    if !compact.len().is_multiple_of(2) {
        return Err(Error::config(format!(
            "hex must have an even number of digits, got {}",
            compact.len()
        )));
    }
    let bytes = (0..compact.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&compact[i..i + 2], 16)
                .map_err(|_| Error::config(format!("'{}' is not hex", &compact[i..i + 2])))
        })
        .collect::<Result<Vec<u8>>>()?;
    let cap = session.settings.max_blob_bytes;
    if bytes.len() > cap {
        return Err(Error::config(format!(
            "that blob is {} bytes; the editor stops at max_blob_bytes ({cap})",
            bytes.len()
        )));
    }
    // The staged edit carries the bytes; the response only reports what was decoded.
    record_edit(
        session,
        index,
        EditKind::SetCell,
        row,
        Some(column),
        Some(Value::Bytes(bytes.clone())),
        vec![],
    )?;
    Ok(json!({ "ok": true, "bytes": bytes.len() }))
}

/// Bytes as a hex dump: 16 per line, an eight-digit offset column, and an ASCII gutter where a
/// printable byte is itself and anything else is a dot — the layout every hex editor taught.
fn hex_dump(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let offset = i * 16;
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        let ascii: String = chunk
            .iter()
            .map(|b| {
                if (0x20..0x7f).contains(b) {
                    *b as char
                } else {
                    '.'
                }
            })
            .collect();
        out.push_str(&format!("{offset:08x}  {:<47}  {ascii}\n", hex.join(" ")));
    }
    out
}

/// One cell as readable text: the formatting rules live here, the UI paints the string it gets —
/// the same split as `grid.formView`. Bad content is never an error: an invalid-JSON cell or a
/// binary blob is exactly what Quick Look exists to read.
fn quick_look(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let row = p.int_opt("row", 0).max(0) as usize;
    let column = p.str_arg("column")?;
    let tab = &session.tabs[index];
    let row_count = tab.grid.result.rows.len();
    if row >= row_count {
        return Err(Error::not_found(format!(
            "no row {row} on this page (it holds {row_count} rows)"
        )));
    }
    let value = tab
        .grid
        .result
        .value_at(row, &column)
        .cloned()
        .ok_or_else(|| {
            Error::not_found(format!(
                "no column '{column}': this table holds {}",
                tab.grid.result.columns.join(", ")
            ))
        })?;
    Ok(match value {
        Value::Null => json!({"kind": "null", "text": ""}),
        Value::Json(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(parsed) => json!({
                "kind": "json",
                "text": serde_json::to_string_pretty(&parsed).unwrap_or(text),
            }),
            Err(_) => json!({
                "kind": "json",
                "text": text,
                "note": "invalid JSON — shown verbatim",
            }),
        },
        // Text-shaped columns holding JSON still get the pretty path: SQLite has no json type,
        // so `SELECT json_object(...)` lands as TEXT.
        Value::Text(text) => {
            let trimmed = text.trim_start();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(parsed) => json!({
                        "kind": "json",
                        "text": serde_json::to_string_pretty(&parsed).unwrap_or(text),
                    }),
                    Err(_) => json!({
                        "kind": "text",
                        "text": text,
                        "note": "invalid JSON — shown verbatim",
                    }),
                }
            } else {
                json!({"kind": "text", "text": text})
            }
        }
        Value::Bytes(bytes) => json!({
            "kind": "bytes",
            "size": bytes.len(),
            "text": hex_dump(&bytes),
        }),
        other => json!({"kind": other.type_name(), "text": other.display()}),
    })
}

/// The row as a JSON document, with the requested foreign keys expanded one level.
///
/// One level only: a self-referencing key would otherwise recurse forever, and the nested row is
/// what the UI's "follow" chevron is for. A failed expansion is data, not an error — a dangling key
/// must not blank the panel — so the reference carries `__error__` instead of a row.
fn row_json(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let row = p.int_opt("row", 0).max(0) as usize;
    let requested = p.list("expand");
    let (conn, table, columns, values) = {
        let tab = &session.tabs[index];
        let row_count = tab.grid.result.rows.len();
        if row >= row_count {
            return Err(Error::not_found(format!(
                "no row {row} on this page (it holds {row_count} rows)"
            )));
        }
        let conn = tab
            .conn
            .ok_or_else(|| Error::config("this data tab is not bound to a connection"))?;
        let table = tab
            .grid
            .table
            .clone()
            .ok_or_else(|| Error::config("this data tab is not bound to a table"))?;
        let columns = tab.grid.result.columns.clone();
        let values = tab.grid.result.rows[row].clone();
        (conn, table, columns, values)
    };
    // The keys come from the catalog, not the page: a page shows values, not constraints.
    let foreign_keys = session
        .table_of(conn, table.schema.as_deref(), &table.table)?
        .foreign_keys;
    // No list means "expand the row's own keys": the UI has no catalog, so asking it to name the
    // foreign keys would be asking it to guess.
    let expand: Vec<String> = if requested.is_empty() {
        foreign_keys
            .iter()
            .filter(|fk| fk.columns.len() == 1 && fk.ref_columns.len() == 1)
            .map(|fk| fk.columns[0].clone())
            .collect()
    } else {
        requested
    };
    // Every requested name must be a column of this table: a typo would silently expand nothing.
    for name in &expand {
        if !columns.iter().any(|c| c == name) {
            return Err(Error::not_found(format!(
                "no column '{name}': this table holds {}",
                columns.join(", ")
            )));
        }
    }
    let mut doc = serde_json::Map::new();
    for (i, name) in columns.iter().enumerate() {
        let value = values.get(i).cloned().unwrap_or(Value::Null);
        doc.insert(name.clone(), value.to_json());
    }
    for name in &expand {
        // Only a column the catalog calls a foreign key expands; a plain column stays a scalar.
        // A compound key needs every one of its columns to resolve, and the page hands us one at
        // a time, so only a single-column key expands. Skipping beats inventing the other values.
        let Some(fk) = foreign_keys.iter().find(|fk| {
            fk.columns.len() == 1 && fk.ref_columns.len() == 1 && fk.columns[0] == *name
        }) else {
            continue;
        };
        // The original value, not the JSON copy: a round-trip turns `Bytes` into base64 text and a
        // `UInt` above `i64::MAX` into a float, so the WHERE would search for the wrong literal.
        let Some(position) = columns.iter().position(|c| c == name) else {
            continue;
        };
        let Some(value) = values.get(position).cloned() else {
            continue;
        };
        if value.is_null() {
            continue;
        }
        doc.insert(name.clone(), expand_reference(session, conn, fk, &value)?);
    }
    Ok(json!({ "table": table.key(), "row": doc }))
}

/// One foreign key expansion: the referenced row as a document, or a `__error__` when the key
/// matches nothing. The nested row is *not* expanded again — one level, by design.
fn expand_reference(
    session: &mut Session,
    conn: usize,
    fk: &nook_model::ForeignKey,
    key: &Value,
) -> Result<serde_json::Value> {
    let sql = {
        let driver = session.driver_for(conn)?;
        let dialect = driver.dialect();
        let column = fk
            .ref_columns
            .first()
            .ok_or_else(|| Error::internal("a foreign key with no referenced column"))?;
        format!(
            "SELECT * FROM {} WHERE {} = {} LIMIT 1",
            nook_core::quote_qualified(fk.ref_schema.as_deref(), &fk.ref_table, dialect),
            nook_core::quote_ident(column, dialect),
            nook_core::sql_literal(key, dialect)
        )
    };
    let result = {
        let driver = session.driver_for(conn)?;
        driver.run(&sql, 1)?
    };
    let Some(row) = result.rows.first() else {
        return Ok(json!({
            "__error__": format!(
                "no row in {} with {} = {}",
                fk.ref_table,
                fk.ref_columns.join(", "),
                key.display()
            ),
        }));
    };
    let mut doc = serde_json::Map::new();
    for (i, name) in result.columns.iter().enumerate() {
        let value = row.get(i).cloned().unwrap_or(Value::Null);
        doc.insert(name.clone(), value.to_json());
    }
    Ok(json!({
        // Schema-qualified: a Postgres follow needs the schema, and `public.customers` is the
        // name the plan specifies.
        "__ref__": match &fk.ref_schema {
            Some(schema) => format!("{schema}.{}", fk.ref_table),
            None => fk.ref_table.clone(),
        },
        "row": doc,
    }))
}

/// Where a foreign-key cell points: the target table, and a `WHERE` that selects the one row.
///
/// The target's primary key is the WHERE when it has one. A keyless target gets every column
/// instead, with a note saying so — a filter that silently matched nothing would look like a
/// broken link rather than a table without a key.
fn follow_foreign_key(session: &mut Session, p: &Params) -> Result<serde_json::Value> {
    let index = active_grid_tab(session)?;
    let row = p.int_opt("row", 0).max(0) as usize;
    let column = p.str_arg("column")?;
    let (conn, table, value) = {
        let tab = &session.tabs[index];
        let row_count = tab.grid.result.rows.len();
        if row >= row_count {
            return Err(Error::not_found(format!(
                "no row {row} on this page (it holds {row_count} rows)"
            )));
        }
        let conn = tab
            .conn
            .ok_or_else(|| Error::config("this data tab is not bound to a connection"))?;
        let table = tab
            .grid
            .table
            .clone()
            .ok_or_else(|| Error::config("this data tab is not bound to a table"))?;
        let value = tab
            .grid
            .result
            .value_at(row, &column)
            .cloned()
            .ok_or_else(|| {
                Error::not_found(format!(
                    "no column '{column}': this table holds {}",
                    tab.grid.result.columns.join(", ")
                ))
            })?;
        (conn, table, value)
    };
    let source = session.table_of(conn, table.schema.as_deref(), &table.table)?;
    let fk = source
        .foreign_keys
        .iter()
        .find(|fk| fk.columns.iter().any(|c| c == &column))
        .ok_or_else(|| {
            Error::config(format!("{column} is not a foreign key of {}", table.key()))
        })?;
    if value == Value::Null {
        return Err(Error::config(format!(
            "{column} is NULL on row {row}: there is no row to follow"
        )));
    }
    let target = session.table_of(conn, fk.ref_schema.as_deref(), &fk.ref_table)?;
    let dialect = session.driver_for(conn)?.dialect();
    let key_columns = target.primary_key();
    let reference_column = fk
        .ref_columns
        .first()
        .ok_or_else(|| Error::internal("a foreign key with no referenced column"))?
        .clone();
    let mut out = json!({
        "conn": session.conns.get(conn).map(|c| c.cfg.id.clone()).unwrap_or_default(),
        "schema": fk.ref_schema,
        "table": fk.ref_table,
    });
    if !key_columns.is_empty() {
        // A compound key needs every column's value and a page row holds one, so only a
        // single-column key resolves; anything else says why instead of guessing.
        if key_columns.len() != 1 || fk.columns.len() != 1 {
            return Err(Error::unsupported(format!(
                "{} is part of a compound key; Nook would need every column's value",
                fk.name
            )));
        }
        out["where"] = json!(format!(
            "{} = {}",
            nook_core::quote_ident(&key_columns[0], dialect),
            nook_core::sql_literal(&value, dialect)
        ));
        return Ok(out);
    }
    // No key on the target: match on every column of the one row the reference names. The row is
    // read first, because a WHERE built from column names alone would match nothing.
    let sql = format!(
        "SELECT * FROM {} WHERE {} = {} LIMIT 1",
        nook_core::quote_qualified(fk.ref_schema.as_deref(), &fk.ref_table, dialect),
        nook_core::quote_ident(&reference_column, dialect),
        nook_core::sql_literal(&value, dialect)
    );
    let found = session.driver_for(conn)?.run(&sql, 1)?;
    let Some(row) = found.rows.first() else {
        return Err(Error::not_found(format!(
            "no row in {} with {} = {}",
            fk.ref_table,
            reference_column,
            value.display()
        )));
    };
    let where_clause = found
        .columns
        .iter()
        .enumerate()
        .map(|(i, name)| {
            // `col = NULL` matches nothing in every dialect, so a row with a NULL column would
            // produce a filter that selects no rows at all. `IS NULL` is the only honest form.
            let value = row.get(i).unwrap_or(&Value::Null);
            let ident = nook_core::quote_ident(name, dialect);
            match value {
                Value::Null => format!("{ident} IS NULL"),
                other => format!("{ident} = {}", nook_core::sql_literal(other, dialect)),
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ");
    out["where"] = json!(where_clause);
    out["note"] = json!(format!(
        "{} has no primary key — matching on every column",
        fk.ref_table
    ));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nook_core::Value;
    use nook_model::{Column, ResultSet};

    fn grid_tab() -> (Session, usize) {
        let mut session = Session::new();
        let conn = nook_core::ConnectionConfig::new("mem", nook_core::DriverKind::Sqlite);
        let mut conn = crate::Conn::new(conn);
        // The grid's columns and key are read from the catalog, so the connection carries the same
        // table the tab claims to show: a tab bound to nothing would edit nothing.
        let mut id = Column::new("id", "INTEGER");
        id.primary_key = true;
        let mut users = nook_model::Table::new(None, "users");
        users.columns = vec![id, Column::new("email", "TEXT")];
        conn.catalog = Some(nook_model::Catalog {
            driver: "SQLite".into(),
            schemas: vec![nook_model::Schema {
                name: "main".into(),
                tables: vec![users],
                ..Default::default()
            }],
            ..nook_model::Catalog::default()
        });
        session.conns.push(conn);
        let index = session.add_tab(TabKind::Data, "users", Some(0));
        let tab = &mut session.tabs[index];
        tab.grid.result = ResultSet {
            columns: vec!["id".into(), "email".into()],
            column_types: vec!["INTEGER".into(), "TEXT".into()],
            rows: vec![
                vec![Value::Int(1), Value::text("a@b.c")],
                vec![Value::Int(2), Value::text("d@e.f")],
            ],
            ..ResultSet::default()
        };
        tab.grid.table = Some(TableRef {
            schema: None,
            table: "users".into(),
        });
        tab.grid.primary_key = vec!["id".into()];
        tab.grid.columns = vec![Column::new("id", "INTEGER"), Column::new("email", "TEXT")];
        tab.payload.insert("dialect".into(), json!("sqlite"));
        let index = session.tabs.len() - 1;
        (session, index)
    }

    /// One page of a table is one statement, and it is written in one place: the grid's `SELECT`
    /// and the inspector's preview must never drift apart.
    #[test]
    fn a_page_select_quotes_the_table_and_pages_it() {
        let table = TableRef {
            schema: Some("main".into()),
            table: "customers".into(),
        };
        // SQLite has no schemas of its own, so the schema drops out there and stays in Postgres.
        assert_eq!(
            crate::cmd::page_select(
                &table,
                nook_core::Dialect::Sqlite,
                "  status = 'trial'  ",
                " id DESC ",
                50,
                100,
            ),
            "SELECT * FROM \"customers\" WHERE status = 'trial' ORDER BY id DESC LIMIT 50 OFFSET 100"
        );
        assert_eq!(
            crate::cmd::page_select(&table, nook_core::Dialect::Postgres, "", "", 50, 100,),
            "SELECT * FROM \"main\".\"customers\" LIMIT 50 OFFSET 100"
        );
        // No filter, no order, no schema: the plain first page.
        let bare = TableRef {
            schema: None,
            table: "users".into(),
        };
        assert_eq!(
            crate::cmd::page_select(&bare, nook_core::Dialect::Sqlite, "", "", 0, 0),
            "SELECT * FROM \"users\" LIMIT 1 OFFSET 0"
        );
    }

    #[test]
    fn editing_a_cell_records_sql_and_coerces_types() {
        let (mut session, _) = grid_tab();
        let p =
            Params::from_json(json!({"row": 0, "column": "id", "value": "42"})).expect("params");
        let out = edit_cell(&mut session, &p).expect("edit");
        assert_eq!(out["pending"], 1);
        assert!(out["sql"]
            .as_str()
            .unwrap_or("")
            .contains("SET \"id\" = 42"));
        assert!(out["sql"]
            .as_str()
            .unwrap_or("")
            .contains("WHERE \"id\" = 1"));
    }

    #[test]
    fn editing_without_a_primary_key_is_refused() {
        let (mut session, tab) = grid_tab();
        // The key comes from the catalog, so a keyless table is one the catalog has no key for —
        // and the set of columns the tab would plan against has to match it, or the page is reloaded.
        let table = session.conns[0]
            .catalog
            .as_mut()
            .and_then(|c| c.schemas.get_mut(0))
            .and_then(|s| s.tables.first_mut())
            .expect("the fixture holds a table");
        table.columns[0].primary_key = false;
        session.tabs[tab].grid.primary_key.clear();
        let p =
            Params::from_json(json!({"row": 0, "column": "email", "value": "x"})).expect("params");
        let err = edit_cell(&mut session, &p).expect_err("a keyless table is refused");
        assert!(
            err.to_string().contains("primary key"),
            "the refusal has to name the reason: {err}"
        );
    }

    #[test]
    fn load_grid_rebinds_the_key_after_set_primary_key() {
        // The schema path of bug 2: the catalog gains a key, then a reload must pick it up —
        // otherwise the grid the "Open design table" banner routes the user to fix stays read-only.
        let (mut session, tab) = grid_tab();
        // A table without a key, in both the catalog and the tab.
        let table = session.conns[0]
            .catalog
            .as_mut()
            .and_then(|c| c.schemas.get_mut(0))
            .and_then(|s| s.tables.first_mut())
            .expect("the fixture holds a table");
        table.columns[0].primary_key = false;
        session.tabs[tab].grid.primary_key.clear();
        // The catalog edit that `schema.setPrimaryKey` performs through refresh_catalog.
        session.conns[0].catalog.as_mut().unwrap().schemas[0].tables[0].columns[0].primary_key =
            true;
        // A real in-memory SQLite behind the connection: the reload actually runs a page SELECT.
        let mut db = nook_drivers::sqlite::Sqlite::memory().expect("memory db");
        nook_drivers::Driver::run_script(
            &mut db,
            "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT);\
             INSERT INTO users (id, email) VALUES (1, 'a@b.c'), (2, 'd@e.f');",
            10,
        )
        .expect("setup");
        session.conns[0].driver = Some(Box::new(db));
        crate::cmd::load_grid(&mut session, tab).expect("reload");
        assert_eq!(session.tabs[tab].grid.primary_key, vec!["id".to_string()]);
        assert!(
            session.tabs[tab].grid.editable(),
            "the grid must be editable again"
        );
    }

    #[test]
    fn revert_and_pending_report_the_buffer() {
        let (mut session, tab) = grid_tab();
        let p =
            Params::from_json(json!({"row": 1, "column": "email", "value": "z"})).expect("params");
        edit_cell(&mut session, &p).expect("edit");
        let out = pending(&mut session, &Params::none()).expect("pending");
        assert_eq!(out["dirty"], true);
        let out = revert(&mut session, &Params::none()).expect("revert");
        assert_eq!(out["discarded"], 1);
        assert!(!session.tabs[tab].grid.dirty());
    }

    #[test]
    fn exporting_the_grid_writes_a_file() {
        let (mut session, _) = grid_tab();
        let dir = std::env::temp_dir().join("nook_grid_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("rows.csv").to_string_lossy().into_owned();
        let p = Params::from_json(json!({"format": "csv", "path": path})).expect("params");
        let out = export(&mut session, &p).expect("export");
        assert_eq!(out["lines"], 3);
        assert!(std::fs::read_to_string(&path)
            .expect("read")
            .contains("a@b.c"));
        let _ = std::fs::remove_file(&path);
    }

    /// The sentence a surface that cannot run a grid command quotes is the one the command answers
    /// with: the results register's Export is painted disabled on a query tab, and the reason it
    /// hovers is this string, read from here rather than copied.
    #[test]
    fn the_no_grid_refusal_is_the_sentence_export_answers_with() {
        let mut session = Session::new();
        let index = session.add_tab(TabKind::Query, "Query", None);
        session.active_tab = Some(index);
        let p = Params::from_json(json!({"format": "csv"})).expect("params");
        let e = export(&mut session, &p).expect_err("a query tab has no data grid to export");
        assert_eq!(e.message(), no_grid_reason());
    }
    /// The form must show what `grid.apply` will send, not the page as it was fetched: an edit
    /// recorded through `grid.editCell` overlays the cell the form paints, or the edit looks lost
    /// until the apply and the refetch.
    #[test]
    fn the_form_view_shows_pending_edits_over_the_page() {
        let (mut session, _) = grid_tab();
        session
            .run(
                "grid.editCell",
                json!({"row": 1, "column": "email", "value": "new@e.f"}),
            )
            .expect("edit");
        let out = session
            .run("grid.formView", json!({"row": 1}))
            .expect("form view");
        assert_eq!(out["fields"][1]["value"], "new@e.f", "{:#?}", out);
        assert_eq!(out["fields"][1]["display"], "new@e.f");
        // A row the edit does not touch still shows the page value.
        let other = session
            .run("grid.formView", json!({"row": 0}))
            .expect("form view");
        assert_eq!(other["fields"][1]["value"], "a@b.c");
    }

    /// The form paints a pending edit onto the record the edit was made on, not onto the row it was
    /// made at. `grid.sort`, a filter and a page turn all reload the page under the pending buffer, and
    /// the apply keys its statement by the record — the overlay has to follow the same rule, or the form
    /// shows one record's edit on another record. The end-to-end version of this, through a real sort
    /// against a real database, is `an_edit_applied_after_a_sort_lands_on_the_record_it_was_made_on`.
    #[test]
    fn the_form_view_paints_a_pending_edit_on_its_own_record_after_the_page_moves() {
        let (mut session, tab) = grid_tab();
        session
            .run(
                "grid.editCell",
                json!({"row": 0, "column": "email", "value": "z@z.z"}),
            )
            .expect("edit");
        // What sorting descending by email does to the page: the same two records, the other way round.
        session.tabs[tab].grid.result.rows.swap(0, 1);

        // Row 0 holds id 2 now, which nobody edited, so it shows its own email.
        let other = session
            .run("grid.formView", json!({"row": 0}))
            .expect("form view");
        assert_eq!(other["fields"][1]["value"], "d@e.f", "{:#?}", other);
        // Row 1 holds the record the edit was made on, so it shows the edit.
        let moved = session
            .run("grid.formView", json!({"row": 1}))
            .expect("form view");
        assert_eq!(moved["fields"][1]["value"], "z@z.z", "{:#?}", moved);
    }

    /// The last edit to a cell wins, matching what `grid.apply` sends: the pending list is applied
    /// in order, so a second edit to the same cell must not be hidden by the first.
    #[test]
    fn the_form_view_prefers_the_last_pending_edit_to_a_cell() {
        let (mut session, _) = grid_tab();
        for email in ["one@e.f", "two@e.f"] {
            session
                .run(
                    "grid.editCell",
                    json!({"row": 0, "column": "email", "value": email}),
                )
                .expect("edit");
        }
        let out = session
            .run("grid.formView", json!({"row": 0}))
            .expect("form view");
        assert_eq!(out["fields"][1]["value"], "two@e.f");
    }

    #[test]
    fn find_scans_visible_cells() {
        let (mut session, _) = grid_tab();
        let p = Params::from_json(json!({"text": "e.f"})).expect("params");
        let out = find(&mut session, &p).expect("find");
        assert_eq!(out["matches"].as_array().map(Vec::len), Some(1));
        let p = Params::from_json(json!({"text": "nothing"})).expect("params");
        assert_eq!(
            find(&mut session, &p).expect("find")["matches"]
                .as_array()
                .map(Vec::len),
            Some(0)
        );
    }

    /// `grid.countRows` counts the rows the grid is currently asking for, so the moment the filter,
    /// the order or the rows themselves change the old count stops describing the page: it must not
    /// be kept and printed as the new match total. `set_page_size` and the pager reload the same
    /// slice, so they leave it standing deliberately — only the commands that change *which* rows
    /// are asked for clear it.
    #[test]
    fn changing_the_filter_or_the_order_drops_a_stale_row_count() {
        let (mut session, tab) = grid_tab();
        session.tabs[tab].grid.total_rows = Some(12_481);
        let p = Params::from_json(json!({"column": "id"})).expect("params");
        sort(&mut session, &p).expect("sort");
        assert!(session.tabs[tab].grid.total_rows.is_none());

        session.tabs[tab].grid.total_rows = Some(12_481);
        let p = Params::from_json(json!({"where": "status = 'live'"})).expect("params");
        let out = filter(&mut session, &p).expect("filter");
        assert!(session.tabs[tab].grid.total_rows.is_none());
        assert!(
            out["total_rows"].is_null(),
            "the summary printed the pre-filter count as the current match total"
        );

        session.tabs[tab].grid.total_rows = Some(12_481);
        clear_filter(&mut session, &Params::none()).expect("clear");
        assert!(session.tabs[tab].grid.total_rows.is_none());
    }
    /// The form view is one record as a vertical form: every column with its type, its value and
    /// whether it is a key. It reads the page the grid already holds, so it asks the server nothing.
    #[test]
    fn the_form_view_reports_one_record_as_fields() {
        let (mut session, _) = grid_tab();
        let out = session
            .run("grid.formView", json!({"row": 1}))
            .expect("form view");
        assert_eq!(out["row"], 1);
        assert_eq!(out["editable"], true);
        let fields = out["fields"].as_array().expect("a field per column");
        assert_eq!(fields.len(), 2, "{:#?}", fields);
        assert_eq!(fields[0]["name"], "id");
        assert_eq!(fields[0]["type"], "INTEGER");
        assert_eq!(fields[0]["value"], 2);
        assert_eq!(fields[0]["primary_key"], true);
        assert_eq!(fields[1]["name"], "email");
        assert_eq!(fields[1]["value"], "d@e.f");
        assert_eq!(fields[1]["primary_key"], false);
    }

    /// A row the page does not hold is an error, never a panic and never a silent empty form: the
    /// grid would then edit a record that is not there.
    #[test]
    fn the_form_view_rejects_a_row_the_page_does_not_hold() {
        let (mut session, _) = grid_tab();
        assert!(session.run("grid.formView", json!({"row": 99})).is_err());
    }

    #[test]
    fn counting_rows_without_a_connection_is_an_error() {
        let (mut session, _) = grid_tab();
        let e = count_rows(&mut session, &Params::none());
        assert!(e.is_err());
    }

    /// A saved width must come back when the table is bound again — the whole point of persisting
    /// it. The store keeps a bare `{name: width}` object; a wrapper key would read back as one entry
    /// named after the wrapper and every width would silently vanish on reopen.
    #[test]
    fn column_widths_survive_a_store_round_trip() {
        let _guard = crate::test_support::scratch_config();
        let (mut session, index) = grid_tab();
        let table = TableRef {
            schema: None,
            table: "users".into(),
        };
        crate::cmd::bind_grid_table(&mut session, index, table.clone()).expect("bind");
        session.active_tab = Some(index);

        let p = Params::from_json(json!({"widths": {"email": 320.0, "id": 90.0}})).expect("params");
        let out = column_widths_cmd(&mut session, &p).expect("save widths");
        assert_eq!(out["widths"]["email"], 320.0);
        assert_eq!(out["widths"]["id"], 90.0);

        // The cache the paint path reads moved with the store.
        let cached = &session.tabs[index].grid.widths;
        assert_eq!(cached.len(), 2, "both widths cached: {cached:?}");

        // A fresh read through the same key the bind uses — this is what a reopened tab sees.
        let conn_id = session.conns[0].cfg.id.clone();
        let stored = crate::cmd::column_widths(&conn_id, &table).expect("read widths");
        assert_eq!(
            stored.len(),
            2,
            "the stored object must parse as a bare name→width map, got {stored:?}"
        );
        assert!(stored.iter().any(|(n, w)| n == "email" && *w == 320.0));

        // And the whole path end to end: rebinding the table restores them into the cache.
        session.tabs[index].grid.widths.clear();
        crate::cmd::bind_grid_table(&mut session, index, table).expect("rebind");
        assert_eq!(session.tabs[index].grid.widths.len(), 2);
    }

    /// A width stored for a column the table no longer has must not survive the bind: the UI
    /// re-sends the whole cache on every drag, and this command refuses an unknown name, so one
    /// stale entry would make every later resize fail to save.
    #[test]
    fn a_width_for_a_dropped_column_is_not_rebound() {
        let _guard = crate::test_support::scratch_config();
        let (mut session, index) = grid_tab();
        let table = TableRef {
            schema: None,
            table: "users".into(),
        };
        // Written straight to the store: the command itself would refuse the name.
        crate::store::save_ui_state(
            &format!("widths:{}:{}", session.conns[0].cfg.id, table.key()),
            r#"{"email":320.0,"gone":200.0}"#,
        )
        .expect("seed");

        crate::cmd::bind_grid_table(&mut session, index, table).expect("bind");
        let cached = &session.tabs[index].grid.widths;
        assert_eq!(
            cached.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
            vec!["email"],
            "only the column the table still has survives: {cached:?}"
        );
    }
}
