//! Runtime tools (kubectl, k9s, gcloud): detect what is missing and download it into the
//! app's own folder, so users never have to open a terminal. Uses `curl` and `tar`, which ship
//! with macOS and Windows 10+.

use crate::auth::gcloud_command;
use crate::k8s::K8sEvent;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use std::sync::mpsc;

/// Folder where the app keeps the tools it downloads itself.
pub fn tools_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
        PathBuf::from(base).join("Klar").join("tools")
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join("Library/Application Support/Klar/tools")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".local/share/klar/tools")
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
        let send = |msg: &str, percent: f32| {
            let _ = tx.send(K8sEvent::ToolsProgress {
                message: msg.to_string(),
                percent,
            });
            wake();
        };
        let result = install_all(&missing, &send);
        let _ = tx.send(K8sEvent::ToolsDone(result));
        wake();
    });
}

// Share of the progress bar for each step (only the steps that actually run are counted).
const W_KUBECTL: f32 = 15.0;
const W_K9S: f32 = 10.0;
const W_GCLOUD_DOWNLOAD: f32 = 55.0;
const W_GCLOUD_EXTRACT: f32 = 10.0;
const W_GCLOUD_SETUP: f32 = 5.0;
const W_PLUGIN: f32 = 5.0;

/// Turns step progress into one overall percentage and reports it (only when it changes).
struct Progress<'a> {
    send: &'a dyn Fn(&str, f32),
    total: f32,
    done: Cell<f32>,
    last: Cell<i32>,
    message: RefCell<String>,
}

impl Progress<'_> {
    fn begin(&self, message: &str) {
        *self.message.borrow_mut() = message.to_string();
        self.emit(0.0, true);
    }

    /// `fraction` (0..1) of a step worth `weight` is done.
    fn part(&self, weight: f32, fraction: f32) {
        self.emit(weight * fraction.clamp(0.0, 1.0), false);
    }

    fn finish(&self, weight: f32) {
        self.done.set(self.done.get() + weight);
        self.emit(0.0, true);
    }

    fn emit(&self, extra: f32, force: bool) {
        let pct = if self.total > 0.0 {
            ((self.done.get() + extra) / self.total * 100.0).clamp(0.0, 100.0)
        } else {
            100.0
        };
        let whole = pct as i32;
        if force || whole != self.last.get() {
            self.last.set(whole);
            (self.send)(&self.message.borrow(), pct);
        }
    }
}

/// Temporary folder for downloaded archives (cleaned up after each install).
fn temp_dir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("klar-install");
    std::fs::create_dir_all(&dir).map_err(|e| crate::tf!("สร้างโฟลเดอร์ชั่วคราวไม่ได้: {e}", "Could not create a temporary folder: {e}"))?;
    Ok(dir)
}

