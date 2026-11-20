//! Design tokens and the small widget helpers the rest of the UI builds from.
//! No screen in `crates/ui` invents a colour, a radius, a spacing value or a font size: it asks here.

use egui::{
    Color32, CornerRadius, FontId, Frame, Margin, Rect, Response, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};

/// Spacing scale — the eight steps the design defines. Nothing in the UI invents a value between
/// them; `--s1..--s8` in the design reference are these numbers.
pub const S1: i8 = 2;
pub const S2: i8 = 4;
pub const S3: i8 = 6;
pub const S4: i8 = 8;
pub const S5: i8 = 12;
pub const S6: i8 = 16;
pub const S7: i8 = 20;
pub const S8: i8 = 24;

/// Layout heights. A row is 26px compact; a table head 28; a tree row 32; a control 28.
pub const ROW_H: f32 = 26.0;
pub const ROW_COMFORT_H: f32 = 30.0;
pub const HEAD_H: f32 = 28.0;
pub const TREE_H: f32 = 32.0;
pub const BUTTON_H: f32 = 28.0;
/// The top bar's bordered controls, as the design's `.iconbtn` and `.ws` are 26.
pub const ICON_BUTTON_H: f32 = 26.0;
/// The 12px glyph the design's `.sm` icons use inside a 26px control.
pub const GLYPH_SM: f32 = 12.0;
/// The design's `.menu`: a 28px label button with 6px of padding and no chrome of its own.
pub const MENU_H: f32 = 28.0;
/// The design's `.keycap`: an 11px line, 4px of padding and a 1px border.
pub const KEYCAP_H: f32 = 21.0;
/// The design's `.engine` tag: the same 11px line with 4px of padding and a 1px border.
pub const TAG_H: f32 = 21.0;
/// The box every bar control gives its 14-unit glyph.
pub const GLYPH_BOX: f32 = 14.0;
pub const SEG_H: f32 = 26.0;
pub const SEG_ITEM_H: f32 = 24.0;
pub const CHIP_H: f32 = 24.0;
pub const APP_BAR_H: f32 = 40.0;
pub const TAB_H: f32 = 34.0;
/// The tab bar's own height: the design writes `calc(--h-tabs + 1px)`, so the bar is one pixel taller
/// than its tabs and the active tab's bottom edge covers that row.
pub const TAB_BAR_H: f32 = TAB_H + 1.0;
pub const TOOLBAR_H: f32 = 40.0;
pub const STATUS_H: f32 = 26.0;

/// The query editor's own geometry: `.ed` is the design's 320 — sixteen 20px rows in a 52px gutter —
/// its `.tb` buttons are 26, its statement strip's chips and the completion popup's items are 20.
pub const EDITOR_H: f32 = 320.0;
pub const GUTTER_W: f32 = 52.0;
pub const EDITOR_ROW_H: f32 = 20.0;
pub const STMT_H: f32 = 20.0;
pub const STMT_BAND_H: f32 = 56.0;
pub const TOOLBAR_BTN_H: f32 = 26.0;
pub const POP_W: f32 = 456.0;
pub const POP_HD_H: f32 = 24.0;
pub const POP_FT_H: f32 = 24.0;
pub const POP_ITEM_H: f32 = 20.0;

/// The results register's own geometry: `.rtabs` is the tab strip's `34 + 1`, its `.note` pill and the
/// empty block are 20 and 196, its JSON gutter is 48, and `.eband` is padded with `--s5` on all four
/// sides.
pub const REG_TABS_H: f32 = TAB_H + 1.0;
pub const NOTE_H: f32 = 20.0;
pub const EMPTY_BLOCK_H: f32 = 196.0;
pub const JSON_GUTTER_W: f32 = 48.0;
pub const REG_ERR_PAD: i8 = S5;

/// Panel widths and the small geometry the shell and the lists share.
pub const TREE_W: f32 = 300.0;
/// The object inspector, from the design's `--w-inspector`.
pub const INSPECTOR_W: f32 = 320.0;
/// Where the tree's, the grid's and the inspector's pane heads all end, from the design's
/// `--h-panehead` (40 + 24 + 40 + 28). The three panes start their bodies on the same line only if
/// each one's bands add up to this, so it is one number rather than three sums.
pub const PANE_HEAD_H: f32 = 132.0;
pub const INDENT_STEP: f32 = 14.0;
pub const INDICATOR_W: f32 = 2.0;

/// The grid's own band heights. `.fchips` is 32 — the design's 18px chip with 7px of air above and
/// below — and `.stalestrip` is the status bar's 26. `.pkbadge` and `.taglite` are 16px boxes.
pub const FCHIP_H: f32 = 18.0;
const FCHIP_AIR: f32 = 7.0;
pub const FCHIPS_H: f32 = FCHIP_H + 2.0 * FCHIP_AIR;
pub const STALE_STRIP_H: f32 = STATUS_H;
pub const MICRO_TAG_H: f32 = 16.0;
/// `.errband` and `.robanner` keep 10px of air above and below their two lines.
pub const NOTICE_PAD_V: f32 = 10.0;
/// Activity rail: a 44px column of 32px workspace buttons.
pub const RAIL_W: f32 = 44.0;
pub const RAIL_BUTTON: f32 = 32.0;
/// A tab is never narrower than 96 nor wider than 260, and its close button is 24px.
pub const TAB_MIN_W: f32 = 96.0;
pub const TAB_MAX_W: f32 = 260.0;
pub const TAB_CLOSE: f32 = 24.0;
/// The tab bar's control cluster: `.tabctl` is a full-height left border, 8px of padding, its 24px
/// button and 8px more, and the design leaves the bar no right padding, so the cluster's right edge
/// is the bar's right edge.
pub const TAB_CTL_W: f32 = 1.0 + S4 as f32 + TAB_CLOSE + S4 as f32;

/// Corner radii: 4 for controls, 6 for containers, a pill for chips.
pub const R_CTL: u8 = 4;
pub const R_BOX: u8 = 6;
/// `--r-chip: 999px`. epaint clamps a corner radius to half the rect's short side, so this renders
/// as a true pill at any control height and as a square corner on a hairline.
pub const R_CHIP: u8 = 255;

/// Type scale.
pub const T_XS: f32 = 11.0;
pub const T_SM: f32 = 12.0;
pub const T_MD: f32 = 13.0;
pub const T_LG: f32 = 14.0;
pub const T_XL: f32 = 18.0;
pub const T_MONO: f32 = 12.0;

/// The dialog layer's own geometry. `.dlg` is 560 wide and the confirmation's narrow `.dlg` is 440;
/// `.dlg-h`'s `16 20 12` of padding wraps the title block, so the head is 68; `.tab` is a 32px item and
/// the strip draws the 1px hairline at its bottom edge, so that band is 33; `.inp` is 28, the dialog's
/// `.seg` is 28 around 26px items that fit inside its 1px border, `.chip` is 20 and the keychain chip
/// inside the password field is 18, `.cb` is 14, and the test row's `.dot` is 8.
pub const DIALOG_W: f32 = 560.0;
pub const DIALOG_NARROW_W: f32 = 440.0;
/// `.dlg-h`'s 68: `--s6` above the title, the title block, `--s5` below it — the plan's pin. The
/// sheet's rule is `padding: --s6 --s7 --s5`, and its two pages render that head 65.08 and 67.69 tall
/// because they inherit different line heights (`components/dialogs.html` puts 1.35 on `.ui`,
/// `sections/modals.html` 1.45 on `body`, and the title's `T_LG` and subtitle's `T_SM` lines carry no
/// line height of their own). 68 is the mark this app holds to in both places, so the block has to fit
/// inside it.
pub const DLG_HEAD_H: f32 = 68.0;
/// `.tabs`: the strip's hairline under a 32px item. The two pages render the strip 32 and 34 tall —
/// `components/dialogs.html` hard-codes `.tab`'s `height: 32px`, `sections/modals.html` reuses the
/// shell's `--h-tabs` token, which is 34 — and 33 is the mark this app holds to.
pub const DLG_TAB_H: f32 = 32.0;
pub const DLG_TABS_H: f32 = DLG_TAB_H + 1.0;
pub const INPUT_H: f32 = 28.0;
pub const DLG_SEG_H: f32 = 28.0;
pub const DLG_SEG_ITEM_H: f32 = DLG_SEG_H - 2.0;
pub const DLG_CHIP_H: f32 = 20.0;
pub const KEYCHIP_H: f32 = 18.0;
pub const CHECKBOX: f32 = 14.0;
pub const STATUS_DOT: f32 = 8.0;
/// `.dlg-f`: the design's `padding: --s5 --s7` around 28px rows, its 1px top border, and the
/// editor's own `gap: --s4` between the two rows — 1 + 12 + 28 + 8 + 28 + 12 = 89. The confirmation's
/// single row is the same band without the second row and its gap: 1 + 12 + 28 + 12 = 53.
pub const DLG_FOOT_H: f32 = 1.0 + 2.0 * S5 as f32 + S4 as f32 + 2.0 * BUTTON_H;
pub const DLG_FOOT_ONE_H: f32 = 1.0 + 2.0 * S5 as f32 + BUTTON_H;
/// The line height the sheet sets on its running text (`.helper`, `.msg`, `.note`). `.para` is a
/// sibling class of the same family — `--t-md` ink, `--muted` — but every sheet pins it at 1.5, not
/// 1.45, so it is not part of this set.
const DLG_LINE_H: f32 = 1.45;
/// The `.cb` box's own 2px corner, a square one inside `--r-ctl`, and the 9px `.cb`/`.badge` mark it
/// and the `.msg` badge print.
const CB_RADIUS: u8 = 2;
const MARK_GLYPH: f32 = 9.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tokens {
    pub dark: bool,
    pub radius: u8,

    pub background: Color32,
    pub panel: Color32,
    pub panel_alt: Color32,
    pub panel_high: Color32,
    pub stroke: Color32,
    pub stroke_strong: Color32,

    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,

    pub accent: Color32,
    pub accent_dim: Color32,
    pub accent_line: Color32,
    pub accent_text: Color32,

    pub ok: Color32,
    pub warn: Color32,
    pub error: Color32,
    pub info: Color32,
    /// Soft fills for a semantic state: a chip, a badge, a selected-but-modified cell.
    pub ok_fill: Color32,
    pub warn_fill: Color32,
    pub error_fill: Color32,

    pub grid_header: Color32,
    pub grid_row_alt: Color32,
    pub grid_line: Color32,
    pub selection: Color32,
    /// The cursor cell of a selection rectangle: one step stronger than `selection`, so the live
    /// corner reads as "this is the cell" rather than just part of the block.
    pub selection_strong: Color32,
    pub dirty: Color32,
    pub dirty_text: Color32,
    pub hover: Color32,
    /// Dimming layer under a modal.
    pub scrim: Color32,

    pub chart: [Color32; 6],
}

impl Tokens {
    pub fn of(theme: &str) -> Tokens {
        match theme {
            "light" => Tokens {
                dark: false,
                radius: R_BOX,
                background: Color32::from_rgb(0xf4, 0xf6, 0xf9),
                panel: Color32::from_rgb(0xff, 0xff, 0xff),
                panel_alt: Color32::from_rgb(0xee, 0xf2, 0xf7),
                panel_high: Color32::from_rgb(0xe3, 0xe9, 0xf1),
                stroke: Color32::from_rgb(0xd3, 0xdb, 0xe6),
                stroke_strong: Color32::from_rgb(0xb9, 0xc4, 0xd3),
                text: Color32::from_rgb(0x13, 0x1a, 0x24),
                text_dim: Color32::from_rgb(0x4d, 0x58, 0x66),
                text_faint: Color32::from_rgb(0x5a, 0x65, 0x75),
                accent: Color32::from_rgb(0x1d, 0x4e, 0xd8),
                accent_dim: Color32::from_rgb(0xe2, 0xea, 0xfd),
                accent_line: Color32::from_rgb(0xb9, 0xcd, 0xf8),
                accent_text: Color32::WHITE,
                ok: Color32::from_rgb(0x0f, 0x7a, 0x49),
                warn: Color32::from_rgb(0x8a, 0x5a, 0x00),
                error: Color32::from_rgb(0xb3, 0x2a, 0x2a),
                info: Color32::from_rgb(0x14, 0x63, 0x7f),
                ok_fill: Color32::from_rgb(0xe6, 0xf5, 0xee),
                warn_fill: Color32::from_rgb(0xfd, 0xf3, 0xdd),
                error_fill: Color32::from_rgb(0xfb, 0xea, 0xea),
                grid_header: Color32::from_rgb(0xee, 0xf2, 0xf7),
                grid_row_alt: Color32::from_rgb(0xf7, 0xf9, 0xfc),
                grid_line: Color32::from_rgb(0xe7, 0xec, 0xf3),
                selection: Color32::from_rgb(0xd6, 0xe4, 0xff),
                selection_strong: Color32::from_rgb(0xb9, 0xcd, 0xf8),
                dirty: Color32::from_rgb(0xff, 0xf2, 0xce),
                dirty_text: Color32::from_rgb(0x7a, 0x51, 0x00),
                hover: Color32::from_rgb(0xe3, 0xe9, 0xf1),
                // rgba(19,26,36,.35) — the design's light scrim.
                scrim: Color32::from_rgba_unmultiplied(0x13, 0x1a, 0x24, 89),
                chart: [
                    Color32::from_rgb(0x25, 0x63, 0xeb),
                    Color32::from_rgb(0x0e, 0x9f, 0x6e),
                    Color32::from_rgb(0xd9, 0x7a, 0x0b),
                    Color32::from_rgb(0xcf, 0x3f, 0x3f),
                    Color32::from_rgb(0x7c, 0x5c, 0xd6),
                    Color32::from_rgb(0x1c, 0x8b, 0xa8),
                ],
            },
            _ => Tokens {
                dark: true,
                radius: R_BOX,
                background: Color32::from_rgb(0x1f, 0x1f, 0x1f),
                panel: Color32::from_rgb(0x18, 0x18, 0x18),
                panel_alt: Color32::from_rgb(0x20, 0x20, 0x20),
                panel_high: Color32::from_rgb(0x2b, 0x2b, 0x2b),
                stroke: Color32::from_rgb(0x2f, 0x2f, 0x2f),
                stroke_strong: Color32::from_rgb(0x3c, 0x3c, 0x3c),
                text: Color32::from_rgb(0xcc, 0xcc, 0xcc),
                text_dim: Color32::from_rgb(0x9d, 0x9d, 0x9d),
                text_faint: Color32::from_rgb(0x96, 0x96, 0x96),
                accent: Color32::from_rgb(0x4d, 0xaa, 0xfc),
                accent_dim: Color32::from_rgb(0x04, 0x2a, 0x45),
                accent_line: Color32::from_rgb(0x00, 0x78, 0xd4),
                accent_text: Color32::from_rgb(0x06, 0x26, 0x3f),
                ok: Color32::from_rgb(0x4e, 0xc9, 0xb0),
                warn: Color32::from_rgb(0xcc, 0xa7, 0x00),
                error: Color32::from_rgb(0xf2, 0x6b, 0x6b),
                info: Color32::from_rgb(0x37, 0x94, 0xff),
                ok_fill: Color32::from_rgb(0x12, 0x33, 0x2a),
                warn_fill: Color32::from_rgb(0x33, 0x2a, 0x00),
                error_fill: Color32::from_rgb(0x3a, 0x1d, 0x1d),
                grid_header: Color32::from_rgb(0x20, 0x20, 0x20),
                grid_row_alt: Color32::from_rgb(0x24, 0x24, 0x24),
                grid_line: Color32::from_rgb(0x30, 0x30, 0x30),
                selection: Color32::from_rgb(0x17, 0x32, 0x4f),
                selection_strong: Color32::from_rgb(0x24, 0x50, 0x7f),
                dirty: Color32::from_rgb(0x33, 0x2a, 0x00),
                dirty_text: Color32::from_rgb(0xe5, 0xc0, 0x7b),
                hover: Color32::from_rgb(0x2b, 0x2b, 0x2b),
                // rgba(15,15,15,.6) — the design's dark scrim.
                scrim: Color32::from_rgba_unmultiplied(0x0f, 0x0f, 0x0f, 153),
                chart: [
                    Color32::from_rgb(0x63, 0x94, 0xff),
                    Color32::from_rgb(0x4c, 0xd1, 0x8a),
                    Color32::from_rgb(0xe8, 0xb9, 0x4b),
                    Color32::from_rgb(0xf2, 0x7a, 0x7a),
                    Color32::from_rgb(0xa7, 0x8b, 0xf0),
                    Color32::from_rgb(0x63, 0xc2, 0xe8),
                ],
            },
        }
    }

    pub fn level_color(&self, level: nook_engine::Level) -> Color32 {
        match level {
            nook_engine::Level::Info => self.text_dim,
            nook_engine::Level::Warn => self.warn,
            nook_engine::Level::Error => self.error,
            nook_engine::Level::Success => self.ok,
        }
    }

