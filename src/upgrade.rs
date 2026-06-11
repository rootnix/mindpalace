use crate::paths::install_dir;
use crate::util::{run, which};

fn release_asset() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Some("mp-macos-arm64"),
        ("macos", "x86_64") => Some("mp-macos-x64"),
        ("linux", "x86_64") => Some("mp-linux-x64"),
        ("linux", "aarch64") => Some("mp-linux-arm64"),
        ("windows", "x86_64") => Some("mp-windows-x64.exe"),
        _ => None,
    }
}

/// Update the mindpalace installation: pull the repo checkout (integrations,
/// templates, skill) and swap the running binary for the latest release.
pub fn cmd_upgrade(_args: &[String]) {
    let dir = install_dir();

    // 1. repo checkout (integrations / templates ride alongside the binary)
    if dir.join(".git").exists() {
        let (_, out) = run(
            &["git", "-C", &dir.to_string_lossy(), "pull", "--ff-only"],
            false,
        );
        println!("{}", if out.is_empty() { "up to date" } else { &out });
    }

    // 2. binary self-update from GitHub Releases
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let exe = exe.canonicalize().unwrap_or(exe);
    if exe.components().any(|c| c.as_os_str() == "target") {
        println!("dev build (cargo target dir) — skipping binary self-update");
        return;
    }
    let Some(asset) = release_asset() else {
        println!(
            "no prebuilt binary for {}-{} — rebuild from source: cargo build --release",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        return;
    };
    if which("curl").is_none() {
        println!("curl not found — cannot self-update the binary");
        return;
    }
    let url = format!("https://github.com/rootnix/mindpalace/releases/latest/download/{asset}");
    let fresh = exe.with_extension("new");
    let old = exe.with_extension("old");
    let _ = std::fs::remove_file(&old); // leftover from a previous Windows swap
    let (rc, out) = run(
        &["curl", "-fsSL", "--retry", "2", "-o", &fresh.to_string_lossy(), &url],
        false,
    );
    if rc != 0 {
        println!("binary download failed ({}) — kept the current binary", out);
        let _ = std::fs::remove_file(&fresh);
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&fresh, std::fs::Permissions::from_mode(0o755));
    }
    // Swap: rename works for a running exe on every platform (delete does not
    // on Windows, hence the .old hop).
    if std::fs::rename(&exe, &old).is_err() {
        println!("could not move the current binary aside — kept it");
        let _ = std::fs::remove_file(&fresh);
        return;
    }
    if std::fs::rename(&fresh, &exe).is_err() {
        let _ = std::fs::rename(&old, &exe); // roll back
        println!("could not install the new binary — rolled back");
        return;
    }
    let _ = std::fs::remove_file(&old); // fails on Windows while running; cleaned next time
    println!("mp binary updated to the latest release");
}
