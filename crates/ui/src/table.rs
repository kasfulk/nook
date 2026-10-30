//! One table widget for every place a result set is shown: the data grid, query results, monitor
//! tables and the log. It is painted by hand instead of built from `egui::Grid` so that a row can
//! carry a background, a hover highlight, a dirty marker and per-cell hit testing.
//!
//! Layout: horizontal scroll wraps the header *and* the body so they move together while the header
//! stays put vertically; the body scrolls on its own below it.

use egui::{Align2, Color32, CornerRadius, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use nook_core::Value;

use crate::theme::{self as design, Tokens, HEAD_H};

const GUTTER_W: f32 = 54.0;
const CELL_PAD: f32 = 9.0;
const MIN_COL_W: f32 = 76.0;
const MAX_COL_W: f32 = 280.0;
const MAX_DRAG_W: f32 = 2000.0;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TableColumn {
    pub name: String,
    /// What the head paints: the name without a `__format__` marker.
    pub label: String,
    pub type_name: String,
    pub primary_key: bool,
    /// The display hint the column name carried, if any: `money`, `check` or `link`.
    pub hint: Option<String>,
}

impl TableColumn {
    pub fn new(name: impl Into<String>, type_name: impl Into<String>) -> Self {
        let name = name.into();
        // The marker is a display instruction, not part of the header: `price__format__money`
        // heads as `price`. `format_hint` is the one owner, shared with `grid.hints`.
        let (label, hint) = match nook_model::format_hint(&name) {
            Some((label, kind)) => (label.to_owned(), Some(kind.to_owned())),
            None => (name.clone(), None),
        };
        TableColumn {
            name,
            label,
            type_name: type_name.into(),
            primary_key: false,
            hint,
        }
    }

    pub fn pk(mut self, primary_key: bool) -> Self {
        self.primary_key = primary_key;
        self
    }

    fn numeric(&self) -> bool {
        let t = self.type_name.to_ascii_uppercase();
        [
            "INT", "SERIAL", "NUMERIC", "DECIMAL", "REAL", "DOUBLE", "FLOAT", "MONEY",
        ]
        .iter()
        .any(|k| t.contains(k))
    }
}

/// Per-table view state that has to survive across frames.
#[derive(Debug, Clone, Default)]
pub struct TableState {
    /// The cursor cell of the selection: the live corner, and what "the selected cell" means
    /// to every caller that wants one cell.
    pub selected: Option<(usize, usize)>,
    /// The drag origin of the selection rectangle. A plain click collapses it onto `selected`.
    pub anchor: Option<(usize, usize)>,
    pub editing: Option<(usize, usize)>,
    pub buffer: String,
    focus_editor: bool,
}

impl TableState {
    /// Open the inline editor on a cell.
    pub fn begin_edit(&mut self, row: usize, column: usize, value: String) {
        self.editing = Some((row, column));
        self.buffer = value;
        self.focus_editor = true;
    }

    pub fn stop_edit(&mut self) {
        self.editing = None;
        self.buffer.clear();
        self.focus_editor = false;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TableEvent {
    None,
    Select(usize, usize),
    /// The pointer moved the live corner of the rectangle: the anchor stays, the cursor moves.
    SelectExtend(usize, usize),
    BeginEdit(usize, usize),
    Commit(usize, usize, String),
    Cancel,
    Sort(String),
    /// Hide one column, by name (the head's context menu).
    HideColumn(String),
    /// Un-hide every hidden column of this table (the head's context menu).
    ShowAllColumns,
    /// Follow the foreign key in this cell: (row, column).
    FollowForeignKey(usize, usize),
    /// The user dragged a column's right edge to a new width.
    ResizeColumn(String, f32),
    /// The user released the drag; the caller should persist the width.
    ResizeEnd,
}

/// Right-click handler for one table row: the row index in, menu items out.
pub type RowMenu<'a> = &'a mut dyn FnMut(&mut egui::Ui, usize);

pub struct TableView<'a> {
    // `editable` is the widget's read-only switch: false for the inspector's preview, for the results
    // of a statement, and for a grid whose table the engine would refuse to edit (no primary key).
    // It gates the inline editor and the dirty stripe, so a read-only view cannot look editable.
    pub columns: &'a [TableColumn],
    /// Columns whose value points at another table. Their cells paint as links (accent, underlined)
    /// and a click asks the caller to follow the reference instead of selecting the cell.
    pub foreign_keys: &'a [String],
    /// Column names hidden from view. They keep their slot at width zero — indices into
    /// `rows` stay result indices, so painting and commits land on the right cell.
    pub hidden: &'a [String],
    pub rows: &'a [Vec<Value>],
    /// Cells with an unsaved edit, as (row, column).
    pub pending: Vec<(usize, usize)>,
    /// Rows that are new and not yet written to the server.
    pub inserted: Vec<usize>,
    pub selected: Option<(usize, usize)>,
    /// The drag origin of the selection rectangle, painted as `tokens.selection` over the
    /// rectangle with the cursor cell stronger.
    pub anchor: Option<(usize, usize)>,
    pub sort: Option<(String, bool)>,
    pub editable: bool,
    pub sortable: bool,
    /// Row-number column on the left.
    pub gutter: bool,
    /// Height of the scrolling body. `0` for a table that has no body at all: the head is painted on
    /// its own — still scrollable sideways — and the caller owns the space under it (the grid's
    /// `.empty` block does).
    pub height: f32,
    /// Row height: the density setting through `theme::row_h`.
    pub row_h: f32,
    pub empty_text: &'a str,
    /// Right-click menu for a row, as `Some(callback)` — called with the row index. `None` for the
    /// default of no menu; the design tab's column list uses it to open the per-column editors.
    pub row_menu: Option<RowMenu<'a>>,
    /// Width overrides from a user drag, as `(column_name, width)`. The widget honours these over
    /// content measurement and the numeric cap, so a drag is the authority.
    pub widths: &'a [(String, f32)],
}

fn text_width(ui: &Ui, text: &str, font: egui::FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font, Color32::PLACEHOLDER)
        .size()
        .x
}

/// `12481` reads as `12,481`: counts are set in tabular figures everywhere the design shows them.
pub fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// `1 column` but `2 columns`.
pub fn plural(n: u64, one: &str, many: &str) -> String {
    if n == 1 {
        one.to_owned()
    } else {
        many.to_owned()
    }
}

/// Truncate to `max` characters with an ellipsis; used by the tree and the tab strip.
pub fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// Truncate `text` to the width `max` with an ellipsis, so a cell or a pill prints what its own rect
/// has room for rather than overrunning it. The first run of text that fits is kept, so a caller that
/// reserves the width of the ellipsis is never given more than it asked for.
pub fn ellipsize(ui: &Ui, text: &str, font: egui::FontId, max: f32) -> String {
    if text_width(ui, text, font.clone()) <= max {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate: String = format!("{}…", chars.iter().collect::<String>());
        if text_width(ui, &candidate, font.clone()) <= max {
            return candidate;
        }
    }
    "…".to_owned()
}

fn total_width(widths: &[f32]) -> f32 {
    widths.iter().sum()
}

/// A column is numeric when its values are, which is more reliable than the declared type:
/// `id INTEGER PRIMARY KEY` arrives as `INTEGER`, a computed sum as nothing at all.
fn numeric_column(rows: &[Vec<Value>], index: usize) -> bool {
    let mut seen = false;
    for value in rows.iter().filter_map(|row| row.get(index)) {
        match value {
            Value::Null => continue,
            Value::Int(_) | Value::UInt(_) | Value::Float(_) => seen = true,
            _ => return false,
        }
    }
    seen
}

/// What a cell shows. A hint changes the painted text and never the value, so the width
/// measurement must call this too: `12.5` widens to `12.50` and `1` becomes one glyph, and a
/// column sized from `display()` would clip or overlap the cell it then paints.
fn hinted(column: &TableColumn, value: &Value) -> String {
    match column.hint.as_deref() {
        Some("money") => match value {
            Value::Null => value.display(),
            Value::Float(f) => format!("{f:.2}"),
            Value::Int(i) => format!("{:.2}", *i as f64),
            other => other.display(),
        },
        // U+2714 and U+00D7: the only tick/cross pair the bundled fonts actually carry
        // (U+2713 and U+2717 paint as tofu).
        Some("check") => match value {
            Value::Null => "—".to_owned(),
            Value::Bool(true) => "✔".to_owned(),
            Value::Bool(false) => "×".to_owned(),
            // A check column is often an int 0/1 (SQLite has no bool).
            Value::Int(i) => if *i != 0 { "✔" } else { "×" }.to_owned(),
            Value::Text(text) => {
                let t = text.trim().to_ascii_lowercase();
                if matches!(t.as_str(), "1" | "t" | "true" | "y" | "yes") {
                    "✔".to_owned()
                } else {
                    "×".to_owned()
                }
            }
            _ => "×".to_owned(),
        },
        _ => value.display(),
    }
}

/// The URL a cell would open, if any: only `http(s)` reaches the OS handler, so a `javascript:`
/// or `file:` value in a `__format__link` column is inert text rather than a handed-off string.
fn openable_url(value: &Value) -> Option<&str> {
    match value {
        Value::Text(url) | Value::Json(url)
            if url.starts_with("http://") || url.starts_with("https://") =>
        {
            Some(url)
        }
        _ => None,
    }
}

fn column_widths(ui: &Ui, tokens: &Tokens, view: &TableView<'_>) -> Vec<f32> {
    let header_font = tokens.font(12.5);
    let cell_font = tokens.mono(12.5);
    let badge = design::pk_badge_width(ui, tokens, "PK");
    view.columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            if view.hidden.contains(&column.name) {
                // Hidden keeps the slot but not the space: width zero, nothing painted.
                return 0.0;
            }
            // A width override from a user drag wins over content measurement and the numeric cap:
            // the user's drag is the authority, not the longest value in the column.
            if let Some((_, override_w)) = view.widths.iter().find(|(name, _)| name == &column.name)
            {
                return override_w.clamp(MIN_COL_W, MAX_DRAG_W);
            }
            let mut width =
                text_width(ui, &column.label, header_font.clone()) + CELL_PAD * 2.0 + 18.0;
            if column.primary_key {
                // The `.pkbadge` itself plus the design's `gap: var(--s2)` before it.
                width += badge + f32::from(design::S2);
            }
            let longest = view
                .rows
                .iter()
                .take(400)
                .filter_map(|row| row.get(index))
                .map(|value| {
                    text_width(ui, &hinted(column, value), cell_font.clone()) + CELL_PAD * 2.0
                })
                .fold(0.0_f32, f32::max);
            width = width.max(longest).clamp(MIN_COL_W, MAX_COL_W);
            if column.numeric() || numeric_column(view.rows, index) {
                width = width.min(160.0);
            }
            width
        })
        .collect()
}

