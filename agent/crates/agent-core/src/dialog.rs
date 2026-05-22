#![allow(dead_code)]
use anyhow::{bail, Context, Result};
use std::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub enum DialogAnswer {
    Yes,
    No,
}

/// Ask the user a yes/no question via a cross-platform GUI dialog.
///
/// On Linux: uses `zenity --question`, falls back to `kdialog --yesno`.
/// On Windows: uses PowerShell + System.Windows.Forms.MessageBox.
/// On macOS:  uses `osascript` display dialog.
///
/// Returns `Yes` if the user confirms, `No` otherwise.
pub fn ask(title: &str, message: &str) -> Result<DialogAnswer> {
    match std::env::consts::OS {
        "linux"   => ask_linux(title, message),
        "windows" => ask_windows(title, message),
        "macos"   => ask_macos(title, message),
        other => {
            warn!("Unsupported OS '{}'; falling back to stdout", other);
            ask_stdout(title, message)
        }
    }
}

/// Show a system notification (cross-platform).
///
/// On Linux: uses `notify-send`, falls back to `zenity --info`.
/// On Windows: uses `msg` command.
/// On macOS: uses `osascript` display notification.
pub fn notify(title: &str, message: &str) -> Result<()> {
    match std::env::consts::OS {
        "linux"   => notify_linux(title, message),
        "windows" => notify_windows(title, message),
        "macos"   => notify_macos(title, message),
        other => {
            warn!("Unsupported OS '{}'; falling back to stdout", other);
            info!("NOTIFICATION: [{}] {}", title, message);
            Ok(())
        }
    }
}

/* ── Linux ──────────────────────────────────────────────────────────── */

fn ask_linux(title: &str, message: &str) -> Result<DialogAnswer> {
    // Try zenity first (most common on GNOME/XFCE)
    let out = Command::new("zenity")
        .args(["--question", "--title", title, "--text", message])
        .output();

    match out {
        Ok(o) => {
            if o.status.success() {
                return Ok(DialogAnswer::Yes);
            }
            if o.status.code() == Some(1) {
                return Ok(DialogAnswer::No);
            }
        }
        Err(e) => warn!("zenity failed: {}", e),
    }

    // Fallback to kdialog (KDE)
    let out = Command::new("kdialog")
        .args(["--yesno", message, "--title", title])
        .output();

    match out {
        Ok(o) => {
            if o.status.success() {
                return Ok(DialogAnswer::Yes);
            }
            if o.status.code() == Some(1) {
                return Ok(DialogAnswer::No);
            }
        }
        Err(e) => warn!("kdialog failed: {}", e),
    }

    // Final fallback
    ask_stdout(title, message)
}

fn notify_linux(title: &str, message: &str) -> Result<()> {
    match Command::new("notify-send")
        .args([title, message])
        .output()
    {
        Ok(o) if o.status.success() => Ok(()),
        Ok(_) => {
            warn!("notify-send exited with error; trying zenity fallback");
            Command::new("zenity")
                .args(["--info", "--title", title, "--text", message])
                .output()
                .context("zenity fallback failed")?;
            Ok(())
        }
        Err(e) => {
            warn!("notify-send failed: {}; using stdout fallback", e);
            info!("NOTIFICATION: [{}] {}", title, message);
            Ok(())
        }
    }
}

/* ── Windows ───────────────────────────────────────────────────────── */

fn ask_windows(title: &str, message: &str) -> Result<DialogAnswer> {
    let ps = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         $r = [System.Windows.Forms.MessageBox]::Show('{}', '{}', 'YesNo', 'Question'); \
         if ($r -eq 'Yes') {{ exit 0 }} else {{ exit 1 }}",
        escape_ps(message), escape_ps(title)
    );

    let out = Command::new("powershell")
        .args(["-Command", &ps])
        .output()
        .context("PowerShell MessageBox failed")?;

    if out.status.success() {
        Ok(DialogAnswer::Yes)
    } else {
        Ok(DialogAnswer::No)
    }
}

