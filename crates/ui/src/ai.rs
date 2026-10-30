//! The AI band: draft SQL from a sentence, and the provider settings that make it work.
//!
//! It is a band inside the query tab rather than a modal, because the draft's destination is that
//! tab's editor — a dialog would cover the thing the form exists to fill. Every field is an existing
//! command: the provider, the model and the base URL are `app.setSetting ai_*`, the key is
//! `ai.setKey` (keychain), and the draft is `ai.generate` with `open_tab`, so this module adds no
//! engine surface and holds no state the engine does not own.
//!
//! The base URL sits with the provider and not with the key: it is an address, not a secret. That is
//! why `ai.setKey` never saved one.

use egui::{RichText, Sense, Vec2};

use nook_ai::Provider;
use nook_engine::Session;

use crate::app::AiForm;
use crate::icons::Glyph;
use crate::strings::tr;
use crate::theme::{self as design, Emphasis, Mark, Tokens};

/// Everything the UI thread gathers before handing `ai.generate` to a background worker.
///
/// The catalog, the keychain and the provider settings are read here, on the UI thread, because
/// they cost a `security` process or a driver call and must not run concurrently with a frame
/// that is in the middle of painting. The worker receives only immutable snapshots.
pub struct AiContext {
    pub provider: nook_ai::Provider,
    pub model: String,
    pub base_url: String,
    pub prompt: String,
    pub key: Option<String>,
    /// The session's active connection when the draft was spawned — the tab the draft opens lands
    /// on it, so its tables resolve against the database the user was looking at.
    pub conn: Option<usize>,
    /// The user's `ai.*` knobs (`max_tokens`, `temperature`, `timeout_secs`) so the draft honours
    /// the same configuration the CLI and MCP `ai.generate` do, instead of silently falling back
    /// to the library defaults.
    pub user: nook_ai::AiConfig,
    /// Pre-built `table_context` strings, one per mention that resolved — the worker cannot call
    /// `session.table_of`, so the UI thread does it and hands the result over.
    pub table_contexts: Vec<String>,
    pub dialect: nook_core::Dialect,
}

/// What the background worker returns: the drafted SQL and the provider/model it came from, or the
/// engine's own error sentence.
pub type AiOutcome = Result<(String, String, String), String>;

/// One in-flight `ai.generate`: the receiver the worker writes into, kept in `UiState` so a
/// frame can poll it. The drain clears it only once an outcome has actually landed.
pub struct AiRequest {
    pub receiver: std::sync::mpsc::Receiver<AiOutcome>,
    /// The connection the draft was written for; the tab it opens lands on it.
    pub conn: Option<usize>,
}

impl std::fmt::Debug for AiRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AiRequest").finish_non_exhaustive()
    }
}

/// Gathers everything the worker needs, on the UI thread. Called once, right before the thread is
/// spawned. The keychain read costs one `security` process; the catalog read is the driver's cached
/// answer — both are safe to do here and not on a background thread racing the next frame.
fn collect_ai_context(session: &Session, form: &AiForm) -> Result<AiContext, String> {
    let provider = form.provider;
    let model = form.effective_model();
    if model.is_empty() {
        return Err(format!(
            "set a model for {} — a {} server serves whatever it was started with",
            provider.label(),
            provider.label()
        ));
    }
    let base_url = form.base_url.trim();

    // Resolve the active connection's dialect the way `ai.generate` does: explicit `conn` param
    // comes from the tab, else the active connection, else SQLite.
    let conn_index = session
        .active_conn
        .filter(|&i| session.conns.get(i).is_some_and(|c| c.driver.is_some()));
    let dialect = conn_index
        .and_then(|i| session.conns.get(i))
        .and_then(|c| c.driver.as_ref())
        .map(|d| d.dialect())
        .unwrap_or(nook_core::Dialect::Sqlite);

    // Mention resolution: the UI thread has the session, the worker does not.
    let mut names: Vec<String> = Vec::new();
    for mention in nook_ai::mentioned(&form.prompt) {
        if !names.contains(&mention) {
            names.push(mention);
        }
    }
    let mut table_contexts: Vec<String> = Vec::new();
    if let Some(index) = conn_index {
        for name in &names {
            let reference = nook_engine::TableRef::parse(name);
            match session.table_of(index, reference.schema.as_deref(), &reference.table) {
                Ok(table) => {
                    table_contexts.push(nook_ai::table_context(&table));
                }
                Err(_) => {
                    // The engine's own `ai.generate` logs the same warning; a mention that matched
                    // nothing is a typo and the request goes out one table lighter.
                }
            }
        }
    }

    let key = nook_engine::profiles::ai_key(provider).ok().flatten();

    if provider.needs_key() && key.as_deref().is_none_or(|k| k.trim().is_empty()) {
        return Err(format!(
            "no API key for {}; run ai.setKey, or set NOOK_AI_KEY for the run",
            provider.label()
        ));
    }

    Ok(AiContext {
        provider,
        model,
        base_url: base_url.to_string(),
        prompt: form.prompt.clone(),
        key,
        conn: conn_index,
        user: session.settings.ai.clone(),
        table_contexts,
        dialect,
    })
}

/// Hand a prompt to a background worker and return the handle to poll it from later frames.
///
/// All expensive reads (keychain, catalog, config) happen on the calling thread before the worker
/// is spawned, so the worker only does the `curl` round-trip and can run on a plain OS thread.
pub fn spawn_ai_request(context: AiContext) -> AiRequest {
    let conn = context.conn;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let outcome = run_ai_worker(context);
        let _ = tx.send(outcome);
    });
    AiRequest { receiver: rx, conn }
}

/// The worker's whole job: build the request, call the provider, extract the SQL.
fn run_ai_worker(context: AiContext) -> AiOutcome {
    let user_cfg = context.user.clone();
    let cfg = nook_ai::AiConfig {
        provider: context.provider,
        model: context.model.clone(),
        base_url: context.base_url.clone(),
        max_tokens: user_cfg.max_tokens,
        temperature: user_cfg.temperature,
        timeout_secs: user_cfg.timeout_secs,
    };
    let system = nook_ai::system_prompt(context.dialect, &context.table_contexts);
    let client = nook_ai::CurlChat;
    let answer = match nook_ai::Chat::chat(
        &client,
        &cfg,
        context.key.as_deref(),
        &system,
        &context.prompt,
    ) {
        Ok(answer) => answer,
        Err(e) => return Err(e.to_string()),
    };
    let sql = nook_ai::extract_sql(&answer);
    if sql.is_empty() {
        return Err("the model answered with nothing".to_string());
    }
    Ok((sql, context.model, context.provider.label().to_string()))
}