fn install_all(missing: &[String], send: &dyn Fn(&str, f32)) -> Result<(), String> {
    let dir = tools_dir();
    std::fs::create_dir_all(&dir).map_err(|e| crate::tf!("สร้างโฟลเดอร์ {} ไม่ได้: {e}", "Could not create folder {}: {e}", dir.display()))?;
    let has = |name: &str| missing.iter().any(|m| m == name);
    let need_plugin = has("gcloud") || has("gke-gcloud-auth-plugin");

    let mut total = 0.0;
    if has("kubectl") {
        total += W_KUBECTL;
    }
    if has("k9s") {
        total += W_K9S;
    }
    if has("gcloud") {
        total += W_GCLOUD_DOWNLOAD + W_GCLOUD_EXTRACT + W_GCLOUD_SETUP;
    }
    if need_plugin {
        total += W_PLUGIN;
    }
    let prog = Progress {
        send,
        total,
        done: Cell::new(0.0),
        last: Cell::new(-1),
        message: RefCell::new(String::new()),
    };

    let tmp = temp_dir()?;
    let result = (|| {
        if has("kubectl") {
            install_kubectl(&dir, &prog)?;
        }
        if has("k9s") {
            install_k9s(&dir, &tmp, &prog)?;
        }
        if has("gcloud") {
            install_gcloud(&dir, &tmp, &prog)?;
        }
        if need_plugin {
            prog.begin(crate::tr!("กำลังติดตั้ง gke-gcloud-auth-plugin...", "Installing gke-gcloud-auth-plugin..."));
            let out = gcloud_command(&["components", "install", "gke-gcloud-auth-plugin", "--quiet"])
                .output()
                .map_err(|e| crate::tf!("เรียก gcloud ไม่สำเร็จ: {e}", "Could not run gcloud: {e}"))?;
            if !out.status.success() {
                return Err(crate::tf!(
                    "ติดตั้ง gke-gcloud-auth-plugin ไม่สำเร็จ: {}", "Could not install gke-gcloud-auth-plugin: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                ));
            }
            prog.finish(W_PLUGIN);
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    if result.is_ok() {
        prog.begin(crate::tr!("ติดตั้งเสร็จแล้ว", "Installed"));
    }
    result
}

fn arch() -> &'static str {
    if cfg!(target_arch = "aarch64") { "arm64" } else { "amd64" }
}

fn install_kubectl(dir: &Path, prog: &Progress) -> Result<(), String> {
    prog.begin(crate::tr!("กำลังโหลด kubectl...", "Downloading kubectl..."));
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
    download_tracked(&url, &dest, &|f| prog.part(W_KUBECTL, f))?;
    make_executable(&dest)?;
    prog.finish(W_KUBECTL);
    Ok(())
}

fn install_k9s(dir: &Path, tmp: &Path, prog: &Progress) -> Result<(), String> {
    prog.begin(crate::tr!("กำลังโหลด k9s...", "Downloading k9s..."));
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
    let archive = tmp.join(format!("k9s.{ext}"));
    let unpack = tmp.join("k9s");
    download_tracked(&url, &archive, &|f| prog.part(W_K9S * 0.9, f))?;
    extract(&archive, &unpack)?;
    let dest = dir.join(exe);
    let _ = std::fs::remove_file(&dest);
    std::fs::copy(unpack.join(exe), &dest).map_err(|e| crate::tf!("ติดตั้ง k9s ไม่สำเร็จ: {e}", "Could not install k9s: {e}"))?;
    make_executable(&dest)?;
    prog.finish(W_K9S);
    Ok(())
}

fn install_gcloud(dir: &Path, tmp: &Path, prog: &Progress) -> Result<(), String> {
    prog.begin(crate::tr!("กำลังโหลด gcloud (ไฟล์ใหญ่ อาจใช้เวลาสักพัก)...", "Downloading gcloud (large file, this can take a while)..."));
    let base = "https://dl.google.com/dl/cloudsdk/channels/rapid/downloads";
    let (file, ext) = if cfg!(windows) {
        ("google-cloud-cli-windows-x86_64.zip".to_string(), "zip")
    } else if cfg!(target_os = "macos") {
        let a = if cfg!(target_arch = "aarch64") { "arm" } else { "x86_64" };
        (format!("google-cloud-cli-darwin-{a}.tar.gz"), "tar.gz")
    } else {
        ("google-cloud-cli-linux-x86_64.tar.gz".to_string(), "tar.gz")
    };
    let archive = tmp.join(format!("gcloud.{ext}"));
    download_tracked(&format!("{base}/{file}"), &archive, &|f| {
        prog.part(W_GCLOUD_DOWNLOAD, f)
    })?;
    prog.finish(W_GCLOUD_DOWNLOAD);

    prog.begin(crate::tr!("กำลังแตกไฟล์ gcloud...", "Extracting gcloud..."));
    let sdk = dir.join("google-cloud-sdk");
    let _ = std::fs::remove_dir_all(&sdk);
    extract(&archive, dir)?;
    prog.finish(W_GCLOUD_EXTRACT);

    prog.begin(crate::tr!("กำลังตั้งค่า gcloud...", "Setting up gcloud..."));
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
    .map_err(|e| crate::tf!("ตั้งค่า gcloud ไม่สำเร็จ: {e}", "Could not set up gcloud: {e}"))?;
    if !out.status.success() {
        return Err(crate::tf!(
            "ตั้งค่า gcloud ไม่สำเร็จ: {}", "Could not set up gcloud: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    prog.finish(W_GCLOUD_SETUP);
    Ok(())
}

/// Size of the file at `url` (after redirects), if the server says.
fn content_length(url: &str) -> Option<u64> {
    let out = Command::new("curl")
        .args(["-sIL", "--max-time", "20", url])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            if k.trim().eq_ignore_ascii_case("content-length") {
                v.trim().parse::<u64>().ok()
            } else {
                None
            }
        })
        .filter(|n| *n > 0)
        .last()
}

/// Downloads with curl and reports how much of the file has arrived (0..1) while it runs.
fn download_tracked(url: &str, dest: &Path, on_fraction: &dyn Fn(f32)) -> Result<(), String> {
    let total = content_length(url);
    let _ = std::fs::remove_file(dest);
    let mut child = Command::new("curl")
        .args(["-fL", "--retry", "3", "-sS", "-o"])
        .arg(dest)
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| crate::tf!("เรียก curl ไม่สำเร็จ: {e}", "Could not run curl: {e}"))?;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(e) => return Err(crate::tf!("ดาวน์โหลดไม่สำเร็จ: {e}", "Download failed: {e}")),
        }
        if let (Some(t), Ok(meta)) = (total, std::fs::metadata(dest)) {
            on_fraction(meta.len() as f32 / t as f32);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    let out = child
        .wait_with_output()
        .map_err(|e| crate::tf!("ดาวน์โหลดไม่สำเร็จ: {e}", "Download failed: {e}"))?;
    if out.status.success() {
        on_fraction(1.0);
        Ok(())
    } else {
        Err(crate::tf!(
            "ดาวน์โหลดไม่สำเร็จ: {url}\n{}", "Download failed: {url}\n{}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
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
        .map_err(|e| crate::tf!("เรียก curl ไม่สำเร็จ: {e}", "Could not run curl: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(crate::tf!(
            "ดาวน์โหลดไม่สำเร็จ: {url}\n{}", "Download failed: {url}\n{}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

fn curl_text(url: &str) -> Result<String, String> {
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "30", url])
        .output()
        .map_err(|e| crate::tf!("เรียก curl ไม่สำเร็จ: {e}", "Could not run curl: {e}"))?;
    if !out.status.success() {
        return Err(crate::tf!("เปิด {url} ไม่ได้", "Could not open {url}"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| crate::tf!("สร้างโฟลเดอร์ไม่ได้: {e}", "Could not create the folder: {e}"))?;
    let out = Command::new("tar")
        .arg("-xf")
        .arg(archive)
        .arg("-C")
        .arg(dest)
        .output()
        .map_err(|e| crate::tf!("เรียก tar ไม่สำเร็จ: {e}", "Could not run tar: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(crate::tf!(
            "แตกไฟล์ไม่สำเร็จ: {}", "Extraction failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| crate::tf!("ตั้งสิทธิ์รันไฟล์ไม่ได้: {e}", "Could not make the file executable: {e}"))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}
