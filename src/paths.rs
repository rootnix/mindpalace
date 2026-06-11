use crate::util::{die, home};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn root() -> PathBuf {
    match std::env::var_os("MINDPALACE_ROOT") {
        Some(r) if !r.is_empty() => r.into(),
        _ => home().join(".mindpalace"),
    }
}

/// The mindpalace checkout/install dir holding integrations/ and templates/.
/// Installed layout: <dir>/bin/mp → exe's grandparent. Dev builds live in
/// target/{debug,release}/ — walk ancestors for the integrations/ marker.
pub fn install_dir() -> PathBuf {
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok())
        .map(strip_verbatim)
        .unwrap_or_default();
    for anc in exe.ancestors().skip(1) {
        if anc.join("integrations").is_dir() {
            return anc.to_path_buf();
        }
    }
    exe.parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

/// Windows fs::canonicalize returns `\\?\C:\...` verbatim paths, which break
/// external consumers (e.g. `claude plugin marketplace add` rejects them).
/// Strip the prefix back to a plain path (dunce-style); no-op elsewhere.
fn strip_verbatim(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        return PathBuf::from(rest.to_string());
    }
    p
}

pub fn git_root(cwd: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s))
    }
}

/// In-repo shared store: <git root>/.mindpalace, if present.
pub fn local_store_at(cwd: &Path) -> Option<PathBuf> {
    let store = git_root(cwd)?.join(".mindpalace");
    store.is_dir().then_some(store)
}

pub fn local_store() -> Option<PathBuf> {
    local_store_at(&std::env::current_dir().ok()?)
}

pub fn username() -> String {
    let raw = Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("USER").ok().filter(|s| !s.is_empty()))
        .or_else(|| std::env::var("USERNAME").ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "anon".into());
    // Replace each run of disallowed chars with a single '-', trim, lowercase.
    let mut out = String::new();
    let mut in_bad = false;
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
            out.push(c.to_ascii_lowercase());
            in_bad = false;
        } else if !in_bad {
            out.push('-');
            in_bad = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "anon".into()
    } else {
        trimmed
    }
}

pub fn resolve_slug(args: &[String]) -> String {
    if let Some(first) = args.first() {
        return first.clone();
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    if let Some(root) = git_root(&cwd) {
        let marker = root.join(".mindpalace-project");
        if marker.exists() {
            if let Some(s) = crate::util::read_lossy(&marker) {
                return s.trim().to_string();
            }
        }
        return root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
    }
    die("no slug given and not in a git repo")
}

fn valid_local_page(page: &str) -> bool {
    let rest = page.strip_prefix("journal/").unwrap_or(page);
    rest.len() >= 4
        && rest.ends_with(".md")
        && rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' '))
}

fn valid_global_page(page: &str) -> bool {
    let rest = match page
        .strip_prefix("topics/")
        .or_else(|| page.strip_prefix("projects/"))
    {
        Some(r) => r,
        None => return false,
    };
    rest.len() >= 4
        && rest.ends_with(".md")
        && rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/' | ' '))
}

pub fn safe_page(page: &str) -> PathBuf {
    let mut page = page.trim().trim_start_matches('/').to_string();
    if let Some(stripped) = page.strip_prefix(".mindpalace/") {
        // accept `mp list` output verbatim
        page = stripped.to_string();
    }
    let store = local_store();
    if let Some(store) = store {
        if !page.starts_with("topics/") && !page.starts_with("projects/") {
            // shared in-repo store: top-level pages + journal/<user>.md
            if page.contains("..") || !valid_local_page(&page) {
                die(&format!("invalid page path for the shared store: '{page}'"));
            }
            return store.join(page);
        }
    }
    if page == "index.md" || page == "README.md" {
        return root().join(page);
    }
    if page.contains("..") || !valid_global_page(&page) {
        die(&format!(
            "invalid page path: '{page}' (use topics/x.md or projects/<slug>/x.md)"
        ));
    }
    root().join(page)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_page_validation() {
        assert!(valid_local_page("index.md"));
        assert!(valid_local_page("journal/alice.md"));
        assert!(valid_local_page("a b.md"));
        assert!(!valid_local_page("a/b.md"));
        assert!(!valid_local_page(".md"));
        assert!(!valid_local_page("x.txt"));
    }

    #[test]
    fn global_page_validation() {
        assert!(valid_global_page("topics/aws.md"));
        assert!(valid_global_page("projects/lira/index.md"));
        assert!(!valid_global_page("other/x.md"));
        assert!(!valid_global_page("topics/.md"));
    }
}