/// Paint the table and report what the user did. At most one event per frame.
pub fn show(
    ui: &mut Ui,
    tokens: &Tokens,
    view: &mut TableView<'_>,
    state: &mut TableState,
    id: egui::Id,
) -> TableEvent {
    if view.columns.is_empty() {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(view.empty_text)
                    .size(12.5)
                    .color(tokens.text_faint),
            );
        });
        return TableEvent::None;
    }

    let widths = column_widths(ui, tokens, view);
    // Rows and gridlines span the whole viewport, so a narrow table does not leave dead stripes.
    let viewport_w = ui.available_width().max(total_width(&widths));
    if std::env::var("NOOK_TABLE_DEBUG").is_ok() {
        eprintln!(
            "table: cols={} rows={} widths={:?} avail={:?} height={}",
            view.columns.len(),
            view.rows.len(),
            widths,
            ui.available_size(),
            view.height
        );
    }
    let gutter_w = if view.gutter { GUTTER_W } else { 0.0 };
    let total_w = total_width(&widths) + gutter_w;

    if view.height <= 0.0 {
        // No body, so nothing to scroll vertically — but the head alone must still move sideways,
        // or a table wider than the pane is clipped with no bar to reach the columns past the edge.
        // The area's height is the head's own, so the caller's `.empty` block still lands 28px under
        // the hairline; `auto_shrink` collapses the viewport to the content and the floor below only
        // keeps it from egui's 64px default should the bar style ever stop floating.
        return egui::ScrollArea::horizontal()
            .id_salt(id.with("h"))
            .auto_shrink([false, true])
            .min_scrolled_height(HEAD_H)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                head(ui, tokens, view, &widths, viewport_w, total_w, id)
            })
            .inner;
    }

    let mut event = TableEvent::None;
    egui::ScrollArea::horizontal()
        .id_salt(id.with("h"))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            event = head(ui, tokens, view, &widths, viewport_w, total_w, id);

            egui::ScrollArea::vertical()
                .id_salt(id.with("v"))
                .max_height(view.height)
                .auto_shrink([false, false])
                // A cell drag paints a rectangle; letting the area also claim the drag for
                // scrolling would do both at once. The wheel and the bar still scroll.
                .scroll_source(egui::scroll_area::ScrollSource {
                    drag: egui::scroll_area::DragScroll::Never,
                    ..Default::default()
                })
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    // Every visible cell's rect, so a drag that leaves the cell it started on can
                    // still be mapped to the cell under the pointer: egui routes the drag to the
                    // widget that was pressed, so the neighbours never see it themselves.
                    let mut cells: Vec<(usize, usize, Rect)> = Vec::new();
                    let mut dragging = false;
                    for (row_index, row) in view.rows.iter().enumerate() {
                        let (row_rect, row_response) =
                            ui.allocate_exact_size(Vec2::new(total_w, view.row_h), Sense::click());
                        let full = egui::Rect::from_min_max(
                            row_rect.min,
                            egui::pos2(row_rect.left() + viewport_w, row_rect.bottom()),
                        );
                        let is_new = view.inserted.contains(&row_index);
                        // With a rectangle in play (anchor and cursor differ), the rectangle is
                        // the selection; the full-row bar would paint over it.
                        let rect_active =
                            view.selected.zip(view.anchor).is_some_and(|(s, a)| s != a);
                        let selected_row = !rect_active
                            && view.selected.map(|(r, _)| r == row_index).unwrap_or(false);

                        if selected_row {
                            ui.painter()
                                .rect_filled(full, CornerRadius::ZERO, tokens.selection);
                        } else if is_new {
                            ui.painter().rect_filled(
                                full,
                                CornerRadius::ZERO,
                                tokens.accent_dim.gamma_multiply(0.7),
                            );
                        } else if row_index % 2 == 1 {
                            ui.painter()
                                .rect_filled(full, CornerRadius::ZERO, tokens.grid_row_alt);
                        }
                        if row_response.hovered() && !selected_row {
                            ui.painter()
                                .rect_filled(full, CornerRadius::ZERO, tokens.hover);
                        }
                        if view.gutter {
                            let gutter = egui::Rect::from_min_size(
                                row_rect.min,
                                Vec2::new(gutter_w, view.row_h),
                            );
                            ui.painter().rect_filled(
                                gutter,
                                CornerRadius::ZERO,
                                tokens.panel_alt.gamma_multiply(0.6),
                            );
                        }
                        if is_new {
                            ui.painter().rect_filled(
                                Rect::from_min_size(row_rect.min, Vec2::new(2.0, view.row_h)),
                                CornerRadius::ZERO,
                                tokens.accent,
                            );
                        }

                        // Gutter: row number, plus a dot when the row has pending edits.
                        if view.gutter {
                            let gutter =
                                Rect::from_min_size(row_rect.min, Vec2::new(gutter_w, view.row_h));
                            ui.painter().text(
                                egui::pos2(gutter.right() - CELL_PAD, gutter.center().y),
                                Align2::RIGHT_CENTER,
                                format!("{}", row_index + 1),
                                tokens.mono(11.0),
                                tokens.text_faint,
                            );
                            if view.pending.iter().any(|(r, _)| *r == row_index) {
                                ui.painter().circle_filled(
                                    egui::pos2(gutter.left() + 10.0, gutter.center().y),
                                    3.0,
                                    tokens.warn,
                                );
                            }
                            ui.painter().vline(
                                gutter.right() - 0.5,
                                gutter.top()..=gutter.bottom(),
                                Stroke::new(1.0, tokens.grid_line),
                            );
                        }

                        let mut x = row_rect.left() + gutter_w;
                        for (column_index, column) in view.columns.iter().enumerate() {
                            // Hidden: advance the slot, paint and interact nothing. The x
                            // cursor moves by zero, so the next visible column stays put.
                            if widths[column_index] <= 0.0 {
                                continue;
                            }
                            let cell = Rect::from_min_size(
                                egui::pos2(x, row_rect.top()),
                                Vec2::new(widths[column_index], view.row_h),
                            );
                            cells.push((row_index, column_index, cell));
                            // An editor only exists while the grid can be edited: a reload that
                            // fails clears the result under an open editor, and a `Commit` from it
                            // would be refused by the engine.
                            let editing =
                                view.editable && state.editing == Some((row_index, column_index));
                            let dirty = view.editable
                                && (view.pending.contains(&(row_index, column_index)) || is_new);
                            let value = row.get(column_index).cloned().unwrap_or(Value::Null);
                            let response = ui.interact(
                                cell,
                                id.with(("cell", row_index, column_index)),
                                Sense::click_and_drag(),
                            );
                            let (cursor, anchor) = (view.selected, view.anchor);
                            let in_rect = cursor.zip(anchor).is_some_and(|(c, a)| {
                                row_index >= a.0.min(c.0)
                                    && row_index <= a.0.max(c.0)
                                    && column_index >= a.1.min(c.1)
                                    && column_index <= a.1.max(c.1)
                            }) || cursor == Some((row_index, column_index));
                            let cell_selected = in_rect;

                            if cell_selected && !selected_row {
                                let fill = if cursor == Some((row_index, column_index)) {
                                    // The cursor cell is the strong corner: it must read as
                                    // "this is the cell", not just part of the rectangle.
                                    tokens.selection_strong
                                } else {
                                    tokens.selection
                                };
                                ui.painter().rect_filled(cell, CornerRadius::ZERO, fill);
                            }
                            if dirty {
                                ui.painter().rect_filled(
                                    cell,
                                    CornerRadius::ZERO,
                                    tokens.dirty.gamma_multiply(0.55),
                                );
                                ui.painter().rect_filled(
                                    Rect::from_min_size(cell.min, Vec2::new(2.0, cell.height())),
                                    CornerRadius::ZERO,
                                    tokens.warn,
                                );
                            }
                            if response.hovered() && !editing {
                                ui.painter().rect_stroke(
                                    cell,
                                    CornerRadius::ZERO,
                                    Stroke::new(1.0, tokens.stroke_strong),
                                    StrokeKind::Inside,
                                );
                            }

                            if editing {
                                let inner = cell.shrink2(Vec2::new(3.0, 2.0));
                                let editor = ui.put(
                                    inner,
                                    egui::TextEdit::singleline(&mut state.buffer)
                                        .font(tokens.mono(12.5))
                                        .margin(Vec2::new(4.0, 1.0))
                                        .desired_width(inner.width()),
                                );
                                if state.focus_editor {
                                    editor.request_focus();
                                    state.focus_editor = false;
                                }
                                let entered = ui.input(|i| i.key_pressed(egui::Key::Enter));
                                let escaped = ui.input(|i| i.key_pressed(egui::Key::Escape));
                                if escaped {
                                    event = TableEvent::Cancel;
                                } else if entered
                                    || (editor.lost_focus() && event == TableEvent::None)
                                {
                                    event = TableEvent::Commit(
                                        row_index,
                                        column_index,
                                        state.buffer.clone(),
                                    );
                                }
                            } else {
                                let display = hinted(column, &value);
                                // A foreign key that holds a value is a link: it points at a row
                                // the user can go to. A NULL key points nowhere and stays plain.
                                // A `__format__link` column is a link too, but only when it holds
                                // something the OS can open: painting `javascript:` as a link
                                // would promise a click that does nothing.
                                let is_url = column.hint.as_deref() == Some("link")
                                    && openable_url(&value).is_some();
                                let is_link = !matches!(value, Value::Null)
                                    && view.foreign_keys.iter().any(|fk| fk == &column.name);
                                let colour = match &value {
                                    Value::Null => tokens.text_faint,
                                    Value::Bytes(_) => tokens.info,
                                    _ if is_link || is_url => tokens.accent,
                                    _ if dirty => tokens.dirty_text,
                                    _ => tokens.text,
                                };
                                let font = tokens.mono(12.5);
                                let max = cell.width() - CELL_PAD * 2.0;
                                let text = ellipsize(ui, &display, font.clone(), max);
                                let (anchor, position) = if column.numeric()
                                    || matches!(column.hint.as_deref(), Some("money" | "check"))
                                    || numeric_column(view.rows, column_index)
                                {
                                    (
                                        Align2::RIGHT_CENTER,
                                        egui::pos2(cell.right() - CELL_PAD, cell.center().y),
                                    )
                                } else {
                                    (
                                        Align2::LEFT_CENTER,
                                        egui::pos2(cell.left() + CELL_PAD, cell.center().y),
                                    )
                                };
                                let painter = ui.painter().with_clip_rect(cell);
                                if is_link || is_url {
                                    let underline_y = position.y + font.size * 0.62;
                                    let width = painter
                                        .layout_no_wrap(text.clone(), font.clone(), colour)
                                        .size()
                                        .x
                                        .min(max);
                                    let x0 = match anchor {
                                        Align2::RIGHT_CENTER => position.x - width,
                                        _ => position.x,
                                    };
                                    painter.hline(
                                        x0..=(x0 + width),
                                        underline_y,
                                        Stroke::new(1.0, colour),
                                    );
                                }
                                painter.text(position, anchor, text, font, colour);
                                if response.hovered() && display.len() > 60 {
                                    response.clone().on_hover_text(display);
                                }
                                // A drag paints the rectangle live: the pointer's cell becomes
                                // the cursor while the anchor stays where the drag began. A
                                // plain click collapses both onto the one cell; shift-click
                                // keeps the anchor and moves only the cursor.
                                if response.dragged() {
                                    dragging = true;
                                    event = TableEvent::SelectExtend(row_index, column_index);
                                } else if response.clicked()
                                    && ui.input(|i| i.modifiers.shift)
                                    && view.anchor.is_some()
                                {
                                    event = TableEvent::SelectExtend(row_index, column_index);
                                } else if response.clicked()
                                    && is_url
                                    && ui.input(|i| i.modifiers.command)
                                    && openable_url(&value).is_some()
                                {
                                    // Opening a browser is an OS action, so it costs the same
                                    // deliberate gesture a foreign key does. Only http(s) is
                                    // openable — a stray `javascript:` or `file:` value is not a
                                    // link at all, so the click falls through to selecting the cell
                                    // instead of being swallowed.
                                    if let Some(url) = openable_url(&value) {
                                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                                    }
                                } else if response.clicked()
                                    && is_link
                                    && ui.input(|i| i.modifiers.command)
                                {
                                    // A plain click still selects (and copies, and edits on
                                    // double-click); following is the deliberate gesture, so a
                                    // link never costs the cell its ordinary behaviour.
                                    event = TableEvent::FollowForeignKey(row_index, column_index);
                                } else if response.clicked() {
                                    event = TableEvent::Select(row_index, column_index);
                                }
                                if view.editable && response.double_clicked() {
                                    event = TableEvent::BeginEdit(row_index, column_index);
                                }
                                if let Some(menu) = view.row_menu.as_deref_mut() {
                                    response.context_menu(|ui| menu(ui, row_index));
                                }
                            }

                            ui.painter().rect_stroke(
                                cell,
                                CornerRadius::ZERO,
                                Stroke::new(0.75, tokens.grid_line),
                                StrokeKind::Inside,
                            );
                            x += widths[column_index];
                        }

                        ui.painter().hline(
                            full.left()..=full.right(),
                            row_rect.bottom() - 0.5,
                            Stroke::new(0.5, tokens.grid_line),
                        );
                        if std::env::var("NOOK_TABLE_DEBUG").is_ok() && row_index < 2 {
                            eprintln!(
                                "row {row_index}: rect={:?} clip={:?} first={:?}",
                                row_rect,
                                ui.painter().clip_rect(),
                                view.rows[row_index].first().map(|v| v.display())
                            );
                        }
                    }
                    // A drag that left the cell it started on: map the pointer to the cell under it,
                    // so the rectangle grows instead of collapsing to the pressed cell. Past the last
                    // row or into the gutter the pointer sits on no cell, so it clamps to the nearest
                    // one — a drag that runs off the edge still selects the rows it passed.
                    if dragging {
                        if let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) {
                            let hit = cells
                                .iter()
                                .find(|(_, _, rect)| rect.contains(pointer))
                                .map(|&(row, column, _)| (row, column))
                                .or_else(|| {
                                    cells
                                        .iter()
                                        .min_by(|a, b| {
                                            let da = a.2.distance_to_pos(pointer);
                                            let db = b.2.distance_to_pos(pointer);
                                            da.total_cmp(&db)
                                        })
                                        .map(|&(row, column, _)| (row, column))
                                });
                            if let Some((row, column)) = hit {
                                event = TableEvent::SelectExtend(row, column);
                            }
                        }
                    }
                });
        });

    event
}

