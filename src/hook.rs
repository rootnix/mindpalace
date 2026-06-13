//! `mp hook <event>` — agent-host hook handlers (currently Claude Code).
//!
//! Replaces the old session_start.sh / stop_nudge.sh so hooks work on
//! Windows too: the plugin's hooks.json simply runs `mp hook session-start`
//! and `mp hook stop`.

use crate::paths::git_root;
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

/// True if HEAD was committed within `window`. A repo that commits often
/// (e.g. many small commits per session) is usually clean at Stop time, so a
/// bare `git status` dirty check would never nudge it — a recent commit is
/// just as much "work this session changed" as an uncommitted edit.
fn committed_within(cwd: &Path, window: Duration) -> bool {
    let out = std::process::Command::new("git")
        .args(["log", "-1", "--format=%ct"])
        .current_dir(cwd)
        .output();
    let Ok(out) = out else { return false };
    if !out.status.success() {
        return false;
    }
    let Ok(ts) = String::from_utf8_lossy(&out.stdout).trim().parse::<u64>() else {
        return false;
    };
    let commit = SystemTime::UNIX_EPOCH + Duration::from_secs(ts);
    SystemTime::now()
        .duration_since(commit)
        .map(|d| d < window)
        .unwrap_or(false)
}

/// Stop: every time a coding turn ends, if this session touched code, nudge
/// the agent to capture what a teammate would need. The agent — not this
/// hook — decides whether anything durable was learned; an empty turn just
/// finishes. No once-per-session marker, no recency window: the cost of a
/// redundant nudge is one self-check the agent absorbs, the cost of a missed
/// one is a teammate left without context. We bias toward capture.
///
/// `stop_hook_active` is the only loop guard: after we block, the agent's
/// follow-up Stop carries that flag and we return immediately, so the nudge
/// fires at most once per agent turn, not in a loop.
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
    let cwd: std::path::PathBuf = data
        .get("cwd")
        .and_then(Value::as_str)
        .map(Into::into)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| ".".into());

    // "Did this session do code work?" = uncommitted changes OR a commit
    // within the window. The commit clause is what keeps commit-often repos
    // from silently slipping past the nudge.
    let window = Duration::from_secs(2 * 3600);
    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&cwd)
        .output()
        .map(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false);
    if !dirty && !committed_within(&cwd, window) {
        return;
    }

    let reason = "mindpalace: this session changed code. Capture what a teammate \
inheriting this repo would need — a decision and its why, a gotcha, a \
constraint, a key command line — by appending to the EXISTING pages with \
`mp edit <page> <old> <new>` (diff-style, never a full rewrite) plus a dated \
one-line `mp log \"<note>\"`. Put operational commands in the project's \
commands.md with secrets as [REDACTED]. If everything durable from this \
turn is already recorded, just finish your reply — don't invent filler.";
    println!("{}", json!({ "decision": "block", "reason": reason }));
}

pub fn cmd_hook(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("session-start") => session_start(),
        Some("stop") => stop(),
        _ => die("usage: mp hook <session-start|stop>"),
    }
}
