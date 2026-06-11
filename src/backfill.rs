use crate::paths::{git_root, resolve_slug, root};
use crate::util::{char_slice, die, home, read_lossy};
use serde_json::Value;
use std::path::{Path, PathBuf};

const USER_MSG_CAP: usize = 1500;
const ASSISTANT_MSG_CAP: usize = 2500;
const DIGEST_CAP: usize = 60_000;

/// Remove every <open>...<close> span (non-greedy, like the Python regex).
fn strip_spans(text: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        match rest.find(open) {
            Some(start) => {
                out.push_str(&rest[..start]);
                match rest[start + open.len()..].find(close) {
                    Some(end_rel) => {
                        rest = &rest[start + open.len() + end_rel + close.len()..];
                    }
                    None => {
                        // unmatched open tag: regex would not match — keep as-is
                        out.push_str(&rest[start..]);
                        break;
                    }
                }
            }
            None => {
                out.push_str(rest);
                break;
            }
        }
    }
    out
}

fn clip(text: &str, cap: usize) -> String {
    let text = strip_spans(text, "<system-reminder>", "</system-reminder>");
    let text = strip_spans(&text, "<environment_context>", "</environment_context>");
    let text = text.trim();
    if text.chars().count() <= cap {
        text.to_string()
    } else {
        format!("{} …[truncated]", char_slice(text, cap))
    }
}

struct Turn {
    ts: String,
    role: String,
    text: String,
}

/// Turns from a Claude Code session transcript.
fn digest_claude(path: &Path) -> Vec<Turn> {
    let mut turns = Vec::new();
    let Some(content) = read_lossy(path) else {
        return turns;
    };
    for line in content.lines() {
        let Ok(o) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let typ = o.get("type").and_then(Value::as_str).unwrap_or("");
        if (typ != "user" && typ != "assistant")
            || o.get("isSidechain").and_then(Value::as_bool).unwrap_or(false)
        {
            continue;
        }
        let content = o.get("message").and_then(|m| m.get("content"));
        let texts: Vec<&str> = match content {
            Some(Value::String(s)) => vec![s.as_str()],
            Some(Value::Array(blocks)) => blocks
                .iter()
                .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|b| b.get("text").and_then(Value::as_str))
                .collect(),
            _ => continue,
        };
        let cap = if typ == "user" { USER_MSG_CAP } else { ASSISTANT_MSG_CAP };
        let joined: Vec<&str> = texts.into_iter().filter(|t| !t.is_empty()).collect();
        let text = clip(&joined.join("\n"), cap);
        if !text.is_empty() {
            let ts = o.get("timestamp").map(json_str).unwrap_or_default();
            turns.push(Turn {
                ts: char_slice(&ts, 16).to_string(),
                role: typ.to_string(),
                text,
            });
        }
    }
    turns
}

/// str(value) for the timestamp field — mirrors Python's str() coercion.
fn json_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Turns from a Codex rollout transcript.
fn digest_codex(path: &Path) -> Vec<Turn> {
    let mut turns = Vec::new();
    let Some(content) = read_lossy(path) else {
        return turns;
    };
    for line in content.lines() {
        let Ok(o) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let p = o.get("payload").cloned().unwrap_or(Value::Null);
        if o.get("type").and_then(Value::as_str) != Some("response_item")
            || p.get("type").and_then(Value::as_str) != Some("message")
        {
            continue;
        }
        let role = p.get("role").and_then(Value::as_str).unwrap_or("");
        if role != "user" && role != "assistant" {
            continue;
        }
        let texts: Vec<&str> = p
            .get("content")
            .and_then(Value::as_array)
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| {
                        matches!(
                            b.get("type").and_then(Value::as_str),
                            Some("input_text") | Some("output_text")
                        )
                    })
                    .filter_map(|b| b.get("text").and_then(Value::as_str))
                    .filter(|t| !t.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        let cap = if role == "user" { USER_MSG_CAP } else { ASSISTANT_MSG_CAP };
        let text = clip(&texts.join("\n"), cap);
        if !text.is_empty() && !text.contains("<user_instructions>") {
            let ts = o.get("timestamp").map(json_str).unwrap_or_default();
            turns.push(Turn {
                ts: char_slice(&ts, 16).to_string(),
                role: role.to_string(),
                text,
            });
        }
    }
    turns
}

fn jsonl_files_recursive(base: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![base.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "jsonl") {
                out.push(p);
            }
        }
    }
    out
}

/// First line only — codex transcripts can be tens of MB; don't slurp them
/// just to probe the session_meta header.
fn read_first_line(path: &Path) -> Option<String> {
    use std::io::BufRead;
    let f = std::fs::File::open(path).ok()?;
    let mut line = Vec::new();
    std::io::BufReader::new(f).read_until(b'\n', &mut line).ok()?;
    Some(String::from_utf8_lossy(&line).into_owned())
}

fn mtime(p: &Path) -> std::time::SystemTime {
    p.metadata()
        .and_then(|m| m.modified())
        .unwrap_or(std::time::UNIX_EPOCH)
}

