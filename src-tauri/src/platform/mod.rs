use std::{
    io::Write,
    process::{Command, Stdio},
};

pub(crate) fn reveal_in_finder(path: &str) -> Result<(), String> {
    let status = Command::new("open")
        .arg("-R")
        .arg(path)
        .status()
        .map_err(|err| format!("Failed to launch Finder: {err}"))?;
    if !status.success() {
        return Err(format!("Finder could not reveal {path}"));
    }
    Ok(())
}

pub(crate) fn choose_folder(prompt: &str) -> Result<Option<String>, String> {
    let escaped = prompt.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!("POSIX path of (choose folder with prompt \"{escaped}\")");
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|err| format!("Failed to open the folder picker: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("-128") || stderr.to_lowercase().contains("cancel") {
            return Ok(None);
        }
        return Err(format!("The folder picker failed: {}", stderr.trim()));
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return Ok(None);
    }
    Ok(Some(path.trim_end_matches('/').to_string()))
}

pub(crate) fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|err| format!("Failed to access the clipboard: {err}"))?;
    child
        .stdin
        .take()
        .ok_or_else(|| "Failed to access the clipboard input".to_string())?
        .write_all(text.as_bytes())
        .map_err(|err| format!("Failed to write to the clipboard: {err}"))?;
    let status = child
        .wait()
        .map_err(|err| format!("Failed to copy to the clipboard: {err}"))?;
    if !status.success() {
        return Err("The clipboard copy failed".to_string());
    }
    Ok(())
}

pub(crate) fn open_privacy_settings(pane: &str) -> Result<(), String> {
    let url = match pane {
        "microphone" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
        }
        "system-audio" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_AudioCapture"
        }
        _ => "x-apple.systempreferences:com.apple.preference.security",
    };
    let status = Command::new("open")
        .arg(url)
        .status()
        .map_err(|err| format!("Failed to open System Settings: {err}"))?;
    if !status.success() {
        return Err("System Settings could not be opened".to_string());
    }
    Ok(())
}