/// Reap a finished `ai.generate` request from the state. The outcome rules:
///
/// - While the request is in flight (`ai_request` is `Some` but the channel is empty) nothing
///   happens: the receiver must survive an empty poll, because it is the only handle left to the
///   worker's result, and dropping it would strand the draft.
/// - A success opens a new query tab with the drafted SQL and closes the band.
/// - An error is logged and toasted; the band stays open so the user can fix the cause and retry.
///
/// Free of `NookApp` so the whole flow — spawn, empty polls, landing, tab opening — is testable
/// without an `eframe::CreationContext`.
pub fn drain_ai_outcome(session: &mut Session, state: &mut crate::app::UiState) {
    let Some(request) = &state.ai_request else {
        return;
    };
    let Ok(outcome) = request.receiver.try_recv() else {
        return;
    };
    let conn = request.conn;
    state.ai_request = None;
    match outcome {
        Ok((sql, model, provider)) => {
            let index = session.add_tab(nook_engine::TabKind::Query, "AI query", conn);
            if let Some(tab) = session.tabs.get_mut(index) {
                tab.sql.text = sql;
            }
            session.log(
                nook_engine::Level::Success,
                format!("ai: {model} drafted SQL via {provider} — opened in a new tab"),
            );
            state.ai_band = false;
        }
        Err(message) => {
            session.log(nook_engine::Level::Error, format!("ai.generate: {message}"));
            state.toast(message);
        }
    }
}

/// The prompt field's height: three 20px rows and its own padding.
const PROMPT_H: f32 = 74.0;
/// The band's floor. A window too short for even this scrolls the band rather than collapsing it to
/// a strip of clipped controls.
const BAND_MIN_H: f32 = 180.0;
/// The room the editor keeps whatever the band does. This *is* the band's cap: a fixed ceiling of its
/// own would clip the footer as soon as the form grew a step, which is a number to re-guess on every
/// design change, while "leave the editor its room" is the actual constraint.
const EDITOR_MIN_H: f32 = 220.0;
/// The mention column's width: the design's 168px widened for `schema.table` keys, which is what
/// `ai.tables` spells for every Postgres table.
const MENTION_W: f32 = 200.0;
/// How many candidates the column offers. Autocomplete wants more than the design's three — a bare
/// `@` against a 228-table catalog is useless at three — and the column's own `ScrollArea` absorbs
/// the rows that do not fit the prompt's height.
const MENTIONS: usize = 8;
/// The numbered step's label column, from the design's 132px at the app's own type sizes.
const STEP_LABEL_W: f32 = 96.0;
/// The provider control's width: a single dropdown, held at the old four-segment figure so the column
/// stays a stable, generous 236px (the shell's minimum window assumes it) instead of shrinking to the label.
const PROVIDER_W: f32 = 236.0;
/// The base-URL field's width: the design's 300px trimmed so `01`+`02` share a row at the shell's
/// default width.
const BASE_URL_W: f32 = 260.0;
/// The key field's width: the design's 220px trimmed so `03`+`04` share the second row.
const KEY_W: f32 = 200.0;
/// The model field's width: the design's 220px trimmed, matching the key field.
const MODEL_W: f32 = 200.0;
/// The narrowest a step's control column may be squeezed to. Below the design's own widths a pane
/// left by a dragged panel would otherwise paint the columns past itself; at this floor the fields
/// are cramped but still fields.
const COL_MIN_W: f32 = 132.0;

/// Paint the band when it is open, and run what its controls asked for.
///
/// `session` is borrowed mutably only to run commands; the form is the working copy, so a frame that
/// is only being painted never writes to the engine.
pub fn band(
    ui: &mut egui::Ui,
    session: &mut Session,
    state: &mut crate::app::UiState,
    tokens: &Tokens,
) {
    if !state.ai_band {
        return;
    }
    let mut form = state.ai_form.clone();
    // The candidate list follows the prompt's live `@token`: when the token moves, the engine's own
    // `ai.tables` answers the new filter (its catalog is cached, so this costs no driver round-trip).
    // `None` — the prompt's tail is not a mention — restores the unfiltered top-N fallback.
    let live = active_mention_query(&form.prompt);
    refresh_mentions(session, &mut form, live);
    let mut generate_clicked = false;
    let mut close = false;
    let generating = state.ai_request.is_some();
    let cap = (ui.available_height() - EDITOR_MIN_H).max(BAND_MIN_H);

    egui::Frame::new()
        .fill(tokens.panel_alt)
        .inner_margin(egui::Margin::symmetric(design::S6, design::S5))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .max_height(cap)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = f32::from(design::S5);
                        head(ui, tokens, &form, &mut close);
                        rule(ui, tokens);
                        prompt_row(ui, tokens, &mut form);
                        rule(ui, tokens);
                        steps(ui, tokens, &mut form);
                        rule(ui, tokens);
                        foot(
                            ui,
                            tokens,
                            &form,
                            generating,
                            &mut generate_clicked,
                            &mut close,
                        );
                    });
                });
        });

    // Dirty settings flush — unchanged from before.
    if form.dirty {
        form.dirty = false;
        let (provider, model, base_url) = {
            let ai = &session.settings.ai;
            (ai.provider, ai.model.clone(), ai.base_url.clone())
        };
        for (key, value, unchanged) in [
            (
                "ai_provider",
                form.provider.as_str().to_string(),
                form.provider == provider,
            ),
            ("ai_model", form.model.clone(), form.model == model),
            (
                "ai_base_url",
                form.base_url.clone(),
                form.base_url == base_url,
            ),
        ] {
            if unchanged {
                continue;
            }
            if let Err(e) = session.run(
                "app.setSetting",
                serde_json::json!({"key": key, "value": value}),
            ) {
                session.log(
                    nook_engine::Level::Warn,
                    format!("ai: {key} was not saved: {e}"),
                );
            }
        }
    }
    // The key is a separate command: it goes to the keychain, never to the store. Committed when
    // the field is left, or — because `lost_focus()` is only true the frame *after* the click —
    // right here when a command is about to run, so a key typed and then Generated is on the
    // keychain before the worker starts.
    let about_to_run = generate_clicked || close;
    if should_commit_key(&form, about_to_run) {
        form.key_dirty = false;
        match session.run(
            "ai.setKey",
            serde_json::json!({"provider": form.provider.as_str(), "key": form.key.clone()}),
        ) {
            Ok(_) => form.key_set = true,
            Err(e) => session.log(nook_engine::Level::Error, format!("ai.setKey: {e}")),
        }
    }
    form.commit_key = false;
    if form.forget_key {
        form.forget_key = false;
        match session.run(
            "ai.clearKey",
            serde_json::json!({"provider": form.provider.as_str()}),
        ) {
            Ok(_) => {
                form.key_set = false;
                form.key.clear();
            }
            Err(e) => session.log(nook_engine::Level::Error, format!("ai.clearKey: {e}")),
        }
    }
    // The draft is now a background worker: gather the context on the UI thread (keychain, catalog,
    // config — each costs a `security` process or a driver call), then hand it to a plain OS thread
    // that does only the `curl` round-trip. The receiver lives in `state.ai_request` and is
    // drained by `NookApp::drain_ai_outcomes` on later frames, which is when the tab opens.
    if generate_clicked && !generating {
        match collect_ai_context(session, &form) {
            Ok(context) => {
                state.ai_request = Some(spawn_ai_request(context));
                session.log(
                    nook_engine::Level::Info,
                    format!(
                        "ai: drafting SQL with {} · {} — this may take a moment",
                        form.provider.label(),
                        form.effective_model()
                    ),
                );
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                session.log(nook_engine::Level::Error, format!("ai.generate: {e}"));
                state.toast(e);
            }
        }
    }
    // Close only when the user asked — the draft arriving is handled by `drain_ai_outcomes`, which
    // closes the band and switches to the new tab itself.
    if close {
        // A close also cancels an in-flight draft: the receiver is dropped with the request, so
        // the thread's `tx.send` simply returns `Err` and the worker exits without acting.
        state.ai_request = None;
        state.ai_band = false;
    }
    state.ai_form = form;
}

