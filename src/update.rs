//! In-app updates from GitHub Releases. The release assets have fixed names so the newest one
//! is always at `releases/latest/download/<ASSET>`.

use crate::k8s::K8sEvent;
use crate::tools::{download, download_with_headers};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

/// GitHub `owner/repo` that publishes the releases.
pub const REPO: &str = "Suthirach/K-HAVE-U";
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(target_os = "macos")]
const ASSET: &str = "Klar-Mac.dmg";
#[cfg(target_os = "windows")]
const ASSET: &str = "Klar-Windows-Setup.exe";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const ASSET: &str = "";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateState {
    Idle,
    Available(String),
    Installing(String),
}

fn version_parts(v: &str) -> Vec<u64> {
    v.trim()
        .trim_start_matches('v')
        .split('.')
        .map(|p| {
            p.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>()
                .unwrap_or(0)
        })
        .collect()
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    let mut a = version_parts(latest);
    let mut b = version_parts(current);
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a > b
}

/// Per-user settings file (GitHub token, "first-run setup done"). A fixed absolute path, so the
/// Windows installer can write the token here before the app ever runs.
fn settings_path() -> PathBuf {
    crate::tools::tools_dir()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("update-settings.json")
}

fn read_settings() -> Value {
    let parsed = std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
    match parsed {
        Some(v) if v.is_object() => v,
        _ => serde_json::json!({}),
    }
}

