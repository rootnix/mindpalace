# mindpalace

![mindpalace](readme.png)

A global engineering wiki for AI coding agents — everything your agents
learn, everywhere, all in one place.

Claude Code today, Codex tomorrow, another tool next month — each session
rediscovers context the last one already had. mindpalace gives them one
shared, durable memory: a plain-markdown wiki at `~/.mindpalace` that every
agent reads at session start and writes back to as it works.

```
~/.mindpalace/
├── index.md               # global index
├── projects/<slug>/       # per-project: index.md, journal.md, decisions.md, ...
└── topics/                # cross-project: aws.md, postgres.md, ...
```

## Install

Single static binary — no runtime dependencies (git is used for repo-aware
features). macOS, Linux, and Windows.

```sh
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/rootnix/mindpalace/main/install.sh | sh
mp init -g
```

```powershell
# Windows
irm https://raw.githubusercontent.com/rootnix/mindpalace/main/install.ps1 | iex
mp init -g
```

Prebuilt binaries: macOS (arm64/x64), Linux (x64/arm64, static musl),
Windows (x64) — the installer picks the right one and falls back to a
`cargo` source build on anything else.

`mp init -g` creates your wiki and auto-detects every installed agent tool:

| Tool | Integration |
|---|---|
| **Claude Code** | Plugin (auto): injects project context at session start, nudges write-back at stop, ships an `mp` usage skill |
| **Codex** | Marker block in `~/.codex/AGENTS.md` |
| **Gemini CLI** | Marker block in `~/.gemini/GEMINI.md` |
| **Copilot CLI** | Marker block in `~/.copilot/copilot-instructions.md` |
| **Antigravity** | Same `~/.gemini/GEMINI.md` block (deduped with Gemini) |
| **Windsurf** | Compact block in `~/.codeium/windsurf/memories/global_rules.md` (6k cap respected) |
| **Pi** | Marker block in `~/.pi/agent/AGENTS.md` |
| **Hermes** | Compact block in `~/.hermes/SOUL.md` (experimental — no official global file) |
| **Cline** | `~/Documents/Cline/Rules/mindpalace.md` |
| **Roo Code** | `~/.roo/rules/mindpalace.md` |
| **Kilo Code** | `~/.kilocode/rules/mindpalace.md` |
| **Cursor** | Per-project: `mp init --agent cursor` → `.cursor/rules/mindpalace.mdc` (no global rules file exists) |
| anything with a shell | The `mp` CLI works everywhere |

```sh
mp init -g                  # auto-detect + integrate everything installed
mp init -g --agent codex    # force one tool (creates config dirs if needed)
mp init --agent cursor      # project-scoped file, versioned with the repo
mp init --list              # support matrix + current integration status
```

Project-scoped variants (`--agent cursor|cline|roo|kilocode|windsurf`
without `-g`) write the rules file into the repo — useful when a team wants
the integration to arrive via clone instead of per-machine setup.

Re-run `mp init -g` anytime — it's idempotent (marker-block dedupe; tools
sharing one file, like Gemini and Antigravity, get a single block).
`--dry-run` shows what it would do.

## CLI

```sh
mp context [slug]            # project index + recent journal
mp jump <slug>               # pull another project's knowledge
mp search <query> [-p slug]  # grep the whole wiki, grouped by page
mp read <page>               # e.g. mp read projects/myapp/index.md
mp log "<note>"              # dated quick note to the project journal
mp edit <page> <old> <new>   # exact-unique string replacement (THE update tool)
mp write <page>              # new page from stdin
mp init                      # scaffold projects/<slug>/ for the current repo
mp upgrade                   # update mindpalace (repo + binary self-update)
```

Project slug resolves from the git root name, or a `.mindpalace-project`
marker file in the repo root.

## The discipline (why this works when "just write notes" doesn't)

Wikis maintained by LLMs rot in two specific ways: full-page regeneration
drifts content that wasn't meant to change, and unverified claims accumulate
without dates. mindpalace bakes the countermeasures into the tool:

1. **Diff edits only** — `mp edit` is exact-unique string replacement;
   `mp write` refuses to overwrite without `--force`. Untouched text stays
   byte-identical.
2. **Provenance** — journal entries are auto-dated; the convention for page
   claims is `(observed YYYY-MM-DD)`.
3. **Durable knowledge only** — decisions, gotchas, constraints, runbooks.
   Not session logs, not what the code already says.
4. **Journal is an inbox** — recurring themes get distilled into pages.
5. **Commands too, secrets never** — key operational command lines live in
   the project's `commands.md`, with credentials replaced by `[REDACTED]`
   plus a pointer to where the real value lives.
6. **When unsure, log it** — a one-line journal note beats silence; "nothing
   durable" is still an acceptable answer to the stop-nudge when truly
   nothing happened.

## Backfill — import your pre-mindpalace sessions

Knowledge from sessions that happened *before* you installed mindpalace is
still on disk (Claude Code and Codex keep full transcripts). One command
prepares it for the wiki:

```sh
cd myproject && mp backfill        # or --limit 10 for the most recent N
```

This finds every past Claude Code / Codex session for the project, strips
tool noise into compact chronological digests under
`~/.mindpalace/.backfill/<slug>/`, and prints a ready-to-paste prompt.
Paste that prompt into your agent: it reads the digests and distills the
durable knowledge into the wiki via `mp edit` / `mp log` (mp itself has no
LLM — it prepares the material, your agent does the judgment). Finish with
`mp backfill --clean`.

## Team usage — `mp share`

Run `mp share` inside a project repo to make that project's wiki shared:

```sh
cd myproject && mp share
git add .mindpalace && git commit
```

This creates `<repo>/.mindpalace/`, **versioned with the project itself** —
no extra remote, no sync daemon; the wiki travels with the repo and is
reviewable in PRs. It seeds from whatever the first sharer's global wiki
already knows about the project.

The conflict model:
- **Communal pages** (`index.md`, `decisions.md`, ...) — anyone edits via
  `mp edit` (diff edits merge cleanly; same-line collisions are ordinary,
  rare git conflicts).
- **Personal journals** (`journal/<user>.md`) — `mp log` writes only to your
  own file, so the append hot-path never conflicts.

Inside a shared repo, `mp context` / `log` / `edit` / `search` use the
in-repo store automatically (search also spans your global `topics/`).
Your global `~/.mindpalace` remains for solo projects and cross-project
topics; add a private remote to back it up.

## Requirements

git. That's it — `mp` is a single static Rust binary.

## License

Apache-2.0