/// Whether the key field's contents go to the keychain on this frame: the field was left, a command
/// is about to run, or the band is being put away — the latter two because `lost_focus()` trails the
/// click that caused it by a frame, so Generate's own frame would otherwise ask for a key the
/// keychain has not been given yet, and a close would drop the frame that would have committed it.
/// A keychain write is a `security` process, so none of the three fires per keystroke.
fn should_commit_key(form: &AiForm, about_to_run: bool) -> bool {
    (form.commit_key || about_to_run) && form.key_dirty && !form.key.trim().is_empty()
}

/// The head: the title, the provider · model subtitle, and the way out.
///
/// Title is always shown. The subtitle is dropped when the pane is too narrow for it; the Close
/// button is always reachable. The hint ("draft opens in a new tab") moved to the footer, where
/// the reader actually looks for it.
fn head(ui: &mut egui::Ui, tokens: &Tokens, form: &AiForm, close: &mut bool) {
    let title = "Generate SQL";
    let subtitle = format!("{} · {}", form.provider.label(), form.effective_model());
    let avail = ui.available_width();
    let close_w = design::toolbar_button_width(ui, tokens, tr("Close"), None);
    let title_w = design::text_width(ui, tokens.font(design::T_XL), title);
    let subtitle_w = design::measured_width(ui, tokens, &subtitle);
    let subtitle_fits =
        avail >= title_w + f32::from(design::S5) + subtitle_w + f32::from(design::S5) + close_w;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = f32::from(design::S5);
        ui.label(
            RichText::new(title)
                .font(tokens.font(design::T_XL))
                .color(tokens.text),
        );
        if subtitle_fits {
            ui.label(
                RichText::new(subtitle)
                    .font(tokens.mono(design::T_XS))
                    .color(tokens.text_dim),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if design::toolbar_button(
                ui,
                tokens,
                "Close",
                None,
                tokens.panel_high,
                Emphasis::Plain,
                true,
                |_| {},
            )
            .clicked()
            {
                *close = true;
            }
        });
    });
}

/// The prompt, and the mentions `@` can resolve to. Wrapped like [`steps`], so the fixed-width mention
/// column drops below the prompt rather than pushing the band's content past the pane.
fn prompt_row(ui: &mut egui::Ui, tokens: &Tokens, form: &mut AiForm) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = f32::from(design::S5);
        let width = (ui.available_width() - MENTION_W - f32::from(design::S5)).max(120.0);
        design::input_tall(ui, tokens, width, PROMPT_H, false, true, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut form.prompt)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text("users who signed up this week, with their order count @users")
                    .font(tokens.mono(design::T_MD))
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::ZERO)
                    .text_color(tokens.text),
            )
        });
        mentions(ui, tokens, form);
    });
}

/// The mention column: what `ai.tables` offered for the prompt's live `@token`, refreshed by
/// [`refresh_mentions`] before this paints. A pure view — the list is the engine's own answer, the
/// click's branch is [`apply_mention`]'s, and this holds no state of its own.
fn mentions(ui: &mut egui::Ui, tokens: &Tokens, form: &mut AiForm) {
    ui.allocate_ui_with_layout(
        Vec2::new(MENTION_W, PROMPT_H),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing.y = f32::from(design::S2);
            let live = form.mention_filter.is_some();
            let header = match &form.mention_filter {
                Some(q) if !q.is_empty() => format!("tables · @{q}"),
                Some(_) => "tables".to_string(),
                None => "tables".to_string(),
            };
            ui.label(
                RichText::new(header)
                    .font(tokens.mono(design::T_XS))
                    .color(tokens.text_faint),
            );
            if form.mentions.is_empty() {
                ui.label(
                    RichText::new(if live {
                        "no table matches"
                    } else {
                        "no connection open"
                    })
                    .font(tokens.mono(design::T_XS))
                    .color(tokens.text_faint),
                );
                return;
            }
            // The window is wider than the prompt box is tall, so the candidates scroll inside the
            // column's fixed height: the band's row can never grow past the prompt it sits beside.
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = f32::from(design::S2);
                    for (name, columns) in &form.mentions {
                        let label = format!("@{name}");
                        let response = ui
                            .add(
                                egui::Label::new(
                                    RichText::new(format!("{label}  {columns} col"))
                                        .font(tokens.mono(design::T_XS))
                                        .color(tokens.text_dim),
                                )
                                .sense(Sense::click()),
                            )
                            .on_hover_text(if live {
                                format!("Replace the @token with {label}")
                            } else {
                                format!("Append {label} to the prompt")
                            });
                        if response.clicked() {
                            apply_mention(&mut form.prompt, name);
                        }
                    }
                    // Only the fallback mode counts the catalog: a filtered window is capped by
                    // `MENTIONS` by design, so its remainder is not a thing the user is missing.
                    if !live && form.mentions_total > form.mentions.len() {
                        ui.label(
                            RichText::new(format!(
                                "+{} more",
                                form.mentions_total - form.mentions.len()
                            ))
                            .font(tokens.mono(design::T_XS))
                            .color(tokens.text_faint),
                        );
                    }
                });
        },
    );
}

