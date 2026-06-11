# mindpalace

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

```sh
curl -fsSL https://raw.githubusercontent.com/rootnix/mindpalace/main/install.sh | sh
mp init -g
```

`mp init -g` creates your wiki and auto-detects installed agent tools:

| Tool | Integration |
|---|---|
| **Claude Code** | Plugin (auto): injects project context at session start, nudges write-back at stop (once per session), ships an `mp` usage skill |
| **Codex** | Appends the mindpalace contract to `~/.codex/AGENTS.md` |
| anything with a shell | The `mp` CLI works everywhere |

Re-run `mp init -g` anytime — it's idempotent. `--dry-run` shows what it
would do.

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
mp upgrade                   # update mindpalace itself
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
5. **Nothing new learned → write nothing** — the stop-nudge accepts "nothing
   durable" as an answer.

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

git, python3 (stdlib only — no dependencies).

## License

Apache-2.0