/// The head: names, the sort caret and the `.pkbadge`, painted inside the caller's horizontal scroll
/// area — the one that carries the body, or the head's own when the caller asked for no body.
fn head(
    ui: &mut Ui,
    tokens: &Tokens,
    view: &TableView<'_>,
    widths: &[f32],
    viewport_w: f32,
    total_w: f32,
    id: egui::Id,
) -> TableEvent {
    let mut event = TableEvent::None;
    let gutter_w = if view.gutter { GUTTER_W } else { 0.0 };
    let badge_w = design::pk_badge_width(ui, tokens, "PK");
    // Header: names, sort caret, primary-key tag.
    let (header_rect, _) = ui.allocate_exact_size(Vec2::new(total_w, HEAD_H), Sense::hover());
    let header_fill = egui::Rect::from_min_max(
        header_rect.min,
        egui::pos2(header_rect.left() + viewport_w, header_rect.bottom()),
    );
    ui.painter().rect_filled(
        header_fill,
        CornerRadius {
            nw: tokens.radius,
            ne: tokens.radius,
            sw: 0,
            se: 0,
        },
        tokens.grid_header,
    );
    if view.gutter {
        let gutter_head = egui::Rect::from_min_size(header_rect.min, Vec2::new(gutter_w, HEAD_H));
        ui.painter()
            .rect_filled(gutter_head, CornerRadius::ZERO, tokens.panel_alt);
        ui.painter().text(
            gutter_head.center(),
            Align2::CENTER_CENTER,
            "#",
            tokens.font(11.0),
            tokens.text_faint,
        );
    }
    ui.painter().hline(
        header_fill.left()..=header_fill.right(),
        header_rect.bottom() - 0.5,
        Stroke::new(1.0, tokens.stroke),
    );
    let mut x = header_rect.left() + gutter_w;
    for (index, column) in view.columns.iter().enumerate() {
        if widths[index] <= 0.0 {
            continue;
        }
        let cell = Rect::from_min_size(
            egui::pos2(x, header_rect.top()),
            Vec2::new(widths[index], HEAD_H),
        );
        let response = ui
            .interact(cell, id.with(("head", index)), Sense::click())
            .on_hover_text(format!("{}  {}", column.name, column.type_name));
        if response.hovered() {
            ui.painter()
                .rect_filled(cell, CornerRadius::ZERO, tokens.hover);
        }
        // The head's context menu: hide this column, or bring every hidden one back. Only
        // offered when the caller allows sorting — the data grid — so the read-only previews
        // keep their full column set.
        if view.sortable {
            // The head is the only way back: hiding the last visible column would leave no header
            // to right-click, so the menu keeps one column standing.
            let visible = view.columns.len().saturating_sub(view.hidden.len());
            response.context_menu(|ui| {
                if visible > 1
                    && ui
                        .button(format!("Hide column \"{}\"", column.name))
                        .clicked()
                {
                    event = TableEvent::HideColumn(column.name.clone());
                }
                if !view.hidden.is_empty() && ui.button("Show all columns").clicked() {
                    event = TableEvent::ShowAllColumns;
                }
            });
        }
        let sorted = view
            .sort
            .as_ref()
            .filter(|(name, _)| name == &column.name)
            .map(|(_, desc)| *desc);
        let label = match sorted {
            Some(true) => format!("{} ▼", column.label),
            Some(false) => format!("{} ▲", column.label),
            None => column.label.clone(),
        };
        ui.painter().text(
            egui::pos2(cell.left() + CELL_PAD, cell.center().y),
            Align2::LEFT_CENTER,
            label,
            tokens.font(12.5),
            if sorted.is_some() {
                tokens.accent
            } else {
                tokens.text
            },
        );
        if column.primary_key {
            // `.pkbadge`, through the same definition the toolbar's badge uses.
            let tag = Rect::from_min_size(
                egui::pos2(
                    cell.right() - CELL_PAD - badge_w,
                    cell.center().y - design::MICRO_TAG_H / 2.0,
                ),
                Vec2::new(badge_w, design::MICRO_TAG_H),
            );
            design::paint_pk_badge(ui, tokens, tag);
        }
        ui.painter().vline(
            cell.right() - 0.5,
            cell.top()..=cell.bottom(),
            Stroke::new(1.0, tokens.grid_line),
        );
        // The sort click is decided first so the resize strip below can overwrite it: a release
        // that ends a drag must persist the width, and losing that to a same-frame Sort would drop
        // the whole width map silently.
        if response.clicked() && view.sortable {
            event = TableEvent::Sort(column.name.clone());
        }
        // The resize strip: a 6px-wide drag handle on the right edge of each visible column.
        // Only offered when the caller allows sorting — the data grid — so read-only previews
        // keep their fixed layout.
        if view.sortable {
            let strip = Rect::from_min_size(
                egui::pos2(cell.right() - 3.0, cell.top()),
                Vec2::new(6.0, HEAD_H),
            );
            let drag = ui.interact(strip, id.with(("resize", index)), Sense::drag());
            if drag.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if drag.dragged() {
                // Clamped here, not only where the override is applied: this value is what the
                // caller stores and re-sends whole on `ResizeEnd`, and the engine refuses a width
                // above 4000 — so an unclamped drag past the limit would make the save fail
                // silently and lose every width in the map, not just this one.
                let new_width = (widths[index] + drag.drag_delta().x).clamp(MIN_COL_W, MAX_DRAG_W);
                event = TableEvent::ResizeColumn(column.name.clone(), new_width);
            }
            if drag.drag_stopped() {
                event = TableEvent::ResizeEnd;
            }
        }
        x += widths[index];
    }
    event
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A headless frame the way the app drives one: no texture uploads, one screen rect.
    fn screen(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(819.0, 900.0),
            )),
            events,
            ..Default::default()
        }
    }

    fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }
    }

    fn view<'a>(
        columns: &'a [TableColumn],
        rows: &'a [Vec<Value>],
        editable: bool,
        height: f32,
    ) -> TableView<'a> {
        view_hiding(columns, rows, editable, height, &[])
    }

    /// The same view with columns hidden: the slots stay, so every index a test reads is a
    /// result index — which is the property the hidden-column paint depends on.
    fn view_hiding<'a>(
        columns: &'a [TableColumn],
        rows: &'a [Vec<Value>],
        editable: bool,
        height: f32,
        hidden: &'a [String],
    ) -> TableView<'a> {
        TableView {
            columns,
            hidden,
            foreign_keys: &[],
            rows,
            pending: Vec::new(),
            inserted: Vec::new(),
            selected: None,
            anchor: None,
            sort: None,
            editable,
            sortable: true,
            gutter: true,
            height,
            row_h: design::row_h(nook_engine::Density::Compact),
            empty_text: "no rows",
            row_menu: None,
            widths: &[],
        }
    }

    /// Double-click the cell at `(row, column)` and report the event the widget produced. The first
    /// frame only lays the table out, so the second one can be pointed at the cell's real rect.
    fn double_click_cell(
        view: &mut TableView<'_>,
        id: egui::Id,
        row: usize,
        column: usize,
    ) -> TableEvent {
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let mut state = TableState::default();
        let mut out = ctx.run_ui(screen(Vec::new()), |ui| {
            let _ = show(ui, &tokens, view, &mut state, id);
        });
        // The frame paints into a texture delta the headless context never applies.
        out.textures_delta.clear();
        let cell = ctx
            .read_response(id.with(("cell", row, column)))
            .expect("the cell registered its rect in the first frame")
            .rect;
        let mut events = vec![egui::Event::PointerMoved(cell.center())];
        for _ in 0..2 {
            events.push(press(cell.center(), true));
            events.push(press(cell.center(), false));
        }
        let mut event = TableEvent::None;
        let mut out = ctx.run_ui(screen(events), |ui| {
            event = show(ui, &tokens, view, &mut state, id);
        });
        out.textures_delta.clear();
        event
    }

    /// One click on the cell at `(row, column)` with the given modifiers, reporting the event.
    fn click_cell(
        view: &mut TableView<'_>,
        id: egui::Id,
        row: usize,
        column: usize,
        modifiers: egui::Modifiers,
    ) -> TableEvent {
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let mut state = TableState::default();
        let mut out = ctx.run_ui(screen(Vec::new()), |ui| {
            let _ = show(ui, &tokens, view, &mut state, id);
        });
        out.textures_delta.clear();
        let cell = ctx
            .read_response(id.with(("cell", row, column)))
            .expect("the cell registered its rect in the first frame")
            .rect;
        let press_at = |pressed: bool| egui::Event::PointerButton {
            pos: cell.center(),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        };
        // `input.modifiers` is the frame's state, and egui learns it from the `Modifiers` event,
        // not from the per-button copy: without it the widget reads a bare click.
        let events = vec![
            egui::Event::ModifiersChanged(modifiers),
            egui::Event::PointerMoved(cell.center()),
            press_at(true),
            press_at(false),
        ];
        let mut event = TableEvent::None;
        let mut out = ctx.run_ui(screen(events), |ui| {
            event = show(ui, &tokens, view, &mut state, id);
        });
        out.textures_delta.clear();
        event
    }

    /// The engine refuses `grid.editCell` on a grid with no primary key, so the widget must not offer
    /// the editor there — a `BeginEdit` the caller has to swallow is the UI advertising a dead action.
    #[test]
    fn a_read_only_view_opens_no_editor_on_a_double_click() {
        let columns = [
            TableColumn::new("id", "INTEGER").pk(true),
            TableColumn::new("email", "TEXT"),
        ];
        let rows = vec![vec![Value::Int(1), Value::text("a@b.c")]];
        let id = egui::Id::new("read-only-table");
        let event = double_click_cell(&mut view(&columns, &rows, false, 120.0), id, 0, 1);
        assert_ne!(
            event,
            TableEvent::BeginEdit(0, 1),
            "a read-only grid must not ask for an editor"
        );
        assert_eq!(
            event,
            TableEvent::Select(0, 1),
            "the click still selects the cell"
        );
    }

    /// The other half of the same contract: an editable grid does open the editor on a double click.
    #[test]
    fn an_editable_view_opens_the_editor_on_a_double_click() {
        let columns = [
            TableColumn::new("id", "INTEGER").pk(true),
            TableColumn::new("email", "TEXT"),
        ];
        let rows = vec![vec![Value::Int(1), Value::text("a@b.c")]];
        let id = egui::Id::new("editable-table");
        let event = double_click_cell(&mut view(&columns, &rows, true, 120.0), id, 0, 1);
        assert_eq!(event, TableEvent::BeginEdit(0, 1));
    }

    /// Right-clicking a row of a view that carries a `row_menu` must reach the callback with that
    /// row — the design tab's column menu is the only caller, and a wrong index there would edit
    /// the wrong column.
    #[test]
    fn a_secondary_click_reaches_the_row_menu_with_the_row() {
        let columns = [
            TableColumn::new("id", "INTEGER").pk(true),
            TableColumn::new("email", "TEXT"),
        ];
        let rows = vec![vec![Value::Int(1), Value::text("a@b.c")]];
        let mut picked: Option<usize> = None;
        let mut menu = |ui: &mut egui::Ui, row: usize| {
            picked = Some(row);
            let _ = ui.button("Rename column");
        };
        let mut table_view = view(&columns, &rows, false, 120.0);
        table_view.row_menu = Some(&mut menu);
        let id = egui::Id::new("menu-table");
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let mut state = TableState::default();
        let mut out = ctx.run_ui(screen(Vec::new()), |ui| {
            let _ = show(ui, &tokens, &mut table_view, &mut state, id);
        });
        out.textures_delta.clear();
        let cell = ctx
            .read_response(id.with(("cell", 0, 1)))
            .expect("the cell registered its rect in the first frame")
            .rect;
        let events = vec![
            egui::Event::PointerMoved(cell.center()),
            egui::Event::PointerButton {
                pos: cell.center(),
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
            egui::Event::PointerButton {
                pos: cell.center(),
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            },
        ];
        let mut second = ctx.run_ui(screen(events), |ui| {
            let _ = show(ui, &tokens, &mut table_view, &mut state, id);
        });
        second.textures_delta.clear();
        assert_eq!(picked, Some(0), "the menu opens for the right-clicked row");
    }

    /// An empty page asks for the head alone, and that head must still be laid out at its full width
    /// inside a horizontal scroll area: the pane here is narrower than the table (54px of gutter plus
    /// five 76px columns), so a head clipped to the pane would leave the columns past its edge with
    /// no bar to reach them. It must also advance the caller by exactly the head's height, so the
    /// `.empty` block that follows lands under the hairline as the design has it.
    #[test]
    fn an_empty_page_still_reports_the_heads_rects() {
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let columns: Vec<TableColumn> = (0..5)
            .map(|index| TableColumn::new(format!("column_{index}"), "TEXT"))
            .collect();
        let rows: Vec<Vec<Value>> = Vec::new();
        let id = egui::Id::new("empty-page-table");
        let pane_w = 300.0;
        let mut state = TableState::default();
        let screen = |events: Vec<egui::Event>| egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(pane_w, 400.0),
            )),
            events,
            ..Default::default()
        };
        let mut advanced = 0.0;
        let mut run = |events: Vec<egui::Event>| {
            let mut out = ctx.run_ui(screen(events), |ui| {
                // `grid::show` zeroes the pane's spacing so its bands are flush; the measure below
                // must be the head's, not the default spacing of a wrapped row.
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                let before = ui.cursor().top();
                let mut inline = view(&columns, &rows, true, 0.0);
                let event = show(ui, &tokens, &mut inline, &mut state, id);
                assert_eq!(event, TableEvent::None);
                advanced = ui.cursor().top() - before;
            });
            // The frame paints into a texture delta the headless context never applies.
            out.textures_delta.clear();
            advanced
        };
        let advanced = run(Vec::new());
        assert!(
            (advanced - HEAD_H).abs() < 0.5,
            "the head-only page advanced {advanced}, not its head's {HEAD_H}"
        );
        let head = |index: usize| {
            ctx.read_response(id.with(("head", index)))
                .expect("the head's column registered its rect")
                .rect
        };
        let tail = columns.len() - 1;
        assert!(
            head(tail).right() > pane_w,
            "the head must keep its full width, past the pane it is drawn in"
        );
        for index in 0..columns.len() {
            assert!(
                (head(index).height() - HEAD_H).abs() < 0.5,
                "column {index} is not the head's height"
            );
            if index > 0 {
                assert!(
                    (head(index).left() - head(index - 1).right()).abs() < 0.5,
                    "the head's columns must tile the row"
                );
            }
        }

        // The bar is what makes the columns past the pane reachable: a wheel over the head moves it
        // sideways until the last column sits inside the pane. Without the scroll area the head
        // cannot move at all, whatever its width.
        let mut reached = false;
        for _ in 0..10 {
            let advanced = run(vec![
                egui::Event::PointerMoved(egui::pos2(pane_w / 2.0, HEAD_H / 2.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(-40.0, 0.0),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::default(),
                },
            ]);
            assert!(
                (advanced - HEAD_H).abs() < 0.5,
                "the head-only page advanced {advanced}, not its head's {HEAD_H}"
            );
            if head(tail).right() <= pane_w + 0.5 {
                reached = true;
                break;
            }
        }
        assert!(
            reached,
            "the head never scrolled sideways, so the column at x={} is unreachable",
            head(tail).right()
        );
    }

    /// Hiding a middle column must not shift the cells around it: the widget's `column` index is a
    /// result index, so the third column's value keeps painting in the third slot's place — with
    /// the second slot collapsed to nothing.
    #[test]
    fn a_hidden_middle_column_leaves_the_other_cells_on_their_own_values() {
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let columns = [
            TableColumn::new("id", "INTEGER"),
            TableColumn::new("secret", "TEXT"),
            TableColumn::new("city", "TEXT"),
        ];
        let rows = vec![vec![
            Value::Int(1),
            Value::text("hunter2"),
            Value::text("London"),
        ]];
        let hidden = [String::from("secret")];
        let id = egui::Id::new("hidden-middle-column");
        let mut state = TableState::default();
        let mut rects = Vec::new();
        let mut out = ctx.run_ui(screen(Vec::new()), |ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            let mut inline = view_hiding(&columns, &rows, false, 300.0, &hidden);
            let _ = show(ui, &tokens, &mut inline, &mut state, id);
        });
        out.textures_delta.clear();
        for column in 0..3 {
            rects.push(
                ctx.read_response(id.with(("cell", 0, column)))
                    .map(|response| response.rect),
            );
        }
        // The hidden slot paints and interacts nothing at all — no rect, so nothing can click it.
        assert!(
            rects[1].is_none(),
            "the hidden column still registered an interactable rect"
        );
        // The visible neighbours keep their own values and their own order.
        let first = rects[0].expect("the first column paints");
        let third = rects[2].expect("the third column paints");
        assert!(
            third.left() >= first.right() - 0.5,
            "the third column overlapped the first: {} then {}",
            first.right(),
            third.left()
        );
    }

    #[test]
    fn a_format_marker_becomes_a_hint_and_leaves_the_header_reading_the_real_name() {
        let money = TableColumn::new("price__format__money", "REAL");
        assert_eq!(money.label, "price");
        assert_eq!(money.hint.as_deref(), Some("money"));
        // The column still names the real column: sorting and edits address the database's name.
        assert_eq!(money.name, "price__format__money");

        let check = TableColumn::new("active__format__check", "INTEGER");
        assert_eq!(check.hint.as_deref(), Some("check"));

        // An unknown marker is not a hint: the header keeps the whole name, so a typo is visible.
        let odd = TableColumn::new("x__format__stars", "TEXT");
        assert_eq!(odd.label, "x__format__stars");
        assert_eq!(odd.hint, None);

        // A plain column carries no hint and heads as itself.
        let plain = TableColumn::new("city", "TEXT");
        assert_eq!(plain.label, "city");
        assert_eq!(plain.hint, None);
    }

    #[test]
    fn a_hinted_cell_measures_the_text_it_paints() {
        // The width pass and the paint pass must agree: `12.5` widens to `12.50`, and a check
        // collapses to one glyph, so a column sized from `display()` clips or overlaps.
        let money = TableColumn::new("price__format__money", "REAL");
        assert_eq!(hinted(&money, &Value::Float(12.5)), "12.50");
        assert_eq!(hinted(&money, &Value::Int(7)), "7.00");

        let check = TableColumn::new("active__format__check", "INTEGER");
        assert_eq!(hinted(&check, &Value::Int(1)), "✔");
        assert_eq!(hinted(&check, &Value::Int(0)), "×");
        assert_eq!(hinted(&check, &Value::Text("yes".into())), "✔");
        assert_eq!(hinted(&check, &Value::Null), "—");

        // No hint paints exactly what `display()` does, so a plain column keeps its old width.
        let plain = TableColumn::new("city", "TEXT");
        let text = Value::Text("London".into());
        assert_eq!(hinted(&plain, &text), text.display());
    }

    #[test]
    fn a_foreign_key_cell_still_selects_on_a_plain_click_and_follows_on_a_command_click() {
        let columns = [
            TableColumn::new("id", "INTEGER").pk(true),
            TableColumn::new("customer_id", "INTEGER"),
        ];
        let rows = vec![vec![Value::Int(10), Value::Int(7)]];
        let foreign_keys = [String::from("customer_id")];
        let build = || {
            let mut view = view(&columns, &rows, true, 300.0);
            view.foreign_keys = &foreign_keys;
            view
        };
        // A plain click is the cell's ordinary behaviour: it selects, so copy and the inline
        // editor keep working on a link column.
        let event = click_cell(
            &mut build(),
            egui::Id::new("fk-plain-click"),
            0,
            1,
            egui::Modifiers::default(),
        );
        assert!(
            matches!(event, TableEvent::Select(0, 1)),
            "a plain click must still select the cell, got {event:?}"
        );
        // Following is the deliberate gesture.
        let command = egui::Modifiers {
            command: true,
            mac_cmd: true,
            ..egui::Modifiers::default()
        };
        let event = click_cell(
            &mut build(),
            egui::Id::new("fk-command-click"),
            0,
            1,
            command,
        );
        assert!(
            matches!(event, TableEvent::FollowForeignKey(0, 1)),
            "a command-click must follow the key, got {event:?}"
        );
    }

    #[test]
    fn a_link_cell_that_holds_no_url_still_selects_on_a_command_click() {
        // Only http(s) is openable, so a `javascript:` or `ftp:` value must not eat the click:
        // an unopenable link is ordinary text, and a swallowed click is a cell you cannot copy.
        let columns = [TableColumn::new("site__format__link", "TEXT")];
        let rows = vec![vec![Value::Text("javascript:alert(1)".into())]];
        let build = || view(&columns, &rows, false, 300.0);
        let command = egui::Modifiers {
            command: true,
            mac_cmd: true,
            ..egui::Modifiers::default()
        };
        let event = click_cell(&mut build(), egui::Id::new("link-not-a-url"), 0, 0, command);
        assert!(
            matches!(event, TableEvent::Select(0, 0)),
            "an unopenable link must fall through to selecting, got {event:?}"
        );

        // The helper is the one place that decides what may be handed to the OS, and the paint
        // reads it too: a cell that cannot open must not wear the link's accent or underline.
        assert_eq!(
            openable_url(&Value::Text("https://x.test".into())),
            Some("https://x.test")
        );
        assert_eq!(
            openable_url(&Value::Text("javascript:alert(1)".into())),
            None
        );
        assert_eq!(openable_url(&Value::Text("ftp://x.test".into())), None);
        assert_eq!(openable_url(&Value::Null), None);
    }

    /// Dragging the right edge of a header must emit `ResizeColumn` with the new width, and
    /// releasing must emit `ResizeEnd` so the caller can persist it.
    #[test]
    fn dragging_a_header_edge_emits_resize_events() {
        let columns = [
            TableColumn::new("id", "INTEGER").pk(true),
            TableColumn::new("email", "TEXT"),
        ];
        let rows = vec![vec![Value::Int(1), Value::text("a@b.c")]];
        let id = egui::Id::new("resize-drag");
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let mut state = TableState::default();
        let mut out = ctx.run_ui(screen(Vec::new()), |ui| {
            let mut view = view(&columns, &rows, true, 120.0);
            let _ = show(ui, &tokens, &mut view, &mut state, id);
        });
        out.textures_delta.clear();
        let head = ctx
            .read_response(id.with(("head", 0)))
            .expect("the first header registered its rect")
            .rect;
        let _resize = ctx
            .read_response(id.with(("resize", 0)))
            .expect("the resize strip registered its rect")
            .rect;
        // The drag strip sits on the right edge of the header cell.
        let start = egui::pos2(head.right() - 3.0, head.center().y);
        let end = egui::pos2(head.right() + 40.0, head.center().y);
        // First, just check that the strip is hovered when the pointer is over it.
        let events = vec![egui::Event::PointerMoved(start)];
        let mut out = ctx.run_ui(screen(events), |ui| {
            let mut view = view(&columns, &rows, true, 120.0);
            let _ = show(ui, &tokens, &mut view, &mut state, id);
        });
        out.textures_delta.clear();
        let resize_response = ctx.read_response(id.with(("resize", 0))).unwrap();
        assert!(
            resize_response.hovered(),
            "the resize strip should be hovered"
        );
        // Now try the drag, split across multiple frames so egui can recognize it.
        // Frame 1: press the button.
        let events = vec![press(start, true)];
        let mut out = ctx.run_ui(screen(events), |ui| {
            let mut view = view(&columns, &rows, true, 120.0);
            let _ = show(ui, &tokens, &mut view, &mut state, id);
        });
        out.textures_delta.clear();
        // Frame 2: move the pointer while pressed.
        let events = vec![egui::Event::PointerMoved(end)];
        let mut event = TableEvent::None;
        let mut out = ctx.run_ui(screen(events), |ui| {
            let mut view = view(&columns, &rows, true, 120.0);
            event = show(ui, &tokens, &mut view, &mut state, id);
        });
        out.textures_delta.clear();
        assert!(
            matches!(event, TableEvent::ResizeColumn(_, _)),
            "dragging the header edge must emit ResizeColumn, got {event:?}"
        );
    }

    /// A width override must win over the content measurement and the numeric cap: the user's
    /// drag is the authority, not the longest value in the column.
    #[test]
    fn a_width_override_wins_over_content_and_numeric_cap() {
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let columns = [
            TableColumn::new("id", "INTEGER"),
            TableColumn::new("email", "TEXT"),
        ];
        let rows = vec![vec![Value::Int(1), Value::text("a@b.c")]];
        let widths = [(String::from("id"), 400.0)];
        let mut state = TableState::default();
        let mut measured = 0.0;
        let mut out = ctx.run_ui(screen(Vec::new()), |ui| {
            let mut view = TableView {
                columns: &columns,
                hidden: &[],
                foreign_keys: &[],
                rows: &rows,
                pending: Vec::new(),
                inserted: Vec::new(),
                selected: None,
                anchor: None,
                sort: None,
                editable: true,
                sortable: true,
                gutter: true,
                height: 120.0,
                row_h: design::row_h(nook_engine::Density::Compact),
                empty_text: "no rows",
                row_menu: None,
                widths: &widths,
            };
            let _ = show(
                ui,
                &tokens,
                &mut view,
                &mut state,
                egui::Id::new("override"),
            );
            measured = column_widths(ui, &tokens, &view)[0];
        });
        out.textures_delta.clear();
        assert!(
            (measured - 400.0).abs() < 0.5,
            "the override must win, got {measured}"
        );
    }

    /// A width below the minimum is clamped up: a 10px column is unreadable, so the floor holds.
    #[test]
    fn a_width_below_the_minimum_is_clamped_up() {
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let columns = [TableColumn::new("id", "INTEGER")];
        let rows = vec![vec![Value::Int(1)]];
        let widths = [(String::from("id"), 10.0)];
        let mut state = TableState::default();
        let mut measured = 0.0;
        let mut out = ctx.run_ui(screen(Vec::new()), |ui| {
            let mut view = TableView {
                columns: &columns,
                hidden: &[],
                foreign_keys: &[],
                rows: &rows,
                pending: Vec::new(),
                inserted: Vec::new(),
                selected: None,
                anchor: None,
                sort: None,
                editable: true,
                sortable: true,
                gutter: true,
                height: 120.0,
                row_h: design::row_h(nook_engine::Density::Compact),
                empty_text: "no rows",
                row_menu: None,
                widths: &widths,
            };
            let _ = show(ui, &tokens, &mut view, &mut state, egui::Id::new("clamp"));
            measured = column_widths(ui, &tokens, &view)[0];
        });
        out.textures_delta.clear();
        assert!(
            measured >= MIN_COL_W,
            "the width must be clamped to the minimum, got {measured}"
        );
    }
    /// Drive a multi-frame drag on one header's resize strip and return the last event the frame
    /// reported. Each step is its own frame, which is what egui needs to recognise a drag.
    fn drag_header(
        ctx: &egui::Context,
        tokens: &Tokens,
        id: egui::Id,
        start: egui::Pos2,
        steps: &[f32],
        columns: &[TableColumn],
        rows: &[Vec<Value>],
        widths: &[(String, f32)],
    ) -> TableEvent {
        // The eight-argument shape is a test fixture, not an API: splitting it into a struct would
        // be more lines than the two call sites it serves.
        #![allow(clippy::too_many_arguments)]
        let mut state = TableState::default();
        let mut out = ctx.run_ui(screen(vec![press(start, true)]), |ui| {
            let mut view = view(columns, rows, true, 200.0);
            view.widths = widths;
            let _ = show(ui, tokens, &mut view, &mut state, id);
        });
        out.textures_delta.clear();
        let mut last = TableEvent::None;
        let mut tip = start;
        for dx in steps {
            tip = egui::pos2(start.x + dx, start.y);
            let mut event = TableEvent::None;
            let mut out = ctx.run_ui(screen(vec![egui::Event::PointerMoved(tip)]), |ui| {
                let mut view = view(columns, rows, true, 200.0);
                view.widths = widths;
                event = show(ui, tokens, &mut view, &mut state, id);
            });
            out.textures_delta.clear();
            last = event;
        }
        // Release, or the next drag in the same context never starts: egui sees a button that is
        // already down and does not open a new gesture.
        let mut out = ctx.run_ui(screen(vec![press(tip, false)]), |ui| {
            let mut view = view(columns, rows, true, 200.0);
            view.widths = widths;
            let _ = show(ui, tokens, &mut view, &mut state, id);
        });
        out.textures_delta.clear();
        last
    }

    /// The resize strip must win over the horizontal scroll area's drag-to-scroll: dragging a header
    /// edge resizes the column and must not pan the view. The table here is wider than the viewport
    /// (54px gutter + 12 columns at 80px = 1014px against 819px), so the area's drag handling is
    /// live and would claim the gesture if the strip did not win the hit test.
    #[test]
    fn a_resize_drag_does_not_pan_the_horizontal_scroll_area() {
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let columns: Vec<TableColumn> = (0..12)
            .map(|i| TableColumn::new(format!("col_{i}"), "TEXT"))
            .collect();
        let rows = vec![vec![Value::text("x"); 12]];
        let widths: Vec<(String, f32)> = (0..12).map(|i| (format!("col_{i}"), 80.0)).collect();
        let id = egui::Id::new("wide-resize");
        let mut state = TableState::default();
        let mut out = ctx.run_ui(screen(Vec::new()), |ui| {
            let mut view = view(&columns, &rows, true, 200.0);
            view.widths = &widths;
            let _ = show(ui, &tokens, &mut view, &mut state, id);
        });
        out.textures_delta.clear();
        let head_before = ctx
            .read_response(id.with(("head", 0)))
            .expect("the first header registered its rect")
            .rect;
        let cell_before = ctx
            .read_response(id.with(("cell", 0, 0)))
            .expect("the first body cell registered its rect")
            .rect;
        let start = egui::pos2(head_before.right() - 3.0, head_before.center().y);

        let mut out = ctx.run_ui(screen(vec![press(start, true)]), |ui| {
            let mut view = view(&columns, &rows, true, 200.0);
            view.widths = &widths;
            let _ = show(ui, &tokens, &mut view, &mut state, id);
        });
        out.textures_delta.clear();
        let mut last = TableEvent::None;
        for step in 1..=3 {
            let pos = egui::pos2(start.x + 20.0 * step as f32, start.y);
            let mut event = TableEvent::None;
            let mut out = ctx.run_ui(screen(vec![egui::Event::PointerMoved(pos)]), |ui| {
                let mut view = view(&columns, &rows, true, 200.0);
                view.widths = &widths;
                event = show(ui, &tokens, &mut view, &mut state, id);
            });
            out.textures_delta.clear();
            last = event;
            // The body must not have moved: a pan would take the header with it.
            let cell_during = ctx
                .read_response(id.with(("cell", 0, 0)))
                .expect("the body cell still registered its rect")
                .rect;
            assert!(
                (cell_during.left() - cell_before.left()).abs() < 0.5,
                "step {step}: the body shifted from {} to {}, the scroll area claimed the drag",
                cell_before.left(),
                cell_during.left()
            );
        }
        assert!(
            matches!(last, TableEvent::ResizeColumn(_, _)),
            "the drag must emit ResizeColumn, got {last:?}"
        );
    }

    /// A drag past either end stops at the bound and emits the bounded value. The clamp has to be at
    /// the emit site, not only where the override is applied: the caller stores this number and
    /// re-sends the whole map on release, and the engine refuses a width above 4000 — so an
    /// unclamped value would fail the save and lose every width in the map, not just this one.
    ///
    /// Both drags start from a column narrow enough that its right edge is on screen: at
    /// `MAX_DRAG_W` the strip sits past the viewport, so the gesture would never begin there.
    #[test]
    fn a_resize_drag_emits_a_clamped_width() {
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let columns = [TableColumn::new("id", "INTEGER")];
        let rows = vec![vec![Value::Int(1)]];

        for (id, steps, check) in [
            (
                egui::Id::new("clamp-min"),
                [-100.0, -400.0, -900.0],
                (MIN_COL_W, true),
            ),
            (
                egui::Id::new("clamp-max"),
                [500.0, 2_000.0, 5_000.0],
                (MAX_DRAG_W, false),
            ),
        ] {
            let widths = [(String::from("id"), 120.0)];
            let mut state = TableState::default();
            let mut out = ctx.run_ui(screen(Vec::new()), |ui| {
                let mut view = view(&columns, &rows, true, 200.0);
                view.widths = &widths;
                let _ = show(ui, &tokens, &mut view, &mut state, id);
            });
            out.textures_delta.clear();
            let head = ctx
                .read_response(id.with(("head", 0)))
                .expect("the header registered its rect")
                .rect;
            let start = egui::pos2(head.right() - 3.0, head.center().y);
            match drag_header(&ctx, &tokens, id, start, &steps, &columns, &rows, &widths) {
                TableEvent::ResizeColumn(_, w) => {
                    if check.1 {
                        assert!(
                            w >= check.0,
                            "a leftward drag must stop at MIN_COL_W={}, emitted {w}",
                            check.0
                        );
                    } else {
                        assert!(
                            w <= check.0,
                            "a rightward drag must stop at MAX_DRAG_W={}, emitted {w}",
                            check.0
                        );
                    }
                }
                other => panic!("expected ResizeColumn for {id:?}, got {other:?}"),
            }
        }
    }
}