/// The four numbered fields, as the design's 2x2 table: provider + base URL on the first row, key +
/// model on the second.
///
/// A fixed grid, not a wrap: the wrap left the second row's steps ragged — the user's own screenshot —
/// and the four columns at their design widths fit the shell's default band twice over. The grid
/// absorbs a narrow pane by shrinking every column to the [`COL_MIN_W`] floor and scrolling, the same
/// answer [`band`]'s own `ScrollArea` gives the band's taller content; laid out unwrapped past the
/// pane, the scroll area's clip would hide the key field and widen the band's own content, dragging
/// the footer's right-aligned buttons off with it.
fn steps(ui: &mut egui::Ui, tokens: &Tokens, form: &mut AiForm) {
    // The design's own widths, or what the pane can actually give a column once the step's label and
    // the gap are paid for. A fixed column would paint past a pane a dragged panel left narrow — the
    // scroll area does not clip horizontally, so the band's content would widen with it and drag the
    // footer's right-aligned buttons off the pane.
    let room = ui.available_width();
    // The grid's own arithmetic: two rows of label + gap + control, and one gap between the two
    // columns of a row. What the pane cannot afford is taken off every column equally, to the
    // [`COL_MIN_W`] floor — below it the grid scrolls, never paints past the band.
    let columns = 2.0;
    let gaps = columns - 1.0;
    let spend = columns * (STEP_LABEL_W + f32::from(design::S5)) + gaps * f32::from(design::S6);
    let share = ((room - spend) / columns).max(COL_MIN_W);
    let column = |wanted: f32| wanted.min(share);
    // One width per column, the widest control's own: a table's column does not rag — 01 and 03 take
    // the same box, 02 and 04 too, whatever the provider. (`KEY_W`/`MODEL_W` keep naming the floors
    // the engine's controls want; the column is what the pane sees.)
    let control_w = column(PROVIDER_W.max(BASE_URL_W).max(KEY_W).max(MODEL_W));
    let grid = egui::Grid::new("ai-steps")
        .num_columns(2)
        .spacing([f32::from(design::S6), f32::from(design::S5)]);
    grid.show(ui, |ui| {
        step(
            ui,
            tokens,
            "01",
            "Provider",
            control_w,
            "where the request goes",
            |ui| {
                provider_control(ui, tokens, form, control_w);
            },
        );
        step(
            ui,
            tokens,
            "02",
            "Base URL",
            control_w,
            "saved with the provider — an address, not a secret",
            |ui| {
                base_url_control(ui, tokens, form, control_w);
            },
        );
        ui.end_row();
        step(
            ui,
            tokens,
            "03",
            "API key",
            control_w,
            "in the keychain, never in the file",
            |ui| {
                if form.provider.needs_key() {
                    key_control(ui, tokens, form, control_w);
                } else {
                    ui.label(
                        RichText::new("not needed")
                            .font(tokens.mono(design::T_XS))
                            .color(tokens.text_dim),
                    );
                }
            },
        );
        step(
            ui,
            tokens,
            "04",
            "Model",
            control_w,
            if form.provider.models().is_empty() {
                "this server serves what it was started with"
            } else {
                "type any id — the shipped list is a shortcut"
            },
            |ui| {
                model_control(ui, tokens, form, control_w);
            },
        );
        ui.end_row();
    });
}

/// The model: an editable field for every provider — the vendors ship ids faster than this app can
/// follow, so the provider's own list is a shortcut beside the field, never a fence around it. The
/// engine sends the typed id verbatim, and an empty field keeps the engine's rule: the provider's
/// own default, named by the hint.
///
/// ponytail: a typed id survives only as the active provider's model (a provider switch clears it,
/// the switch's own rule). Persisting a per-provider history means a list in `AiConfig` and a store
/// migration — add when someone loses a typed id to the switch and minds.
fn model_control(ui: &mut egui::Ui, tokens: &Tokens, form: &mut AiForm, width: f32) {
    let models = form.provider.models();
    let hint = if models.is_empty() {
        "name the model the server serves".to_owned()
    } else {
        format!("empty = {}", form.provider.default_model())
    };
    ui.allocate_ui_with_layout(
        Vec2::new(width, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing.y = f32::from(design::S2);
            ui.horizontal(|ui| {
                // List-less: the field keeps the whole column; otherwise it yields the shortcut's
                // width and the gap between them.
                let field_w = if models.is_empty() {
                    width
                } else {
                    width
                        - design::icon_button_width(ui, tokens, "", true)
                        - ui.spacing().item_spacing.x
                };
                design::input(ui, tokens, field_w, false, true, |ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut form.model)
                            .desired_width(f32::INFINITY)
                            .hint_text(hint)
                            .font(tokens.mono(design::T_SM))
                            .frame(egui::Frame::NONE)
                            .margin(egui::Margin::ZERO)
                            .text_color(tokens.text),
                    );
                    if response.changed() {
                        form.dirty = true;
                    }
                    response
                });
                if models.is_empty() {
                    return;
                }
                let shortcut = design::icon_button(ui, tokens, "", Some(Glyph::Chevron), true);
                egui::Popup::menu(&shortcut).show(|ui| {
                    ui.set_min_width(width);
                    for model in models {
                        // Against `effective_model()`, not the raw field: an empty field means the
                        // provider's own default, and the hint names it — the list marks the one a
                        // request would send.
                        if ui
                            .selectable_label(
                                form.effective_model() == *model,
                                RichText::new(*model).font(tokens.mono(design::T_XS)),
                            )
                            .clicked()
                        {
                            form.model = (*model).to_string();
                            form.dirty = true;
                        }
                    }
                });
            });
        },
    );
}

/// One numbered step as its own nested grid: the number+name column on the left, the control on the
/// right with the description under it — the second row's right cell, so the description always
/// starts at the control's left edge and every cell ends on a description line, whatever the step.
///
/// `width` is the control column's own width, declared so the outer [`steps`] grid sizes its cell
/// from the plan rather than the content.
fn step(
    ui: &mut egui::Ui,
    tokens: &Tokens,
    number: &str,
    name: &str,
    width: f32,
    description: &str,
    add: impl FnOnce(&mut egui::Ui),
) {
    egui::Grid::new(egui::Id::new(("ai-step", number)))
        .num_columns(2)
        .spacing([f32::from(design::S5), f32::from(design::S2)])
        .show(ui, |ui| {
            step_label(ui, tokens, number, name);
            // Reserve the control column: a combo sizes to its text, and the description under
            // it must start at the column's own left edge, not the combo's.
            ui.allocate_ui_with_layout(
                Vec2::new(width, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                add,
            );
            ui.end_row();
            ui.weak("");
            if !description.is_empty() {
                ui.label(
                    RichText::new(description)
                        .font(tokens.mono(design::T_XS))
                        .color(tokens.text_faint),
                );
            }
            ui.end_row();
        });
}

/// A numbered step's label column, from the design's 132px.
fn step_label(ui: &mut egui::Ui, tokens: &Tokens, number: &str, name: &str) {
    ui.allocate_ui_with_layout(
        Vec2::new(STEP_LABEL_W, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.label(
                RichText::new(number)
                    .font(tokens.mono(design::T_XL))
                    .color(tokens.accent),
            );
            ui.label(
                RichText::new(name.to_uppercase())
                    .font(tokens.mono(design::T_XS))
                    .color(tokens.text_dim),
            );
        },
    );
}

/// The provider control: one dropdown listing every provider the engine knows. Choosing one switches
/// the model, address, key and the `ai_provider` setting the way the engine does.
fn provider_control(ui: &mut egui::Ui, tokens: &Tokens, form: &mut AiForm, width: f32) {
    egui::ComboBox::from_id_salt("ai-provider")
        .width(width)
        .selected_text(RichText::new(form.provider.label()).font(tokens.mono(design::T_XS)))
        .show_ui(ui, |ui| {
            for provider in Provider::ALL.iter().copied() {
                if ui
                    .selectable_label(provider == form.provider, provider.label())
                    .clicked()
                {
                    choose_provider(form, provider);
                }
            }
        });
}

/// Switch provider the way the engine does: the model and the address become empty, so the new
/// provider's own defaults answer for them.
fn choose_provider(form: &mut AiForm, provider: Provider) {
    if form.provider == provider {
        return;
    }
    form.provider = provider;
    form.model.clear();
    form.base_url.clear();
    form.key.clear();
    form.key_dirty = false;
    form.commit_key = false;
    form.key_set = key_is_set(provider);
    form.dirty = true;
}

/// The base URL: the provider's own default as the placeholder, so an empty field means "use it".
fn base_url_control(ui: &mut egui::Ui, tokens: &Tokens, form: &mut AiForm, width: f32) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing.y = f32::from(design::S2);
            design::input(ui, tokens, width, false, true, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut form.base_url)
                        .desired_width(f32::INFINITY)
                        .hint_text(form.provider.default_base())
                        .font(tokens.mono(design::T_SM))
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .text_color(tokens.text),
                );
                if response.changed() {
                    form.dirty = true;
                }
                response
            });
        },
    );
}