    pub fn corner(&self) -> CornerRadius {
        CornerRadius::same(self.radius)
    }

    pub fn font(&self, size: f32) -> FontId {
        FontId::proportional(size)
    }

    pub fn mono(&self, size: f32) -> FontId {
        FontId::monospace(size)
    }

    /// Colours, spacing, rounding and text styles for the whole app.
    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = self.panel;
        visuals.window_fill = self.panel;
        visuals.window_stroke = Stroke::new(1.0, self.stroke);
        visuals.extreme_bg_color = self.background;
        visuals.faint_bg_color = self.panel_alt;
        visuals.override_text_color = Some(self.text);
        visuals.hyperlink_color = self.accent;
        visuals.selection.bg_fill = self.selection;
        visuals.selection.stroke = Stroke::new(1.0, self.accent);
        visuals.widgets.noninteractive.bg_fill = self.panel;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.stroke);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text);
        visuals.widgets.inactive.bg_fill = self.panel_alt;
        visuals.widgets.inactive.weak_bg_fill = self.panel_alt;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, self.stroke);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text);
        visuals.widgets.hovered.bg_fill = self.panel_high;
        visuals.widgets.hovered.weak_bg_fill = self.panel_high;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, self.stroke_strong);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.text);
        visuals.widgets.active.bg_fill = self.accent_dim;
        visuals.widgets.active.weak_bg_fill = self.accent_dim;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, self.accent);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, self.text);
        for rounding in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
        ] {
            rounding.corner_radius = CornerRadius::same(R_CTL);
        }
        ctx.set_visuals(visuals);

        let theme = ctx.theme();
        let mut style = (*ctx.style_of(theme)).clone();
        for (text_style, size) in [
            (egui::TextStyle::Body, T_MD),
            (egui::TextStyle::Button, T_MD),
            (egui::TextStyle::Small, T_SM),
            (egui::TextStyle::Heading, T_XL),
            (egui::TextStyle::Monospace, T_MONO),
        ] {
            let mut font = text_style.resolve(&style);
            font.size = size;
            style.text_styles.insert(text_style, font);
        }
        style.spacing.item_spacing = Vec2::new(f32::from(S4), f32::from(S4));
        style.spacing.button_padding = Vec2::new(f32::from(S4), f32::from(S1));
        style.spacing.scroll.bar_width = 10.0;
        style.spacing.window_margin = Margin::same(S6);
        style.spacing.menu_margin = Margin::same(S2);
        style.spacing.indent = INDENT_STEP;
        style.spacing.interact_size.y = BUTTON_H;
        ctx.set_style_of(theme, style);
    }
}

/// Frame for a top or bottom bar. The bar paints its own fill and hairline inside `band_h`, so the
/// frame adds nothing at all: with a stroke or a margin here the bar would be taller than its token,
/// which is exactly what the previous frame did.
pub fn bar_frame(tokens: &Tokens) -> Frame {
    Frame::new()
        .fill(tokens.panel)
        .inner_margin(Margin::same(0))
        .outer_margin(Margin::ZERO)
        .corner_radius(CornerRadius::ZERO)
}

/// Frame for a side or bottom panel.
pub fn side_panel_frame(tokens: &Tokens) -> Frame {
    Frame::new()
        .fill(tokens.panel)
        .inner_margin(Margin::same(0))
        .stroke(Stroke::new(1.0, tokens.stroke))
        .corner_radius(CornerRadius::ZERO)
}

/// Card container: welcome page, monitor tiles, job list.
pub fn card_frame(tokens: &Tokens) -> Frame {
    Frame::new()
        .fill(tokens.panel_alt)
        .stroke(Stroke::new(1.0, tokens.stroke))
        .corner_radius(CornerRadius::same(R_BOX))
        .inner_margin(Margin::same(S5))
}

/// Vertical separator between toolbar clusters.
pub fn divider(ui: &mut Ui, tokens: &Tokens) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 16.0), Sense::hover());
    ui.painter().vline(
        rect.center().x,
        rect.top()..=rect.bottom(),
        Stroke::new(1.0, tokens.stroke),
    );
}

/// Horizontal band of a fixed height: a fill, content aligned vertically, and — important — a bounded
/// rect, so a band can never consume the panel it sits in the way `horizontal_centered` does inside
/// an unbounded layout. Returns the band rect, so the caller can draw its own hairline inside it.
pub fn band_h(ui: &mut Ui, tokens: &Tokens, height: f32, add: impl FnOnce(&mut Ui)) -> egui::Rect {
    band_fill(ui, height, tokens.panel, S5, egui::Align::Center, add)
}

/// `band_h` with an explicit fill, horizontal inset and alignment: the tab strip sits on `panel_alt`
/// with the design's 8px lead-in and its tabs against the band's top edge, while every other bar uses
/// the panel fill, 12px and a centred row.
pub fn band_fill(
    ui: &mut Ui,
    height: f32,
    fill: Color32,
    inset: i8,
    align: egui::Align,
    add: impl FnOnce(&mut Ui),
) -> egui::Rect {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
    let inner = egui::Rect::from_min_max(
        egui::pos2(rect.left() + f32::from(inset), rect.top()),
        egui::pos2(rect.right() - f32::from(inset), rect.bottom()),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(align)),
    );
    add(&mut child);
    rect
}

/// 1px separator drawn *inside* the bottom edge of a band. A bar that draws its own border outside
/// its rect grows by it, which is how the shell bars ended up one pixel taller than their tokens.
pub fn band_hairline(ui: &Ui, tokens: &Tokens, rect: egui::Rect) {
    ui.painter().hline(
        rect.left()..=rect.right(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, tokens.stroke),
    );
}

/// `band_hairline` for a bar the design gives a `border-top`: the status bar's line sits at the top
/// of its 26px band, not at the bottom where it would also fall on the window's last row.
pub fn band_top_hairline(ui: &Ui, tokens: &Tokens, rect: egui::Rect) {
    ui.painter().hline(
        rect.left()..=rect.right(),
        rect.top() + 0.5,
        Stroke::new(1.0, tokens.stroke),
    );
}

/// One line of `text` in `font`, so a band can reserve a row's own fact before it paints the row's
/// pills around it.
pub fn text_width(ui: &Ui, font: FontId, text: &str) -> f32 {
    measured(ui, font, text)
}

/// What a bounded row prints for the pills it left out.
pub fn more_note(count: usize) -> String {
    format!("+{count} more")
}

/// The width of that pill: one mono line in the chip's own padding, so a row can reserve it before it
/// decides how many pills fit.
pub fn more_pill_width(ui: &Ui, tokens: &Tokens, count: usize) -> f32 {
    measured(ui, tokens.mono(T_XS), &more_note(count)) + 2.0 * f32::from(S3) + 2.0
}

/// The `+N more` pill, at the height of the row that prints it.
pub fn more_pill(
    ui: &mut Ui,
    tokens: &Tokens,
    count: usize,
    height: f32,
    fill: Color32,
) -> Response {
    let label = more_note(count);
    let width = more_pill_width(ui, tokens, count);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let radius = CornerRadius::same(R_CHIP);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, fill);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, tokens.stroke),
        StrokeKind::Inside,
    );
    painter.text(
        egui::pos2(rect.left() + f32::from(S3), rect.center().y),
        egui::Align2::LEFT_CENTER,
        label.clone(),
        tokens.mono(T_XS),
        tokens.text_dim,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), &label)
    });
    response
}

/// Which pills of a row fit its band, and how many are counted in a `+N more` pill on each side.
///
/// A window can be empty — `start == end + 1` — when the band is narrower than even one elided pill.
/// That is a row that paints nothing rather than a row that paints past the pane: both callers paint
/// `start..=end`, which is empty that way round, so they need no case of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fitted {
    /// The first pill painted.
    pub start: usize,
    /// The last pill painted, inclusive.
    pub end: usize,
    /// The pills left out before `start`, which the head note counts — 0 when the row could not afford
    /// the note, so a caller that paints one only when the count is non-zero paints nothing it did not
    /// plan.
    pub head: usize,
    /// The pills left out after `end`, which the tail note counts. 0 like [`Fitted::head`].
    pub tail: usize,
}

impl Fitted {
    /// The empty window: the band is too narrow for the pills, so the row paints none of them.
    fn none() -> Fitted {
        Fitted {
            start: 1,
            end: 0,
            head: 0,
            tail: 0,
        }
    }

    /// Does this window paint a pill? False for [`Fitted::none`], which is what a budget narrower than
    /// one elided pill comes to.
    fn paints(&self) -> bool {
        self.start <= self.end
    }

    /// What painting this window costs: its pills, one `gap` between each pair, and one `note_w` pill
    /// for each side it counted.
    fn width(&self, gap: f32, note_w: f32, widths: &[f32]) -> f32 {
        if widths.is_empty() || self.start > self.end {
            return 0.0;
        }
        let pills = widths[self.start..=self.end].iter().sum::<f32>()
            + gap * (self.end - self.start) as f32;
        let head = if self.head > 0 { gap + note_w } else { 0.0 };
        let tail = if self.tail > 0 { gap + note_w } else { 0.0 };
        pills + head + tail
    }
}

/// A bounded pill row that ends with the band's own fact: the window of pills to paint and whether
/// the fact fits beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// The pills this row paints.
    pub fitted: Fitted,
    /// Paint the band's fact after the pills? False when the fact's own width does not fit the band,
    /// and when reserving it would leave the pills less than the window needs — the case the app's own
    /// minimum window reaches.
    pub fact: bool,
}

/// The window of a pill row that fits `budget`, `gap` between neighbours and one `note_w` pill for
/// each side the row truncates — keeping `active` inside the window when the caller names one.
///
/// Both rows this bounds are `nowrap` flex rows in the design with the band's own fact pinned to the
/// right, and nothing in them stops the pills painting over the fact and past the pane: the results
/// register's run total was clipped mid-glyph at five statements on the 776px pane and dropped
/// altogether at six. So a row is planned before it is painted — the fact keeps the width it measured
/// and the pills that do not fit are counted rather than drawn.
///
/// The pill the caller names is what the window is built around when the left-hand run does not reach
/// it, which is the case that matters: the register opens on the run's last statement.
///
/// A `+N more` pill is a pill like any other, so it is dropped when it cannot fit beside the pills
/// that do, and `head`/`tail` come back 0 — both rows this bounds state the count otherwise (the
/// editor's toolbar counts the statements, the register's `.rbar` note names the ordinal `of` it), so
/// the row spends what it has on the pills rather than on the tally of the ones it cannot paint.
///
/// `ponytail:` a row that does not fit hides pills instead of scrolling; the design's own answer
/// would be to let the row scroll. The upgrade path is a horizontal `ScrollArea` inside the band,
/// which needs a band as wide as its content — `band_fill` is the pane's width and its fill stops at
/// the pane's edge.
pub fn fit_row(
    budget: f32,
    gap: f32,
    widths: &[f32],
    note_w: f32,
    active: Option<usize>,
) -> Fitted {
    let fitted = window(budget, gap, widths, note_w, active);
    if fitted.width(gap, note_w, widths) <= budget {
        return fitted;
    }
    // The notes do not fit even though the pills do: paint the pills and count nothing.
    let bare = Fitted {
        head: 0,
        tail: 0,
        ..window(budget, gap, widths, 0.0, active)
    };
    if bare.width(gap, 0.0, widths) <= budget {
        return bare;
    }
    // Not even one pill fits: paint none rather than paint it over the pane's edge. An elision stops
    // at its ellipsis, so this is a band a glyph or two wide, and what the row would have said there is
    // said elsewhere — the toolbar counts the statements and the `.rbar` names the active one.
    Fitted::none()
}

/// [`fit_row`]'s window, with the notes it would reserve for the pills it leaves out. Its caller checks
/// that the window it returns really fits before painting the counts it planned.
fn window(budget: f32, gap: f32, widths: &[f32], note_w: f32, active: Option<usize>) -> Fitted {
    let count = widths.len();
    if count == 0 {
        return Fitted {
            start: 0,
            end: 0,
            head: 0,
            tail: 0,
        };
    }
    let span = |first: usize, last: usize| -> f32 {
        widths[first..=last].iter().sum::<f32>() + gap * (last - first) as f32
    };
    if span(0, count - 1) <= budget {
        return Fitted {
            start: 0,
            end: count - 1,
            head: 0,
            tail: 0,
        };
    }
    // What fits from the left, with one note reserved for the rest.
    let mut end = 0;
    while end + 1 < count && span(0, end + 1) + gap + note_w <= budget {
        end += 1;
    }
    if active.is_none_or(|active| active <= end) {
        return Fitted {
            start: 0,
            end,
            head: 0,
            tail: count - 1 - end,
        };
    }
    // The active pill is past that prefix, so the row is windowed around it instead: the note it
    // leaves at the head does not stand between the user and the statement on screen.
    let active = active.unwrap_or(0).min(count - 1);
    let tail = if active + 1 < count {
        gap + note_w
    } else {
        0.0
    };
    let mut start = active;
    while start > 0 && note_w + gap + span(start - 1, active) + tail <= budget {
        start -= 1;
    }
    Fitted {
        start,
        end: active,
        head: start,
        tail: count - 1 - active,
    }
}

/// Plan a row of pills that ends with the band's own fact: the fact keeps the width it measured, the
/// pills are windowed into what is left of `budget`, and when that reservation leaves the pills less
/// than the window they need the fact is dropped rather than painted over them.
///
/// This is the design's `nowrap` row with its fact pinned right, and the shell does not only ever make
/// it the design's 776px: the window's own minimum leaves the central pane 236px. So the row gives
/// things up in the order that costs the user least — the fact first, because it is the row's summary
/// and the pills' own facts already say what each statement cost; then, inside [`fit_row`], the
/// counts; and the volume a pill prints is bounded by its caller, which elides a tab's or a chip's
/// text to `budget` so no pill is ever planned wider than the band it is painted in.
///
/// `ponytail:` a pill that cannot be elided into the band it is given is dropped whole, so a band
/// narrower than one of them — about 60px of ordinal, ellipsis and paddings, against the 236 the app's
/// minimum window leaves the pane — paints none of them, and the fact is never painted in their place.
/// The upgrade path is the scrolling band [`fit_row`] names.
pub fn fit_row_with_fact(
    budget: f32,
    gap: f32,
    widths: &[f32],
    note_w: f32,
    fact_w: f32,
    active: Option<usize>,
) -> Row {
    // The fact is pinned to the band's right edge, so it keeps its width only when what is left of the
    // band still holds a pill: a row whose pills have all been dropped to keep its total would be the
    // wrong way round, and a band that cannot hold the fact and a pill is the case the app's own
    // minimum window reaches.
    let with_fact = budget - fact_w - gap;
    if with_fact >= 0.0 {
        let fitted = fit_row(with_fact, gap, widths, note_w, active);
        if fitted.paints() && fitted.width(gap, note_w, widths) <= with_fact {
            return Row { fitted, fact: true };
        }
    }
    Row {
        fitted: fit_row(budget, gap, widths, note_w, active),
        fact: false,
    }
}

/// Toolbar-height band: the shape every view header uses.
pub fn band(ui: &mut Ui, tokens: &Tokens, add: impl FnOnce(&mut Ui)) -> egui::Rect {
    band_h(ui, tokens, TOOLBAR_H, add)
}

/// Filled, high-emphasis button: 28px tall, `--r-ctl`, accent fill.
pub fn primary_button(ui: &mut Ui, tokens: &Tokens, label: &str, enabled: bool) -> Response {
    let text = RichText::new(label).size(T_MD).color(if enabled {
        tokens.accent_text
    } else {
        tokens.text_dim
    });
    let fill = if enabled {
        tokens.accent
    } else {
        tokens.panel_high
    };
    ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .fill(fill)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(R_CTL))
            .min_size(Vec2::new(0.0, BUTTON_H)),
    )
}

/// The design's `.btn.ba`: a 24px button outlined in `accent_line` with `accent` ink, which fills
/// with `accent_dim` under the pointer. The quiet half of the pair whose other half is
/// [`primary_button`], and the one the panes use for "open the thing this panel is describing".
pub fn accent_button(ui: &mut Ui, tokens: &Tokens, label: &str, enabled: bool) -> Response {
    let text = RichText::new(label).size(T_SM).color(if enabled {
        tokens.accent
    } else {
        tokens.text_dim
    });
    let response = ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(
                1.0,
                if enabled {
                    tokens.accent_line
                } else {
                    tokens.stroke
                },
            ))
            .corner_radius(CornerRadius::same(R_CTL))
            .min_size(Vec2::new(0.0, CHIP_H)),
    );
    if response.hovered() && enabled {
        let rect = response.rect;
        ui.painter()
            .rect_filled(rect, CornerRadius::same(R_CTL), tokens.accent_dim);
    }
    response
}

