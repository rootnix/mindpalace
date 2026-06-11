use crate::paths::{local_store, resolve_slug, root, safe_page, username};
use crate::util::{char_slice, die, flat_md, read_lossy, rel_str, today, walk_md};
use std::io::Read;
use std::path::Path;

pub fn cmd_project(_args: &[String]) {
    println!("{}", resolve_slug(&[]));
}

/// Build the `mp context` output (also used by `mp hook session-start`).
pub fn context_output(args: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let store = if args.is_empty() { local_store() } else { None };
    if let Some(store) = store {
        let idx = store.join("index.md");
        if idx.exists() {
            out.push("## mindpalace (shared, in-repo): .mindpalace/index.md".into());
            let text = read_lossy(&idx).unwrap_or_default();
            out.push(char_slice(text.trim(), 3000).to_string());
        }
        let pages: Vec<String> = flat_md(&store)
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .filter(|n| n != "index.md" && n != "README.md")
            .collect();
        if !pages.is_empty() {
            out.push("\n## mindpalace: other pages — read with `mp read <page>`".into());
            out.push(pages.join(", "));
        }
        let mut entries: Vec<String> = Vec::new();
        for j in flat_md(&store.join("journal")) {
            let who = j
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if let Some(text) = read_lossy(&j) {
                entries.extend(
                    text.lines()
                        .filter(|l| l.starts_with("- "))
                        .map(|l| format!("{l} [{who}]")),
                );
            }
        }
        if !entries.is_empty() {
            entries.sort(); // dated entries sort chronologically
            out.push("\n## mindpalace: recent journal (all members)".into());
            let tail = &entries[entries.len().saturating_sub(8)..];
            out.push(tail.join("\n"));
        }
        return out;
    }
    let slug = resolve_slug(args);
    let pdir = root().join("projects").join(&slug);
    if !pdir.exists() {
        out.push(format!(
            "(mindpalace: no wiki yet for project '{slug}' — `mp init` to start one)"
        ));
        return out;
    }
    let idx = pdir.join("index.md");
    if idx.exists() {
        out.push(format!("## mindpalace: projects/{slug}/index.md"));
        let text = read_lossy(&idx).unwrap_or_default();
        out.push(char_slice(text.trim(), 3000).to_string());
    }
    let pages: Vec<String> = flat_md(&pdir)
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .filter(|n| n != "index.md" && n != "journal.md")
        .collect();
    if !pages.is_empty() {
        out.push(format!(
            "\n## mindpalace: other pages — read with `mp read projects/{slug}/<page>`"
        ));
        out.push(pages.join(", "));
    }
    let journal = pdir.join("journal.md");
    if let Some(text) = read_lossy(&journal) {
        let entries: Vec<&str> = text.lines().filter(|l| l.starts_with("- ")).collect();
        if !entries.is_empty() {
            out.push("\n## mindpalace: recent journal".into());
            let tail = &entries[entries.len().saturating_sub(6)..];
            out.push(tail.join("\n"));
        }
    }
    out
}

pub fn cmd_context(args: &[String]) {
    for line in context_output(args) {
        println!("{line}");
    }
}

pub fn cmd_search(args: &[String]) {
    let mut args: Vec<String> = args.to_vec();
    let mut proj: Option<String> = None;
    if let Some(i) = args.iter().position(|a| a == "-p") {
        if i + 1 >= args.len() {
            die("usage: mp search <query...> [-p slug]");
        }
        proj = Some(args[i + 1].clone());
        args.drain(i..=i + 1);
    }
    if args.is_empty() {
        die("usage: mp search <query...> [-p slug]");
    }
    let query = args.join(" ");
    let base = match &proj {
        Some(p) => root().join("projects").join(p),
        None => root(),
    };
    // (base, root_for_rel, prefix)
    let mut targets: Vec<(std::path::PathBuf, std::path::PathBuf, String)> = Vec::new();
    if proj.is_none() {
        if let Some(store) = local_store() {
            targets.push((store.clone(), store, ".mindpalace/".into()));
        }
    }
    if base.exists() {
        targets.push((base, root(), String::new()));
    }
    if targets.is_empty() {
        die("wiki not initialized — run `mp init -g`");
    }
    let terms: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();
    // hits preserve discovery order (Python dict insertion order)
    let mut hits: Vec<(String, Vec<String>)> = Vec::new();
    let mut seen: std::collections::HashSet<std::path::PathBuf> = Default::default();
    for (b, rroot, prefix) in &targets {
        for path in walk_md(b) {
            if !seen.insert(path.clone()) {
                continue;
            }
            let rel = format!("{prefix}{}", rel_str(&path, rroot));
            if path.to_string_lossy().replace('\\', "/").contains("/.git/")
                || rel.starts_with(".git/")
            {
                continue;
            }
            let Some(text) = read_lossy(&path) else {
                continue;
            };
            let mut matched: Vec<String> = Vec::new();
            for (n, line) in text.lines().enumerate() {
                let low = line.to_lowercase();
                if terms.iter().any(|t| low.contains(t)) {
                    matched.push(format!("  {}: {}", n + 1, char_slice(line.trim(), 160)));
                }
            }
            if !matched.is_empty() {
                hits.push((rel, matched));
            }
        }
    }
    if hits.is_empty() {
        println!("no hits for '{query}'");
        return;
    }
    hits.sort_by_key(|(_, lines)| std::cmp::Reverse(lines.len()));
    for (rel, lines) in hits.iter().take(10) {
        println!("{rel} ({} hit(s))", lines.len());
        println!("{}", lines[..lines.len().min(5)].join("\n"));
    }
}

