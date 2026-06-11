---
name: mindpalace
description: The user's global cross-tool engineering wiki at ~/.mindpalace. Use when you need context beyond the current repo (other projects, infra, past decisions), when the user references past work you don't know about, or when you finish work that produced durable knowledge worth keeping (decisions, gotchas, constraints, runbooks). Triggers - "mindpalace", "위키에 적어", "전에 어떻게 했지", working across projects, or wrapping up substantial work.
---

# mindpalace — global engineering wiki

One wiki at `~/.mindpalace`, shared by every agent tool (Claude Code, Codex, ...)
and every project. It is the user's durable memory: read it to get context
fast, write it so the next session (any tool) starts smarter.

## CLI (always available as `mp`, or `~/.mindpalace/bin/mindpalace`)

```bash
mp context [slug]          # project index + recent journal (injected at session start)
mp search <query> [-p slug]# grep the whole wiki, grouped by page
mp list [slug]             # all pages
mp read <page>             # e.g. mp read projects/lira/index.md
mp log "<note>"            # dated quick note to the current project's journal
mp edit <page> <old> <new> # exact-unique string replacement (THE update tool)
mp write <page>            # new page from stdin (refuses overwrite without --force)
mp init-project [slug]     # scaffold projects/<slug>/
```

Layout: `projects/<slug>/` (per-project: index.md, journal.md, decisions.md,
gotchas.md, ...) and `topics/` (cross-project: aws.md, postgres.md, ...).
Project slug resolves from the git root name or a `.mindpalace-project` marker file.

## When to READ
- Task mentions another project, shared infra, or past decisions → `mp search`.
- Session start context (auto-injected) names pages — read the relevant ones
  before re-deriving what they already record.

## When to WRITE (and how)
Write only DURABLE knowledge: decisions + why, gotchas that cost time,
non-obvious constraints, runbooks, cross-project facts. Not session logs, not
what the code already says.

- Quick capture during/after work: `mp log "PG pool needs check=check_connection on Aurora failover"`
- Page updates: `mp edit` with the exact current text — NEVER rewrite a
  whole page (regeneration drifts; diffs preserve everything untouched).
- Date claims inline: `(observed 2026-06-12)`.
- Found something stale? Fix it in place immediately.
- New cross-project theme → new `topics/<name>.md` + add a line to `index.md`.
- Nothing durable learned → write nothing.

## Discipline (non-negotiable)
1. Diff edits over rewrites.
2. Provenance dates on claims.
3. Distill journal entries into pages when a theme recurs — the journal is an
   inbox, not the archive.
