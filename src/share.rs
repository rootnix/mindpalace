use crate::paths::{git_root, resolve_slug, root, username};
use crate::util::{die, flat_md, rel_str, today};

const SHARE_README: &str = "# .mindpalace — shared project wiki

This project's knowledge base, versioned WITH the project. Every team
member's coding agents read it at session start and write back as they work.

Rules:
- Pages at the top level (index.md, decisions.md, gotchas.md, ...) are
  COMMUNAL — anyone updates them via `mp edit` (exact-unique diff edits;
  never rewrite whole pages). Date claims \"(observed YYYY-MM-DD)\".
- `journal/<user>.md` is PERSONAL — `mp log` writes only to your own file,
  so the append hot-path never merge-conflicts.
- Distill recurring journal themes into the communal pages.
- Cross-project knowledge belongs in your global wiki (`topics/`), not here.
";

/// Create <git root>/.mindpalace and seed it from your global wiki.
pub fn cmd_share(_args: &[String]) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let Some(repo) = git_root(&cwd) else {
        die("not inside a git repo");
    };
    let store = repo.join(".mindpalace");
    if store.exists() {
        die(".mindpalace already exists in this repo — it is already shared");
    }
    let slug = resolve_slug(&[]);
    if std::fs::create_dir(&store).is_err() {
        die(&format!("could not create {}", store.display()));
    }
    let _ = std::fs::create_dir(store.join("journal"));
    let _ = std::fs::write(store.join("README.md"), SHARE_README);
    // LF checkout everywhere — Windows teammates with core.autocrlf=true
    // would otherwise get CRLF pages that diff edits can't match cleanly.
    let _ = std::fs::write(store.join(".gitattributes"), "* text eol=lf\n");

    let mut seeded: Vec<String> = Vec::new();
    let gdir = root().join("projects").join(&slug);
    if gdir.exists() {
        for p in flat_md(&gdir) {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let dst = if name == "journal.md" {
                store.join("journal").join(format!("{}.md", username()))
            } else {
                store.join(&name)
            };
            if std::fs::copy(&p, &dst).is_ok() {
                seeded.push(rel_str(&dst, &store));
            }
        }
        // breadcrumb: global copy goes dormant, in-repo store is canonical
        let _ = std::fs::write(
            gdir.join("index.md"),
            format!(
                "# {slug} (shared)\n\nThis project's wiki moved INTO the repo: `<repo>/.mindpalace/` (canonical since {}). `mp` commands run inside the repo use it automatically.\n",
                today()
            ),
        );
    }
    if !store.join("index.md").exists() {
        let _ = std::fs::write(
            store.join("index.md"),
            format!("# {slug}\n\n(one-paragraph: what this project is)\n\n## Key facts\n"),
        );
    }

    println!("shared store created: {}", store.display());
    if !seeded.is_empty() {
        println!("seeded from your global wiki: {}", seeded.join(", "));
    }
    println!("\nnext:");
    println!("  git -C {} add .mindpalace && git commit", repo.display());
    println!("  teammates: install mindpalace (`mp init -g`) — inside this repo,");
    println!("  mp context/log/edit automatically use the shared store.");
}
