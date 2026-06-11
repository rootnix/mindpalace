# mindpalace — your global engineering wiki

One wiki (`~/.mindpalace`) accumulated by ALL coding agents (Claude Code,
Codex, ...) across ALL projects. Agents read it for context at session start
and write durable knowledge back as they work.

## Layout
- `index.md` — global index (keep one line per page)
- `projects/<slug>/` — per-project pages (`index.md` required; add
  `decisions.md`, `gotchas.md`, ... as needed) + `journal.md` (auto-dated
  quick notes via `mp log`)
- `topics/` — cross-project knowledge (aws.md, postgres.md, ...)

## Discipline
1. DIFF EDITS, never full-page rewrites — `mp edit` does exact-unique
   string replacement; untouched text stays byte-identical (prevents drift).
2. PROVENANCE — date claims: "(observed 2026-06-12)". Journal entries are
   auto-dated.
3. Durable knowledge only — decisions, gotchas, runbooks, "why", non-obvious
   constraints. NOT session logs, NOT what the code already says.
4. Fix staleness in place the moment you notice it.
5. Nothing new learned → write nothing.

This directory is its own git repo — add a private remote for backup, or a
shared team remote to use one palace together.