/// Quiet button for toolbars: nothing painted until the pointer is over it.
pub fn ghost_button(ui: &mut Ui, tokens: &Tokens, label: &str, enabled: bool) -> Response {
    // Disabled has to read as "not now", not as "not drawn".
    let text = RichText::new(label).size(T_MD).color(if enabled {
        tokens.text
    } else {
        tokens.text_dim
    });
    let response = ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(R_CTL))
            .min_size(Vec2::new(0.0, BUTTON_H)),
    );
    if response.hovered() && enabled {
        let rect = response.rect;
        ui.painter()
            .rect_filled(rect, CornerRadius::same(R_CTL), tokens.hover);
    }
    response
}

/// Segmented control: a 26px bordered container holding 24px items split by hairlines, the selected
/// item on `accent_dim` with a 2px accent indicator along its bottom edge — the shape the design
/// defines for `.seg`. Returns the index that was clicked.
pub fn segmented(ui: &mut Ui, tokens: &Tokens, options: &[(&str, bool)]) -> Option<usize> {
    if options.is_empty() {
        return None;
    }
    let font = tokens.font(T_XS);
    let widths: Vec<f32> = options
        .iter()
        .map(|(label, _)| segment_item_width(ui, tokens, label))
        .collect();
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(widths.iter().sum::<f32>() + 2.0, SEG_H),
        Sense::hover(),
    );
    let radius = CornerRadius::same(R_CTL);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, tokens.panel_alt);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, tokens.stroke),
        StrokeKind::Inside,
    );

    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink(1.0))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 0.0;
    let mut clicked = None;
    for (index, ((label, selected), width)) in options.iter().zip(&widths).enumerate() {
        let (item, response) =
            child.allocate_exact_size(Vec2::new(*width, SEG_ITEM_H), Sense::click());
        // Clip the first and last fill to the container's rounding, the way `overflow: hidden` does.
        let fill_radius = match (index, options.len()) {
            (0, 1) => radius,
            (0, _) => CornerRadius {
                nw: R_CTL,
                sw: R_CTL,
                ..CornerRadius::ZERO
            },
            (last, len) if last == len - 1 => CornerRadius {
                ne: R_CTL,
                se: R_CTL,
                ..CornerRadius::ZERO
            },
            _ => CornerRadius::ZERO,
        };
        let painter = child.painter();
        if *selected {
            painter.rect_filled(item, fill_radius, tokens.accent_dim);
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(item.left(), item.bottom() - INDICATOR_W),
                    item.max,
                ),
                CornerRadius::ZERO,
                tokens.accent,
            );
        } else if response.hovered() {
            painter.rect_filled(item, fill_radius, tokens.hover);
        }
        if index > 0 {
            painter.vline(
                item.left(),
                item.top()..=item.bottom(),
                Stroke::new(1.0, tokens.stroke),
            );
        }
        painter.text(
            item.center(),
            egui::Align2::CENTER_CENTER,
            *label,
            font.clone(),
            if *selected {
                tokens.text
            } else {
                tokens.text_dim
            },
        );
        if response.clicked() {
            clicked = Some(index);
        }
    }
    clicked
}

/// The item rounding inside a pane's segmented control: the design's 3px, one less than `R_CTL`.
const SEG_ITEM_RADIUS: u8 = 3;

/// The room the items of a pane segmented control share: the container less its 1px border and its
/// inset, less the gaps between the items, split equally. `None` when that leaves under a pixel for
/// an item — the caller asked for a strip its labels cannot fit, and an inverted child rect panics.
pub fn segment_item(outer: Vec2, inset: f32, count: usize) -> Option<(f32, f32)> {
    if count == 0 {
        return None;
    }
    let inner = Vec2::new(
        (outer.x - 2.0 * (1.0 + inset)).max(0.0),
        (outer.y - 2.0 * (1.0 + inset)).max(0.0),
    );
    let count = count as f32;
    let item_width = ((inner.x - inset * (count - 1.0)) / count).max(0.0);
    if item_width < 1.0 || inner.y < 1.0 {
        return None;
    }
    Some((item_width, inner.y))
}

/// The design's other segmented control: the one its panes use (`.tabs .seg` in the inspector,
/// `.filterblk .seg` in the tree). Same container as [`segmented`], but the items are equal widths
/// across `width`, `inset` is both the inner padding and the gap between items, and the selection is
/// marked with an `accent_line` ring rather than the top bar's bottom indicator. `height` and `inset`
/// are the caller's literals because the design states them per pane: 28 with a 1px inset in the
/// tree, 26 with a 2px inset in the inspector. The items take what is left over, so the selected
/// item's ink stays the caller's choice — the inspector names it in `accent`, the tree in `text`.
pub fn pane_segmented(
    ui: &mut Ui,
    tokens: &Tokens,
    options: &[(&str, bool)],
    width: f32,
    height: f32,
    inset: f32,
    selected_ink: Color32,
) -> Option<usize> {
    if options.is_empty() {
        return None;
    }
    let font = tokens.font(T_XS);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let radius = CornerRadius::same(R_CTL);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, tokens.panel_alt);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, tokens.stroke),
        StrokeKind::Inside,
    );

    // The border row and the padding are spent before the items get their share.
    let inner = rect.shrink(1.0 + inset);
    let (item_width, item_height) = segment_item(rect.size(), inset, options.len())?;
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing.x = inset;
    let mut clicked = None;
    for (index, (label, selected)) in options.iter().enumerate() {
        let (item, response) =
            child.allocate_exact_size(Vec2::new(item_width, item_height), Sense::click());
        let painter = child.painter();
        if *selected {
            painter.rect_filled(item, CornerRadius::same(SEG_ITEM_RADIUS), tokens.accent_dim);
            painter.rect_stroke(
                item,
                CornerRadius::same(SEG_ITEM_RADIUS),
                Stroke::new(1.0, tokens.accent_line),
                StrokeKind::Inside,
            );
        } else if response.hovered() {
            painter.rect_filled(item, CornerRadius::same(SEG_ITEM_RADIUS), tokens.panel_high);
        }
        painter.text(
            item.center(),
            egui::Align2::CENTER_CENTER,
            *label,
            font.clone(),
            if *selected {
                selected_ink
            } else if response.hovered() {
                tokens.text
            } else {
                tokens.text_dim
            },
        );
        if response.clicked() {
            clicked = Some(index);
        }
    }
    clicked
}

/// Small label in a rounded box: counts, states, versions.
///
/// The chip allocates its own 24px box rather than wrapping a `Frame`: a frame inside a horizontal
/// layout stretches to the row height the parent offers (40px inside a toolbar band), which is how a
/// chip stops being a chip.
///
/// ponytail: the design's chips are 600-weight; egui's default proportional family has no weight
/// axis, so the chip reads by colour, size and shape alone. Add a bold font family if one is ever
/// bundled.
pub fn chip(ui: &mut Ui, tokens: &Tokens, text: &str, colour: Color32) -> Response {
    let font = tokens.font(T_XS);
    let text_width = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::PLACEHOLDER)
        .size()
        .x;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(text_width + 2.0 * f32::from(S4), CHIP_H),
        Sense::hover(),
    );
    let radius = CornerRadius::same(R_CHIP);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, tokens.panel_alt);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, tokens.stroke),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font,
        colour,
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), text));
    response
}

/// Coloured dot used for connection state and message levels.
pub fn dot(ui: &mut Ui, colour: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(9.0, 9.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.5, colour);
}

/// Section heading with an optional subtitle.
pub fn section(ui: &mut Ui, tokens: &Tokens, title: &str, subtitle: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(T_MD).color(tokens.text));
        if let Some(subtitle) = subtitle {
            ui.label(RichText::new(subtitle).size(T_XS).color(tokens.text_faint));
        }
    });
}

/// Hand-drawn progress bar: parity meter, monitor size bars, chart bars.
pub fn meter(ui: &mut Ui, tokens: &Tokens, fraction: f32, width: f32, colour: Color32) -> Response {
    let fraction = fraction.clamp(0.0, 1.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 8.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(R_CTL), tokens.panel_alt);
    let filled =
        egui::Rect::from_min_size(rect.min, Vec2::new(rect.width() * fraction, rect.height()));
    ui.painter()
        .rect_filled(filled, CornerRadius::same(R_CTL), colour);
    response
}

/// Key cap, e.g. `Enter`, `Esc`.
pub fn keycap(ui: &mut Ui, tokens: &Tokens, text: &str) -> Response {
    let font = tokens.mono(T_XS);
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(keycap_width(ui, tokens, text), KEYCAP_H),
        Sense::hover(),
    );
    let radius = CornerRadius::same(R_CTL);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, tokens.panel_alt);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, tokens.stroke),
        StrokeKind::Inside,
    );
    // The design's `box-shadow: 0 1px 0 --border`: a hairline under the key, not a blur.
    painter.hline(
        rect.left()..=rect.right(),
        rect.bottom() + 0.5,
        Stroke::new(1.0, tokens.stroke),
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font,
        tokens.text,
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), text));
    response
}

/// The row height a density preference asks for: the design's `--h-row` or `--h-row-comfort`.
pub fn row_h(density: nook_engine::Density) -> f32 {
    match density {
        nook_engine::Density::Compact => ROW_H,
        nook_engine::Density::Comfortable => ROW_COMFORT_H,
    }
}

/// Width of an `.iconbtn` for `label` and an optional glyph, so a caller can reserve the space
/// before it paints — that is how the top bar's palette field knows how wide it may flex.
pub fn icon_button_width(ui: &Ui, tokens: &Tokens, label: &str, glyph: bool) -> f32 {
    let label_width = ui
        .painter()
        .layout_no_wrap(label.to_owned(), tokens.font(T_XS), Color32::PLACEHOLDER)
        .size()
        .x;
    label_width + if glyph { GLYPH_SM + f32::from(S3) } else { 0.0 } + 2.0 * f32::from(S4) + 2.0
}

/// Width of a `.keycap` for `text`.
pub fn keycap_width(ui: &Ui, tokens: &Tokens, text: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), tokens.mono(T_XS), Color32::PLACEHOLDER)
        .size()
        .x
        + 2.0 * f32::from(S3)
        + 2.0
}

/// Width of a `.chip` or `.agent` for `text`: the 6px dot, the 6px gap and 8px of padding each side.
pub fn chip_width(ui: &Ui, tokens: &Tokens, text: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), tokens.font(T_XS), Color32::PLACEHOLDER)
        .size()
        .x
        + 6.0
        + f32::from(S3)
        + 2.0 * f32::from(S4)
        + 2.0
}

/// Width of an item inside a `.seg`: its label plus 8px of padding.
fn segment_item_width(ui: &Ui, tokens: &Tokens, label: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(label.to_owned(), tokens.font(T_XS), Color32::PLACEHOLDER)
        .size()
        .x
        + 2.0 * f32::from(S4)
}

/// Width of the whole `.seg` for `labels`, borders included.
pub fn segmented_width(ui: &Ui, tokens: &Tokens, labels: &[&str]) -> f32 {
    labels
        .iter()
        .map(|label| segment_item_width(ui, tokens, label))
        .sum::<f32>()
        + 2.0
}

/// Width of an `.engine` tag: uppercase mono with 6px of padding each side and a 1px border.
///
/// The text is upper-cased here because that is what [`paint_tag`] draws: measuring the string as
/// passed under-reserves for a lowercase caller.
pub fn tag_width(ui: &Ui, tokens: &Tokens, text: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_uppercase(), tokens.mono(T_XS), Color32::PLACEHOLDER)
        .size()
        .x
        + 2.0 * f32::from(S3)
        + 2.0
}

/// Paint an `.engine` tag inside `rect`: an uppercase mono pill on `surface-3`. The caller owns the
/// box, because the tag is a control's child and not a widget of its own.
pub fn paint_tag(ui: &Ui, tokens: &Tokens, rect: egui::Rect, text: &str) {
    let radius = CornerRadius::same(R_CTL);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, tokens.hover);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, tokens.stroke),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text.to_uppercase(),
        tokens.mono(T_XS),
        tokens.text_dim,
    );
}

/// The design's `.iconbtn` and `.ws`: a 26px bordered control on `surface-2`, its ink `text_dim`.
pub fn icon_button(
    ui: &mut Ui,
    tokens: &Tokens,
    label: &str,
    glyph: Option<crate::icons::Glyph>,
    enabled: bool,
) -> Response {
    let font = tokens.font(T_XS);
    let width = icon_button_width(ui, tokens, label, glyph.is_some());
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(width, ICON_BUTTON_H),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let radius = CornerRadius::same(R_CTL);
    let ink = if enabled {
        tokens.text_dim
    } else {
        tokens.text_faint
    };
    let painter = ui.painter();
    painter.rect_filled(rect, radius, tokens.panel_alt);
    // The hover fill goes under the stroke: egui paints in call order, so a fill after the border
    // would swallow it, and the design keeps the 1px border in every state.
    if response.hovered() && enabled {
        painter.rect_filled(rect, radius, tokens.hover);
    }
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, tokens.stroke),
        StrokeKind::Inside,
    );
    let mut x = rect.left() + f32::from(S4) + 1.0;
    if let Some(glyph) = glyph {
        crate::icons::glyph(
            ui,
            egui::Rect::from_min_size(
                egui::pos2(x, rect.center().y - GLYPH_SM / 2.0),
                Vec2::splat(GLYPH_SM),
            ),
            glyph,
            ink,
        );
        x += GLYPH_SM + f32::from(S3);
    }
    painter.text(
        egui::pos2(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        ink,
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    response
}

/// The design's `.chip` and `.agent`: a 24px pill with a leading 6px dot, filled or outlined.
pub fn state_chip(
    ui: &mut Ui,
    tokens: &Tokens,
    text: &str,
    fill: Option<Color32>,
    border: Option<Color32>,
    ink: Color32,
    dot: Color32,
) -> Response {
    let font = tokens.font(T_XS);
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(chip_width(ui, tokens, text), CHIP_H),
        Sense::hover(),
    );
    let radius = CornerRadius::same(R_CHIP);
    let painter = ui.painter();
    // The design has three of these and they differ exactly in these two arguments: `.chip` (a dirty
    // count) is a filled pill with no border, `.chip.zero` is surface-2 inside a border, and `.agent`
    // is only a border — the bar's own fill shows through.
    if let Some(fill) = fill {
        painter.rect_filled(rect, radius, fill);
    }
    if let Some(border) = border {
        painter.rect_stroke(rect, radius, Stroke::new(1.0, border), StrokeKind::Inside);
    }
    painter.circle_filled(
        egui::pos2(rect.left() + f32::from(S4) + 1.0 + 3.0, rect.center().y),
        3.0,
        dot,
    );
    painter.text(
        egui::pos2(
            rect.left() + f32::from(S4) + 1.0 + 6.0 + f32::from(S3),
            rect.center().y,
        ),
        egui::Align2::LEFT_CENTER,
        text,
        font,
        ink,
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), text));
    response
}

/// Measure one line of `text`, so a band can reserve room before it paints.
/// The width a string takes in a token's own font, for a caller sizing a control around it.
pub fn measured_width(ui: &Ui, tokens: &Tokens, text: &str) -> f32 {
    measured(ui, tokens.mono(T_XS), text)
}

fn measured(ui: &Ui, font: FontId, text: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font, Color32::PLACEHOLDER)
        .size()
        .x
}

/// `.fchips` height: an 18px chip with 7px of air above and below, the design's 32.
pub fn filter_row_h(chip_h: f32) -> f32 {
    chip_h + 2.0 * FCHIP_AIR
}

/// One micro tag's shape: `.pkbadge` and `.taglite` are the same box in different tokens — a 16px
/// box, a pill or 3px radius, a 1px border and one line of text, sized from its own measurement.
#[derive(Clone)]
struct MicroTag {
    font: FontId,
    ink: Color32,
    fill: Color32,
    border: Color32,
    radius: u8,
}

/// The one definition of `.pkbadge`: 11px mono ink, `accent_soft` over an `accent_line` border.
fn pk_style(tokens: &Tokens) -> MicroTag {
    MicroTag {
        font: tokens.mono(T_XS),
        ink: tokens.text,
        fill: tokens.accent_dim,
        border: tokens.accent_line,
        radius: 3,
    }
}

fn micro_tag(ui: &mut Ui, text: &str, tag: &MicroTag) -> Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(micro_tag_width(ui, tag, text), MICRO_TAG_H),
        Sense::hover(),
    );
    paint_micro_tag(ui, rect, text, tag);
    response
}

/// The width a micro tag measures for `text`: the design's padding either side, plus its 1px border.
fn micro_tag_width(ui: &Ui, tag: &MicroTag, text: &str) -> f32 {
    measured(ui, tag.font.clone(), text) + 2.0 * f32::from(S3) + 2.0
}

/// A micro tag painted into a rect the caller has already laid out. The widgets above measure and
/// allocate one; a surface that lays out its own row — the grid's head does — paints the same shape
/// through this, so the two can never drift apart.
fn paint_micro_tag(ui: &Ui, rect: Rect, text: &str, tag: &MicroTag) {
    let painter = ui.painter();
    if tag.fill != Color32::TRANSPARENT {
        painter.rect_filled(rect, CornerRadius::same(tag.radius), tag.fill);
    }
    painter.rect_stroke(
        rect,
        CornerRadius::same(tag.radius),
        Stroke::new(1.0, tag.border),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        tag.font.clone(),
        tag.ink,
    );
}