/// The key: a password field and the state of the keychain, never the secret.
fn key_control(ui: &mut egui::Ui, tokens: &Tokens, form: &mut AiForm, width: f32) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing.y = f32::from(design::S2);
            design::input(ui, tokens, width, false, true, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut form.key)
                        .desired_width(f32::INFINITY)
                        .password(true)
                        .hint_text("paste a key")
                        .font(tokens.mono(design::T_SM))
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .text_color(tokens.text),
                );
                // The key is handed over only when the field is left, or when a command runs — never
                // per keystroke, which would store six half-keys, each one a `security` process. The
                // key is not a saved preference, so it never sets `dirty`.
                if response.changed() {
                    form.key_dirty = true;
                }
                if response.lost_focus() {
                    form.commit_key = true;
                }
                response
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = f32::from(design::S2);
                ui.label(
                    RichText::new(if form.key_set {
                        "in keychain"
                    } else {
                        "not set"
                    })
                    .font(tokens.mono(design::T_XS))
                    .color(if form.key_set {
                        tokens.ok
                    } else {
                        tokens.text_faint
                    }),
                );
                if form.key_set
                    && ui
                        .add(
                            egui::Label::new(
                                RichText::new("forget")
                                    .font(tokens.mono(design::T_XS))
                                    .color(tokens.accent),
                            )
                            .sense(Sense::click()),
                        )
                        .clicked()
                {
                    form.forget_key = true;
                }
            });
        },
    );
}

/// The footer: one hint sentence on the left, the two actions pinned right.
///
/// While a draft is in flight the footer shows a spinner instead of the hint and the Generate
/// button is disabled; Cancel doubles as "cancel the draft and close the band".
fn foot(
    ui: &mut egui::Ui,
    tokens: &Tokens,
    form: &AiForm,
    generating: bool,
    generate_clicked: &mut bool,
    close: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = f32::from(design::S5);
        if generating {
            // A spinner painted with the band's own geometry: 14px, accent-colored, 4-frame rotation.
            let t = ui.ctx().input(|i| i.time);
            let phase = ((t * 2.0).fract() * 4.0) as usize; // 4 ticks, one per quarter
            let size = design::GLYPH_SM;
            let (rect, _response) = ui.allocate_exact_size(Vec2::new(size, size), Sense::hover());
            let painter = ui.painter();
            let centre = rect.center();
            // Four dots, the active one in accent, the rest in text_faint.
            for i in 0..4 {
                let on = i == phase;
                let ink = if on { tokens.accent } else { tokens.text_faint };
                let angle = std::f32::consts::PI / 2.0 * i as f32;
                let r = size * 0.35;
                let pos = egui::pos2(
                    centre.x + r * angle.cos(),
                    centre.y - r * angle.sin(),
                );
                painter.circle_filled(pos, 2.0, ink);
            }
            ui.label(
                RichText::new("Generating…")
                    .font(tokens.mono(design::T_XS))
                    .color(tokens.text_dim),
            );
        } else {
            let sentence =
                "nothing is sent until Generate · @ expands to the table's columns, keys and indexes";
            let buttons = design::toolbar_button_width(ui, tokens, "Generate", Some(Mark::Play))
                + design::toolbar_button_width(ui, tokens, tr("Cancel"), None)
                + 2.0 * f32::from(design::S4);
            if ui.available_width() - buttons >= design::measured_width(ui, tokens, sentence) {
                ui.label(
                    RichText::new(sentence)
                        .font(tokens.mono(design::T_XS))
                        .color(tokens.text_faint),
                );
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = f32::from(design::S4);
            let ready = !form.prompt.trim().is_empty() && form.has_model();
            let generate_enabled = ready && !generating;
            if design::toolbar_button(
                ui,
                tokens,
                "Generate",
                Some(Mark::Play),
                tokens.accent,
                Emphasis::Primary,
                generate_enabled,
                |_| {},
            )
            .on_hover_text("Draft the SQL and open it in a new tab")
            .clicked()
            {
                *generate_clicked = true;
            }
            if design::toolbar_button(
                ui,
                tokens,
                "Cancel",
                None,
                tokens.panel_high,
                Emphasis::Plain,
                true,
                |_| {},
            )
            .on_hover_text(if generating {
                "Cancel the draft and close the band"
            } else {
                "Close the band"
            })
            .clicked()
            {
                *close = true;
            }
        });
    });
}

/// The hairline the design draws between the band's bands.
fn rule(ui: &mut egui::Ui, tokens: &Tokens) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), Sense::hover());
    ui.painter().hline(
        rect.left()..=rect.right(),
        rect.center().y,
        egui::Stroke::new(1.0, tokens.stroke),
    );
}

/// Whether the keychain already holds a key for this provider. Read through the engine, so the UI
/// never guesses and never reads the secret itself.
fn key_is_set(provider: Provider) -> bool {
    matches!(
        nook_engine::profiles::ai_key(provider),
        Ok(Some(key)) if !key.trim().is_empty()
    )
}
/// Whether a character can sit inside a table reference: the identifier characters plus the dot that
/// joins a schema to a table, which is how `ai.tables` spells every key it offers.
fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.'
}

/// The table filter the user is typing after the last `@`, or `None` when that `@` is not a mention
/// in progress: it sits inside a word (`me@example.com`), or the text after it has already run on
/// past the identifier (`@users joined to @orders`).
///
/// Read from the end of the prompt rather than the editor's cursor, ponytail: a prompt carrying two
/// live `@` tokens needs the cursor to say which one is being typed — upgrade when that matters.
fn active_mention_query(prompt: &str) -> Option<String> {
    let at = prompt.rfind('@')?;
    // The `@` has to open a word. Sliced from the whole prompt, so the offset is absolute and a
    // multi-line prompt is cut at the right byte.
    if prompt[..at].chars().last().is_some_and(is_ident_char) {
        return None;
    }
    let tail = &prompt[at + 1..];
    tail.chars().all(is_ident_char).then(|| tail.to_string())
}

/// One click on a candidate: replace the live `@partial` when one is being typed, append the
/// mention when not — the fallback column's affordance. One branch, driven by the same query the
/// candidate list was filtered with, so the two modes can never disagree.
fn apply_mention(prompt: &mut String, name: &str) {
    if active_mention_query(prompt).is_some() {
        replace_active_mention(prompt, name);
    } else {
        if !prompt.is_empty() && !prompt.ends_with(' ') {
            prompt.push(' ');
        }
        prompt.push('@');
        prompt.push_str(name);
        prompt.push(' ');
    }
}