fn write_settings(v: &Value) -> Result<(), String> {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(&path, v.to_string()).map_err(|e| crate::tf!("บันทึกการตั้งค่าไม่ได้: {e}", "Could not save settings: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// GitHub token used to read releases of a private repo (env var first, then the settings file).
pub fn load_token() -> Option<String> {
    if let Ok(t) = std::env::var("KLAR_GITHUB_TOKEN") {
        let t = t.trim().to_string();
        if !t.is_empty() {
            return Some(t);
        }
    }
    read_settings()["github_token"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn save_token(token: &str) -> Result<(), String> {
    let mut v = read_settings();
    v["github_token"] = Value::String(token.to_string());
    write_settings(&v)
}

/// True once the first-run setup screen has been completed (or a token comes from the environment).
pub fn setup_done() -> bool {
    std::env::var("KLAR_GITHUB_TOKEN")
        .map(|t| !t.trim().is_empty())
        .unwrap_or(false)
        || read_settings()["setup_done"].as_bool().unwrap_or(false)
}

pub fn mark_setup_done() -> Result<(), String> {
    let mut v = read_settings();
    v["setup_done"] = Value::Bool(true);
    write_settings(&v)
}

fn api_headers(token: &str, accept: &str) -> Vec<String> {
    vec![
        format!("Authorization: Bearer {token}"),
        format!("Accept: {accept}"),
        "X-GitHub-Api-Version: 2022-11-28".to_string(),
    ]
}

/// Newest release via the GitHub API (works for private repos): (version, id of our asset).
fn api_latest(token: &str) -> Result<(String, Option<u64>), String> {
    let mut cmd = Command::new("curl");
    cmd.args(["-sSL", "--max-time", "20", "-w", "\n%{http_code}"]);
    for h in api_headers(token, "application/vnd.github+json") {
        cmd.arg("-H").arg(h);
    }
    cmd.arg(format!("https://api.github.com/repos/{REPO}/releases/latest"));
    let out = cmd.output().map_err(|e| crate::tf!("เรียก curl ไม่สำเร็จ: {e}", "Could not run curl: {e}"))?;
    let body = String::from_utf8_lossy(&out.stdout).to_string();
    let (json, code) = body.rsplit_once('\n').unwrap_or((body.as_str(), ""));
    match code.trim() {
        "200" => {}
        "401" | "403" => {
            return Err(crate::tr!("GitHub token ไม่ถูกต้อง หมดอายุ หรือไม่มีสิทธิ์อ่าน repo นี้", "GitHub token is invalid, expired, or cannot read this repo").to_string());
        }
        "404" => {
            return Err(crate::tr!("ไม่พบ Release (ยังไม่มี Release หรือ token ไม่มีสิทธิ์อ่าน repo นี้)", "No release found (none published yet, or the token cannot read this repo)").to_string());
        }
        other => return Err(crate::tf!("GitHub ตอบกลับ {other}", "GitHub answered {other}")),
    }
    let v: Value = serde_json::from_str(json).map_err(|e| crate::tf!("อ่านข้อมูล Release ไม่ได้: {e}", "Could not read the release: {e}"))?;
    let tag = v["tag_name"].as_str().unwrap_or("").trim_start_matches('v').to_string();
    let asset_id = v["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["name"].as_str() == Some(ASSET)))
        .and_then(|x| x["id"].as_u64());
    Ok((tag, asset_id))
}

/// Version of the newest GitHub Release (e.g. "0.2.0"), if there is one.
fn latest_version(token: Option<&str>) -> Result<Option<String>, String> {
    if let Some(t) = token.filter(|t| !t.is_empty()) {
        let (tag, _) = api_latest(t)?;
        return Ok(Some(tag).filter(|t| !t.is_empty()));
    }
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let out = Command::new("curl")
        .args(["-sIL", "--max-time", "15", "-o", null, "-w", "%{url_effective}"])
        .arg(format!("https://github.com/{REPO}/releases/latest"))
        .output()
        .map_err(|e| crate::tf!("เรียก curl ไม่สำเร็จ: {e}", "Could not run curl: {e}"))?;
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(url
        .split("/releases/tag/")
        .nth(1)
        .map(|t| t.trim().trim_start_matches('v').to_string())
        .filter(|t| !t.is_empty()))
}

pub fn spawn_check(
    token: Option<String>,
    tx: mpsc::Sender<K8sEvent>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        let result =
            latest_version(token.as_deref()).map(|v| v.filter(|v| is_newer(v, CURRENT_VERSION)));
        let _ = tx.send(K8sEvent::UpdateCheck(result));
        wake();
    });
}

/// Downloads the newest installer, hands over to it and quits so it can replace this app.
pub fn spawn_install(
    token: Option<String>,
    tx: mpsc::Sender<K8sEvent>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || match install_latest(token.as_deref()) {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            let _ = tx.send(K8sEvent::UpdateFailed(e));
            wake();
        }
    });
}

#[allow(dead_code)]
fn asset_url() -> String {
    format!("https://github.com/{REPO}/releases/latest/download/{ASSET}")
}

/// Downloads the release installer to `dest` (through the API when a token is set).
#[allow(dead_code)]
fn fetch_asset(dest: &Path, token: Option<&str>) -> Result<(), String> {
    match token.filter(|t| !t.is_empty()) {
        Some(t) => {
            let (_, id) = api_latest(t)?;
            let id = id.ok_or_else(|| crate::tf!("ไม่พบไฟล์ {ASSET} ใน Release ล่าสุด", "{ASSET} not found in the latest release"))?;
            let url = format!("https://api.github.com/repos/{REPO}/releases/assets/{id}");
            download_with_headers(&url, dest, &api_headers(t, "application/octet-stream"))
        }
        None => download(&asset_url(), dest),
    }
}

#[cfg(target_os = "macos")]
const MAC_UPDATE_SCRIPT: &str = r#"#!/bin/sh
# args: <pid of the running app> <dmg> <path of the installed .app>
PID="$1"; DMG="$2"; DEST="$3"
while kill -0 "$PID" 2>/dev/null; do sleep 0.5; done
MNT="$(mktemp -d)"
hdiutil attach -nobrowse -quiet -mountpoint "$MNT" "$DMG" || exit 1
rm -rf "$DEST.new"
if cp -R "$MNT/Klar.app" "$DEST.new"; then
  rm -rf "$DEST" && mv "$DEST.new" "$DEST"
fi
hdiutil detach -quiet "$MNT"
xattr -dr com.apple.quarantine "$DEST" 2>/dev/null
open "$DEST"
"#;

#[cfg(target_os = "macos")]
fn install_latest(token: Option<&str>) -> Result<(), String> {
    use std::process::Stdio;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let bundle = exe
        .ancestors()
        .find(|p| p.extension().map(|e| e == "app").unwrap_or(false))
        .map(|p| p.to_path_buf())
        .ok_or_else(|| crate::tr!("ไม่ได้เปิดจากแอปที่ติดตั้ง (โหมดพัฒนา) จึงอัปเดตอัตโนมัติไม่ได้", "Not running from an installed app (development mode), so it cannot update itself").to_string())?;
    let dir = std::env::temp_dir().join("klar-update");
    std::fs::create_dir_all(&dir).map_err(|e| crate::tf!("สร้างโฟลเดอร์ชั่วคราวไม่ได้: {e}", "Could not create a temporary folder: {e}"))?;
    let dmg = dir.join(ASSET);
    fetch_asset(&dmg, token)?;
    let script = dir.join("update.sh");
    std::fs::write(&script, MAC_UPDATE_SCRIPT).map_err(|e| crate::tf!("เขียนสคริปต์อัปเดตไม่ได้: {e}", "Could not write the update script: {e}"))?;
    Command::new("sh")
        .arg(&script)
        .arg(std::process::id().to_string())
        .arg(&dmg)
        .arg(&bundle)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| crate::tf!("เริ่มตัวอัปเดตไม่ได้: {e}", "Could not start the updater: {e}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_latest(token: Option<&str>) -> Result<(), String> {
    let setup = std::env::temp_dir().join(ASSET);
    fetch_asset(&setup, token)?;
    Command::new(&setup)
        .args(["/SILENT", "/SUPPRESSMSGBOXES", "/CLOSEAPPLICATIONS"])
        .spawn()
        .map_err(|e| crate::tf!("เริ่มตัวติดตั้งไม่ได้: {e}", "Could not start the installer: {e}"))?;
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn install_latest(token: Option<&str>) -> Result<(), String> {
    let _ = token;
    Err(crate::tr!("ระบบนี้ยังไม่รองรับการอัปเดตอัตโนมัติ", "Automatic updates aren't supported on this system").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("v1.0.0", "0.9.9"));
        assert!(is_newer("0.1.1", "0.1"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
    }
}
