use crate::paths::root;
use crate::util::{run, today, which};
use std::io::{BufRead, IsTerminal, Write};

const REPO: &str = "rootnix/mindpalace";

fn do_star() {
    if which("gh").is_some() {
        let (rc, _) = run(
            &["gh", "api", "-X", "PUT", &format!("user/starred/{REPO}")],
            false,
        );
        if rc == 0 {
            println!("★ starred — thank you!");
            return;
        }
    }
    let url = format!("https://github.com/{REPO}");
    let opener: &[&str] = if cfg!(target_os = "macos") {
        &["open"]
    } else if cfg!(windows) {
        &["cmd", "/c", "start", ""]
    } else {
        &["xdg-open"]
    };
    if which(opener[0]).is_some() || cfg!(windows) {
        let mut cmd: Vec<&str> = opener.to_vec();
        cmd.push(&url);
        let _ = std::process::Command::new(cmd[0]).args(&cmd[1..]).status();
        println!("opened {url} — hit the ★ button. thank you!");
    } else {
        println!("star us at {url}");
    }
}

pub fn cmd_star(_args: &[String]) {
    do_star();
}

/// One-time, interactive-only, never blocks or fails the install.
pub fn maybe_ask_star(dry: bool) {
    if dry || std::env::var_os("MP_NO_STAR").is_some() {
        return;
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return;
    }
    let marker = root().join(".star-asked");
    if marker.exists() {
        return;
    }
    if std::fs::write(&marker, format!("{}\n", today())).is_err() {
        return;
    }
    print!("\n★ Enjoying mindpalace? Star it on GitHub so others find it? [y/N] ");
    let _ = std::io::stdout().flush();
    let mut ans = String::new();
    if std::io::stdin().lock().read_line(&mut ans).is_err() {
        println!();
        return;
    }
    let ans = ans.trim().to_lowercase();
    if ans == "y" || ans == "yes" {
        do_star();
    } else {
        println!("  no problem — `mp star` anytime.");
    }
}
