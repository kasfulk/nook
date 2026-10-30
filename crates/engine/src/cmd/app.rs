//! Application-level commands: settings, tabs, snippets, help, project files.

use nook_core::{Error, Result};
use serde_json::json;

use crate::sqltext::KeywordCase;
use crate::store;
use crate::{CommandSpec, Density, Level, Session, Snippet, TabKind};

pub fn commands() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("app.settings", "Preferences", "App", get_settings)
            .params("{}")
            .help("Report the current preferences."),
        CommandSpec::new("app.setSetting", "Change a preference", "App", set_setting)
            .params(r#"{"key":"theme","value":"light"}"#)
            .help("Set one preference: theme, locale, page_size, max_rows, font_size, confirm_destructive, safe_mode, auto_commit_edits, keyword_case, show_system_objects, max_blob_bytes, ai_provider, ai_model, ai_base_url, ai_max_tokens, ai_temperature, ai_timeout."),
        CommandSpec::new("app.log", "Message log", "App", get_log)
            .params(r#"{"limit":50}"#)
            .help("Recent messages, oldest first."),
        CommandSpec::new("app.clearLog", "Clear the message log", "App", clear_log),
        CommandSpec::new("app.help", "Command help", "Help", help).params(r#"{"id":"sql.run"}"#),
        CommandSpec::new("app.about", "About Nook", "Help", about),
        CommandSpec::new("app.platform", "Platform information", "Help", platform)
            .help("Build target, UI toolkit and where the configuration lives."),
        CommandSpec::new("platform.windows", "Windows build", "App", platform_windows)
            .help("Whether this build ships a Windows binary, and the host it is running on."),
        CommandSpec::new("platform.linux", "Linux build", "App", platform_linux)
            .help("Whether this build ships a Linux binary, and the host it is running on."),
        CommandSpec::new("platform.i18n", "Language", "App", i18n)
            .params(r#"{"locale":"id"}"#)
            .help("Report the chrome's languages, or set one. Only the UI's own strings translate; an error the engine raises stays English."),
        CommandSpec::new("platform.control", "Control channel", "App", control_info)
            .help("The live JSON control channel: port, key file, methods. `--control <port>` starts it."),
        CommandSpec::new("platform.cli", "Command line", "App", cli_info)
            .help("The `nook-cli` binary: version, command count, and whether it is on PATH."),
        CommandSpec::new("app.mcp", "MCP server", "Help", mcp_info)
            .help("How an agent reaches Nook: every command is an MCP tool."),
        CommandSpec::new("tab.new", "New query tab", "App", new_query_tab)
            .params(r#"{"conn":"local","sql":"select 1"}"#)
            .shortcut("cmd+t"),
        CommandSpec::new("tab.newData", "New data tab", "App", new_data_tab)
            .params(r#"{"table":"public.users"}"#),
        CommandSpec::new("tab.list", "List tabs", "App", list_tabs),
        CommandSpec::new("tab.select", "Switch tab", "App", select_tab).params(r#"{"index":0}"#),
        CommandSpec::new("tab.close", "Close tab", "App", close_tab).shortcut("cmd+w"),
        CommandSpec::new("tab.setEditorText", "Set editor text", "App", set_editor_text)
            .params(r#"{"sql":"select 1"}"#),
        CommandSpec::new("tab.rename", "Rename the tab", "App", rename_tab),
        CommandSpec::new("snippet.list", "List snippets", "App", list_snippets),
        CommandSpec::new("snippet.save", "Save a snippet", "App", save_snippet)
            .params(r#"{"name":"Count rows","sql":"select count(*) from t"}"#),
        CommandSpec::new("snippet.delete", "Delete a snippet", "App", delete_snippet),
        CommandSpec::new("state.persist", "Snapshot the open tabs", "App", state_persist),
        CommandSpec::new("state.restore", "Restore tabs from the last snapshot", "App", state_restore),
    ]
}

fn state_persist(session: &mut Session, p: &crate::Params) -> Result<serde_json::Value> {
    // An explicit `tabs` array overrides the live tabs — the shape a script (or a test) uses to
    // write a snapshot directly. Anything else snapshots the real session.
    if let Some(tabs) = p.raw().get("tabs") {
        let snap = json!({ "tabs": tabs, "active": p.int_opt("active", 0) });
        crate::store::save_ui_state("tabs", &snap.to_string())?;
    } else {
        session.persist_tabs()?;
    }
    Ok(json!({ "ok": true }))
}

fn state_restore(session: &mut Session, _p: &crate::Params) -> Result<serde_json::Value> {
    session.restore_tabs();
    let tabs: Vec<serde_json::Value> = session
        .tabs
        .iter()
        .map(|t| {
            json!({
                "kind": format!("{:?}", t.kind),
                "title": t.title,
                "conn": t.conn.and_then(|i| session.conns.get(i)).map(|c| c.cfg.id.clone()),
                "sql": t.sql.text,
            })
        })
        .collect();
    Ok(json!({ "tabs": tabs }))
}

fn get_settings(session: &mut Session, _p: &crate::Params) -> Result<serde_json::Value> {
    let s = &session.settings;
    Ok(json!({
        "theme": s.theme,
        "locale": s.locale,
        "density": s.density.as_str(),
        "page_size": s.page_size,
        "max_rows": s.max_rows,
        "font_size": s.font_size,
        "confirm_destructive": s.confirm_destructive,
        "safe_mode": s.safe_mode,
        "auto_commit_edits": s.auto_commit_edits,
        "keyword_case": match s.keyword_case { KeywordCase::Upper => "upper", KeywordCase::Lower => "lower", KeywordCase::Preserve => "preserve" },
        "show_system_objects": s.show_system_objects,
        "max_blob_bytes": s.max_blob_bytes,
        "ai": {
            "provider": s.ai.provider.as_str(),
            "model": s.ai.model(),
            "base_url": s.ai.base(),
            "endpoint": nook_ai::endpoint(&s.ai),
            "max_tokens": s.ai.max_tokens,
            "temperature": s.ai.temperature,
            "timeout_secs": s.ai.timeout_secs,
            "key_set": matches!(crate::profiles::ai_key(s.ai.provider), Ok(Some(_))),
        },
        "config_dir": crate::profiles::config_dir().to_string_lossy(),
        "profiles": session.profiles.list().len(),
        "jobs": session.jobs.jobs.len(),
    }))
}

fn set_setting(session: &mut Session, p: &crate::Params) -> Result<serde_json::Value> {
    let key = p.str_arg("key")?;
    let value = p.str_arg("value")?;
    match key.as_str() {
        "theme" => {
            if !matches!(value.as_str(), "dark" | "light" | "system") {
                return Err(Error::config("theme must be dark, light or system"));
            }
            session.settings.theme = value.clone();
        }
        "density" => {
            let density = Density::parse(&value)
                .ok_or_else(|| Error::config("density must be compact or comfortable"))?;
            session.settings.density = density;
            // A preference, so it is written to the store and read back on the next run.
            session
                .profiles
                .set_setting("density", serde_json::json!(density.as_str()));
            session.profiles.save()?;
        }
        "page_size" => session.settings.page_size = clamp_usize(&value, 1, 100_000)?,
        "max_rows" => session.settings.max_rows = clamp_usize(&value, 1, 5_000_000)?,
        "font_size" => {
            let size: f32 = value
                .parse()
                .map_err(|_| Error::config("font_size must be a number"))?;
            session.settings.font_size = size.clamp(9.0, 28.0);
        }
        "confirm_destructive" => session.settings.confirm_destructive = truthy(&value),
        "safe_mode" => session.settings.safe_mode = truthy(&value),
        "auto_commit_edits" => session.settings.auto_commit_edits = truthy(&value),
        "show_system_objects" => session.settings.show_system_objects = truthy(&value),
        "max_blob_bytes" => {
            session.settings.max_blob_bytes = clamp_usize(&value, 0, 1 << 30)?;
        }
        "keyword_case" => {
            session.settings.keyword_case = KeywordCase::parse(&value)
                .ok_or_else(|| Error::config("keyword_case must be upper, lower or preserve"))?;
        }
        "ai_provider" => {
            let provider = nook_ai::Provider::parse(&value).ok_or_else(|| {
                Error::config(
                    "ai_provider must be openai, anthropic, deepseek, kimi, glm, ollama or vllm",
                )
            })?;
            // The model and the address belong to the provider that was chosen: a name that belongs
            // to another vendor is a request the server refuses, and a stale endpoint is a request to
            // the wrong host. Both fall back to the new provider's own defaults, which is the rule
            // `AiConfig` documents. Every surface reaches the switch through this arm, so the rule
            // cannot be left to each caller to remember.
            if session.settings.ai.provider != provider {
                session.settings.ai.model.clear();
                session.settings.ai.base_url.clear();
            }
            session.settings.ai.provider = provider;
            save_ai(session)?;
        }
        "ai_model" => {
            session.settings.ai.model = value.clone();
            save_ai(session)?;
        }
        "ai_base_url" => {
            session.settings.ai.base_url = value.clone();
            save_ai(session)?;
        }
        "ai_max_tokens" => {
            session.settings.ai.max_tokens = value
                .parse::<u32>()
                .map_err(|_| Error::config("ai_max_tokens must be a whole number"))?
                .clamp(1, 200_000);
            save_ai(session)?;
        }
        "ai_temperature" => {
            let temperature: f32 = value
                .parse()
                .map_err(|_| Error::config("ai_temperature must be a number"))?;
            session.settings.ai.temperature = temperature.clamp(0.0, 2.0);
            save_ai(session)?;
        }
        "ai_timeout" => {
            let seconds: u64 = value
                .parse()
                .map_err(|_| Error::config("ai_timeout must be a whole number of seconds"))?;
            session.settings.ai.timeout_secs = seconds.clamp(1, 3600);
            save_ai(session)?;
        }
        other => {
            return Err(Error::config(format!(
                "unknown preference '{other}'; try theme, density, page_size, max_rows, font_size, confirm_destructive, safe_mode, auto_commit_edits, keyword_case, show_system_objects, max_blob_bytes, ai_provider, ai_model, ai_base_url, ai_max_tokens, ai_temperature, ai_timeout"
            )))
        }
    }
    session.log(Level::Info, format!("{key} = {value}"));
    get_settings(session, p)
}

/// The AI settings are one row in the app's own database (`store.rs`), written as a unit so a
/// preference change cannot leave the store disagreeing with the running session.
fn save_ai(session: &mut Session) -> Result<()> {
    session.save_ai_configuration()
}

fn clamp_usize(value: &str, min: usize, max: usize) -> Result<usize> {
    let n: usize = value
        .parse()
        .map_err(|_| Error::config(format!("'{value}' is not a whole number")))?;
    Ok(n.clamp(min, max))
}

fn truthy(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn get_log(session: &mut Session, p: &crate::Params) -> Result<serde_json::Value> {
    let limit = p.int_opt("limit", 50).clamp(1, 2000) as usize;
    let start = session.messages.len().saturating_sub(limit);
    let messages: Vec<serde_json::Value> = session.messages[start..]
        .iter()
        .map(|m| {
            json!({
                "level": match m.level { Level::Info => "info", Level::Warn => "warn", Level::Error => "error", Level::Success => "ok" },
                "text": m.text,
            })
        })
        .collect();
    Ok(json!({ "messages": messages }))
}

fn clear_log(session: &mut Session, _p: &crate::Params) -> Result<serde_json::Value> {
    let n = session.messages.len();
    session.messages.clear();
    Ok(json!({ "cleared": n }))
}

fn help(_session: &mut Session, p: &crate::Params) -> Result<serde_json::Value> {
    match p.str_opt("id") {
        Some(id) => {
            let spec = crate::find(&id)
                .ok_or_else(|| Error::not_found(format!("unknown command '{id}'")))?;
            Ok(json!({
                "id": spec.id, "label": spec.label, "group": spec.group,
                "shortcut": spec.shortcut, "params": spec.params, "help": spec.help,
            }))
        }
        None => Ok(json!({
            "commands": crate::registry().iter().map(|c| json!({
                "id": c.id, "label": c.label, "group": c.group, "shortcut": c.shortcut,
            })).collect::<Vec<_>>()
        })),
    }
}

fn about(_session: &mut Session, _p: &crate::Params) -> Result<serde_json::Value> {
    Ok(json!({
        "name": "Nook",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Fast, open database administration: SQLite, PostgreSQL and MySQL in one native app.",
        "license": "MIT OR Apache-2.0",
        "commands": crate::registry().len(),
    }))
}

fn platform(_session: &mut Session, _p: &crate::Params) -> Result<serde_json::Value> {
    Ok(json!({
        "target": format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        "ui": "egui on the GPU (native window, no Electron, no browser)",
        "lanes": ["desktop app", "nook-cli", "MCP over stdio", "JSON control channel"],
        "config_dir": crate::profiles::config_dir().to_string_lossy(),
        "profiles_file": crate::profiles::profiles_path().to_string_lossy(),
        "jobs_file": crate::profiles::jobs_path().to_string_lossy(),
        "ai_db_file": crate::profiles::ai_db_path().to_string_lossy(),
        "keychain": crate::profiles::keychain_available(),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// The CI matrix builds, lints and tests the Windows and Linux targets on every push, so this build
/// supports both. The command reports the target it was compiled for as well, because "supported"
/// and "running here" are different questions.
fn platform_windows(_session: &mut Session, _p: &crate::Params) -> Result<serde_json::Value> {
    Ok(platform_support("windows"))
}

fn platform_linux(_session: &mut Session, _p: &crate::Params) -> Result<serde_json::Value> {
    Ok(platform_support("linux"))
}

fn platform_support(platform: &str) -> serde_json::Value {
    json!({
        "platform": platform,
        "supported": true,
        "host": std::env::consts::OS,
        "target": format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
    })
}

/// The chrome's languages, or a switch between them. `locale` is a preference, so it is written to
/// the profile store and read back on the next run, the way `density` is.
fn i18n(session: &mut Session, p: &crate::Params) -> Result<serde_json::Value> {
    let Some(locale) = p.get("locale").and_then(|v| v.as_str()) else {
        return Ok(json!({
            "locale": session.settings.locale,
            "languages": nook_ui_languages(),
            "scope": "the chrome; engine errors stay English",
        }));
    };
    if !nook_ui_languages().iter().any(|(code, _)| *code == locale) {
        return Err(Error::config(format!(
            "locale must be one of {}",
            nook_ui_languages()
                .iter()
                .map(|(code, _)| *code)
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    session.settings.locale = locale.to_string();
    session.profiles.set_setting("locale", json!(locale));
    session.profiles.save()?;
    Ok(json!({
        "locale": locale,
        "languages": nook_ui_languages(),
        "saved": true,
    }))
}

/// The languages the UI ships. The engine cannot depend on the UI crate, so the list lives here and
/// a UI test asserts the two agree.
fn nook_ui_languages() -> Vec<(&'static str, &'static str)> {
    vec![("en", "English"), ("id", "Bahasa Indonesia")]
}

/// What the control channel is doing right now. The channel belongs to the desktop app, so this
/// reports the port the process was started with and whether its key file is there — the answer a
/// script needs before it tries to connect.
fn control_info(session: &mut Session, _p: &crate::Params) -> Result<serde_json::Value> {
    // The desktop app records the port it bound in the session's messages; the CLI and MCP have no
    // channel, and say so rather than inventing a port.
    let bound = session
        .messages
        .iter()
        .rev()
        .find_map(|m| m.text.strip_prefix("control channel on 127.0.0.1:"))
        .map(|rest| {
            rest.split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string()
        });
    let key_dir = crate::profiles::config_dir();
    let port = bound.as_ref().and_then(|p| p.parse::<u16>().ok());
    let key_path = port.map(|p| key_dir.join(format!("control-key.{p}")));
    Ok(json!({
        "running": port.is_some(),
        "port": port,
        "key_file": key_path.as_ref().map(|p| p.to_string_lossy().to_string()),
        "key_present": key_path.as_ref().map(|p| p.is_file()).unwrap_or(false),
        "transport": "one JSON object per line over TCP on 127.0.0.1",
        "auth": "the first line carries the key from the key file",
        "methods": nook_control_methods(),
        "start": "nook --control <port>",
    }))
}

/// The control channel's methods. The UI owns the server; this is the list it accepts, kept here so
/// a caller can learn the surface from the engine alone.
fn nook_control_methods() -> Vec<&'static str> {
    vec![
        "engine.run",
        "ui.state",
        "ui.selectTab",
        "ui.selectTable",
        "ui.setEditorText",
        "ui.find",
        "ui.setTheme",
        "ui.setAiBand",
        "ui.messages",
        "ui.shot",
        "ui.commands",
        "ui.selectResult",
        "ui.selectRange",
        "ui.quickLook",
        "ui.setResultView",
        "ui.openConnection",
        "ui.setConnectionTab",
        "ui.setConnectionField",
        "ui.quickOpen",
        "ui.quit",
    ]
}

/// `nook-cli` from the engine's side: the binary that runs the same command ids from a shell.
fn cli_info(_session: &mut Session, _p: &crate::Params) -> Result<serde_json::Value> {
    let on_path = std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path).any(|dir| {
                let candidate = dir.join(if cfg!(windows) {
                    "nook-cli.exe"
                } else {
                    "nook-cli"
                });
                candidate.is_file()
            })
        })
        .unwrap_or(false);
    Ok(json!({
        "binary": "nook-cli",
        "version": env!("CARGO_PKG_VERSION"),
        "commands": crate::registry().len(),
        "on_path": on_path,
        "shares_command_ids": true,
        "entry": "nook-cli <command> or nook-cli mcp",
    }))
}

fn mcp_info(_session: &mut Session, _p: &crate::Params) -> Result<serde_json::Value> {
    Ok(json!({
        "transport": "stdio",
        "protocol": "2025-06-18",
        "entry": "nook-cli mcp",
        "tools": crate::registry().len(),
        "note": "tool names are command ids, arguments are command parameters",
        "groups": {
            "connection": registry_group("Connection"),
            "query": registry_group("Query"),
            "data": registry_group("Data"),
            "transfer": registry_group("Transfer"),
        },
    }))
}

fn registry_group(group: &str) -> usize {
    crate::registry()
        .iter()
        .filter(|c| c.group == group)
        .count()
}

fn new_query_tab(session: &mut Session, p: &crate::Params) -> Result<serde_json::Value> {
    let conn = match p.conn_id() {
        Some(id) => Some(session.conn_index(&id)?),
        None => session.active_conn,
    };
    let index = session.add_tab(TabKind::Query, "Query", conn);
    if let Some(sql) = p.str_opt("sql") {
        if let Some(tab) = session.tabs.get_mut(index) {
            tab.sql.text = sql;
        }
    }
    Ok(json!({ "tab": index }))
}

fn new_data_tab(session: &mut Session, p: &crate::Params) -> Result<serde_json::Value> {
    let conn = session.resolve_conn(p)?;
    let table_name = p.str_arg("table")?;
    let table = crate::TableRef::parse(&table_name);
    let index = session.add_tab(TabKind::Data, table.key(), Some(conn));
    super::bind_grid_table(session, index, table)?;
    super::load_grid(session, index)?;
    Ok(json!({ "tab": index }))
}

fn list_tabs(session: &mut Session, _p: &crate::Params) -> Result<serde_json::Value> {
    Ok(json!({
        "active": session.active_tab,
        "tabs": session.tabs.iter().enumerate().map(|(i, t)| json!({
            "index": i, "id": t.id, "title": t.title, "kind": format!("{:?}", t.kind).to_lowercase(),
            "conn": t.conn, "dirty": t.grid.dirty(),
        })).collect::<Vec<_>>()
    }))
}

fn select_tab(session: &mut Session, p: &crate::Params) -> Result<serde_json::Value> {
    let index = p.int_arg("index").unwrap_or(0);
    if index < 0 || index as usize >= session.tabs.len() {
        return Err(Error::not_found(format!("no tab at {index}")));
    }
    session.active_tab = Some(index as usize);
    Ok(json!({ "tab": index }))
}

fn close_tab(session: &mut Session, p: &crate::Params) -> Result<serde_json::Value> {
    let index = match p.int_opt("index", -1) {
        n if n >= 0 => n as usize,
        _ => session
            .active_tab
            .ok_or_else(|| Error::internal("no active tab"))?,
    };
    session.close_tab(index)?;
    Ok(json!({ "closed": index }))
}

fn set_editor_text(session: &mut Session, p: &crate::Params) -> Result<serde_json::Value> {
    let sql = p.str_arg("sql")?;
    let tab = session.active_tab_mut()?;
    tab.sql.text = sql.clone();
    Ok(json!({ "length": sql.chars().count() }))
}

fn rename_tab(session: &mut Session, p: &crate::Params) -> Result<serde_json::Value> {
    let title = p.str_arg("title")?;
    let tab = session.active_tab_mut()?;
    tab.title = title;
    Ok(json!({ "ok": true }))
}

fn list_snippets(session: &mut Session, _p: &crate::Params) -> Result<serde_json::Value> {
    Ok(json!({
        "snippets": session.snippets.iter().map(|s| json!({
            "name": s.name, "sql": s.sql, "tags": s.tags
        })).collect::<Vec<_>>()
    }))
}

fn save_snippet(session: &mut Session, p: &crate::Params) -> Result<serde_json::Value> {
    let name = p.str_arg("name")?;
    let sql = p
        .str_opt("sql")
        .or_else(|| session.active_tab().ok().map(|t| t.sql.text.clone()))
        .ok_or_else(|| Error::config("nothing to save"))?;
    if sql.trim().is_empty() {
        return Err(Error::config("refusing to save an empty snippet"));
    }
    let tags = p.list("tags");
    // The store is the durable twin; the vec is a cache of its rows. Capture the final value from
    // whichever arm ran — an update edits in place, so `last()` would be some other snippet.
    let saved = match session.snippets.iter_mut().find(|s| s.name == name) {
        Some(existing) => {
            existing.sql = sql;
            existing.tags = tags;
            existing.clone()
        }
        None => {
            let s = Snippet {
                name: name.clone(),
                sql,
                tags,
            };
            session.snippets.push(s.clone());
            s
        }
    };
    store::save_snippet(&saved.name, &saved.sql, &saved.tags)?;
    Ok(json!({ "snippets": session.snippets.len() }))
}

fn delete_snippet(session: &mut Session, p: &crate::Params) -> Result<serde_json::Value> {
    let name = p.str_arg("name")?;
    let before = session.snippets.len();
    session.snippets.retain(|s| s.name != name);
    if session.snippets.len() == before {
        return Err(Error::not_found(format!("no snippet named '{name}'")));
    }
    store::delete_snippet(&name)?;
    Ok(json!({ "removed": name }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::scratch_config;

    fn params(value: serde_json::Value) -> crate::Params {
        crate::Params::from_json(value).expect("params")
    }

    #[test]
    fn density_is_a_preference_that_survives_a_restart() {
        let _guard = scratch_config();
        let mut session = Session::new();
        assert_eq!(session.settings.density, crate::Density::Compact);

        let out = set_setting(
            &mut session,
            &params(json!({"key": "density", "value": "comfortable"})),
        )
        .expect("set density");
        assert_eq!(out["density"], "comfortable");

        let restarted = Session::new();
        assert_eq!(restarted.settings.density, crate::Density::Comfortable);

        let bad = set_setting(
            &mut session,
            &params(json!({"key": "density", "value": "spacious"})),
        )
        .expect_err("spacious is not a density");
        assert!(bad.to_string().contains("compact or comfortable"), "{bad}");

        let unknown = set_setting(&mut session, &params(json!({"key": "nope", "value": "1"})))
            .expect_err("unknown key");
        assert!(unknown.to_string().contains("density"), "{unknown}");
    }

    #[test]
    fn platform_commands_report_support_and_the_running_host() {
        let mut session = Session::new();
        let windows = platform_windows(&mut session, &params(json!({}))).expect("windows");
        let linux = platform_linux(&mut session, &params(json!({}))).expect("linux");

        assert_eq!(windows["platform"], "windows");
        assert_eq!(linux["platform"], "linux");
        assert_eq!(windows["supported"], true);
        assert_eq!(linux["supported"], true);
        assert_eq!(windows["host"], std::env::consts::OS);
        assert!(crate::registry().iter().any(|c| c.id == "platform.windows"));
        assert!(crate::registry().iter().any(|c| c.id == "platform.linux"));
    }
}