/// `.pkbadge`: the primary-key mark the design puts beside a table's name and in the grid head.
pub fn pk_badge(ui: &mut Ui, tokens: &Tokens, text: &str) -> Response {
    micro_tag(ui, text, &pk_style(tokens))
}

/// The width `.pkbadge` measures, so a surface that reserves room for one asks here rather than
/// guessing — the grid head does exactly that before it paints it.
pub fn pk_badge_width(ui: &Ui, tokens: &Tokens, text: &str) -> f32 {
    micro_tag_width(ui, &pk_style(tokens), text)
}

/// `.pkbadge` painted into `rect`: `pk_badge`'s geometry, for a caller that lays the badge out
/// itself. `rect` is the design's 16px-tall box, so a caller gets it from `pk_badge_width` and
/// `MICRO_TAG_H`.
pub fn paint_pk_badge(ui: &Ui, tokens: &Tokens, rect: Rect) {
    paint_micro_tag(ui, rect, "PK", &pk_style(tokens));
}

/// `.taglite`: the design's micro tag for one quiet fact about the grid, e.g. `read-only`.
pub fn tag_lite(ui: &mut Ui, tokens: &Tokens, text: &str) -> Response {
    micro_tag(
        ui,
        text,
        &MicroTag {
            font: tokens.font(T_XS),
            ink: tokens.text_dim,
            fill: Color32::TRANSPARENT,
            border: tokens.stroke_strong,
            radius: R_CHIP,
        },
    )
}

/// Height of `.errband` and `.robanner`: 10px of padding above and below a 16px title line and a
/// 16px message line, 2px apart.
pub fn notice_band_h() -> f32 {
    2.0 * NOTICE_PAD_V + 16.0 + f32::from(S1) + 16.0
}

/// `.errband`: the page that failed to load — an `error_soft` fill, a 1px `--error` bottom border,
/// the warning triangle (`.ei`), the h4, the engine's message in mono, and the caller's actions
/// pinned to the right.
pub fn error_band(
    ui: &mut Ui,
    tokens: &Tokens,
    title: &str,
    message: &str,
    add: impl FnOnce(&mut Ui),
) -> egui::Rect {
    notice_band(
        ui,
        tokens,
        tokens.error_fill,
        tokens.error,
        (crate::icons::Glyph::Warning, tokens.error),
        tokens.mono(T_SM),
        tokens.error,
        title,
        message,
        add,
    )
}

/// `.robanner`: the same box in `warn_soft`, for the grid that cannot be edited. Its paragraph is
/// the design's proportional `--t-xs`, unlike the error band's mono message.
pub fn ro_banner(
    ui: &mut Ui,
    tokens: &Tokens,
    title: &str,
    text: &str,
    add: impl FnOnce(&mut Ui),
) -> egui::Rect {
    notice_band(
        ui,
        tokens,
        tokens.warn_fill,
        tokens.warn,
        (crate::icons::Glyph::Lock, tokens.warn),
        tokens.font(T_XS),
        tokens.text_dim,
        title,
        text,
        add,
    )
}

#[allow(clippy::too_many_arguments)]
fn notice_band(
    ui: &mut Ui,
    tokens: &Tokens,
    fill: Color32,
    edge: Color32,
    mark: (crate::icons::Glyph, Color32),
    message_font: FontId,
    message_ink: Color32,
    title: &str,
    message: &str,
    add: impl FnOnce(&mut Ui),
) -> egui::Rect {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, notice_band_h()), Sense::hover());
    {
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::ZERO, fill);
        painter.hline(
            rect.left()..=rect.right(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, edge),
        );
    }
    let inner = egui::Rect::from_min_max(
        egui::pos2(rect.left() + f32::from(S5), rect.top() + NOTICE_PAD_V),
        egui::pos2(rect.right() - f32::from(S5), rect.bottom() - NOTICE_PAD_V),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Min)),
    );
    child.spacing_mut().item_spacing.y = f32::from(S1);
    child.horizontal(|ui| {
        // The band's leading mark: the design's `.ei`/`.ri` beside the h4, 1px down from its cap.
        let (mark_rect, _) = ui.allocate_exact_size(Vec2::splat(GLYPH_BOX), Sense::hover());
        crate::icons::glyph(ui, mark_rect.translate(Vec2::Y), mark.0, mark.1);
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(T_MD).color(tokens.text));
            ui.label(RichText::new(message).font(message_font).color(message_ink));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| add(ui));
    });
    rect
}

/// The design's `.btn`: a 26px control with 8px of padding and 6px between its mark, its label and
/// whatever the caller appends. `fill` is the sheet's resting fill, because the editor's `.tb` and the
/// register's `.rbar` state different ones — `panel_high` and `panel_alt`.
///
/// `add` runs inside the button, after the label, and the button grows to fit it. The closure paints
/// before the button's own chrome is laid down — egui has no second pass — so the fill, the pointer's
/// fill and the border go into shape slots reserved before the caller's content.
#[allow(clippy::too_many_arguments)]
pub fn toolbar_button(
    ui: &mut Ui,
    tokens: &Tokens,
    label: &str,
    mark: Option<Mark>,
    fill: Color32,
    emphasis: Emphasis,
    enabled: bool,
    add: impl FnOnce(&mut Ui),
) -> Response {
    let face = button_face(tokens, fill, emphasis, enabled);
    let font = tokens.font(T_SM);
    let (width, lead, label_width) = button_width(ui, tokens, label, mark);
    let radius = CornerRadius::same(R_CTL);
    let slots = {
        let painter = ui.painter();
        [
            painter.add(egui::Shape::Noop),
            painter.add(egui::Shape::Noop),
            painter.add(egui::Shape::Noop),
        ]
    };
    let (id, placed) = ui.allocate_space(Vec2::new(width, TOOLBAR_BTN_H));
    // The caller's content starts a gap after the label: `lead` already carries the padding, the
    // border and the mark's own gap.
    let trailing = placed.left() + lead + label_width + f32::from(S3);
    let tail = {
        let room = Rect::from_min_max(
            egui::pos2(trailing, placed.top()),
            egui::pos2(
                ui.available_rect_before_wrap().right().max(trailing),
                placed.bottom(),
            ),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(room)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        add(&mut child);
        child.min_rect().width()
    };
    // The button grows to fit what the caller appended, and answers for the whole of itself: a
    // response that stopped at the label would leave the caret it drew outside its own hit box.
    let rect = if tail > 0.0 {
        let grown = Rect::from_min_max(
            placed.min,
            egui::pos2(placed.right() + f32::from(S3) + tail, placed.bottom()),
        );
        ui.advance_cursor_after_rect(grown);
        grown
    } else {
        placed
    };
    let response = ui.interact(
        rect,
        id,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );

    let painter = ui.painter();
    if face.fill != Color32::TRANSPARENT {
        painter.set(slots[0], egui::Shape::rect_filled(rect, radius, face.fill));
    }
    if let Some(hover) = face.hover {
        if response.hovered() {
            painter.set(slots[1], egui::Shape::rect_filled(rect, radius, hover));
        }
    }
    painter.set(
        slots[2],
        egui::Shape::rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, face.border),
            StrokeKind::Inside,
        ),
    );
    let mut x = rect.left() + f32::from(S4) + 1.0;
    if let Some(mark) = mark {
        let size = mark.size();
        let box_rect = Rect::from_min_size(egui::pos2(x, rect.center().y - size.y / 2.0), size);
        match mark {
            Mark::Glyph(glyph) => {
                crate::icons::glyph(ui, box_rect, glyph, face.ink);
            }
            Mark::Play => {
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        box_rect.left_top(),
                        egui::pos2(box_rect.right(), box_rect.center().y),
                        box_rect.left_bottom(),
                    ],
                    face.ink,
                    Stroke::NONE,
                ));
            }
            Mark::Stop => {
                painter.rect_filled(box_rect, CornerRadius::same(1), face.ink);
            }
        }
        x += size.x + f32::from(S3);
    }
    painter.text(
        egui::pos2(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        face.ink,
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    response
}

/// The width [`toolbar_button`] takes for `label` and `mark`, so a caller can prove its toolbar fits
/// inside the pane before the band is laid out.
pub fn toolbar_button_width(ui: &Ui, tokens: &Tokens, label: &str, mark: Option<Mark>) -> f32 {
    button_width(ui, tokens, label, mark).0
}

/// A `.btn`'s total width, the width it spends before its label (a padding, the border, the mark and
/// its gap) and the label's own width — the caller's trailing content starts after that last one.
fn button_width(ui: &Ui, tokens: &Tokens, label: &str, mark: Option<Mark>) -> (f32, f32, f32) {
    let label_width = measured(ui, tokens.font(T_SM), label);
    let lead = f32::from(S4) + 1.0 + mark.map_or(0.0, |mark| mark.size().x + f32::from(S3));
    (lead + label_width + f32::from(S4) + 1.0, lead, label_width)
}

/// The mark a `.btn` carries in front of its label: a stroked [`crate::icons::Glyph`], or one of the
/// editor's filled marks — the run triangle and the stop square, which are fills and so are not glyphs
/// (`icons` is a stroked-geometry module, and a fill would change every existing glyph's contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Glyph(crate::icons::Glyph),
    Play,
    Stop,
}

impl Mark {
    /// The box the mark occupies, which is what a button reserves for it.
    pub fn size(self) -> Vec2 {
        match self {
            Mark::Glyph(_) => Vec2::splat(GLYPH_BOX),
            Mark::Play => Vec2::new(7.0, 10.0),
            Mark::Stop => Vec2::splat(8.0),
        }
    }
}

/// How much a `.btn` insists on being noticed: the design's plain button on its sheet's fill, its
/// `.primary` one on `accent`, its `.danger` one for a destructive action (the dialog's Delete, the
/// confirmation's Drop) and its `.ghost` one, which paints only its ink. `Off` is the face a disabled
/// dialog button wears (`panel_high` fill, `text_dim` ink), which is not what a disabled toolbar button
/// is: [`button_face`]'s `enabled` argument keeps its own transparent face for those.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emphasis {
    Plain,
    Primary,
    Danger,
    Ghost,
    Off,
}

/// What one `.btn` paints: its fill at rest, its 1px border, its ink, and the fill the pointer gets —
/// `None` when the button does not react to the pointer at all, which is what disabled means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonFace {
    pub fill: Color32,
    pub border: Color32,
    pub ink: Color32,
    pub hover: Option<Color32>,
}

/// The design's `.btn` faces, from the sheet's resting fill: the plain button (`stroke` border,
/// `text` ink, `hover` under the pointer), `.primary` (`accent` fill, `accent_line` border,
/// `accent_text` ink), `.danger` (`error` fill and border, `panel` ink — the destructive button reads
/// on the dialog's own surface, not on a chip), `.ghost` (nothing of its own but `text_dim` ink and the
/// pointer's fill) and `.off` (`panel_high` fill, `stroke` border, `text_dim` ink, no reaction).
/// Disabled beats emphasis, because the only disabled button in these frames is the editor's Stop.
pub fn button_face(
    tokens: &Tokens,
    sheet: Color32,
    emphasis: Emphasis,
    enabled: bool,
) -> ButtonFace {
    if !enabled {
        return ButtonFace {
            fill: Color32::TRANSPARENT,
            border: tokens.stroke,
            ink: tokens.text_faint,
            hover: None,
        };
    }
    match emphasis {
        Emphasis::Primary => ButtonFace {
            fill: tokens.accent,
            border: tokens.accent_line,
            ink: tokens.accent_text,
            hover: None,
        },
        Emphasis::Danger => ButtonFace {
            fill: tokens.error,
            border: tokens.error,
            ink: tokens.panel,
            hover: None,
        },
        Emphasis::Ghost => ButtonFace {
            fill: Color32::TRANSPARENT,
            border: Color32::TRANSPARENT,
            ink: tokens.text_dim,
            hover: Some(tokens.hover),
        },
        Emphasis::Off => ButtonFace {
            fill: tokens.panel_high,
            border: tokens.stroke,
            ink: tokens.text_dim,
            hover: None,
        },
        Emphasis::Plain => ButtonFace {
            fill: sheet,
            border: tokens.stroke,
            ink: tokens.text,
            hover: Some(tokens.hover),
        },
    }
}

/// The design's `.note`: a 20px `panel_alt` pill inside a `stroke` border, with an optional 5px dot,
/// carrying one mono fact in `text_dim`.
pub fn note_pill(ui: &mut Ui, tokens: &Tokens, text: &str, dot: Option<Color32>) -> Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(note_pill_width(ui, tokens, text, dot.is_some()), NOTE_H),
        Sense::hover(),
    );
    let radius = CornerRadius::same(R_CHIP);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, tokens.panel_alt);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, tokens.stroke),
        StrokeKind::Inside,
    );
    let x = if let Some(dot) = dot {
        let centre = note_dot_centre(rect);
        painter.circle_filled(centre, NOTE_DOT / 2.0, dot);
        centre.x + NOTE_DOT / 2.0 + f32::from(S3)
    } else {
        rect.left() + f32::from(S4) + 1.0
    };
    painter.text(
        egui::pos2(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        tokens.mono(T_XS),
        tokens.text_dim,
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), text));
    response
}

/// The width a `.note` takes for `text`, with or without its dot.
pub fn note_pill_width(ui: &Ui, tokens: &Tokens, text: &str, dot: bool) -> f32 {
    measured(ui, tokens.mono(T_XS), text)
        + 2.0 * f32::from(S4)
        + 2.0
        + if dot { NOTE_DOT + f32::from(S3) } else { 0.0 }
}

/// Where the `.note`'s 5px dot sits: one padding and the border in from the pill's left edge, on its
/// vertical centre.
pub fn note_dot_centre(pill: Rect) -> egui::Pos2 {
    egui::pos2(
        pill.left() + f32::from(S4) + 1.0 + NOTE_DOT / 2.0,
        pill.center().y,
    )
}

/// `.band`: the statement error band under the strip — `error_fill` with a 1px `error` top edge, the
/// design's `.ico` box beside a 13px title, an 11px mono sub-line, and the caller's actions pinned to
/// the right. The design's own `.band` carries 8px of padding and its content at the top; the 56 it
/// measures to is what the register's layout is built on, so the text column is centred in it.
pub fn statement_band(
    ui: &mut Ui,
    tokens: &Tokens,
    title: &str,
    message: &str,
    add: impl FnOnce(&mut Ui),
) -> egui::Rect {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, STMT_BAND_H), Sense::hover());
    {
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::ZERO, tokens.error_fill);
        painter.hline(
            rect.left()..=rect.right(),
            rect.top() + 0.5,
            Stroke::new(1.0, tokens.error),
        );
    }
    let inner = Rect::from_min_max(
        egui::pos2(rect.left() + f32::from(S5), rect.top() + STMT_BAND_PAD_V),
        egui::pos2(
            rect.right() - f32::from(S5),
            rect.bottom() - STMT_BAND_PAD_V,
        ),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Min)),
    );
    child.spacing_mut().item_spacing.x = f32::from(S4);
    child.horizontal(|ui| {
        let (mark, _) = ui.allocate_exact_size(Vec2::splat(STMT_MARK_BOX), Sense::hover());
        let mark = mark.translate(Vec2::Y * f32::from(S1));
        {
            let painter = ui.painter();
            painter.rect_filled(mark, CornerRadius::same(3), tokens.error);
            painter.text(
                mark.center(),
                egui::Align2::CENTER_CENTER,
                "!",
                tokens.mono(T_XS),
                tokens.background,
            );
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = f32::from(S1);
            ui.add(egui::Label::new(RichText::new(title).size(T_MD).color(tokens.text)).truncate());
            ui.add(
                egui::Label::new(
                    RichText::new(message)
                        .font(tokens.mono(T_XS))
                        .color(tokens.text_dim),
                )
                .truncate(),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            add(ui);
        });
    });
    rect
}

