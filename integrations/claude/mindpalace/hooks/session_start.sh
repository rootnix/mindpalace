#!/bin/bash
# SessionStart: inject the current project's wiki context.
MP="$(command -v mp || true)"
[ -x "$MP" ] || MP="$HOME/.local/share/mindpalace/bin/mp"
[ -x "$MP" ] || exit 0
CTX="$("$MP" context 2>/dev/null)"
[ -n "$CTX" ] || exit 0
echo "<mindpalace-context>"
echo "$CTX"
echo ""
echo "mindpalace = the user's global engineering wiki (~/.mindpalace), shared"
echo "across all agent tools and projects. CLI: mp search <q> | read <page> |"
echo "edit <page> <old> <new> | log <note> | write <page>. Search it when you"
echo "need context beyond this repo. Write durable knowledge back (diff edits"
echo "via 'mp edit'; quick notes via 'mp log') — decisions, gotchas,"
echo "constraints. Never full-page rewrites; date your claims."
echo "</mindpalace-context>"