fn notify_windows(title: &str, message: &str) -> Result<()> {
    // msg * pops a local dialog. Works even without BurntToast.
    let out = Command::new("msg")
        .args(["*", "/TIME:5", &format!("{}: {}", title, message)])
        .output();

    match out {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            warn!("msg command error: {}", stderr);
            bail!("Windows msg notification failed: {}", stderr)
        }
        Err(e) => {
            warn!("msg command failed: {}; trying PowerShell fallback", e);
            let ps = format!(
                "Add-Type -AssemblyName System.Windows.Forms; \
                 [System.Windows.Forms.MessageBox]::Show('{}', '{}', 'OK', 'Information')",
                escape_ps(message), escape_ps(title)
            );
            Command::new("powershell")
                .args(["-Command", &ps])
                .output()
                .context("PowerShell fallback for notify failed")?;
            Ok(())
        }
    }
}

fn escape_ps(s: &str) -> String {
    s.replace("'", "''")
}

/* ── macOS ─────────────────────────────────────────────────────────── */

fn ask_macos(title: &str, message: &str) -> Result<DialogAnswer> {
    let script = format!(
        "display dialog \"{}\" with title \"{}\" buttons {{\"No\", \"Yes\"}} default button \"Yes\"",
        message.replace('"', "\\\""),
        title.replace('"', "\\\""),
    );

    let out = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .context("osascript dialog failed")?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("Yes") || out.status.success() {
        Ok(DialogAnswer::Yes)
    } else {
        Ok(DialogAnswer::No)
    }
}

fn notify_macos(title: &str, message: &str) -> Result<()> {
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        message.replace('"', "\\\""),
        title.replace('"', "\\\""),
    );

    Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .context("osascript notification failed")?;

    Ok(())
}

/* ── Fallback ───────────────────────────────────────────────────────── */

fn ask_stdout(title: &str, message: &str) -> Result<DialogAnswer> {
    println!("\n=== {} ===", title);
    println!("{}", message);
    println!("[Y/N]? ");

    use std::io::{self, BufRead, Write};
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("[Y/N]? ");
        let _ = stdout.flush();
        let mut line = String::new();
        let _ = stdin.lock().read_line(&mut line);
        match line.trim().to_lowercase().as_str() {
            "y" | "yes" => return Ok(DialogAnswer::Yes),
            "n" | "no"  => return Ok(DialogAnswer::No),
            _ => println!("Please enter Y or N"),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_ps_empty() {
        assert_eq!(escape_ps(""), "");
    }

    #[test]
    fn test_escape_ps_no_quotes() {
        assert_eq!(
            escape_ps("Hello World"),
            "Hello World"
        );
    }

    #[test]
    fn test_escape_ps_single_quote() {
        assert_eq!(
            escape_ps("It's working"),
            "It''s working"
        );
    }

    #[test]
    fn test_escape_ps_multiple_quotes() {
        assert_eq!(
            escape_ps("Don't 'stop' me now"),
            "Don''t ''stop'' me now"
        );
    }

    #[test]
    fn test_dialog_answer_yes() {
        let ans = DialogAnswer::Yes;
        assert!(matches!(ans, DialogAnswer::Yes));
    }

    #[test]
    fn test_dialog_answer_no() {
        let ans = DialogAnswer::No;
        assert!(matches!(ans, DialogAnswer::No));
    }

    #[test]
    fn test_dialog_answer_clone() {
        let a = DialogAnswer::Yes;
        let b = a.clone();
        assert!(matches!(b, DialogAnswer::Yes));
    }

    #[test]
    fn test_escape_ps_special_chars() {
        assert_eq!(
            escape_ps("Device 'AINMS' Agent"),
            "Device ''AINMS'' Agent"
        );
    }

    #[test]
    fn test_escape_ps_long_string() {
        let input = "Lorem 'ipsum' dolor 'sit' amet";
        let expected = "Lorem ''ipsum'' dolor ''sit'' amet";
        assert_eq!(escape_ps(input), expected);
    }

    #[test]
    fn test_dialog_answer_debug() {
        let yes = DialogAnswer::Yes;
        let no = DialogAnswer::No;
        assert!(format!("{:?}", yes).contains("Yes"));
        assert!(format!("{:?}", no).contains("No"));
    }
}