/// `.eband`: the register's own error band, between its `.rbar` and its void. `panel_alt` rather than
/// the editor band's `error_fill` — the design states both and they differ — with a 1px `error`
/// hairline under it, `--s5` of padding, the design's 14px `error` mark, the driver's code in an 18px
/// `error`-ringed tag, the message in `text` and the statement in mono `text_dim`.
pub fn register_error_band(
    ui: &mut Ui,
    tokens: &Tokens,
    tag: &str,
    message: &str,
    statement: &str,
    add: impl FnOnce(&mut Ui),
) -> egui::Rect {
    let width = ui.available_width();
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(width, register_error_band_h()), Sense::hover());
    {
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::ZERO, tokens.panel_alt);
        painter.hline(
            rect.left()..=rect.right(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, tokens.error),
        );
    }
    let pad = f32::from(REG_ERR_PAD);
    let inner = Rect::from_min_max(
        egui::pos2(rect.left() + pad, rect.top() + pad),
        egui::pos2(rect.right() - pad, rect.bottom() - pad),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Min)),
    );
    child.spacing_mut().item_spacing.x = f32::from(S5);
    child.horizontal(|ui| {
        // The design's `.eband > svg`: a 14px mark 2px down from the body's cap.
        let (mark, _) = ui.allocate_exact_size(Vec2::splat(REG_ERR_MARK), Sense::hover());
        crate::icons::glyph(
            ui,
            mark.translate(Vec2::Y * f32::from(S1)),
            crate::icons::Glyph::Warning,
            tokens.error,
        );
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = f32::from(S3);
            let (row, _) = ui.allocate_exact_size(
                Vec2::new(
                    measured(ui, tokens.mono(T_XS), tag) + 2.0 * f32::from(S3) + 2.0,
                    REG_ERR_TAG_H,
                ),
                Sense::hover(),
            );
            let radius = CornerRadius::same(R_CTL);
            let painter = ui.painter();
            painter.rect_stroke(
                row,
                radius,
                Stroke::new(1.0, tokens.error),
                StrokeKind::Inside,
            );
            painter.text(
                row.center(),
                egui::Align2::CENTER_CENTER,
                tag,
                tokens.mono(T_XS),
                tokens.error,
            );
            ui.add(
                egui::Label::new(RichText::new(message).size(T_SM).color(tokens.text)).truncate(),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(statement)
                        .font(tokens.mono(T_XS))
                        .color(tokens.text_dim),
                )
                .truncate(),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            add(ui);
        });
    });
    rect
}

/// `.band`'s text column: a 16px title line, 2px of air and a 16px mono sub-line. The design measures
/// the whole band at 56, which is what the strip and the register below it are laid out against, so
/// the padding this leaves — 11px — is derived rather than the design's own 8.
const STMT_TITLE_H: f32 = 16.0;
const STMT_SUB_H: f32 = 16.0;
/// The design's `.ico`: a 16px filled `error` box carrying the bang.
const STMT_MARK_BOX: f32 = 16.0;
const STMT_BAND_PAD_V: f32 = (STMT_BAND_H - STMT_TITLE_H - S1 as f32 - STMT_SUB_H) / 2.0;

/// `.eband`'s own parts: the design's 14px mark, its 18px tag row and the message and mono statement
/// lines, `--s3` apart.
const REG_ERR_MARK: f32 = 14.0;
const REG_ERR_TAG_H: f32 = 18.0;
const REG_ERR_LINE_H: f32 = 16.0;

/// `.eband`'s height: `--s5` of padding, then its three lines.
pub fn register_error_band_h() -> f32 {
    2.0 * f32::from(REG_ERR_PAD)
        + REG_ERR_TAG_H
        + f32::from(S3)
        + REG_ERR_LINE_H
        + f32::from(S3)
        + REG_ERR_LINE_H
}

/// `.note`'s dot: 5px, on the pill's vertical centre.
const NOTE_DOT: f32 = 5.0;

/// `.stalestrip`: the 26px `surface-2` row between the error band and the grid, its note on the
/// left and one mono fact on the right.
pub fn stale_strip(ui: &mut Ui, tokens: &Tokens, note: &str, fact: &str) -> egui::Rect {
    let rect = band_fill(
        ui,
        STALE_STRIP_H,
        tokens.panel_alt,
        S4,
        egui::Align::Center,
        |ui| {
            ui.label(RichText::new(note).size(T_XS).color(tokens.text_dim));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(fact)
                        .size(T_XS)
                        .monospace()
                        .color(tokens.text_dim),
                );
            });
        },
    );
    band_hairline(ui, tokens, rect);
    rect
}

/// What the `.pend` cluster's buttons asked for this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    Revert,
    Apply,
}

/// `.pend`'s sentence: `1 pending`, `3 pending`.
pub fn pending_sentence(counts: (usize, usize, usize)) -> String {
    format!("{} pending", pending_total(counts))
}

/// The staged buffer's size: inserts + updates + deletes.
pub fn pending_total(counts: (usize, usize, usize)) -> usize {
    counts.0 + counts.1 + counts.2
}

/// `.pend`'s counts by kind in the design's order — `+2 ●1 −3` — each kind left out when it is zero.
pub fn pending_counts_text(counts: (usize, usize, usize)) -> String {
    let mut parts: Vec<String> = Vec::new();
    if counts.0 > 0 {
        parts.push(format!("+{}", counts.0));
    }
    if counts.1 > 0 {
        parts.push(format!("●{}", counts.1));
    }
    if counts.2 > 0 {
        parts.push(format!("−{}", counts.2));
    }
    parts.join(" ")
}

/// The cluster's Apply label: `Apply 3`.
pub fn apply_label(n: usize) -> String {
    format!("Apply {n}")
}

/// The drawer footer's Apply label: `Apply 1 change`, `Apply 3 changes`.
pub fn apply_changes_label(n: usize) -> String {
    let noun = crate::table::plural(n as u64, "change", "changes");
    format!("Apply {n} {noun}")
}

/// `.fchips`' right-hand fact: `1,204 matches`.
pub fn matches_label(n: u64) -> String {
    let noun = crate::table::plural(n, "match", "matches");
    format!("{} {noun}", crate::table::grouped(n))
}

/// `.pend`: the design's warn-soft cluster pinned to the toolbar's right edge and as tall as the
/// band it sits in — the counts by kind, Revert and `Apply n`. Returns `None` for an empty buffer:
/// with nothing staged there is no cluster, which is also what the design draws for its own
/// `0 pending` states.
pub fn pending_cluster(
    ui: &mut Ui,
    tokens: &Tokens,
    counts: (usize, usize, usize),
) -> Option<PendingAction> {
    let total = pending_total(counts);
    if total == 0 {
        return None;
    }
    let sentence = pending_sentence(counts);
    let apply = apply_label(total);
    let counts_text = pending_counts_text(counts);
    // The two buttons are 24px boxes with the style's 10px of padding each side and a hairline; the
    // counts and the sentence are one line each, 8px apart, inside S5 of padding and a 1px edge.
    let button = |label: &str| measured(ui, tokens.font(T_SM), label) + 2.0 * 10.0 + 2.0;
    let pieces: [f32; 4] = [
        measured(ui, tokens.font(T_SM), &sentence),
        measured(ui, tokens.mono(T_XS), &counts_text),
        button("Revert"),
        button(&apply),
    ];
    let width = pieces.iter().sum::<f32>()
        + f32::from(S4) * pieces.len() as f32
        + 2.0 * f32::from(S5)
        + f32::from(S2);

    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, ui.available_height()), Sense::hover());
    {
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::ZERO, tokens.warn_fill);
        painter.vline(
            rect.left() + 0.5,
            rect.top()..=rect.bottom(),
            Stroke::new(1.0, tokens.stroke),
        );
    }
    let inner = egui::Rect::from_min_max(
        egui::pos2(rect.left() + f32::from(S5), rect.top()),
        egui::pos2(rect.right() - f32::from(S5), rect.bottom()),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing.x = f32::from(S4);
    let mut action = None;
    if primary_button(&mut child, tokens, &apply, true)
        .on_hover_text("Run the pending statements in one transaction")
        .clicked()
    {
        action = Some(PendingAction::Apply);
    }
    if ghost_button(&mut child, tokens, "Revert", true)
        .on_hover_text("Throw the pending statements away")
        .clicked()
    {
        action = Some(PendingAction::Revert);
    }
    if counts.2 > 0 {
        child.label(
            RichText::new(format!("−{}", counts.2))
                .size(T_XS)
                .monospace()
                .color(tokens.error),
        );
    }
    if counts.1 > 0 {
        child.label(
            RichText::new(format!("●{}", counts.1))
                .size(T_XS)
                .monospace()
                .color(tokens.warn),
        );
    }
    if counts.0 > 0 {
        child.label(
            RichText::new(format!("+{}", counts.0))
                .size(T_XS)
                .monospace()
                .color(tokens.ok),
        );
    }
    child.label(RichText::new(&sentence).size(T_SM).color(tokens.warn));
    action
}

/// Filled rectangle with a hairline border.
pub fn outlined(ui: &Ui, rect: egui::Rect, tokens: &Tokens, radius: CornerRadius, fill: Color32) {
    ui.painter().rect_filled(rect, radius, fill);
    ui.painter().rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, tokens.stroke),
        StrokeKind::Inside,
    );
}

// ---------------------------------------------------------------------------------------------
// The dialog layer: `.dlg` and the bands inside it — the head, the step strip, the body's fields and
// the pinned footer. Each helper paints from the tokens alone, so a caller states the design's
// structure and none of its values. The bands butt against each other, so the caller's own stack spends
// no item spacing between them (`item_spacing.y = 0` on the dialog's `Ui`); the body's `--s5` gap is
// the body's stack, which is the caller's too.
//
// ponytail: the sheet sets `.dlg-t`, `.tab.on` and its chips to 600 weight and the panes' `.btn` has no
// weight axis to set — egui's bundled families are single-weight (see `chip`), so those read by colour,
// the accent run and shape alone. Bundle a bold family if the weight is ever wanted.
// ---------------------------------------------------------------------------------------------

/// `.dlg`: the design's window — `surface`, a 1px `stroke_strong` border, `--r-box`, no shadow. The
/// bands inside carry their own padding (`.dlg-h` `16 20 12`, `.dlg-b` `16 20`, `.dlg-f` `12 20`), so
/// the frame adds none: a margin here would make the dialog wider than its 560.
pub fn dialog_frame(tokens: &Tokens) -> Frame {
    Frame::new()
        .fill(tokens.panel)
        .stroke(Stroke::new(1.0, tokens.stroke_strong))
        .corner_radius(CornerRadius::same(R_BOX))
        .inner_margin(Margin::ZERO)
        .outer_margin(Margin::ZERO)
}

/// `.dlg-h`: the 68px head — `--s6` above the title, the `T_LG` title, the `T_SM` `text_dim` subtitle
/// `--s1` under it, `--s5` below — with the 24px `×` against the inner right edge. `mark` is the
/// confirmation's 20px `.ig` badge in front of the title (the sheet's only `.ig` is `.ig.err`, so it
/// wears `error`), and `mono` prints the subtitle in the mono family for a `.dlg-sub.mono` line.
/// Returns the `×`'s response, so the caller closes when it is clicked.
pub fn dialogue_head(
    ui: &mut Ui,
    tokens: &Tokens,
    mark: Option<&str>,
    title: &str,
    subtitle: &str,
    mono: bool,
) -> Response {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), DLG_HEAD_H), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, tokens.panel);
    let inner = Rect::from_min_max(
        egui::pos2(rect.left() + f32::from(S7), rect.top() + f32::from(S6)),
        egui::pos2(rect.right() - f32::from(S7), rect.bottom() - f32::from(S5)),
    );
    let close = ui.interact(
        Rect::from_min_size(
            egui::pos2(inner.right() - TAB_CLOSE, inner.top()),
            Vec2::splat(TAB_CLOSE),
        ),
        ui.id().with("dlg-close"),
        Sense::click(),
    );

    let mut left = inner.left();
    if let Some(mark) = mark {
        let badge = Rect::from_min_size(
            egui::pos2(left, inner.top() + f32::from(S1)),
            Vec2::splat(f32::from(S7)),
        );
        let radius = CornerRadius::same(R_CTL);
        let painter = ui.painter();
        painter.rect_stroke(
            badge,
            radius,
            Stroke::new(1.0, tokens.error),
            StrokeKind::Inside,
        );
        painter.text(
            badge.center(),
            egui::Align2::CENTER_CENTER,
            mark,
            tokens.mono(T_XS),
            tokens.error,
        );
        left = badge.right() + f32::from(S5);
    }

    let painter = ui.painter();
    let title_galley = painter.layout_no_wrap(title.to_owned(), tokens.font(T_LG), tokens.text);
    let subtitle_top = inner.top() + title_galley.size().y + f32::from(S1);
    painter.galley(egui::pos2(left, inner.top()), title_galley, tokens.text);
    painter.text(
        egui::pos2(left, subtitle_top),
        egui::Align2::LEFT_TOP,
        subtitle,
        if mono {
            tokens.mono(T_SM)
        } else {
            tokens.font(T_SM)
        },
        tokens.text_dim,
    );
    if close.hovered() {
        ui.painter()
            .rect_filled(close.rect, CornerRadius::same(R_CTL), tokens.hover);
    }
    ui.painter().text(
        close.rect.center(),
        egui::Align2::CENTER_CENTER,
        "×",
        tokens.font(T_LG),
        tokens.text_dim,
    );
    close
}

/// `.tabs`: the step strip — `--s7` of lead-in, `T_MD` items `--s6` apart, the active one in `text`
/// with the design's 2px `accent` run along its own bottom edge — and the 1px `stroke` hairline the band
/// draws at its bottom edge, which is why the band is 33 and the hairline lands 100 below the head's
/// top. Returns the item that was clicked.
pub fn dialogue_tabs(
    ui: &mut Ui,
    tokens: &Tokens,
    labels: &[&str],
    active: usize,
) -> Option<usize> {
    let font = tokens.font(T_MD);
    let widths: Vec<f32> = labels
        .iter()
        .map(|label| measured(ui, font.clone(), label))
        .collect();
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), DLG_TABS_H), Sense::hover());
    band_hairline(ui, tokens, rect);
    let inset = f32::from(S7);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(
                egui::pos2(rect.left() + inset, rect.top()),
                egui::pos2(rect.right() - inset, rect.top() + DLG_TAB_H),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Min)),
    );
    child.spacing_mut().item_spacing.x = f32::from(S6);
    let mut clicked = None;
    for (index, (label, width)) in labels.iter().zip(&widths).enumerate() {
        let (item, response) =
            child.allocate_exact_size(Vec2::new(*width, DLG_TAB_H), Sense::click());
        let painter = child.painter();
        if index == active {
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(item.left(), item.bottom() - INDICATOR_W),
                    item.max,
                ),
                CornerRadius::ZERO,
                tokens.accent,
            );
        }
        painter.text(
            item.center(),
            egui::Align2::CENTER_CENTER,
            *label,
            font.clone(),
            if index == active {
                tokens.text
            } else {
                tokens.text_dim
            },
        );
        if response.clicked() {
            clicked = Some(index);
        }
    }
    clicked
}

/// The dialog's `.seg`: a 28px `panel_alt` container inside a 1px `stroke` border, holding 26px mono
/// items `--s5` of padding wide, split by the same hairline, the active item on `accent` with
/// `accent_text` ink. The pane's [`pane_segmented`] is the same idea with the pane's own inset and ring;
/// the top bar's [`segmented`] marks its selection with the accent indicator instead. Returns the item
/// that was clicked.
pub fn dialogue_segmented(ui: &mut Ui, tokens: &Tokens, options: &[(&str, bool)]) -> Option<usize> {
    if options.is_empty() {
        return None;
    }
    let font = tokens.mono(T_SM);
    // `.seg .s` is `padding: 0 --s5` with a 1px `border-right` under `box-sizing: border-box`, so
    // every item but the last is a pixel wider than its own padding — that pixel is the divider.
    let last = options.len() - 1;
    let widths: Vec<f32> = options
        .iter()
        .enumerate()
        .map(|(index, (label, _))| {
            measured(ui, font.clone(), label)
                + 2.0 * f32::from(S5)
                + if index == last { 0.0 } else { 1.0 }
        })
        .collect();
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(widths.iter().sum::<f32>() + 2.0, DLG_SEG_H),
        Sense::hover(),
    );
    let radius = CornerRadius::same(R_CTL);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, tokens.panel_alt);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, tokens.stroke),
        StrokeKind::Inside,
    );

    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink(1.0))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 0.0;
    let mut clicked = None;
    for (index, ((label, selected), width)) in options.iter().zip(&widths).enumerate() {
        let (item, response) =
            child.allocate_exact_size(Vec2::new(*width, DLG_SEG_ITEM_H), Sense::click());
        // The container's `overflow: hidden`: the first and last fills take its rounding.
        let fill_radius = match (index, options.len()) {
            (0, 1) => radius,
            (0, _) => CornerRadius {
                nw: R_CTL,
                sw: R_CTL,
                ..CornerRadius::ZERO
            },
            (last, len) if last == len - 1 => CornerRadius {
                ne: R_CTL,
                se: R_CTL,
                ..CornerRadius::ZERO
            },
            _ => CornerRadius::ZERO,
        };
        let painter = child.painter();
        if *selected {
            painter.rect_filled(item, fill_radius, tokens.accent);
        }
        if index > 0 {
            // The design's `border-right` on the item before this one: the shared edge, drawn inside
            // the previous item's extra pixel rather than over this one's padding.
            painter.vline(
                item.left() - 1.0,
                item.top()..=item.bottom(),
                Stroke::new(1.0, tokens.stroke),
            );
        }
        painter.text(
            item.center(),
            egui::Align2::CENTER_CENTER,
            *label,
            font.clone(),
            if *selected {
                tokens.accent_text
            } else {
                tokens.text_dim
            },
        );
        if response.clicked() {
            clicked = Some(index);
        }
    }
    clicked
}

