#!/bin/bash
# Stop: once per session, if the repo has uncommitted changes and the wiki was
# never touched, nudge the agent to record durable knowledge before stopping.
INPUT="$(cat)"
python3 - "$INPUT" <<'PY'
import json, os, subprocess, sys, time
from pathlib import Path

try:
    data = json.loads(sys.argv[1])
except Exception:
    sys.exit(0)
if data.get("stop_hook_active"):
    sys.exit(0)
sid = data.get("session_id") or "unknown"
cwd = data.get("cwd") or os.getcwd()

root = Path(os.environ.get("MINDPALACE_ROOT", Path.home() / ".mindpalace"))
marker_dir = root / ".nudged"
marker_dir.mkdir(parents=True, exist_ok=True)
marker = marker_dir / sid
if marker.exists():
    sys.exit(0)

try:
    out = subprocess.run(
        ["git", "-C", cwd, "status", "--porcelain"],
        capture_output=True, text=True, timeout=5,
    )
    dirty = bool(out.stdout.strip()) and out.returncode == 0
except Exception:
    dirty = False
if not dirty:
    sys.exit(0)

recent = False
for p in root.rglob("*.md"):
    if ".git" in p.parts:
        continue
    try:
        if time.time() - p.stat().st_mtime < 6 * 3600:
            recent = True
            break
    except OSError:
        pass
marker.write_text("")
if recent:
    sys.exit(0)
print(json.dumps({
    "decision": "block",
    "reason": (
        "mindpalace check (once per session): this session changed files but "
        "wrote nothing to the global wiki. If you learned something DURABLE — "
        "a decision, gotcha, constraint, or cross-project fact — record it "
        "now (`mp log \"<note>\"` for quick notes, `mp edit` for page "
        "updates). If nothing durable was learned, just finish your reply; "
        "you will not be asked again this session."
    ),
}))
PY