/// Replace the live `@partial` token with the full mention, leaving a space after it so the next
/// word starts clean. A prompt with no live mention is untouched.
fn replace_active_mention(prompt: &mut String, full: &str) {
    let Some(query) = active_mention_query(prompt) else {
        return;
    };
    let keep = &prompt[..prompt.len() - query.len()]; // everything up to and including the `@`
    *prompt = format!("{keep}{full} ");
}

#[cfg(test)]
mod mention_tests {
    use super::*;

    #[test]
    fn a_plain_prompt_has_no_live_mention() {
        assert_eq!(active_mention_query("count the rows"), None);
    }

    #[test]
    fn a_bare_at_sign_is_the_start_of_a_filter() {
        assert_eq!(active_mention_query("summarise @"), Some("".to_string()));
    }

    #[test]
    fn a_partial_table_name_is_the_filter() {
        assert_eq!(
            active_mention_query("summarise @bangu"),
            Some("bangu".to_string())
        );
    }

    #[test]
    fn a_closed_mention_followed_by_prose_is_not_live() {
        assert_eq!(
            active_mention_query("count @public.bangunan for 2024"),
            None,
        );
    }

    #[test]
    fn the_schema_dot_counts_as_identifier() {
        assert_eq!(
            active_mention_query("join @public."),
            Some("public.".to_string())
        );
    }

    #[test]
    fn replacing_a_partial_writes_the_full_mention_and_a_space() {
        let mut prompt = "summarise @bangu".to_string();
        replace_active_mention(&mut prompt, "public.bangunan");
        assert_eq!(prompt, "summarise @public.bangunan ");
    }

    #[test]
    fn replacing_without_a_live_mention_leaves_the_prompt_alone() {
        let mut prompt = "summarise @public.bangunan for 2024".to_string();
        replace_active_mention(&mut prompt, "public.rt");
        assert_eq!(prompt, "summarise @public.bangunan for 2024");
    }

    #[test]
    fn an_email_address_is_not_a_mention() {
        // The `@` sits inside a word, so it never becomes a table the user did not ask for — and a
        // click cannot rewrite the address.
        assert_eq!(active_mention_query("mail me@example.com"), None);
        assert_eq!(active_mention_query("cc me@example.com please"), None);
    }

    #[test]
    fn a_mention_after_punctuation_is_live() {
        // A gate that demanded whitespace would kill this, the commonest shape in prose.
        assert_eq!(
            active_mention_query("(from @users"),
            Some("users".to_string())
        );
        assert_eq!(
            active_mention_query("join @orders,"),
            None,
            "the comma closes the token"
        );
    }

    #[test]
    fn a_mention_on_the_second_line_is_sliced_from_the_whole_prompt() {
        let mut prompt = "count rows\n@bangu".to_string();
        assert_eq!(active_mention_query(&prompt), Some("bangu".to_string()));
        replace_active_mention(&mut prompt, "public.bangunan");
        assert_eq!(
            prompt, "count rows\n@public.bangunan ",
            "the newline and the @ survive"
        );
    }

    #[test]
    fn a_click_replaces_when_a_mention_is_live_and_appends_when_not() {
        let mut live = "summarise @bangu".to_string();
        apply_mention(&mut live, "public.bangunan");
        assert_eq!(
            live, "summarise @public.bangunan ",
            "live token is replaced"
        );

        let mut fallback = "summarise the bangunan rows".to_string();
        apply_mention(&mut fallback, "public.bangunan");
        assert_eq!(
            fallback, "summarise the bangunan rows @public.bangunan ",
            "no live token: the mention is appended"
        );

        let mut empty = String::new();
        apply_mention(&mut empty, "public.bangunan");
        assert_eq!(
            empty, "@public.bangunan ",
            "an empty prompt takes the mention bare"
        );
    }
}

/// Seed the form from the session and the catalog. Called once when the band opens, so a frame's
/// typing is never overwritten by the session's own values.
pub fn seed(session: &mut Session, form: &mut AiForm) {
    let ai = &session.settings.ai;
    form.provider = ai.provider;
    form.model = ai.model.clone();
    form.base_url = ai.base_url.clone();
    form.key.clear();
    form.key_set = key_is_set(ai.provider);
    form.key_dirty = false;
    form.commit_key = false;
    form.dirty = false;
    form.forget_key = false;
    // A fresh form's filter is already None, so the dedup guard would skip the first fetch; seed
    // must always list the catalog, hence the Some("") sentinel that None rewrites below.
    refresh_mentions(session, form, Some(String::new()));
    form.mention_filter = None;
}

/// Point the form's candidate window at the prompt's live `@token`. The cache — the query this
/// window was fetched with — is compared before the engine is asked: `ai.tables` reads a cached
/// catalog, but a run per frame would still be a command dispatch and a JSON build for nothing.
fn refresh_mentions(session: &mut Session, form: &mut AiForm, query: Option<String>) {
    if query == form.mention_filter {
        return;
    }
    // The engine matches `key.contains(filter)` on dotted `schema.table` keys, so the query goes
    // through as-is: `@public.` matches exactly every `public.*` table, and a bare `@` (empty
    // filter) lists the catalog.
    let params = match query.as_deref() {
        Some(f) if !f.is_empty() => serde_json::json!({ "filter": f }),
        _ => serde_json::json!({}),
    };
    let value = session.run("ai.tables", params);
    let (list, total) = match value {
        Ok(value) => {
            let total = value["count"].as_u64().unwrap_or(0) as usize;
            let list = value["tables"]
                .as_array()
                .map(|tables| {
                    tables
                        .iter()
                        .filter_map(|table| {
                            Some((
                                table["table"].as_str()?.to_string(),
                                table["columns"].as_u64().unwrap_or(0) as usize,
                            ))
                        })
                        .take(MENTIONS)
                        .collect()
                })
                .unwrap_or_default();
            (list, total)
        }
        // A band with no connection open, or a catalog that failed to read, offers nothing rather
        // than a UI-side guess.
        Err(_) => (Vec::new(), 0),
    };
    form.mentions = list;
    form.mentions_total = total;
    form.mention_filter = query;
}