/// `.flab`: the field's label line. Its box is the design's `--s6`, which is what makes a `.fld` 50
/// tall — the label, the `--s3` gap the caller's stack spends and a 28px [`input`] — so a field built
/// from these two helpers is the height the measurement table names.
pub fn field_label(ui: &mut Ui, tokens: &Tokens, text: &str) -> Response {
    ui.add_sized(
        Vec2::new(ui.available_width(), f32::from(S6)),
        egui::Label::new(RichText::new(text).size(T_XS).color(tokens.text_dim)),
    )
}

/// `.inp`: the 28px field box — `panel_alt`, a 1px `stroke` border, `--r-ctl`, `--s4` of padding — with
/// the design's focus and error treatment: `error` swaps the border to `error`, and the field holding
/// egui's focus swaps it to `accent` and adds the 2px `accent` ring 1px outside. `add` paints the
/// field's own widget and returns its response, so the box knows whether it has the focus; that widget
/// must paint no chrome of its own (`Frame::NONE` and `Margin::ZERO` on a `TextEdit`, so the box's
/// `--s4` is the whole padding) and carries its own ink — the placeholder is `text_faint`, which egui's
/// default hint colour is not.
pub fn input(
    ui: &mut Ui,
    tokens: &Tokens,
    width: f32,
    error: bool,
    enabled: bool,
    add: impl FnOnce(&mut Ui) -> Response,
) -> Response {
    input_sized(ui, tokens, width, INPUT_H, error, enabled, add)
}

/// The same box at a given height, for a field that holds more than one line — the AI band's prompt is
/// three rows at the design's 74px, where every other field is the 28px [`INPUT_H`]. Its content is
/// laid out top-down so a multi-line `TextEdit` starts at the box's first line rather than being
/// centred in it; a one-line field is centred, which at 28px is the same place.
pub fn input_tall(
    ui: &mut Ui,
    tokens: &Tokens,
    width: f32,
    height: f32,
    error: bool,
    enabled: bool,
    add: impl FnOnce(&mut Ui) -> Response,
) -> Response {
    input_sized(ui, tokens, width, height, error, enabled, add)
}

/// The one box both entry points draw, so a change to the border or the focus ring cannot land in one
/// and miss the other. A box taller than the one-line field aligns its content to the top, so a
/// multi-line `TextEdit` starts at the box's first line instead of being centred in it.
fn input_sized(
    ui: &mut Ui,
    tokens: &Tokens,
    width: f32,
    height: f32,
    error: bool,
    enabled: bool,
    add: impl FnOnce(&mut Ui) -> Response,
) -> Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let radius = CornerRadius::same(R_CTL);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, tokens.panel_alt);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, if error { tokens.error } else { tokens.stroke }),
        StrokeKind::Inside,
    );

    let align = if height > INPUT_H {
        egui::Align::Min
    } else {
        egui::Align::Center
    };
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(f32::from(S4), 0.0)))
            .layout(egui::Layout::left_to_right(align)),
    );
    let response = child.add_enabled_ui(enabled, |ui| add(ui)).inner;
    if response.has_focus() {
        // Only the border and the ring: the box's own fill is already under the text.
        let painter = ui.painter();
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, tokens.accent),
            StrokeKind::Inside,
        );
        painter.rect_stroke(
            rect.expand(1.0),
            CornerRadius::same(R_CTL + 1),
            Stroke::new(2.0, tokens.accent),
            StrokeKind::Middle,
        );
    }
    response
}

/// The design's `.chip` at a given height: the dialog's `.facts` chips are 20 ([`DLG_CHIP_H`]) and the
/// keychain chip inside the password field is 18 ([`KEYCHIP_H`]), where [`chip`] is fixed at the row's
/// 24. `panel_alt`, a 1px `stroke`, `--r-chip`, one `T_XS` mono line in `text_dim`.
pub fn tag_of(ui: &mut Ui, tokens: &Tokens, text: &str, height: f32) -> Response {
    let font = tokens.mono(T_XS);
    let width = measured(ui, font.clone(), text) + 2.0 * f32::from(S4);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let radius = CornerRadius::same(R_CHIP);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, tokens.panel_alt);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, tokens.stroke),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font,
        tokens.text_dim,
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), text));
    response
}

/// `.chk` + `.cb`: the 14px box `--s4` before its `T_MD` label, `accent` filled with an `accent_text`
/// ✓ when it is on and `panel_alt` inside a `stroke_strong` border when it is off. A disabled box paints
/// its reason's own "not now" ink and takes no pointer, which is the treatment the design gives a
/// control that cannot act.
pub fn checkbox(
    ui: &mut Ui,
    tokens: &Tokens,
    label: &str,
    checked: &mut bool,
    enabled: bool,
) -> Response {
    let font = tokens.font(T_MD);
    let label_width = measured(ui, font.clone(), label);
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(
            CHECKBOX + f32::from(S4) + label_width,
            CHECKBOX.max(T_MD * DLG_LINE_H),
        ),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if response.clicked() {
        *checked = !*checked;
    }
    let box_rect = Rect::from_center_size(
        egui::pos2(rect.left() + CHECKBOX / 2.0, rect.center().y),
        Vec2::splat(CHECKBOX),
    );
    let radius = CornerRadius::same(CB_RADIUS);
    let painter = ui.painter();
    painter.rect_filled(
        box_rect,
        radius,
        if *checked {
            tokens.accent
        } else {
            tokens.panel_alt
        },
    );
    painter.rect_stroke(
        box_rect,
        radius,
        Stroke::new(
            1.0,
            if *checked {
                tokens.accent
            } else {
                tokens.stroke_strong
            },
        ),
        StrokeKind::Inside,
    );
    if *checked {
        painter.text(
            box_rect.center(),
            egui::Align2::CENTER_CENTER,
            "✓",
            tokens.mono(MARK_GLYPH),
            tokens.accent_text,
        );
    }
    painter.text(
        egui::pos2(rect.left() + CHECKBOX + f32::from(S4), rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        if enabled {
            tokens.text
        } else {
            tokens.text_dim
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, *checked, label)
    });
    response
}
/// `.helper` (and `.why`, which the sheet prints the same way): one `T_XS` `text_dim` line at the
/// design's 1.45 line height. The design pulls `.helper` `--s2` up — it belongs to the field above it,
/// not to the body's `--s5` flow gap — so the helper spends that here, and a caller stacking the body's
/// 12px items needs no case of its own.
pub fn helper_line(ui: &mut Ui, tokens: &Tokens, text: &str) -> Response {
    ui.add_space(-f32::from(S2));
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .size(T_XS)
                .color(tokens.text_dim)
                .line_height(Some(T_XS * DLG_LINE_H)),
        )
        .wrap(),
    )
}

/// `.dlg-f`: the pinned footer — a `stroke` hairline along its top edge, `--s5` of vertical padding
/// around a column of 28px rows, and `--s7` on each side. The editor's own footer overrides the
/// modal's `gap: --s5` with `gap: --s4` and holds two rows, which is [`DLG_FOOT_H`] (89); the
/// confirmation's single row is 53, `--s4 + BUTTON_H` shorter. Returns the band's rect.
pub fn dialogue_foot(ui: &mut Ui, tokens: &Tokens, height: f32, add: impl FnOnce(&mut Ui)) -> Rect {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, tokens.panel);
    band_top_hairline(ui, tokens, rect);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(f32::from(S7), f32::from(S5))))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.spacing_mut().item_spacing.y = f32::from(S4);
    add(&mut child);
    rect
}

/// One `.btn` of the dialog's footer or its test row: [`BUTTON_H`] tall, `--r-ctl`, `--s5` of padding
/// either side of a `T_MD` label, and one of the faces [`Emphasis`] names, from [`button_face`] on the
/// dialog's `panel_alt`. A disabled button wears `.btn.off` whatever its emphasis, and the focused one
/// adds the design's 2px ring 1px outside — `error` for a danger button, `accent` otherwise.
pub fn dialogue_button(
    ui: &mut Ui,
    tokens: &Tokens,
    label: &str,
    emphasis: Emphasis,
    enabled: bool,
) -> Response {
    let face = button_face(
        tokens,
        tokens.panel_alt,
        if enabled { emphasis } else { Emphasis::Off },
        true,
    );
    let font = tokens.font(T_MD);
    let width = measured(ui, font.clone(), label) + 2.0 * f32::from(S5) + 2.0;
    let radius = CornerRadius::same(R_CTL);
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(width, BUTTON_H),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let painter = ui.painter();
    if face.fill != Color32::TRANSPARENT {
        painter.rect_filled(rect, radius, face.fill);
    }
    if let Some(hover) = face.hover {
        if response.hovered() {
            painter.rect_filled(rect, radius, hover);
        }
    }
    if face.border != Color32::TRANSPARENT {
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, face.border),
            StrokeKind::Inside,
        );
    }
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        font,
        face.ink,
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(1.0),
            CornerRadius::same(R_CTL + 1),
            Stroke::new(
                2.0,
                if emphasis == Emphasis::Danger {
                    tokens.error
                } else {
                    tokens.accent
                },
            ),
            StrokeKind::Middle,
        );
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    response
}

