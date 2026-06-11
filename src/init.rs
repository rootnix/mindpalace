use crate::paths::{git_root, install_dir, resolve_slug, root};
use crate::util::{die, home, run, which};
use std::path::PathBuf;

pub const MP_BLOCK: &str = "

<!-- MINDPALACE_START -->
## mindpalace — global engineering wiki

One cross-tool wiki at `~/.mindpalace` (CLI: `mp`), shared by all your
coding agents — read it for context, write durable knowledge back.

- Session start / new task: run `mp context` (project pages + recent
  journal). For anything beyond the current repo, `mp search <query>`.
- After substantial work: record durable knowledge — `mp log \"<note>\"`
  for dated quick notes, `mp edit <page> <old> <new>` for page updates
  (exact-unique string replacement — NEVER rewrite whole pages), and
  `mp write topics/<name>.md` for new cross-project pages.
- Discipline: diff edits only; date claims \"(observed YYYY-MM-DD)\"; fix
  stale content in place; the journal is an inbox — distill recurring
  themes into pages; nothing durable learned → write nothing.
- If the repo contains `.mindpalace/` (a SHARED team wiki versioned with
  the repo), mp commands use it automatically: top-level pages are
  communal, `journal/<user>.md` is yours. Commit wiki changes with your
  code changes.
<!-- MINDPALACE_END -->
";

/// Compact variant for hosts with tight size budgets (Windsurf caps the whole
/// rules file at 6,000 chars; Hermes' SOUL.md is an off-label hook).
pub const MP_BLOCK_COMPACT: &str = "

<!-- MINDPALACE_START -->
mindpalace: global engineering wiki at `~/.mindpalace` (CLI: `mp`), shared
by all your coding agents. Session start: run `mp context`; beyond this
repo: `mp search <query>`. After substantial work write durable knowledge
back: `mp log \"<note>\"` (dated note), `mp edit <page> <old> <new>`
(exact-unique diff edit — never rewrite whole pages), `mp write
topics/<name>.md` (new page). Date claims \"(observed YYYY-MM-DD)\"; nothing
durable learned → write nothing. A repo containing `.mindpalace/` is a
shared team wiki — mp uses it automatically; commit it with your code.
<!-- MINDPALACE_END -->
";

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Plugin,
    Block,
    BlockCompact,
    Rules,
    Project,
}

pub struct Integration {
    pub name: &'static str,
    pub kind: Kind,
    pub detected: bool,
    pub target: Option<PathBuf>,
    pub note: String,
}

