//! Runtime tools (kubectl, k9s, gcloud): detect what is missing and download it into the
//! app's own folder, so users never have to open a terminal. Uses `curl` and `tar`, which ship
//! with macOS and Windows 10+.

use crate::auth::gcloud_command;
use crate::k8s::K8sEvent;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

/// Folder where the app keeps the tools it downloads itself.
pub fn tools_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
        PathBuf::from(base).join("K9s Studio").join("tools")
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join("Library/Application Support/K9s Studio/tools")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".local/share/k9s-studio/tools")
    }
}

/// Appends the tools folder (and the bundled gcloud) to PATH. Tools already installed on the
/// machine keep priority. Call once at startup, before any thread is spawned.
pub fn add_to_path() {
    let dir = tools_dir();
    let sdk_bin = dir.join("google-cloud-sdk").join("bin");
    let sep = if cfg!(windows) { ';' } else { ':' };
    let mut parts: Vec<String> = std::env::var("PATH")
        .unwrap_or_default()
        .split(sep)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
    parts.push(dir.display().to_string());
    parts.push(sdk_bin.display().to_string());
    // SAFETY: called first thing in main, before any other thread exists.
    unsafe { std::env::set_var("PATH", parts.join(&sep.to_string())) };
}

/// Names of the tools that cannot be found on PATH right now.
pub fn missing_tools() -> Vec<String> {
    let mut missing = Vec::new();
    if Command::new("kubectl").args(["version", "--client"]).output().is_err() {
        missing.push("kubectl".to_string());
    }
    if Command::new("k9s").arg("version").output().is_err() {
        missing.push("k9s".to_string());
    }
    match gcloud_command(&["--version"]).output() {
        Ok(o) if o.status.success() => {}
        _ => missing.push("gcloud".to_string()),
    }
    if Command::new("gke-gcloud-auth-plugin").arg("--version").output().is_err() {
        missing.push("gke-gcloud-auth-plugin".to_string());
    }
    missing
}

pub fn spawn_check(tx: mpsc::Sender<K8sEvent>, wake: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        let _ = tx.send(K8sEvent::ToolsStatus(missing_tools()));
        wake();
    });
}

pub fn spawn_install(
    missing: Vec<String>,
    tx: mpsc::Sender<K8sEvent>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        let say = |msg: &str| {
            let _ = tx.send(K8sEvent::ToolsProgress(msg.to_string()));
            wake();
        };
        let result = install_all(&missing, &say);
        let _ = tx.send(K8sEvent::ToolsDone(result));
        wake();
    });
}

fn install_all(missing: &[String], say: &dyn Fn(&str)) -> Result<(), String> {
    let dir = tools_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("สร้างโฟลเดอร์ {} ไม่ได้: {e}", dir.display()))?;
    let has = |name: &str| missing.iter().any(|m| m == name);

    if has("kubectl") {
        install_kubectl(&dir, say)?;
    }
    if has("k9s") {
        install_k9s(&dir, say)?;
    }
    if has("gcloud") {
        install_gcloud(&dir, say)?;
    }
    if has("gcloud") || has("gke-gcloud-auth-plugin") {
        say("กำลังติดตั้ง gke-gcloud-auth-plugin...");
        let out = gcloud_command(&["components", "install", "gke-gcloud-auth-plugin", "--quiet"])
            .output()
            .map_err(|e| format!("เรียก gcloud ไม่สำเร็จ: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "ติดตั้ง gke-gcloud-auth-plugin ไม่สำเร็จ: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
    }
    Ok(())
}

fn arch() -> &'static str {
    if cfg!(target_arch = "aarch64") { "arm64" } else { "amd64" }
}

fn install_kubectl(dir: &Path, say: &dyn Fn(&str)) -> Result<(), String> {
    say("กำลังโหลด kubectl...");
    let version = curl_text("https://dl.k8s.io/release/stable.txt")?;
    let (os, exe) = if cfg!(windows) {
        ("windows", "kubectl.exe")
    } else if cfg!(target_os = "macos") {
        ("darwin", "kubectl")
    } else {
        ("linux", "kubectl")
    };
    let url = format!("https://dl.k8s.io/release/{version}/bin/{os}/{}/{exe}", arch());
    let dest = dir.join(exe);
    download(&url, &dest)?;
    make_executable(&dest)
}

