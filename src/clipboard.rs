use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyMethod {
    /// Wayland or X11 clipboard tool (`wl-copy`, `xclip`, `xsel`).
    System,
    /// OSC 52 sequence written to the terminal.
    Terminal,
}

/// Copies `text` to the system clipboard.
///
/// Tries `wl-copy`, then `xclip`, then `xsel`. If none of those tools are
/// available, writes an OSC 52 sequence to `/dev/tty`, which modern terminals
/// accept as a clipboard paste.
pub fn copy_to_clipboard(text: &str) -> Result<CopyMethod, String> {
    if text.is_empty() {
        return Err("Nothing to copy".to_string());
    }

    if copy_via_command("wl-copy", &[], text) {
        return Ok(CopyMethod::System);
    }
    if copy_via_command("xclip", &["-selection", "clipboard"], text) {
        return Ok(CopyMethod::System);
    }
    if copy_via_command("xsel", &["--clipboard", "--input"], text) {
        return Ok(CopyMethod::System);
    }

    copy_via_osc52(text)
}

fn copy_via_command(program: &str, args: &[&str], text: &str) -> bool {
    let mut child = match Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };

    let written = if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).is_ok()
    } else {
        false
    };

    written && matches!(child.wait(), Ok(status) if status.success())
}

fn copy_via_osc52(text: &str) -> Result<CopyMethod, String> {
    // Terminals commonly cap OSC 52 payloads around 100 KiB.
    let bytes = text.as_bytes();
    let bytes = if bytes.len() > 100_000 {
        &bytes[..100_000]
    } else {
        bytes
    };
    let payload = base64_encode(bytes);
    let sequence = format!("\x1b]52;c;{payload}\x07");

    let mut tty = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .map_err(|_| "No clipboard tool available. Install wl-copy or xclip.".to_string())?;
    tty.write_all(sequence.as_bytes())
        .map_err(|err| format!("Failed to write clipboard sequence: {err}"))?;
    let _ = tty.flush();
    Ok(CopyMethod::Terminal)
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut index = 0;
    while index + 3 <= input.len() {
        let chunk = ((input[index] as u32) << 16)
            | ((input[index + 1] as u32) << 8)
            | (input[index + 2] as u32);
        out.push(TABLE[((chunk >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((chunk >> 12) & 0x3F) as usize] as char);
        out.push(TABLE[((chunk >> 6) & 0x3F) as usize] as char);
        out.push(TABLE[(chunk & 0x3F) as usize] as char);
        index += 3;
    }
    let rest = input.len() - index;
    if rest == 1 {
        let chunk = (input[index] as u32) << 16;
        out.push(TABLE[((chunk >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((chunk >> 12) & 0x3F) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rest == 2 {
        let chunk = ((input[index] as u32) << 16) | ((input[index + 1] as u32) << 8);
        out.push(TABLE[((chunk >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((chunk >> 12) & 0x3F) as usize] as char);
        out.push(TABLE[((chunk >> 6) & 0x3F) as usize] as char);
        out.push('=');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_clipboard_is_rejected() {
        let err = copy_to_clipboard("").unwrap_err();
        assert!(err.contains("Nothing"));
    }

    #[test]
    fn base64_pads_short_inputs() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"logcat line"), "bG9nY2F0IGxpbmU=");
    }
}