/// Registry of agent-tool integrations for `mp init -g`.
///
/// kinds: Plugin       — Claude Code plugin (hooks + skill)
///        Block        — append a marker-delimited block to a shared global
///                       instructions file (idempotent: skipped when
///                       MINDPALACE_START is already present, which also
///                       dedupes tools sharing one file, e.g. gemini +
///                       antigravity on ~/.gemini/GEMINI.md)
///        BlockCompact — same, with the compact block
///        Rules        — own mindpalace.md inside the tool's global rules
///                       directory (file is wholly ours)
///        Project      — no global mechanism; per-project file via
///                       `mp init --agent <name>`
pub fn integrations() -> Vec<Integration> {
    let h = home();
    let copilot_home: PathBuf = match std::env::var_os("COPILOT_HOME") {
        Some(v) if !v.is_empty() => v.into(),
        _ => h.join(".copilot"),
    };
    let cline_dir = [h.join("Documents/Cline/Rules"), h.join("Cline/Rules")]
        .into_iter()
        .find(|d| d.is_dir());
    vec![
        Integration {
            name: "claude",
            kind: Kind::Plugin,
            detected: which("claude").is_some(),
            target: None,
            note: "plugin: session-start injection + stop nudge + mp skill".into(),
        },
        Integration {
            name: "codex",
            kind: Kind::Block,
            detected: h.join(".codex").is_dir(),
            target: Some(h.join(".codex/AGENTS.md")),
            note: String::new(),
        },
        Integration {
            name: "gemini",
            kind: Kind::Block,
            detected: h.join(".gemini").is_dir(),
            target: Some(h.join(".gemini/GEMINI.md")),
            note: String::new(),
        },
        Integration {
            name: "antigravity",
            kind: Kind::Block,
            detected: h.join(".gemini/antigravity").is_dir(),
            target: Some(h.join(".gemini/GEMINI.md")),
            note: "reads the same global file as gemini".into(),
        },
        Integration {
            name: "copilot",
            kind: Kind::Block,
            detected: copilot_home.is_dir(),
            target: Some(copilot_home.join("copilot-instructions.md")),
            note: String::new(),
        },
        Integration {
            name: "windsurf",
            kind: Kind::BlockCompact,
            detected: h.join(".codeium/windsurf").is_dir(),
            target: Some(h.join(".codeium/windsurf/memories/global_rules.md")),
            note: "6,000-char file cap — compact block".into(),
        },
        Integration {
            name: "pi",
            kind: Kind::Block,
            detected: h.join(".pi").is_dir(),
            target: Some(h.join(".pi/agent/AGENTS.md")),
            note: String::new(),
        },
        Integration {
            name: "hermes",
            kind: Kind::BlockCompact,
            detected: h.join(".hermes").is_dir(),
            target: Some(h.join(".hermes/SOUL.md")),
            note: "experimental: no official global instructions file".into(),
        },
        Integration {
            name: "cline",
            kind: Kind::Rules,
            detected: cline_dir.is_some(),
            target: Some(
                cline_dir
                    .unwrap_or_else(|| h.join("Documents/Cline/Rules"))
                    .join("mindpalace.md"),
            ),
            note: String::new(),
        },
        Integration {
            name: "roo",
            kind: Kind::Rules,
            detected: h.join(".roo").is_dir(),
            target: Some(h.join(".roo/rules/mindpalace.md")),
            note: String::new(),
        },
        Integration {
            name: "kilocode",
            kind: Kind::Rules,
            detected: h.join(".kilocode").is_dir(),
            target: Some(h.join(".kilocode/rules/mindpalace.md")),
            note: String::new(),
        },
        Integration {
            name: "cursor",
            kind: Kind::Project,
            detected: h.join(".cursor").is_dir(),
            target: None,
            note: "no global rules file — run `mp init --agent cursor` inside each project"
                .into(),
        },
    ]
}

fn install_block(target: &std::path::Path, block: &str, dry: bool) -> String {
    // Strict read: this is read-modify-write on a file we don't own — never
    // rewrite a user's AGENTS.md with U+FFFD replacements or truncate it.
    let existing = if target.exists() {
        let Ok(bytes) = std::fs::read(target) else {
            return format!("failed: could not read {}", target.display());
        };
        match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => return format!("failed: {} is not valid UTF-8", target.display()),
        }
    } else {
        String::new()
    };
    if target.exists() && existing.contains("MINDPALACE_START") {
        return "already integrated".into();
    }
    if dry {
        return format!("[dry-run] append mindpalace block to {}", target.display());
    }
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(target, existing + block).is_err() {
        return format!("failed to write {}", target.display());
    }
    format!("appended block to {}", target.display())
}

/// Byte compare tolerating a CRLF checkout of our own file.
fn same_content(target: &std::path::Path, content: &str) -> bool {
    std::fs::read(target)
        .map(|b| {
            let s = String::from_utf8_lossy(&b).replace("\r\n", "\n");
            s == content
        })
        .unwrap_or(false)
}

fn install_rules(target: &std::path::Path, dry: bool) -> String {
    let content = format!("{}\n", MP_BLOCK.trim());
    if target.exists() && same_content(target, &content) {
        return "already integrated".into();
    }
    if dry {
        return format!("[dry-run] write {}", target.display());
    }
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(target, content).is_err() {
        return format!("failed to write {}", target.display());
    }
    format!("wrote {}", target.display())
}