fn install_k9s(dir: &Path, say: &dyn Fn(&str)) -> Result<(), String> {
    say("กำลังโหลด k9s...");
    let (os, ext, exe) = if cfg!(windows) {
        ("Windows", "zip", "k9s.exe")
    } else if cfg!(target_os = "macos") {
        ("Darwin", "tar.gz", "k9s")
    } else {
        ("Linux", "tar.gz", "k9s")
    };
    let url = format!(
        "https://github.com/derailed/k9s/releases/latest/download/k9s_{os}_{}.{ext}",
        arch()
    );
    let archive = dir.join(format!("k9s-download.{ext}"));
    let tmp = dir.join("_k9s_tmp");
    download(&url, &archive)?;
    extract(&archive, &tmp)?;
    let dest = dir.join(exe);
    let _ = std::fs::remove_file(&dest);
    std::fs::rename(tmp.join(exe), &dest)
        .or_else(|_| std::fs::copy(tmp.join(exe), &dest).map(|_| ()))
        .map_err(|e| format!("ติดตั้ง k9s ไม่สำเร็จ: {e}"))?;
    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_file(&archive);
    make_executable(&dest)
}

fn install_gcloud(dir: &Path, say: &dyn Fn(&str)) -> Result<(), String> {
    say("กำลังโหลด gcloud (ไฟล์ใหญ่ อาจใช้เวลาสักพัก)...");
    let base = "https://dl.google.com/dl/cloudsdk/channels/rapid/downloads";
    let (file, ext) = if cfg!(windows) {
        ("google-cloud-cli-windows-x86_64.zip".to_string(), "zip")
    } else if cfg!(target_os = "macos") {
        let a = if cfg!(target_arch = "aarch64") { "arm" } else { "x86_64" };
        (format!("google-cloud-cli-darwin-{a}.tar.gz"), "tar.gz")
    } else {
        ("google-cloud-cli-linux-x86_64.tar.gz".to_string(), "tar.gz")
    };
    let archive = dir.join(format!("gcloud-download.{ext}"));
    download(&format!("{base}/{file}"), &archive)?;
    say("กำลังแตกไฟล์ gcloud...");
    extract(&archive, dir)?;
    let _ = std::fs::remove_file(&archive);

    say("กำลังตั้งค่า gcloud...");
    let sdk = dir.join("google-cloud-sdk");
    let flags = [
        "--quiet",
        "--usage-reporting=false",
        "--path-update=false",
        "--command-completion=false",
    ];
    let out = if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", "install.bat"])
            .args(flags)
            .current_dir(&sdk)
            .output()
    } else {
        Command::new("sh").arg(sdk.join("install.sh")).args(flags).output()
    }
    .map_err(|e| format!("ตั้งค่า gcloud ไม่สำเร็จ: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ตั้งค่า gcloud ไม่สำเร็จ: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

/// Downloads `url` to `dest` with curl (follows redirects, retries).
pub fn download(url: &str, dest: &Path) -> Result<(), String> {
    download_with_headers(url, dest, &[])
}

/// Like [`download`], sending extra `Name: value` request headers (e.g. a GitHub token).
pub fn download_with_headers(url: &str, dest: &Path, headers: &[String]) -> Result<(), String> {
    let mut cmd = Command::new("curl");
    cmd.args(["-fL", "--retry", "3"]);
    for h in headers {
        cmd.arg("-H").arg(h);
    }
    let out = cmd
        .arg("-o")
        .arg(dest)
        .arg(url)
        .output()
        .map_err(|e| format!("เรียก curl ไม่สำเร็จ: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "ดาวน์โหลดไม่สำเร็จ: {url}\n{}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

fn curl_text(url: &str) -> Result<String, String> {
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "30", url])
        .output()
        .map_err(|e| format!("เรียก curl ไม่สำเร็จ: {e}"))?;
    if !out.status.success() {
        return Err(format!("เปิด {url} ไม่ได้"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| format!("สร้างโฟลเดอร์ไม่ได้: {e}"))?;
    let out = Command::new("tar")
        .arg("-xf")
        .arg(archive)
        .arg("-C")
        .arg(dest)
        .output()
        .map_err(|e| format!("เรียก tar ไม่สำเร็จ: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "แตกไฟล์ไม่สำเร็จ: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("ตั้งสิทธิ์รันไฟล์ไม่ได้: {e}"))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}