/// The design's `.dot` at a size the caller states: the test row's is [`STATUS_DOT`] where the panes'
/// [`dot`] is 9, and its ink is the row's own state (`ok`, `error`, `warn`).
pub fn status_dot(ui: &mut Ui, colour: Color32, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), size / 2.0, colour);
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pane_head_adds_up_to_the_same_line() {
        // The tree, the grid and the inspector start their bodies on one line only while each head
        // adds up to the design's `--h-panehead`. The tree is 34 + its filter block; the inspector is
        // its object header, its tab strip and the list head.
        assert_eq!(
            crate::tree::HEADER_H + crate::tree::FILTER_BLOCK_H,
            PANE_HEAD_H
        );
        assert_eq!(
            crate::inspector::HEADER_H + TOOLBAR_H + HEAD_H,
            PANE_HEAD_H,
            "the inspector's bands no longer add up to the pane head"
        );
    }

    #[test]
    fn the_filter_row_centres_its_chip_in_the_designs_thirty_two() {
        assert_eq!(FCHIPS_H, 32.0, "the design's `.fchips` is 32px tall");
        assert_eq!(filter_row_h(FCHIP_H), FCHIPS_H);
        assert_eq!(FCHIP_AIR, (FCHIPS_H - FCHIP_H) / 2.0);
        assert_eq!(STALE_STRIP_H, 26.0);
        assert_eq!(notice_band_h(), 54.0, "10 + title + 2 + message + 10");
        assert_eq!(MICRO_TAG_H, 16.0);
    }

    #[test]
    fn the_pending_cluster_counts_each_kind_and_drops_the_empty_ones() {
        assert_eq!(pending_total((3, 0, 4)), 7);
        assert_eq!(pending_sentence((1, 0, 0)), "1 pending");
        assert_eq!(pending_sentence((0, 2, 1)), "3 pending");
        assert_eq!(pending_counts_text((1, 1, 1)), "+1 ●1 −1");
        assert_eq!(pending_counts_text((0, 2, 0)), "●2");
        assert_eq!(pending_counts_text((0, 0, 0)), "");
    }

    #[test]
    fn an_apply_or_match_label_pluralises_or_groups_its_count() {
        assert_eq!(apply_label(1), "Apply 1");
        assert_eq!(apply_changes_label(1), "Apply 1 change");
        assert_eq!(apply_changes_label(3), "Apply 3 changes");
        assert_eq!(matches_label(1), "1 match");
        assert_eq!(matches_label(0), "0 matches");
        assert_eq!(matches_label(12_481), "12,481 matches");
    }

    #[test]
    fn a_pane_segmented_control_shares_its_inner_width_and_refuses_the_impossible() {
        let (width, height) =
            segment_item(Vec2::new(124.0, 28.0), 1.0, 4).expect("four items fit in a 124px strip");
        assert_eq!(width, (124.0 - 4.0 - 3.0) / 4.0);
        assert_eq!(height, 24.0);
        // 28 minus the 1px border and the 1px inset on each side.
        assert!(segment_item(Vec2::new(6.0, 28.0), 2.0, 4).is_none());
        assert!(segment_item(Vec2::new(124.0, 2.0), 1.0, 4).is_none());
        assert!(segment_item(Vec2::new(124.0, 28.0), 1.0, 0).is_none());
    }

    /// WCAG 2.1 relative luminance of an sRGB colour.
    fn luminance(colour: Color32) -> f32 {
        let channel = |value: u8| {
            let value = f32::from(value) / 255.0;
            if value <= 0.039_28 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(colour.r()) + 0.7152 * channel(colour.g()) + 0.0722 * channel(colour.b())
    }

    /// WCAG contrast ratio between two opaque colours.
    fn contrast(one: Color32, other: Color32) -> f32 {
        let (one, other) = (luminance(one), luminance(other));
        let (lighter, darker) = if one > other {
            (one, other)
        } else {
            (other, one)
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    /// A headless frame the way `table.rs` drives one: no texture uploads, the query pane's own rect.
    fn query_frame() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::pos2(344.0, 75.0),
                egui::vec2(776.0, 900.0),
            )),
            ..Default::default()
        }
    }

    /// The toolbar's `.btn` wears one of the design's three faces, and `off` beats emphasis: a
    /// disabled `.btn.primary` is the plain disabled button.
    #[test]
    fn a_toolbar_button_wears_one_of_the_designs_three_faces() {
        for tokens in [Tokens::of("dark"), Tokens::of("light")] {
            let theme = if tokens.dark { "dark" } else { "light" };
            let sheet = tokens.panel_high;

            let plain = button_face(&tokens, sheet, Emphasis::Plain, true);
            assert_eq!(plain.fill, sheet, "a plain button sits on its sheet's fill");
            assert_eq!(plain.border, tokens.stroke);
            assert_eq!(plain.ink, tokens.text);
            assert_eq!(plain.hover, Some(tokens.hover));

            let primary = button_face(&tokens, sheet, Emphasis::Primary, true);
            assert_eq!(primary.fill, tokens.accent);
            assert_eq!(primary.border, tokens.accent_line);
            assert_eq!(primary.ink, tokens.accent_text);
            assert_eq!(
                primary.hover, None,
                "the design's primary has no hover fill"
            );

            for emphasis in [Emphasis::Plain, Emphasis::Primary] {
                let off = button_face(&tokens, sheet, emphasis, false);
                assert_eq!(
                    off.fill,
                    Color32::TRANSPARENT,
                    "`.btn.off` paints no fill of its own"
                );
                assert_eq!(off.ink, tokens.text_faint);
                assert_eq!(off.hover, None, "a disabled button ignores the pointer");
                // A transparent fill means the band's own fill shows through, and those differ: the
                // editor's `.tb` is `panel_alt` inside the pane, the register's `.rbar` is `panel`.
                for under in [tokens.panel_alt, tokens.panel, sheet] {
                    let ratio = contrast(off.ink, under);
                    assert!(
                        ratio >= 2.3,
                        "{theme}: the disabled ink is {ratio:.2}:1 on its band"
                    );
                }
            }
        }
    }

    /// The `.btn` reserves room for its mark before it measures itself, and the editor's two marks are
    /// the design's fills rather than glyphs.
    #[test]
    fn a_toolbar_button_reserves_room_for_the_mark_it_carries() {
        assert_eq!(Mark::Play.size(), Vec2::new(7.0, 10.0));
        assert_eq!(Mark::Stop.size(), Vec2::splat(8.0));
        assert_eq!(
            Mark::Glyph(crate::icons::Glyph::Warning).size(),
            Vec2::splat(GLYPH_BOX)
        );

        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let mut out = ctx.run_ui(query_frame(), |ui| {
            let bare = toolbar_button_width(ui, &tokens, "Run", None);
            let played = toolbar_button_width(ui, &tokens, "Run", Some(Mark::Play));
            assert_eq!(played - bare, Mark::Play.size().x + f32::from(S3));
            // The label, a padding and a 1px border each side, and nothing else.
            assert_eq!(
                bare,
                measured(ui, tokens.font(T_SM), "Run") + 2.0 * f32::from(S4) + 2.0
            );

            // Whatever the caller appends starts a gap after the label and the button grows to fit
            // it: the design's `Export` carries its caret inside the button.
            let width = toolbar_button_width(ui, &tokens, "Export", None);
            let mut placed = Rect::NOTHING;
            let button = toolbar_button(
                ui,
                &tokens,
                "Export",
                None,
                tokens.panel_high,
                Emphasis::Plain,
                true,
                |ui| {
                    placed = ui
                        .allocate_exact_size(Vec2::new(10.0, 10.0), Sense::hover())
                        .0;
                },
            );
            assert_eq!(
                placed.left(),
                button.rect.left() + width - f32::from(S4) - 1.0 + f32::from(S3),
                "the trailing content starts one padding and border short of the right edge"
            );
            assert_eq!(button.rect.width(), width + f32::from(S3) + 10.0);
            assert_eq!(button.rect.height(), TOOLBAR_BTN_H);
        });
        out.textures_delta.clear();
    }

    /// The chrome reaches the frame even though the caller's content is painted first: the fill, the
    /// pointer's fill and the border go into slots reserved before it.
    #[test]
    fn a_toolbar_button_paints_the_face_it_chose() {
        let tokens = Tokens::of("dark");
        let painted = |emphasis: Emphasis, enabled: bool| -> Vec<egui::Shape> {
            let ctx = egui::Context::default();
            tokens.apply(&ctx);
            let mut out = ctx.run_ui(query_frame(), |ui| {
                let _ = toolbar_button(
                    ui,
                    &tokens,
                    "Run",
                    Some(Mark::Play),
                    tokens.panel_high,
                    emphasis,
                    enabled,
                    |ui| {
                        ui.allocate_exact_size(Vec2::new(10.0, 10.0), Sense::hover());
                    },
                );
            });
            out.textures_delta.clear();
            out.shapes
                .into_iter()
                .map(|clipped| clipped.shape)
                .collect()
        };
        let filled = |shapes: &[egui::Shape], colour: Color32| {
            shapes
                .iter()
                .any(|shape| matches!(shape, egui::Shape::Rect(rect) if rect.fill == colour))
        };
        let outlined = |shapes: &[egui::Shape], colour: Color32| {
            shapes.iter().any(
                |shape| matches!(shape, egui::Shape::Rect(rect) if rect.stroke.color == colour),
            )
        };

        let primary = painted(Emphasis::Primary, true);
        assert!(
            filled(&primary, tokens.accent),
            "the primary's fill has to reach the frame"
        );
        assert!(outlined(&primary, tokens.accent_line));

        let off = painted(Emphasis::Plain, false);
        assert!(
            !filled(&off, tokens.panel_high),
            "a disabled button paints no sheet fill"
        );
        assert!(!filled(&off, tokens.accent));
        assert!(outlined(&off, tokens.stroke));
    }

    /// `.note` is 20 tall and its 5px dot sits inside it, one padding in from the left edge.
    #[test]
    fn a_note_pill_centres_its_dot_inside_its_twenty() {
        assert_eq!(NOTE_H, 20.0);
        let pill = Rect::from_min_size(egui::pos2(344.0, 507.0), Vec2::new(214.0, NOTE_H));
        let centre = note_dot_centre(pill);
        assert_eq!(centre.y, pill.center().y);
        assert_eq!(centre.x, 344.0 + f32::from(S4) + 1.0 + NOTE_DOT / 2.0);
        assert!(
            centre.x - NOTE_DOT / 2.0 >= pill.left() && centre.x + NOTE_DOT / 2.0 <= pill.right(),
            "the dot's box has to sit inside the pill"
        );
        const {
            assert!(
                NOTE_DOT < NOTE_H,
                "the dot has to sit inside the pill's height"
            )
        };
    }

    /// Both error bands: their parts add up to the height the register's layout spends on them, and
    /// every inner slot fits inside that height.
    #[test]
    fn both_error_bands_add_up_to_their_constants() {
        assert_eq!(STMT_BAND_H, 56.0, "the design's `.band` is 56");
        // 56 = a 16px title, 2px of air, a 16px mono sub-line, and what that leaves above and below.
        assert_eq!(
            2.0 * STMT_BAND_PAD_V + STMT_TITLE_H + f32::from(S1) + STMT_SUB_H,
            STMT_BAND_H
        );
        const { assert!(STMT_BAND_PAD_V > 0.0) };
        // The `.ico` box and its 2px drop have to fit beside the text column.
        assert!(STMT_MARK_BOX + f32::from(S1) <= STMT_TITLE_H + f32::from(S1) + STMT_SUB_H);

        assert_eq!(REG_ERR_PAD, S5, "the design pads `.eband` with `--s5`");
        assert_eq!(
            register_error_band_h(),
            86.0,
            "12 + 18 + 6 + 16 + 6 + 16 + 12"
        );
        assert_eq!(
            register_error_band_h(),
            2.0 * f32::from(REG_ERR_PAD)
                + REG_ERR_TAG_H
                + f32::from(S3)
                + REG_ERR_LINE_H
                + f32::from(S3)
                + REG_ERR_LINE_H
        );
        // The body's own height: the tag row, the message and the statement, `--s3` apart.
        let body = register_error_band_h() - 2.0 * f32::from(REG_ERR_PAD);
        assert!(REG_ERR_TAG_H + f32::from(S3) + 2.0 * REG_ERR_LINE_H <= body);
        // The design's 14px mark, dropped 2px, has to fit in the same body.
        assert!(REG_ERR_MARK + f32::from(S1) <= body);
    }

    /// The four widgets allocate exactly the heights their constants claim.
    #[test]
    fn the_query_workspaces_bands_are_the_heights_their_constants_claim() {
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let mut heights = [0.0_f32; 4];
        let mut out = ctx.run_ui(query_frame(), |ui| {
            heights[0] = statement_band(
                ui,
                &tokens,
                "near \"FROMM\": syntax error",
                "SQLITE_ERROR · Ln 3, Col 1 · statement 1 · nothing was executed",
                |_| {},
            )
            .height();
            heights[1] = register_error_band(
                ui,
                &tokens,
                "SQLITE_ERROR",
                "no such column: statuss",
                "SELECT id, statuss FROM main.customers;",
                |_| {},
            )
            .height();
            heights[2] = note_pill(
                ui,
                &tokens,
                "statement 2 of 2 · no result set",
                Some(tokens.info),
            )
            .rect
            .height();
            heights[3] = toolbar_button(
                ui,
                &tokens,
                "Run",
                Some(Mark::Play),
                tokens.panel_high,
                Emphasis::Primary,
                true,
                |_| {},
            )
            .rect
            .height();
        });
        out.textures_delta.clear();
        assert_eq!(heights[0], STMT_BAND_H);
        assert_eq!(heights[1], register_error_band_h());
        assert_eq!(heights[2], NOTE_H);
        assert_eq!(heights[3], TOOLBAR_BTN_H);
    }

    /// Every foreground/background pair the interface paints, in both themes, against the floors this
    /// application holds itself to: 4.5:1 for text and for any semantic colour used as text (the
    /// design's contract, including the dim and faint ranks on each surface they sit on); 3.0:1 for
    /// the accent indicator over its soft fill; 1.3:1 for a hairline, because the design's own dark
    /// border measures 1.33:1 on its panel, and below that a border stops reading as a boundary. The
    /// design's named worst cases are the pairs below: dim and faint on `panel_high`, the accent on
    /// `accent_dim`, `accent_text` on `accent`, and each semantic colour on its soft fill.
    #[test]
    fn palettes_clear_their_contrast_floors() {
        for tokens in [Tokens::of("dark"), Tokens::of("light")] {
            let theme = if tokens.dark { "dark" } else { "light" };
            let floors: [(&str, Color32, Color32, f32); 30] = [
                ("text on panel", tokens.text, tokens.panel, 4.5),
                ("text on background", tokens.text, tokens.background, 4.5),
                ("text on panel_alt", tokens.text, tokens.panel_alt, 4.5),
                ("text on panel_high", tokens.text, tokens.panel_high, 4.5),
                ("text on selection", tokens.text, tokens.selection, 4.5),
                (
                    "text on selection_strong",
                    tokens.text,
                    tokens.selection_strong,
                    4.5,
                ),
                ("text on accent_dim", tokens.text, tokens.accent_dim, 4.5),
                (
                    "text on grid_row_alt",
                    tokens.text,
                    tokens.grid_row_alt,
                    4.5,
                ),
                ("text on grid_header", tokens.text, tokens.grid_header, 4.5),
                ("dim on panel", tokens.text_dim, tokens.panel, 4.5),
                ("dim on background", tokens.text_dim, tokens.background, 4.5),
                ("dim on panel_alt", tokens.text_dim, tokens.panel_alt, 4.5),
                ("dim on panel_high", tokens.text_dim, tokens.panel_high, 4.5),
                (
                    "dim on grid_header",
                    tokens.text_dim,
                    tokens.grid_header,
                    4.5,
                ),
                ("faint on panel", tokens.text_faint, tokens.panel, 4.5),
                (
                    "faint on panel_alt",
                    tokens.text_faint,
                    tokens.panel_alt,
                    4.5,
                ),
                (
                    "faint on panel_high",
                    tokens.text_faint,
                    tokens.panel_high,
                    4.5,
                ),
                ("accent on panel", tokens.accent, tokens.panel, 4.5),
                ("accent on panel_alt", tokens.accent, tokens.panel_alt, 4.5),
                (
                    "accent_text on accent",
                    tokens.accent_text,
                    tokens.accent,
                    4.5,
                ),
                ("ok on panel", tokens.ok, tokens.panel, 4.5),
                ("warn on panel", tokens.warn, tokens.panel, 4.5),
                ("error on panel", tokens.error, tokens.panel, 4.5),
                ("info on panel", tokens.info, tokens.panel, 4.5),
                ("ok on ok_fill", tokens.ok, tokens.ok_fill, 4.5),
                ("warn on warn_fill", tokens.warn, tokens.warn_fill, 4.5),
                ("error on error_fill", tokens.error, tokens.error_fill, 4.5),
                ("dirty_text on dirty", tokens.dirty_text, tokens.dirty, 4.5),
                (
                    "indicator on accent_dim",
                    tokens.accent,
                    tokens.accent_dim,
                    3.0,
                ),
                ("hairline on panel", tokens.stroke, tokens.panel, 1.3),
            ];
            for (pair, foreground, background, floor) in floors {
                let ratio = contrast(foreground, background);
                assert!(
                    ratio >= floor,
                    "{theme}: {pair} is {ratio:.2}:1, needs at least {floor}:1"
                );
            }
        }
    }

    /// The scrim has to dim what it covers, in both themes. `Color32` stores premultiplied bytes, so
    /// the check composites those bytes over the panel using the token's alpha instead of treating a
    /// translucent colour as opaque — the opaque comparison passes for any dark value, including one
    /// too faint to dim anything.
    #[test]
    fn scrim_dims_the_surface_below_it() {
        for tokens in [Tokens::of("dark"), Tokens::of("light")] {
            let theme = if tokens.dark { "dark" } else { "light" };
            let alpha = f32::from(tokens.scrim.a()) / 255.0;
            let over = |over: u8, under: u8| {
                (f32::from(over) + f32::from(under) * (1.0 - alpha)).round() as u8
            };
            let dimmed = Color32::from_rgb(
                over(tokens.scrim.r(), tokens.panel.r()),
                over(tokens.scrim.g(), tokens.panel.g()),
                over(tokens.scrim.b(), tokens.panel.b()),
            );
            assert!(
                luminance(dimmed) < luminance(tokens.panel) * 0.9,
                "{theme}: the scrim leaves {:.0}% of the panel's luminance, which is not dimming",
                luminance(dimmed) / luminance(tokens.panel) * 100.0
            );
        }
    }

    /// A planned row never spends more than the band it was planned for, at any budget. The rows this
    /// bounds are painted in a pane the shell can leave a few glyphs wide, and a row that overran it
    /// paints under the next pane's widgets — which is the defect the rows were planned to close, at the
    /// design's width and nowhere else.
    #[test]
    fn a_planned_row_never_spends_more_than_its_band() {
        let gap = f32::from(S3);
        let note_w = 40.0;
        let widths = [168.0_f32; 6];
        let fact_w = 172.0;
        assert!(
            !fit_row(0.0, gap, &widths, note_w, Some(5)).paints(),
            "a band no pill can enter paints nothing"
        );
        let mut budget = 0.0_f32;
        while budget <= 800.0 {
            let fitted = fit_row(budget, gap, &widths, note_w, Some(5));
            if fitted.paints() {
                let spent = fitted.width(gap, note_w, &widths);
                assert!(spent <= budget, "a row planned for {budget} spends {spent}");
            }
            let row = fit_row_with_fact(budget, gap, &widths, note_w, fact_w, Some(5));
            if row.fitted.paints() {
                let spent = row.fitted.width(gap, note_w, &widths)
                    + if row.fact { gap + fact_w } else { 0.0 };
                assert!(
                    spent <= budget,
                    "a row with fact={} planned for {budget} spends {spent}",
                    row.fact
                );
            }
            budget += 1.0;
        }
    }

    /// A headless frame the size of the app's window, so a dialog can sit where `egui::Modal` centres
    /// it: the design's 1440×900 metric, from the origin.
    fn window_frame() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(1440.0, 900.0),
            )),
            ..Default::default()
        }
    }

    /// One press and one release at `pos`, the way `table.rs` drives one.
    fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }
    }

    /// The dialog's own ui: [`DIALOG_W`] wide and stacked downward, at the x a `Modal` centres it at in
    /// a 1440 window and at the top edge the design measures for the Tunnel step.
    fn dialog(ui: &mut Ui) -> Ui {
        ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    egui::pos2((1440.0 - DIALOG_W) / 2.0, 216.5),
                    Vec2::new(DIALOG_W, 900.0),
                ))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        )
    }

    /// A frame's painted shapes, in paint order. The headless context never applies the texture delta.
    fn shapes(mut out: egui::FullOutput) -> Vec<egui::Shape> {
        out.textures_delta.clear();
        out.shapes
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect()
    }

    fn fills(shapes: &[egui::Shape], colour: Color32) -> bool {
        shapes
            .iter()
            .any(|shape| matches!(shape, egui::Shape::Rect(rect) if rect.fill == colour))
    }

    fn strokes(shapes: &[egui::Shape], colour: Color32, width: f32) -> bool {
        shapes.iter().any(|shape| {
            matches!(shape, egui::Shape::Rect(rect)
                if rect.stroke.color == colour && (rect.stroke.width - width).abs() < 0.01)
        })
    }

    /// The ink a `painter.text` call laid down, read back off the galley it painted.
    fn text_ink(shape: &egui::Shape) -> Option<(String, Color32)> {
        match shape {
            egui::Shape::Text(text) => Some((
                text.galley.text().to_owned(),
                text.galley.job.sections[0].format.color,
            )),
            _ => None,
        }
    }

    /// The dialog layer's arithmetic: every band and every gap is the design's mark, and the two dialog
    /// widths are the ones a 1440 window centres at 440 and 500.
    #[test]
    fn the_dialog_layer_adds_up_to_the_designs_marks() {
        assert_eq!(DIALOG_W, 560.0);
        assert_eq!(DIALOG_NARROW_W, 440.0);
        assert_eq!(
            (1440.0 - DIALOG_W) / 2.0,
            440.0,
            "`.dlg` is x 440..1000 in a 1440 window"
        );
        assert_eq!(
            (1440.0 - DIALOG_NARROW_W) / 2.0,
            500.0,
            "the confirmation is x 500..940"
        );
        assert_eq!(
            DIALOG_W - 2.0 * S7 as f32,
            520.0,
            "`.dlg-b`'s content width"
        );

        assert_eq!(DLG_HEAD_H, 68.0);
        assert_eq!(
            DLG_HEAD_H,
            S6 as f32 + S5 as f32 + 40.0,
            "`--s6` + the title block + `--s5`"
        );
        let block = T_LG * DLG_LINE_H + S1 as f32 + T_SM * DLG_LINE_H;
        assert!(
            (block - 39.7).abs() <= 0.5,
            "the title block is a `T_LG` line, `--s1` and a `T_SM` line at the sheet's 1.45"
        );
        assert!(
            block <= DLG_HEAD_H - S6 as f32 - S5 as f32,
            "and those two lines have to fit between the head's padding"
        );

        assert_eq!(DLG_TABS_H, 33.0, "a 32px item and the strip's hairline");
        assert_eq!(DLG_TABS_H, DLG_TAB_H + 1.0);
        assert_eq!(
            S6 as f32 + S3 as f32 + INPUT_H,
            50.0,
            "the design's `.fld`: `--s6` label box, `--s3` gap, 28px control (its pages render it \
             48.84 and 49.94 on the same inherited-line-height difference as the head)"
        );
        assert_eq!(DLG_SEG_H, 28.0);
        assert_eq!(
            DLG_SEG_ITEM_H,
            DLG_SEG_H - 2.0,
            "inside the container's border"
        );
        // The design's own footer band: its 1px top hairline, `--s5` of padding, two 28px rows `--s4`
        // apart (`components/dialogs.html`'s `.dlg-f` plus the editor's column override) — and the
        // same band with the single row a confirmation carries.
        assert_eq!(
            DLG_FOOT_H, 89.0,
            "1 + 12 + a 28px row + 8 + a 28px row + 12"
        );
        assert_eq!(
            DLG_FOOT_H,
            1.0 + 2.0 * S5 as f32 + S4 as f32 + 2.0 * BUTTON_H
        );
        assert_eq!(DLG_FOOT_ONE_H, 53.0, "1 + 12 + a 28px row + 12");
        assert_eq!(DLG_CHIP_H, 20.0);
        assert_eq!(KEYCHIP_H, 18.0);
        assert_eq!(CHECKBOX, 14.0);
        assert_eq!(STATUS_DOT, 8.0);
    }

    /// The bands the helpers lay out are those marks, and the geometry inside them is where the sheet
    /// puts it: the title `--s6` below the head's top edge with its subtitle `--s1` under the title's own
    /// line, the `×` 24×24 with its right edge 20 inside the dialog, the strip's hairline 100 below the
    /// head's top, the strip's indicator over the item's own width, the dialog's seg 26 inside its 28
    /// with a 1px divider between its items, and the footer's hairline on its own top edge.
    #[test]
    fn the_dialog_bands_are_the_designs_marks() {
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let mut bands = [0.0_f32; 5];
        let mut close = Rect::NOTHING;
        let mut foot = Rect::NOTHING;
        let mut nothing_clicked = true;
        let mut tabs_width = 0.0_f32;
        let mut seg_width = 0.0_f32;
        let mut seg_total = 0.0_f32;
        let mut name = String::from("analytics-prod");
        let out = ctx.run_ui(window_frame(), |ui| {
            let mut ui = dialog(ui);
            // The dialog's chrome is flush: the head, the step strip, the body and the footer butt
            // against each other, which is why the strip's hairline lands 100 below the head's top.
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut at = ui.cursor().top();
            close = dialogue_head(
                &mut ui,
                &tokens,
                None,
                "Edit connection",
                "analytics-prod · PostgreSQL",
                false,
            )
            .rect;
            bands[0] = ui.cursor().top() - at;
            at = ui.cursor().top();
            let labels = ["General", "TLS", "Tunnel", "Advanced"];
            nothing_clicked = dialogue_tabs(&mut ui, &tokens, &labels, 0).is_none();
            tabs_width = measured(&ui, tokens.font(T_MD), "General");
            bands[1] = ui.cursor().top() - at;
            at = ui.cursor().top();
            let _ = input(&mut ui, &tokens, 320.0, false, true, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut name)
                        .frame(Frame::NONE)
                        .margin(Margin::ZERO),
                )
            });
            bands[2] = ui.cursor().top() - at;
            at = ui.cursor().top();
            nothing_clicked &= dialogue_segmented(
                &mut ui,
                &tokens,
                &[("Key file", false), ("Agent", false), ("Password", true)],
            )
            .is_none();
            seg_width = measured(&ui, tokens.mono(T_SM), "Password") + 2.0 * f32::from(S5);
            seg_total = ["Key file", "Agent", "Password"]
                .iter()
                .map(|label| measured(&ui, tokens.mono(T_SM), label) + 2.0 * f32::from(S5))
                .sum::<f32>()
                + 2.0 * 1.0
                + 2.0;
            bands[3] = ui.cursor().top() - at;
            at = ui.cursor().top();
            foot = dialogue_foot(&mut ui, &tokens, DLG_FOOT_H, |ui| {
                ui.horizontal(|ui| {
                    let _ = dialogue_button(ui, &tokens, "Cancel", Emphasis::Plain, true);
                });
                ui.horizontal(|ui| {
                    let _ = dialogue_button(ui, &tokens, "Save", Emphasis::Primary, true);
                });
            });
            bands[4] = ui.cursor().top() - at;
        });
        let out = shapes(out);

        for (band, mark) in bands
            .into_iter()
            .zip([DLG_HEAD_H, DLG_TABS_H, INPUT_H, DLG_SEG_H, DLG_FOOT_H])
        {
            assert!(
                (band - mark).abs() <= 0.5,
                "a band of {band} is not the design's {mark}"
            );
        }
        assert!(nothing_clicked, "a frame with no pointer is nobody's click");
        assert_eq!(close.size(), Vec2::splat(TAB_CLOSE));
        assert!(
            (close.right() - 980.0).abs() <= 0.5,
            "the × ends 20 inside the dialog's right edge, at x 980"
        );
        assert_eq!(foot.height(), DLG_FOOT_H);

        let head_band = out
            .iter()
            .find_map(|shape| match shape {
                egui::Shape::Rect(rect)
                    if rect.fill == tokens.panel
                        && (rect.rect.height() - DLG_HEAD_H).abs() <= 0.5 =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .expect("the head paints its own band");
        assert!(
            (head_band.left() - 440.0).abs() <= 0.5 && (head_band.width() - DIALOG_W).abs() <= 0.5,
            "`.dlg` is x 440..1000: {head_band:?}"
        );

        let head_top = 216.5 + f32::from(S6);
        let ink = |want: &str| {
            out.iter()
                .filter_map(text_ink)
                .find(|(text, _)| text == want)
        };
        let title = out
            .iter()
            .find_map(|shape| match shape {
                egui::Shape::Text(text) if text.galley.text() == "Edit connection" => {
                    Some((text.pos.y, text.galley.size().y))
                }
                _ => None,
            })
            .expect("the head paints its title");
        assert!(
            (title.0 - head_top).abs() <= 0.5,
            "the title's line starts --s6 below the head's top edge"
        );
        let subtitle = out
            .iter()
            .find_map(|shape| match shape {
                egui::Shape::Text(text) if text.galley.text().starts_with("analytics-prod ·") => {
                    Some((text.pos.y, text.galley.size().y))
                }
                _ => None,
            })
            .expect("the head paints its subtitle");
        assert!(
            (subtitle.0 - title.0 - title.1 - f32::from(S1)).abs() <= 0.5,
            "the subtitle follows the title's own line, --s1 under it"
        );
        assert!(
            title.1 + f32::from(S1) + subtitle.1 <= DLG_HEAD_H - f32::from(S6) - f32::from(S5),
            "the head's two lines have to fit above its bottom padding"
        );
        assert_eq!(
            ink("Edit connection").map(|(_, ink)| ink),
            Some(tokens.text)
        );
        assert_eq!(
            ink("analytics-prod · PostgreSQL").map(|(_, ink)| ink),
            Some(tokens.text_dim)
        );
        assert_eq!(ink("Tunnel").map(|(_, ink)| ink), Some(tokens.text_dim));
        assert_eq!(ink("General").map(|(_, ink)| ink), Some(tokens.text));
        assert_eq!(
            ink("Password").map(|(_, ink)| ink),
            Some(tokens.accent_text)
        );

        let hairlines: Vec<f32> = out
            .iter()
            .filter_map(|shape| match shape {
                egui::Shape::LineSegment { points, stroke }
                    if points[0].y == points[1].y && stroke.color == tokens.stroke =>
                {
                    Some(points[0].y)
                }
                _ => None,
            })
            .collect();
        assert!(
            hairlines
                .iter()
                .any(|y| (y - (216.5 + DLG_HEAD_H + DLG_TAB_H + 0.5)).abs() <= 0.5),
            "the strip's hairline sits 100 below the head's top edge: {hairlines:?}"
        );
        assert!(
            hairlines
                .iter()
                .any(|y| (y - (foot.top() + 0.5)).abs() <= 0.5),
            "the footer's hairline is on its own top edge: {hairlines:?}"
        );

        let accent_rects = |height: f32| -> Vec<f32> {
            out.iter()
                .filter_map(|shape| match shape {
                    egui::Shape::Rect(rect)
                        if rect.fill == tokens.accent
                            && (rect.rect.height() - height).abs() <= 0.5 =>
                    {
                        Some(rect.rect.width())
                    }
                    _ => None,
                })
                .collect()
        };
        assert!(
            accent_rects(INDICATOR_W)
                .into_iter()
                .any(|width| (width - tabs_width).abs() <= 0.5),
            "the active tab's indicator runs over the item's own width"
        );
        assert!(
            accent_rects(DLG_SEG_ITEM_H)
                .into_iter()
                .any(|width| (width - seg_width).abs() <= 0.5),
            "the dialog's seg is 28 around 26px items"
        );
        assert!(
            out.iter().any(|shape| matches!(shape, egui::Shape::Rect(rect)
                if rect.fill == tokens.panel_alt
                    && (rect.rect.height() - DLG_SEG_H).abs() <= 0.5
                    && (rect.rect.width() - seg_total).abs() <= 0.5)),
            "the seg's box is its three items, the two 1px dividers between them and its own border: \
             {seg_total}"
        );
    }

    /// `.inp` wears the design's error border, and the border plus the 2px ring when it holds the focus.
    #[test]
    fn a_dialog_input_takes_the_error_and_focus_rings() {
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let run = |error: bool, focus: Option<egui::Id>| {
            let mut name = String::from("analytics-prod");
            let mut field = None;
            let out = ctx.run_ui(window_frame(), |ui| {
                if let Some(focus) = focus {
                    ui.ctx().memory_mut(|memory| memory.request_focus(focus));
                }
                let mut ui = dialog(ui);
                let _ = input(&mut ui, &tokens, 320.0, error, true, |ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut name)
                            .frame(Frame::NONE)
                            .margin(Margin::ZERO),
                    );
                    field = Some(response.id);
                    response
                });
            });
            (field.expect("the field ran"), shapes(out))
        };

        let (id, plain) = run(false, None);
        assert!(
            plain
                .iter()
                .any(|shape| matches!(shape, egui::Shape::Rect(rect)
                if rect.fill == tokens.panel_alt && (rect.rect.height() - INPUT_H).abs() <= 0.5)),
            "`.inp` is 28 tall"
        );
        assert!(strokes(&plain, tokens.stroke, 1.0));
        assert!(!strokes(&plain, tokens.error, 1.0));
        assert!(!strokes(&plain, tokens.accent, 2.0));

        let (_, error) = run(true, None);
        assert!(
            strokes(&error, tokens.error, 1.0),
            "`.inp.err` is the error border"
        );
        assert!(!strokes(&error, tokens.accent, 2.0));

        let (_, focused) = run(false, Some(id));
        assert!(
            strokes(&focused, tokens.accent, 1.0),
            "`.inp.focus` is the accent border"
        );
        assert!(
            focused
                .iter()
                .any(|shape| matches!(shape, egui::Shape::Rect(rect)
                if rect.stroke.color == tokens.accent
                    && (rect.stroke.width - 2.0).abs() < 0.01
                    && (rect.rect.height() - (INPUT_H + 2.0)).abs() < 0.01)),
            "and the design's 2px ring, 1px outside it"
        );
    }

    /// The step strip is only the item under the pointer: the gaps between its labels and its `--s7`
    /// lead-in are nobody's.
    #[test]
    fn a_dialog_tab_reports_only_the_item_it_holds() {
        let labels = ["General", "TLS", "Tunnel", "Advanced"];
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let mut items: Vec<Rect> = Vec::new();
        let out = ctx.run_ui(window_frame(), |ui| {
            let mut ui = dialog(ui);
            let _ = dialogue_tabs(&mut ui, &tokens, &labels, 2);
            let mut x = (1440.0 - DIALOG_W) / 2.0 + f32::from(S7);
            for label in labels {
                let width = measured(&ui, tokens.font(T_MD), label);
                items.push(Rect::from_min_size(
                    egui::pos2(x, 216.5),
                    Vec2::new(width, DLG_TAB_H),
                ));
                x += width + f32::from(S6);
            }
        });
        let _ = shapes(out);

        let clicked = |pos: egui::Pos2| -> Option<usize> {
            let mut events = vec![egui::Event::PointerMoved(pos)];
            for _ in 0..2 {
                events.push(press(pos, true));
                events.push(press(pos, false));
            }
            let mut clicked = None;
            let out = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..window_frame()
                },
                |ui| {
                    let mut ui = dialog(ui);
                    clicked = dialogue_tabs(&mut ui, &tokens, &labels, 2);
                },
            );
            let _ = shapes(out);
            clicked
        };

        assert_eq!(
            clicked(items[2].center()),
            Some(2),
            "the active item is still clickable"
        );
        assert_eq!(clicked(items[0].center()), Some(0));
        assert_eq!(clicked(items[3].center()), Some(3));
        assert_eq!(
            clicked(egui::pos2(
                items[0].right() + f32::from(S6) / 2.0,
                items[0].center().y
            )),
            None,
            "the gap between two labels is nobody's"
        );
        assert_eq!(
            clicked(egui::pos2(
                (1440.0 - DIALOG_W) / 2.0 + f32::from(S7) / 2.0,
                items[0].center().y
            )),
            None,
            "the strip's lead-in is nobody's"
        );
    }

    /// The dialog's buttons and its controls paint the faces, heights and inks the tokens name: a
    /// disabled button wears `.btn.off` whatever its emphasis, an enabled one wears its own face, the
    /// checkbox is a 14px `accent` box with an `accent_text` ✓, the test row's dot is 8px, the keychain
    /// chip is 18, and a field's label line is `--s6`.
    #[test]
    fn the_dialogs_controls_paint_the_faces_the_tokens_name() {
        let ctx = egui::Context::default();
        let tokens = Tokens::of("dark");
        tokens.apply(&ctx);
        let mut heights = Vec::new();
        let mut label = Rect::NOTHING;
        let mut checked = true;
        let out = ctx.run_ui(window_frame(), |ui| {
            let mut ui = dialog(ui);
            for (label, emphasis, enabled) in [
                ("Save", Emphasis::Primary, false),
                ("Save", Emphasis::Primary, true),
                ("Delete", Emphasis::Danger, true),
                ("Test connection", Emphasis::Ghost, true),
            ] {
                heights.push(
                    dialogue_button(&mut ui, &tokens, label, emphasis, enabled)
                        .rect
                        .height(),
                );
            }
            let _ = checkbox(
                &mut ui,
                &tokens,
                "Save the SSH password in the login keychain",
                &mut checked,
                true,
            );
            let _ = status_dot(&mut ui, tokens.ok, STATUS_DOT);
            let _ = tag_of(&mut ui, &tokens, "keychain", KEYCHIP_H);
            label = field_label(&mut ui, &tokens, "SSH host").rect;
        });
        let out = shapes(out);

        assert!(
            heights
                .iter()
                .all(|height| (height - BUTTON_H).abs() <= 0.5),
            "every dialog button is 28 tall: {heights:?}"
        );
        assert_eq!(label.height(), f32::from(S6));
        assert!(
            fills(&out, tokens.panel_high),
            "a disabled button wears `.btn.off`"
        );
        assert!(fills(&out, tokens.accent), "`.primary` and the checked box");
        assert!(fills(&out, tokens.error), "`.danger` fills with error");
        assert!(strokes(&out, tokens.error, 1.0), "and borders with it");
        // The focused danger button also wears the design's 2px ring, 1px outside its own box, in
        // `error` — the face a keyboard user reaches by tabbing to Delete.
        let _ = ctx.run_ui(window_frame(), |ui| {
            let response =
                dialogue_button(&mut dialog(ui), &tokens, "Delete", Emphasis::Danger, true);
            ctx.memory_mut(|memory| memory.request_focus(response.id));
        });
        let focused = shapes(ctx.run_ui(window_frame(), |ui| {
            let _ = dialogue_button(&mut dialog(ui), &tokens, "Delete", Emphasis::Danger, true);
        }));
        assert!(
            strokes(&focused, tokens.error, 2.0),
            "a focused danger button wears its own 2px `error` ring"
        );
        assert!(
            out.iter()
                .any(|shape| matches!(shape, egui::Shape::Circle(circle)
                if circle.fill == tokens.ok && (circle.radius - STATUS_DOT / 2.0).abs() < 0.01)),
            "the test row's dot is the design's 8px"
        );

        let box_rect = out
            .iter()
            .find_map(|shape| match shape {
                egui::Shape::Rect(rect)
                    if rect.fill == tokens.accent
                        && (rect.rect.height() - CHECKBOX).abs() <= 0.5
                        && rect.corner_radius == CornerRadius::same(CB_RADIUS) =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .expect("the checkbox paints its checked 14px box");
        assert_eq!(box_rect.width(), CHECKBOX);

        let tick = out
            .iter()
            .filter_map(text_ink)
            .find(|(text, _)| text == "✓");
        assert_eq!(
            tick,
            Some(("✓".to_owned(), tokens.accent_text)),
            "the checked box prints its ✓ in `accent_text`"
        );

        let chip = out.iter().find_map(|shape| match shape {
            egui::Shape::Rect(rect)
                if (rect.rect.height() - KEYCHIP_H).abs() <= 0.5
                    && rect.fill == tokens.panel_alt =>
            {
                Some(rect.rect)
            }
            _ => None,
        });
        assert!(
            chip.is_some(),
            "`tag_of` paints its own height, not the row's 24"
        );
        assert!(
            out.iter()
                .any(|shape| matches!(shape, egui::Shape::Rect(rect)
                if (rect.rect.height() - KEYCHIP_H).abs() <= 0.5
                    && rect.stroke.color == tokens.stroke
                    && (rect.stroke.width - 1.0).abs() < 0.01)),
            "and strokes it with its own hairline"
        );
    }
}
