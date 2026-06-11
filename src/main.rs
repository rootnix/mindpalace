//! mp — mindpalace: a global engineering wiki for AI coding agents.
//!
//! One wiki at ~/.mindpalace, shared by every coding agent (Claude Code,
//! Codex, ...) across every project. Agents read it for context at session
//! start and write durable knowledge back as they work. Single static
//! binary, no runtime dependencies (git required for repo-aware features).

mod backfill;
mod hook;
mod init;
mod paths;
mod share;
mod star;
mod upgrade;
mod util;
mod wiki;

use util::die;

const HELP: &str = "mp — mindpalace: a global engineering wiki for AI coding agents.

One wiki at ~/.mindpalace, shared by every coding agent (Claude Code, Codex,
...) across every project. Agents read it for context at session start and
write durable knowledge back as they work.

Commands:
  mp init -g [--dry-run]       global setup: create the wiki, detect installed
                               agent tools, install all integrations. Supported:
                               claude, codex, gemini, copilot, antigravity,
                               windsurf, pi, hermes, cline, roo, kilocode
  mp init -g --agent <name>    install one integration (even if not detected)
  mp init --agent <name>       project-scoped integration file, versioned with
                               the repo (cursor, cline, roo, kilocode, windsurf)
  mp init --list               support matrix + per-tool integration status
  mp init [slug]               scaffold projects/<slug>/ for the current repo
  mp project                   resolve current project slug (git root name)
  mp context [slug]            project index + recent journal (session-start injection)
  mp jump <slug>               alias for context — pull another project's knowledge
  mp search <query...> [-p slug]   grep across the wiki, grouped by page
  mp list [slug]               list pages
  mp read <page>               print a page (path relative to the wiki root)
  mp write <page> [--force]    create a page from stdin (refuses overwrite without --force)
  mp edit <page> <old> <new>   exact-unique string replacement (THE update tool)
  mp log [slug] <note...>      append a dated entry to the project journal
  mp share                     make this project's wiki SHARED: creates
                               <repo>/.mindpalace (versioned with the repo),
                               seeds it from your global wiki; communal pages
                               + per-person journals (no merge conflicts)
  mp backfill [--limit N]      extract this project's PRE-mindpalace agent
                               sessions (Claude Code, Codex) into digests +
                               a distillation prompt; --clean removes digests
  mp hook <event>              agent-host hook entry points (session-start, stop)
  mp star                      star rootnix/mindpalace on GitHub (gh or browser)
  mp upgrade                   update mindpalace (repo pull + binary self-update)

Wiki root: ~/.mindpalace (override with MINDPALACE_ROOT).";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        println!("{HELP}");
        return;
    };
    let rest = &args[1..];
    match cmd.as_str() {
        "-h" | "--help" | "help" => println!("{HELP}"),
        "init" => init::cmd_init(rest),
        "init-project" => init::init_project_scaffold(rest), // back-compat alias
        "project" => wiki::cmd_project(rest),
        "context" | "jump" => wiki::cmd_context(rest),
        "search" => wiki::cmd_search(rest),
        "list" => wiki::cmd_list(rest),
        "read" => wiki::cmd_read(rest),
        "write" => wiki::cmd_write(rest),
        "edit" => wiki::cmd_edit(rest),
        "log" => wiki::cmd_log(rest),
        "share" => share::cmd_share(rest),
        "backfill" => backfill::cmd_backfill(rest),
        "hook" => hook::cmd_hook(rest),
        "star" => star::cmd_star(rest),
        "upgrade" => upgrade::cmd_upgrade(rest),
        other => die(&format!("unknown command '{other}' — run `mp help`")),
    }
}