/// Preferences ▸ AI: every provider, its address, and whether a key is held — read from
/// `ai.providers`, so the table can never disagree with what a request would use. Picking a row runs
/// the same `app.setSetting ai_provider` the band does.
///
/// The answer is cached in `state` because the dialog repaints every frame and each `ai.providers`
/// spawns one `security` process per provider — seven per frame for a table that only changes when
/// the user acts. The cache is dropped when the dialog opens and after a pick.
pub fn preferences(
    ui: &mut egui::Ui,
    session: &mut Session,
    state: &mut crate::app::UiState,
    tokens: &Tokens,
) {
    let value = match state.ai_providers.clone() {
        Some(value) => value,
        None => match session.run("ai.providers", serde_json::json!({})) {
            Ok(value) => {
                state.ai_providers = Some(value.clone());
                value
            }
            Err(_) => {
                ui.label(
                    RichText::new("the AI settings could not be read")
                        .font(tokens.mono(design::T_XS))
                        .color(tokens.text_faint),
                );
                return;
            }
        },
    };
    let active = value["active"].as_str().unwrap_or_default().to_string();
    let keychain = value["keychain"].as_bool().unwrap_or(false);
    let mut pick: Option<String> = None;
    let providers = value["providers"].as_array().cloned().unwrap_or_default();
    for provider in &providers {
        let id = provider["id"].as_str().unwrap_or_default();
        let is_active = provider["active"].as_bool().unwrap_or(false);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = f32::from(design::S5);
            if ui
                .add(
                    egui::Label::new(
                        RichText::new(provider["label"].as_str().unwrap_or(id))
                            .font(tokens.mono(design::T_SM))
                            .color(if is_active {
                                tokens.accent
                            } else {
                                tokens.text
                            }),
                    )
                    .sense(Sense::click()),
                )
                .clicked()
            {
                pick = Some(id.to_string());
            }
            ui.label(
                RichText::new(provider["default_base_url"].as_str().unwrap_or_default())
                    .font(tokens.mono(design::T_XS))
                    .color(tokens.text_dim),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let needs = provider["needs_key"].as_bool().unwrap_or(true);
                let set = provider["key_set"].as_bool().unwrap_or(false);
                let (text, ink) = match (needs, set) {
                    (false, _) => ("no key needed", tokens.text_faint),
                    (true, true) => ("key in keychain", tokens.ok),
                    (true, false) if keychain => ("no key", tokens.warn),
                    (true, false) => ("no keychain", tokens.error),
                };
                ui.label(
                    RichText::new(text)
                        .font(tokens.mono(design::T_XS))
                        .color(ink),
                );
                if is_active {
                    ui.label(
                        RichText::new("active")
                            .font(tokens.mono(design::T_XS))
                            .color(tokens.accent),
                    );
                }
            });
        });
    }
    if let Some(id) = pick.filter(|id| *id != active) {
        match session.run(
            "app.setSetting",
            serde_json::json!({"key": "ai_provider", "value": id}),
        ) {
            Ok(_) => {
                // The engine cleared the model and the address with the switch (they belong to the
                // provider that was chosen), so the band's working copy has to move with it: left
                // holding the old provider, the next keystroke in any band field would re-write
                // `ai_provider` and silently undo this pick.
                if let Some(provider) = Provider::parse(&id) {
                    choose_provider(&mut state.ai_form, provider);
                    state.ai_form.dirty = false;
                }
                // The active row and the model line moved, so the cached table is stale.
                state.ai_providers = None;
            }
            Err(e) => session.log(nook_engine::Level::Warn, format!("ai: {e}")),
        }
    }
    ui.add_space(f32::from(design::S3));
    ui.label(
        RichText::new(format!(
            "model {} · {}",
            value["model"].as_str().unwrap_or_default(),
            value["base_url"].as_str().unwrap_or_default()
        ))
        .font(tokens.mono(design::T_XS))
        .color(tokens.text_faint),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every step's numbered label and its control from one frame of the band, with the pane they were
    /// painted in.
    type Painted = (Vec<(String, egui::Rect)>, egui::Rect);

    /// The AI band in a pane, one frame: a session holding `users`, the band open, the tokens.
    ///
    /// `name` keeps the test's config directory its own, the same reason the grid tests tag their
    /// database: `seed` and the dirty-flush write the session's settings, and parallel tests would
    /// race on one shared `NOOK_CONFIG_DIR`.
    fn band_pane(name: &str, size: egui::Vec2) -> Painted {
        let dir = std::env::temp_dir().join(format!("nook-ai-band-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the scratch config dir");
        std::env::set_var("NOOK_CONFIG_DIR", &dir);
        let mut session = Session::new();
        session
            .conns
            .push(nook_engine::Conn::new(nook_core::ConnectionConfig::sqlite(
                ":memory:",
            )));
        session.active_conn = Some(0);
        nook_engine::cmd::open_now(&mut session, 0).expect("the in-memory database opens");
        nook_engine::cmd::run_sql(
            &mut session,
            0,
            "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT NOT NULL);",
            Some(10),
        )
        .expect("the fixture schema is valid");
        nook_engine::cmd::refresh_catalog(&mut session, 0).expect("the catalog refreshes");
        let mut state = crate::app::UiState::default();
        state.ai_band = true;
        let tokens = Tokens::of("dark");
        let ctx = egui::Context::default();
        tokens.apply(&ctx);
        let pane = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), size);
        let input = egui::RawInput {
            screen_rect: Some(pane),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default_margins()
                .frame(egui::Frame::new().fill(tokens.panel))
                .show(ui, |ui| {
                    band(ui, &mut session, &mut state, &tokens);
                });
        });
        out.textures_delta.clear();
        (painted_step_text(&out.shapes), pane)
    }

    /// Every text run painted whole under its clip rect, with that rect: a label the band clipped away
    /// is not evidence that the label is on screen. The same filter the grid tests give their texts.
    fn painted_step_text(shapes: &[egui::epaint::ClippedShape]) -> Vec<(String, egui::Rect)> {
        fn walk(shape: &egui::Shape, clip: egui::Rect, out: &mut Vec<(String, egui::Rect)>) {
            match shape {
                egui::Shape::Text(text) => {
                    let rect = text.visual_bounding_rect();
                    if rect.top() >= clip.top() - 0.5 && rect.bottom() <= clip.bottom() + 0.5 {
                        out.push((text.galley.text().to_owned(), rect));
                    }
                }
                egui::Shape::Vec(shapes) => {
                    for s in shapes {
                        walk(s, clip, out);
                    }
                }
                _ => {}
            }
        }
        let mut texts = Vec::new();
        for clipped in shapes {
            walk(&clipped.shape, clipped.clip_rect, &mut texts);
        }
        texts
    }

    /// The band's four numbered fields sit as the design's 2x2 table, and every field's number and
    /// label start at its row's top edge. The grid fixed the wrap's ragged second row — the user's own
    /// screenshot — so the assertion is now per row: 01/02 at one y, 03/04 at one y, and 02's row
    /// below 01's by whatever 02's caption needs.
    ///
    /// The pane is the shell's own plausible band: the two-column spend at the design widths
    /// (2*(label + gap) + 236 + 260 + one column gap) plus the band's margins fits with room to
    /// spare, and a narrower pane shrinks the columns rather than breaking the table.
    #[test]
    fn the_numbered_steps_share_a_top_edge() {
        let (texts, pane) = band_pane("tops", egui::vec2(900.0, 600.0));
        let tops = |needle: &str| {
            texts
                .iter()
                .find(|(text, _)| text == needle)
                .map(|(_, rect)| rect.top())
                .unwrap_or_else(|| panic!("{needle} was not painted on the pane"))
        };
        let one = tops("01");
        let three = tops("03");
        for number in ["01", "02"] {
            let top = tops(number);
            assert!(
                (top - one).abs() < 0.5,
                "{number} sits at {top}, not at {one} where 01 sits"
            );
        }
        for number in ["03", "04"] {
            let top = tops(number);
            assert!(
                (top - three).abs() < 0.5,
                "{number} sits at {top}, not at {three} where 03 sits"
            );
        }
        assert!(
            three > one,
            "the second row sits at {three}, above the first row at {one}"
        );
        // And the controls themselves are top-aligned with their own labels: the provider combo's box
        // starts where its `01` does, on a pane that gives the row its full width.
        // And the footer's Generate is on the pane, not below the fold: the band's cap bounds the
        // grid's two rows plus head, prompt and foot, and a band that scrolls hides the one button
        // the band exists for.
        let generate = texts
            .iter()
            .find(|(text, _)| text == "Generate")
            .map(|(_, rect)| *rect)
            .unwrap_or_else(|| panic!("Generate was not painted"));
        assert!(
            pane.contains_rect(generate),
            "Generate at {generate:?} is below the pane's fold"
        );
        for label in ["PROVIDER", "BASE URL", "API KEY", "MODEL"] {
            let rect = texts
                .iter()
                .find(|(text, _)| *text == label)
                .map(|(_, rect)| *rect)
                .unwrap_or_else(|| panic!("{label} was not painted"));
            assert!(
                pane.contains_rect(rect),
                "{label} at {rect:?} is off the pane"
            );
        }
    }

    /// The whole async flow: spawn a real worker against a fake curl that answers after ~200 ms,
    /// poll with [`drain_ai_outcome`] while it is still in flight (the receiver must survive the
    /// empty polls), and verify the tab opens with the draft when the result lands.
    ///
    /// Serialized on a process-wide lock: it mutates the process-wide `NOOK_CURL_BIN` and one
    /// shared stub file, neither of which parallel test threads may race on.
    ///
    /// The stub is a `#!/bin/sh` script, so it is unix-only; the flow it covers is the drain, which
    /// has no platform in it.
    #[cfg(unix)]
    #[test]
    fn a_spawned_request_survives_empty_polls_and_opens_the_tab() {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let context = AiContext {
            provider: Provider::Ollama,
            model: "llama3".into(),
            base_url: "http://127.0.0.1:0".into(),
            prompt: "users".into(),
            key: None,
            conn: None,
            user: nook_ai::AiConfig::default(),
            table_contexts: Vec::new(),
            dialect: nook_core::Dialect::Sqlite,
        };
        // Point the worker's curl at a stub that delays and answers, so the spawn-frame poll is
        // still empty. Without the override the test would reach the network.
        std::env::set_var("NOOK_CURL_BIN", "/tmp/nook-test-fake-curl");
        std::fs::write(
            "/tmp/nook-test-fake-curl",
            "#!/bin/sh\nsleep 0.1\necho '{\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"SELECT 1 AS one;\"}}]}'\n",
        )
        .expect("write stub");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                "/tmp/nook-test-fake-curl",
                std::fs::Permissions::from_mode(0o755),
            )
            .expect("chmod stub");
        }

        let mut session = Session::new();
        let _ = session.add_tab(nook_engine::TabKind::Query, "query", None);
        let mut state = crate::app::UiState::default();
        state.ai_band = true;
        state.ai_request = Some(spawn_ai_request(context));

        // Poll until the outcome lands, asserting every poll that still sees the request in flight
        // keeps it — that is the property under test, and it holds for the whole interval rather
        // than depending on the shell stub finishing in a fixed number of milliseconds.
        let mut polls_in_flight = 0usize;
        for _ in 0..500 {
            drain_ai_outcome(&mut session, &mut state);
            if state.ai_request.is_none() {
                break;
            }
            polls_in_flight += 1;
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(
            polls_in_flight >= 1,
            "at least one empty poll must have kept the request alive before it landed"
        );
        assert!(state.ai_request.is_none(), "the landed request was drained");
        assert!(!state.ai_band, "the band closes when the draft lands");
        let draft = session
            .tabs
            .last()
            .map(|tab| tab.sql.text.clone())
            .unwrap_or_default();
        assert!(
            draft.contains("SELECT 1 AS one"),
            "the new tab holds the drafted SQL, got: {draft:?}"
        );
    }

    #[test]
    fn a_key_typed_and_generated_is_committed_before_the_request() {
        // The click that moves the focus off the key field is answered in the *next* frame, so
        // `lost_focus()` is still false on the frame Generate runs in. The band commits the key when
        // a command is about to run, which is what keeps a typed key from missing its own request.
        let typed = AiForm {
            key: "sk-typed".into(),
            key_dirty: true,
            ..AiForm::default()
        };
        assert!(
            should_commit_key(&typed, true),
            "Generate's own frame commits the key"
        );
        assert!(
            !should_commit_key(&typed, false),
            "a merely painted frame does not — a keychain write per keystroke is a `security` process"
        );
        assert!(
            should_commit_key(
                &AiForm {
                    commit_key: true,
                    ..typed.clone()
                },
                false
            ),
            "leaving the field commits it"
        );
        assert!(
            !should_commit_key(&AiForm::default(), true),
            "an untouched field commits nothing"
        );
        assert!(
            !should_commit_key(
                &AiForm {
                    key: "  ".into(),
                    key_dirty: true,
                    ..AiForm::default()
                },
                true
            ),
            "whitespace is not a key"
        );
    }

    #[test]
    fn choosing_a_provider_clears_the_model_and_the_address() {
        let mut form = AiForm {
            provider: Provider::OpenAi,
            model: "gpt-4o".into(),
            base_url: "https://example.test/v1".into(),
            ..AiForm::default()
        };
        choose_provider(&mut form, Provider::Ollama);
        assert_eq!(form.provider, Provider::Ollama);
        assert!(
            form.model.is_empty(),
            "the model falls back to the provider's own"
        );
        assert!(form.base_url.is_empty(), "so does the address");
        assert!(form.dirty, "the change has to reach the engine");
    }

    #[test]
    fn choosing_the_provider_already_chosen_changes_nothing() {
        let mut form = AiForm {
            provider: Provider::Kimi,
            model: "moonshot-v1-8k".into(),
            ..AiForm::default()
        };
        choose_provider(&mut form, Provider::Kimi);
        assert_eq!(form.model, "moonshot-v1-8k", "a re-pick is not an edit");
        assert!(!form.dirty, "and it does not write");
    }

    #[test]
    fn only_a_provider_that_cannot_name_a_model_blocks_generate() {
        // Every provider but vLLM has a default the engine would send, so the button is offered as
        // soon as the prompt is; vLLM serves whatever it was started with and must be named.
        let mut form = AiForm {
            prompt: "users by signup week".into(),
            provider: Provider::OpenAi,
            ..AiForm::default()
        };
        assert!(form.has_model(), "the provider's own default names one");
        form.model = "  ".into();
        assert_eq!(
            form.effective_model(),
            "gpt-4o-mini",
            "whitespace is not a model"
        );
        form.provider = Provider::Vllm;
        assert!(!form.has_model(), "vLLM serves what it was started with");
        form.model = "my-org/llama-3-8b".into();
        assert!(form.has_model(), "naming one is enough");
        assert_eq!(form.effective_model(), "my-org/llama-3-8b");
    }

    #[test]
    fn a_provider_that_needs_no_key_reports_none_set() {
        let mut form = AiForm::default();
        choose_provider(&mut form, Provider::Ollama);
        assert!(!form.key_set, "there is no key to have");
        assert!(!form.provider.needs_key());
    }
}
