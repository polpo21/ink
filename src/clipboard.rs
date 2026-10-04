//! System clipboard through `wl-copy`/`wl-paste` (Wayland) or `xclip` (X11).
//! Falls back to an internal clipboard when neither is available.

use std::{
    env,
    io::Write,
    process::{Command, Stdio},
};

#[derive(Default)]
pub struct Clipboard {
    text: String,
}

impl Clipboard {
    pub fn copy(&mut self, text: &str) {
        self.text = text.to_owned();
        if let Some((cmd, args)) = tool(true) {
            let _ = run_copy(cmd, args, text);
        }
    }

    pub fn paste(&self) -> String {
        tool(false)
            .and_then(|(cmd, args)| {
                Command::new(cmd)
                    .args(args)
                    .stderr(Stdio::null())
                    .output()
                    .ok()
            })
            .filter(|out| out.status.success())
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .unwrap_or_else(|| self.text.clone())
    }
}

fn tool(copy: bool) -> Option<(&'static str, &'static [&'static str])> {
    if env::var_os("WAYLAND_DISPLAY").is_some() {
        Some(if copy {
            ("wl-copy", &[])
        } else {
            ("wl-paste", &["--no-newline", "--type", "text"])
        })
    } else if env::var_os("DISPLAY").is_some() {
        Some((
            "xclip",
            if copy {
                &["-selection", "clipboard", "-i"]
            } else {
                &["-selection", "clipboard", "-o"]
            },
        ))
    } else {
        None
    }
}

fn run_copy(cmd: &str, args: &[&str], text: &str) -> std::io::Result<()> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    child.wait()?;
    Ok(())
}