pub fn cmd_list(args: &[String]) {
    if args.is_empty() {
        if let Some(store) = local_store() {
            for path in walk_md(&store) {
                println!(".mindpalace/{}", rel_str(&path, &store));
            }
        }
    }
    let base = match args.first() {
        Some(slug) => root().join("projects").join(slug),
        None => root(),
    };
    if base.exists() {
        for path in walk_md(&base) {
            let rel = rel_str(&path, &root());
            if !rel.starts_with(".git/") {
                println!("{rel}");
            }
        }
    }
}

pub fn cmd_read(args: &[String]) {
    let Some(page) = args.first() else {
        die("usage: mp read <page>");
    };
    let path = safe_page(page);
    if !path.exists() {
        die(&format!("not found: {page}"));
    }
    let Some(text) = read_lossy(&path) else {
        die(&format!("could not read {page}"));
    };
    println!("{text}");
}

pub fn cmd_write(args: &[String]) {
    let force = args.iter().any(|a| a == "--force");
    let args: Vec<&String> = args.iter().filter(|a| *a != "--force").collect();
    let Some(page) = args.first() else {
        die("usage: mp write <page> [--force] < content.md");
    };
    let path = safe_page(page);
    if path.exists() && !force {
        die(&format!(
            "{page} exists — use `mp edit` (diff) or --force for a deliberate rewrite"
        ));
    }
    let mut content = String::new();
    if std::io::stdin().read_to_string(&mut content).is_err() {
        die("could not read stdin");
    }
    if content.trim().is_empty() {
        die("empty content on stdin");
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&path, &content).is_err() {
        die(&format!("could not write {page}"));
    }
    println!("wrote {page} ({} chars)", content.chars().count());
}

pub fn cmd_edit(args: &[String]) {
    if args.len() != 3 {
        die("usage: mp edit <page> <old_string> <new_string>");
    }
    let (page, old, new) = (&args[0], &args[1], &args[2]);
    let path = safe_page(page);
    if !path.exists() {
        die(&format!("not found: {page}"));
    }
    let mut text = crate::util::read_strict(&path, page);
    // CRLF tolerance: a git autocrlf checkout (shared in-repo stores on
    // Windows) yields CRLF pages, while agents pass LF old_strings. Python's
    // universal newlines matched these transparently — normalize to LF.
    if text.contains('\r') && !old.contains('\r') {
        text = text.replace("\r\n", "\n");
    }
    let count = text.matches(old.as_str()).count();
    if count == 0 {
        die("old_string not found — copy the exact current text (whitespace matters)");
    }
    if count > 1 {
        die(&format!(
            "old_string appears {count} times — add surrounding context to make it unique"
        ));
    }
    if std::fs::write(&path, text.replacen(old.as_str(), new, 1)).is_err() {
        die(&format!("could not write {page}"));
    }
    println!("edited {page}");
}

fn append(path: &Path, line: &str) {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap_or_else(|_| die(&format!("could not open {}", path.display())));
    let _ = f.write_all(line.as_bytes());
}

pub fn cmd_log(args: &[String]) {
    if args.is_empty() {
        die("usage: mp log [slug] <note...>");
    }
    if let Some(store) = local_store() {
        let note = args.join(" ");
        let jdir = store.join("journal");
        let _ = std::fs::create_dir_all(&jdir);
        let user = username();
        let journal = jdir.join(format!("{user}.md"));
        if !journal.exists() {
            let _ = std::fs::write(
                &journal,
                format!(
                    "# journal — {user}\n\nDated quick notes; distill durable ones into the shared pages.\n\n"
                ),
            );
        }
        append(&journal, &format!("- {}: {note}\n", today()));
        println!("logged to .mindpalace/journal/{user}.md (commit with your changes)");
        return;
    }
    let (slug, note) = if root().join("projects").join(&args[0]).exists() && args.len() > 1 {
        (args[0].clone(), args[1..].join(" "))
    } else {
        (resolve_slug(&[]), args.join(" "))
    };
    let pdir = root().join("projects").join(&slug);
    let _ = std::fs::create_dir_all(&pdir);
    let journal = pdir.join("journal.md");
    if !journal.exists() {
        let _ = std::fs::write(
            &journal,
            format!("# {slug} journal\n\nDated quick notes; distill durable ones into pages.\n\n"),
        );
    }
    append(&journal, &format!("- {}: {note}\n", today()));
    println!("logged to projects/{slug}/journal.md");
}