fn install_claude_plugin(dry: bool) -> String {
    if which("claude").is_none() {
        return "claude CLI not on PATH — install Claude Code first".into();
    }
    let mkt = install_dir().join("integrations/claude");
    let mkt = mkt.to_string_lossy().into_owned();
    let (rc, out) = run(&["claude", "plugin", "marketplace", "add", &mkt], dry);
    if rc != 0 && out.to_lowercase().contains("already") {
        run(
            &["claude", "plugin", "marketplace", "remove", "mindpalace-local"],
            dry,
        );
        run(&["claude", "plugin", "marketplace", "add", &mkt], dry);
    }
    let (rc2, out2) = run(
        &["claude", "plugin", "install", "mindpalace@mindpalace-local"],
        dry,
    );
    if dry {
        return "[dry-run] install Claude Code plugin".into();
    }
    if rc2 == 0 || out2.to_lowercase().contains("already installed") {
        return "plugin installed".into();
    }
    format!(
        "plugin install failed: {}",
        crate::util::char_slice(&out2, 200)
    )
}

fn tilde(p: &std::path::Path) -> String {
    let h = home().to_string_lossy().into_owned();
    p.to_string_lossy().replacen(&h, "~", 1)
}

pub fn list_integrations() {
    println!("{:<12} {:<12} target", "agent", "status");
    println!("{}", "-".repeat(72));
    for it in integrations() {
        let (status, target) = match it.kind {
            Kind::Plugin => (
                if it.detected { "detected" } else { "not found" }.to_string(),
                "Claude Code plugin".to_string(),
            ),
            Kind::Project => (
                if it.detected { "detected" } else { "not found" }.to_string(),
                format!("(project-scoped) {}", it.note),
            ),
            _ => {
                let t = it.target.as_ref().unwrap();
                let status = if t.exists()
                    && crate::util::read_lossy(t)
                        .unwrap_or_default()
                        .contains("MINDPALACE")
                {
                    "integrated"
                } else if it.detected {
                    "detected"
                } else {
                    "not found"
                };
                (status.to_string(), tilde(t))
            }
        };
        println!("{:<12} {:<12} {}", it.name, status, target);
        if !it.note.is_empty() && it.kind != Kind::Project {
            println!("{:<12} {:<12}   {}", "", "", it.note);
        }
    }
}

fn init_global(dry: bool, agent: Option<&str>) {
    println!("mindpalace install dir: {}", install_dir().display());
    println!("wiki root: {}\n", root().display());

    // 1. wiki skeleton
    let r = root();
    if r.exists() && r.join("index.md").exists() {
        println!("✓ wiki exists");
    } else if dry {
        println!("  [dry-run] create wiki skeleton at {}", r.display());
    } else {
        let _ = std::fs::create_dir_all(&r);
        let _ = std::fs::create_dir_all(r.join("topics"));
        let _ = std::fs::create_dir_all(r.join("projects"));
        let tpl = install_dir().join("templates");
        for name in ["README.md", "index.md"] {
            let (src, dst) = (tpl.join(name), r.join(name));
            if src.exists() && !dst.exists() {
                let _ = std::fs::copy(&src, &dst);
            }
        }
        if !r.join(".git").exists() {
            run(&["git", "-C", &r.to_string_lossy(), "init", "-q"], false);
        }
        println!(
            "✓ wiki created at {} (its own git repo — add a private remote for backup/team sync)",
            r.display()
        );
    }

    // 2. agent integrations
    let registry = integrations();
    if let Some(a) = agent {
        if !registry.iter().any(|it| it.name == a) {
            let names: Vec<&str> = registry.iter().map(|it| it.name).collect();
            die(&format!(
                "unknown agent '{a}' — supported: {}",
                names.join(", ")
            ));
        }
    }
    for it in &registry {
        if let Some(a) = agent {
            if it.name != a {
                continue;
            }
        }
        if !it.detected && agent.is_none() {
            println!("- {}: not found — skipped", it.name);
            continue;
        }
        let msg = match it.kind {
            Kind::Plugin => install_claude_plugin(dry),
            Kind::Project => {
                println!("- {}: {}", it.name, it.note);
                continue;
            }
            Kind::Rules => install_rules(it.target.as_ref().unwrap(), dry),
            Kind::Block => install_block(it.target.as_ref().unwrap(), MP_BLOCK, dry),
            Kind::BlockCompact => {
                install_block(it.target.as_ref().unwrap(), MP_BLOCK_COMPACT, dry)
            }
        };
        let mark = if msg.contains("failed") || msg.contains("not on PATH") {
            "✗"
        } else {
            "✓"
        };
        let extra = if it.note.is_empty() {
            String::new()
        } else {
            format!(" ({})", it.note)
        };
        println!("{mark} {}: {msg}{extra}", it.name);
    }

    println!(
        "\ndone. next: cd into a project and just start working — agents will read/write the wiki. Manual: mp context | mp search | mp log"
    );
    crate::star::maybe_ask_star(dry);
}

