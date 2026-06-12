//! `mp hook <event>` — agent-host hook handlers (currently Claude Code).
//!
//! Replaces the old session_start.sh / stop_nudge.sh so hooks work on
//! Windows too: the plugin's hooks.json simply runs `mp hook session-start`
//! and `mp hook stop`.

use crate::paths::{git_root, root};
use crate::util::die;
use serde_json::{json, Value};
use std::io::Read;
use std::path::Path;
use std::time::{Duration, SystemTime};

const CONTRACT: &str = "mindpalace = the user's global engineering wiki (~/.mindpalace), shared
across all agent tools and projects. CLI: mp search <q> | read <page> |
edit <page> <old> <new> | log <note> | write <page>. Search it when you
need context beyond this repo. Write durable knowledge back (diff edits
via 'mp edit'; quick notes via 'mp log') — decisions, gotchas,
constraints, and key operational command lines (project commands.md,
secrets replaced with [REDACTED]). Never full-page rewrites; date your
claims. When unsure, a one-line 'mp log' beats silence.";

/// SessionStart: inject the current project's wiki context.
/// Must never fail the hook: outside a git repo there is no project context —
/// exit 0 silently (the old session_start.sh wrapper absorbed this case).
fn session_start() {
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    if git_root(&cwd).is_none() {
        return;
    }
    let ctx = crate::wiki::context_output(&[]).join("\n");
    if ctx.trim().is_empty() {
        return;
    }
    println!("<mindpalace-context>");
    println!("{ctx}");
    println!();
    println!("{CONTRACT}");
    println!("</mindpalace-context>");
}

fn recent_md_write(dir: &Path, window: Duration) -> bool {
    let now = SystemTime::now();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        if d.file_name().is_some_and(|n| n == ".git") {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "md") {
                if let Ok(modified) = p.metadata().and_then(|m| m.modified()) {
                    if now.duration_since(modified).unwrap_or(Duration::MAX) < window {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Stop: once per session, if the repo has uncommitted changes and the wiki
/// was never touched, nudge the agent to record durable knowledge.
fn stop() {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return;
    }
    let Ok(data) = serde_json::from_str::<Value>(&input) else {
        return;
    };
    if data
        .get("stop_hook_active")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return;
    }
    let sid = data
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let cwd: std::path::PathBuf = data
        .get("cwd")
        .and_then(Value::as_str)
        .map(Into::into)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| ".".into());

    let marker_dir = root().join(".nudged");
    let _ = std::fs::create_dir_all(&marker_dir);
    let marker = marker_dir.join(&sid);
    if marker.exists() {
        return;
    }

    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&cwd)
        .output()
        .map(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false);
    if !dirty {
        return;
    }

    // Project-scoped recency: a write to ANOTHER project's pages must not
    // suppress this project's nudge (the old whole-wiki 6h scan made the
    // nudge nearly never fire). topics/ counts — cross-project knowledge
    // written this session is wiki activity too.
    let window = Duration::from_secs(2 * 3600);
    let mut scan: Vec<std::path::PathBuf> = vec![root().join("topics")];
    if let Some(top) = git_root(&cwd) {
        let store = top.join(".mindpalace");
        if store.is_dir() {
            scan.push(store);
        }
        let slug_marker = top.join(".mindpalace-project");
        let slug = std::fs::read_to_string(&slug_marker)
            .map(|s| s.trim().to_string())
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| top.file_name().map(|n| n.to_string_lossy().into_owned()));
        if let Some(slug) = slug {
            scan.push(root().join("projects").join(slug));
        }
    } else {
        scan.push(root());
    }
    let recent = scan.iter().any(|d| recent_md_write(d, window));
    let _ = std::fs::write(&marker, "");
    if recent {
        return;
    }
    let reason = "mindpalace check (once per session): this session changed files but \
wrote nothing to the global wiki. If you learned something DURABLE — \
a decision, gotcha, constraint, or cross-project fact — record it \
now (`mp log \"<note>\"` for quick notes, `mp edit` for page updates, \
key command lines with secrets [REDACTED] -> the project's commands.md). \
When unsure, a one-line log beats silence. If truly nothing was \
learned, just finish your reply; \
you will not be asked again this session.";
    println!("{}", json!({ "decision": "block", "reason": reason }));
}

pub fn cmd_hook(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("session-start") => session_start(),
        Some("stop") => stop(),
        _ => die("usage: mp hook <session-start|stop>"),
    }
}