/// (tool, transcript_path) for this project, oldest first by mtime.
fn find_sessions(repo: &Path) -> Vec<(&'static str, PathBuf)> {
    let mut found: Vec<(&'static str, PathBuf)> = Vec::new();
    let munged: String = repo
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let claude_dir = home().join(".claude/projects").join(munged);
    if claude_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&claude_dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_file() && p.extension().is_some_and(|x| x == "jsonl") {
                    found.push(("claude", p));
                }
            }
        }
    }
    let codex_root = home().join(".codex/sessions");
    // Separator-normalized compare: git prints C:/x on Windows while Codex
    // records the native C:\x form.
    let prefix = repo.to_string_lossy().replace('\\', "/");
    if codex_root.is_dir() {
        for f in jsonl_files_recursive(&codex_root) {
            let Some(first_line) = read_first_line(&f) else {
                continue;
            };
            let Ok(meta) = serde_json::from_str::<Value>(&first_line) else {
                continue;
            };
            let cwd = meta
                .get("payload")
                .and_then(|p| p.get("cwd"))
                .map(json_str)
                .unwrap_or_default()
                .replace('\\', "/");
            if meta.get("type").and_then(Value::as_str) == Some("session_meta")
                && (cwd == prefix || cwd.starts_with(&format!("{prefix}/")))
            {
                found.push(("codex", f));
            }
        }
    }
    found.sort_by_key(|(_, p)| mtime(p));
    found
}

/// Extract pre-mindpalace session transcripts into digests an agent can
/// distill into the wiki. mp has no LLM — it prepares the material and the
/// prompt; your agent does the judgment.
pub fn cmd_backfill(args: &[String]) {
    let slug = resolve_slug(&[]);
    let out_dir = root().join(".backfill").join(&slug);
    if args.iter().any(|a| a == "--clean") {
        if out_dir.is_dir() {
            if std::fs::remove_dir_all(&out_dir).is_err() {
                die(&format!("could not remove {}", out_dir.display()));
            }
            println!("removed {}", out_dir.display());
        } else {
            println!("nothing to clean");
        }
        return;
    }
    let mut limit = 0usize;
    if let Some(i) = args.iter().position(|a| a == "--limit") {
        limit = args
            .get(i + 1)
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| die("usage: mp backfill [--limit N] [--clean]"));
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let Some(repo) = git_root(&cwd) else {
        die("not in a git repo");
    };
    let mut sessions = find_sessions(&repo);
    if limit > 0 && sessions.len() > limit {
        sessions = sessions.split_off(sessions.len() - limit);
    }
    if sessions.is_empty() {
        die("no Claude Code / Codex transcripts found for this project");
    }

    if std::fs::create_dir_all(&out_dir).is_err() {
        die(&format!("could not create {}", out_dir.display()));
    }
    let mut written = 0u32;
    for (i, (tool, f)) in sessions.iter().enumerate() {
        let turns = if *tool == "claude" {
            digest_claude(f)
        } else {
            digest_codex(f)
        };
        if turns.is_empty() {
            continue;
        }
        let mut lines: Vec<String> = vec![
            format!(
                "# session digest — {tool} — {} → {}",
                turns[0].ts,
                turns[turns.len() - 1].ts
            ),
            format!("source: {}", f.display()),
            String::new(),
        ];
        let mut size = 0usize;
        for t in &turns {
            let block = format!("## [{}] {}\n{}\n", t.role, t.ts, t.text);
            let block_chars = block.chars().count();
            if size + block_chars > DIGEST_CAP {
                lines.push("…[digest truncated at cap]".into());
                break;
            }
            lines.push(block);
            size += block_chars;
        }
        let date = if turns[0].ts.is_empty() {
            "undated".to_string()
        } else {
            char_slice(&turns[0].ts, 10).to_string()
        };
        let name = format!("{:03}-{tool}-{date}.md", i + 1);
        if std::fs::write(out_dir.join(&name), lines.join("\n")).is_err() {
            die(&format!("could not write {}/{name}", out_dir.display()));
        }
        written += 1;
    }
    println!("{written} session digest(s) → {}\n", out_dir.display());
    println!("{}", "─".repeat(62));
    println!(
        "Paste this into your agent (Claude Code, Codex, ...):

Backfill the mindpalace wiki for project '{slug}' from pre-mindpalace
session digests at {} ({written} files, chronological).
First run `mp context` and `mp list` to see the current wiki. Then read
the digests in order and distill ONLY durable knowledge:
- decisions, constraints, gotchas, architecture rationale -> `mp edit`
  diff-edits into existing pages (or `mp write` for a genuinely new page)
- dated events worth remembering -> `mp log`, quoting the original date
- skip anything the code already says, session chatter, one-off debugging
Date every claim with (observed YYYY-MM-DD) from the digest timestamps.
When done, run `mp backfill --clean` to delete the digests.",
        out_dir.display()
    );
    println!("{}", "─".repeat(62));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_spans_basic() {
        assert_eq!(
            strip_spans("a<x>zzz</x>b<x>q</x>c", "<x>", "</x>"),
            "abc"
        );
        assert_eq!(strip_spans("a<x>zzz", "<x>", "</x>"), "a<x>zzz");
        assert_eq!(strip_spans("plain", "<x>", "</x>"), "plain");
    }

    #[test]
    fn clip_caps_by_chars() {
        let s = "한".repeat(2000);
        let c = clip(&s, 1500);
        assert!(c.ends_with("…[truncated]"));
        assert_eq!(c.chars().count(), 1500 + " …[truncated]".chars().count());
    }
}