const CURSOR_FRONTMATTER: &str = "---
description: mindpalace — global engineering wiki for coding agents
alwaysApply: true
---
";

/// Project-scoped integration files for tools without a global mechanism
/// (or when a team wants the integration versioned with the repo).
fn project_agent_file(agent: &str) -> Option<(&'static str, &'static str)> {
    match agent {
        "cursor" => Some((".cursor/rules/mindpalace.mdc", CURSOR_FRONTMATTER)),
        "cline" => Some((".clinerules/mindpalace.md", "")),
        "roo" => Some((".roo/rules/mindpalace.md", "")),
        "kilocode" => Some((".kilocode/rules/mindpalace.md", "")),
        "windsurf" => Some((".windsurf/rules/mindpalace.md", "")),
        _ => None,
    }
}

fn init_project_agent(agent: &str, dry: bool) {
    let Some((rel, prefix)) = project_agent_file(agent) else {
        die(&format!(
            "no project-scoped integration for '{agent}' — supported: cline, cursor, kilocode, roo, windsurf (others integrate globally via `mp init -g`)"
        ));
    };
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let Some(root) = git_root(&cwd) else {
        die("not inside a git repo");
    };
    let target = root.join(rel);
    let content = format!("{prefix}{}\n", MP_BLOCK.trim());
    if target.exists() && same_content(&target, &content) {
        println!("✓ {agent}: {rel} already integrated");
        return;
    }
    if dry {
        println!("  [dry-run] write {}", target.display());
        return;
    }
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&target, content).is_err() {
        die(&format!("could not write {}", target.display()));
    }
    println!("✓ {agent}: wrote {rel} — commit it with the repo");
}

pub fn init_project_scaffold(args: &[String]) {
    let slug = resolve_slug(args);
    let pdir = root().join("projects").join(&slug);
    let _ = std::fs::create_dir_all(&pdir);
    let idx = pdir.join("index.md");
    if !idx.exists() {
        let _ = std::fs::write(
            &idx,
            format!(
                "# {slug}\n\n(one-paragraph: what this project is, where it lives)\n\n## Key facts\n\n## Pages\n"
            ),
        );
        println!("created projects/{slug}/index.md — fill in the basics");
    } else {
        println!("projects/{slug}/ already exists");
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    if let Some(root) = git_root(&cwd) {
        let is_same = root
            .file_name()
            .is_some_and(|n| n.to_string_lossy() == slug);
        let marker = root.join(".mindpalace-project");
        if !is_same && !marker.exists() {
            let _ = std::fs::write(&marker, format!("{slug}\n"));
            println!("wrote {}/.mindpalace-project", root.display());
        }
    }
}

pub fn cmd_init(args: &[String]) {
    let dry = args.iter().any(|a| a == "--dry-run");
    let mut args: Vec<String> = args.iter().filter(|a| *a != "--dry-run").cloned().collect();
    let mut agent: Option<String> = None;
    if let Some(i) = args.iter().position(|a| a == "--agent") {
        if i + 1 >= args.len() {
            die("usage: mp init [-g] --agent <name>");
        }
        agent = Some(args[i + 1].clone());
        args.drain(i..=i + 1);
    }
    if args.iter().any(|a| a == "--list") {
        list_integrations();
        return;
    }
    if args.iter().any(|a| a == "-g" || a == "--global") {
        init_global(dry, agent.as_deref());
    } else if let Some(a) = agent {
        init_project_agent(&a, dry);
    } else {
        init_project_scaffold(&args);
    }
}
