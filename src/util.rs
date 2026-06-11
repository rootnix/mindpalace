use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn die(msg: &str) -> ! {
    eprintln!("mp: {msg}");
    std::process::exit(1);
}

pub fn home() -> PathBuf {
    // Windows: USERPROFILE first — HOME is often set by unix tooling to a
    // non-profile (or POSIX-style) value; Python's Path.home() and Node's
    // os.homedir() ignore HOME there, and agent hosts write ~/.claude etc.
    // under the profile dir.
    let order: [&str; 2] = if cfg!(windows) {
        ["USERPROFILE", "HOME"]
    } else {
        ["HOME", "USERPROFILE"]
    };
    for var in order {
        if let Some(h) = std::env::var_os(var) {
            if !h.is_empty() {
                return h.into();
            }
        }
    }
    die("cannot determine the home directory (HOME / USERPROFILE unset)")
}

/// Days-since-epoch → (year, month, day). Howard Hinnant's civil algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Run a command, capture stdout+stderr. Mirrors the Python `_run` helper.
/// The program is resolved through which() so Windows .cmd/.bat shims
/// (e.g. npm-installed `claude.cmd`) spawn correctly.
pub fn run(cmd: &[&str], dry: bool) -> (i32, String) {
    if dry {
        println!("  [dry-run] {}", cmd.join(" "));
        return (0, String::new());
    }
    let program = which(cmd[0]).unwrap_or_else(|| PathBuf::from(cmd[0]));
    match Command::new(program).args(&cmd[1..]).output() {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            (o.status.code().unwrap_or(1), s.trim().to_string())
        }
        Err(e) => (1, e.to_string()),
    }
}

/// PATH lookup; on Windows also tries .exe/.cmd/.bat.
pub fn which(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    // Windows: spawnable extensions first — an extension-less file (npm sh
    // shim) cannot be executed by CreateProcess.
    let exts: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat", ""]
    } else {
        &[""]
    };
    for dir in std::env::split_paths(&paths) {
        for ext in exts {
            let cand = dir.join(format!("{name}{ext}"));
            if cand.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let ok = cand
                        .metadata()
                        .map(|m| m.permissions().mode() & 0o111 != 0)
                        .unwrap_or(false);
                    if !ok {
                        continue;
                    }
                }
                return Some(cand);
            }
        }
    }
    None
}

/// Read a file as UTF-8, replacing invalid sequences (Python errors="replace").
/// For READ-ONLY paths only — read-modify-write paths must use read_strict so
/// invalid bytes are never silently rewritten as U+FFFD.
pub fn read_lossy(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// Strict read for read-modify-write paths (Python read_text() parity).
pub fn read_strict(path: &Path, what: &str) -> String {
    let Ok(bytes) = std::fs::read(path) else {
        die(&format!("could not read {what}"));
    };
    match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => die(&format!("{what} is not valid UTF-8 — refusing to rewrite it")),
    }
}

/// All *.md files under `base`, recursive, sorted by full path string.
pub fn walk_md(base: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![base.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            // do not follow symlinked dirs (Python rglob parity; avoids
            // cycle amplification)
            let ft = e.file_type();
            if ft.as_ref().is_ok_and(|t| t.is_dir()) {
                stack.push(p);
            } else if ft.is_ok_and(|t| t.is_file())
                && p.extension().is_some_and(|x| x == "md")
            {
                out.push(p);
            }
        }
    }
    out.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
    out
}

/// *.md files directly inside `base` (non-recursive), sorted.
pub fn flat_md(base: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(base) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_file() && p.extension().is_some_and(|x| x == "md") {
                out.push(p);
            }
        }
    }
    out.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
    out
}

/// Forward-slash relative path string (Windows parity with the Python CLI).
pub fn rel_str(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// First `n` chars of `s` (Python slicing is by character, not byte).
pub fn char_slice(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_616), (2026, 6, 12));
    }

    #[test]
    fn char_slice_multibyte() {
        assert_eq!(char_slice("한글abc", 2), "한글");
        assert_eq!(char_slice("ab", 10), "ab");
    }
}
