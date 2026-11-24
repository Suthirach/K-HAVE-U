use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::mpsc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Developer,
    Viewer,
}

impl Role {
    pub fn label(self) -> &'static str {
        match self {
            Role::Admin => "Admin (STG + PROD)",
            Role::Developer => "Developer (STG Write / PROD View)",
            Role::Viewer => "Viewer (Read-Only)",
        }
    }

    pub fn can_mutate_prod(self) -> bool {
        matches!(self, Role::Admin)
    }

    pub fn can_mutate_stg(self) -> bool {
        matches!(self, Role::Admin | Role::Developer)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserAccount {
    pub username: String,
    pub password: String,
    pub role: Role,
    pub display_name: String,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub username: String,
    pub display_name: String,
    pub role: Role,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GoogleAuthConfig {
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    #[serde(default)]
    pub email_roles: HashMap<String, Role>,
}

impl GoogleAuthConfig {
    pub fn load() -> Self {
        let mut cfg = std::fs::read_to_string("k9s-ui.json")
            .ok()
            .and_then(|s| serde_json::from_str::<GoogleAuthConfig>(&s).ok())
            .unwrap_or_default();

        if cfg.google_client_id.is_none() {
            cfg.google_client_id = std::env::var("GOOGLE_CLIENT_ID").ok().filter(|s| !s.is_empty());
        }
        if cfg.google_client_secret.is_none() {
            cfg.google_client_secret = std::env::var("GOOGLE_CLIENT_SECRET").ok().filter(|s| !s.is_empty());
        }
        cfg
    }

    pub fn role_for_email(&self, email: &str) -> Role {
        let lower = email.trim().to_ascii_lowercase();
        self.email_roles.get(&lower).copied().unwrap_or(Role::Admin)
    }
}

pub enum AuthEvent {
    Success(Session),
    Error(String),
}

pub fn default_accounts() -> Vec<UserAccount> {
    vec![
        UserAccount {
            username: "admin".into(),
            password: "admin123".into(),
            role: Role::Admin,
            display_name: "Infra Admin".into(),
        },
        UserAccount {
            username: "dev".into(),
            password: "dev123".into(),
            role: Role::Developer,
            display_name: "App Developer".into(),
        },
        UserAccount {
            username: "viewer".into(),
            password: "viewer123".into(),
            role: Role::Viewer,
            display_name: "Team Viewer".into(),
        },
    ]
}

pub fn authenticate(accounts: &[UserAccount], username: &str, password: &str) -> Result<Session, String> {
    let u = username.trim();
    if u.is_empty() || password.is_empty() {
        return Err("กรุณากรอก Username และ Password".into());
    }
    for acc in accounts {
        if acc.username.eq_ignore_ascii_case(u) && acc.password == password {
            return Ok(Session {
                username: acc.username.clone(),
                display_name: acc.display_name.clone(),
                role: acc.role,
            });
        }
    }
    Err("Username หรือ Password ไม่ถูกต้อง".into())
}

pub fn spawn_google_login(
    cfg: GoogleAuthConfig,
    force_browser: bool,
    tx: mpsc::Sender<AuthEvent>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        let res = if let Some(client_id) = cfg.google_client_id.clone() {
            oauth_browser_login(&client_id, cfg.google_client_secret.as_deref(), &cfg)
        } else {
            gcloud_google_login(force_browser, &cfg)
        };
        match res {
            Ok(sess) => {
                let _ = tx.send(AuthEvent::Success(sess));
            }
            Err(err) => {
                let _ = tx.send(AuthEvent::Error(err));
            }
        }
        wake();
    });
}

pub fn gcloud_command(args: &[&str]) -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg("gcloud").args(args);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = Command::new("gcloud");
        cmd.args(args);
        cmd
    }
}

fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open").arg(url).spawn();
}

fn gcloud_google_login(force_browser: bool, cfg: &GoogleAuthConfig) -> Result<Session, String> {
    if force_browser {
        let status = gcloud_command(&["auth", "login", "--quiet"])
            .status()
            .map_err(|e| format!("เรียก gcloud ไม่สำเร็จ: {e}"))?;
        if !status.success() {
            return Err("การเข้าสู่ระบบ Google ผ่าน browser ถูกยกเลิกหรือไม่สำเร็จ".into());
        }
    }

    let out = gcloud_command(&["auth", "list", "--filter=status:ACTIVE", "--format=value(account)"])
        .output()
        .map_err(|e| format!("ไม่พบ gcloud CLI: {e}"))?;

    let email = String::from_utf8_lossy(&out.stdout).trim().lines().next().unwrap_or("").trim().to_string();
    if email.is_empty() {
        // Trigger interactive browser login if no active account yet
        let status = gcloud_command(&["auth", "login", "--quiet"])
            .status()
            .map_err(|e| format!("เปิด Google Login ไม่สำเร็จ: {e}"))?;
        if !status.success() {
            return Err("ยังไม่ได้ล็อกอิน Google Account (กรุณาล็อกอินใน browser)".into());
        }
        let out2 = gcloud_command(&["auth", "list", "--filter=status:ACTIVE", "--format=value(account)"])
            .output()
            .map_err(|e| format!("อ่านบัญชี Google ไม่สำเร็จ: {e}"))?;
        let email2 = String::from_utf8_lossy(&out2.stdout).trim().lines().next().unwrap_or("").trim().to_string();
        if email2.is_empty() {
            return Err("ไม่พบบัญชี Gmail ที่ล็อกอินอยู่".into());
        }
        return Ok(Session {
            role: cfg.role_for_email(&email2),
            display_name: email2.clone(),
            username: email2,
        });
    }

    Ok(Session {
        role: cfg.role_for_email(&email),
        display_name: email.clone(),
        username: email,
    })
}

fn oauth_browser_login(
    client_id: &str,
    client_secret: Option<&str>,
    cfg: &GoogleAuthConfig,
) -> Result<Session, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("เปิด callback port ไม่ได้: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    let verifier_seed = format!(
        "k9s-studio-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let verifier = URL_SAFE_NO_PAD.encode(verifier_seed.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&code_challenge={}&code_challenge_method=S256&prompt=select_account",
        url_encode(client_id),
        url_encode(&redirect_uri),
        url_encode(&challenge),
    );

    open_browser(&auth_url);

    let (mut stream, _) = listener.accept().map_err(|e| format!("รอ callback จาก Google ไม่สำเร็จ: {e}"))?;
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).unwrap_or(0);
    let req = String::from_utf8_lossy(&buf[..n]);
    let code = extract_query_param(&req, "code")
        .ok_or_else(|| "ไม่ได้รับ authorization code จาก Google".to_string())?;

    let html = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<html><body style='font-family:sans-serif;text-align:center;padding:60px;background:#111;color:#fff'><h2>เข้าสู่ระบบ Google สำเร็จ!</h2><p>ปิดหน้านี้แล้วกลับไปที่แอป K9s Studio ได้เลยครับ</p></body></html>";
    let _ = stream.write_all(html.as_bytes());

    let mut form = format!(
        "code={}&client_id={}&redirect_uri={}&code_verifier={}&grant_type=authorization_code",
        url_encode(&code),
        url_encode(client_id),
        url_encode(&redirect_uri),
        url_encode(&verifier),
    );
    if let Some(secret) = client_secret {
        form.push_str(&format!("&client_secret={}", url_encode(secret)));
    }

    let token_out = Command::new("curl")
        .args(["-sS", "-X", "POST", "https://oauth2.googleapis.com/token", "-d", &form])
        .output()
        .map_err(|e| format!("เรียก Google Token API ไม่สำเร็จ: {e}"))?;
    let token_json: Value = serde_json::from_slice(&token_out.stdout)
        .map_err(|e| format!("อ่าน Token JSON ไม่สำเร็จ: {e}"))?;
    let access_token = token_json["access_token"]
        .as_str()
        .ok_or_else(|| format!("Google Token error: {token_json}"))?;

    let user_out = Command::new("curl")
        .args([
            "-sS",
            "-H",
            &format!("Authorization: Bearer {access_token}"),
            "https://www.googleapis.com/oauth2/v3/userinfo",
        ])
        .output()
        .map_err(|e| format!("ดึงข้อมูลผู้ใช้ Google ไม่สำเร็จ: {e}"))?;
    let user_json: Value = serde_json::from_slice(&user_out.stdout)
        .map_err(|e| format!("อ่าน UserInfo JSON ไม่สำเร็จ: {e}"))?;
    let email = user_json["email"]
        .as_str()
        .ok_or_else(|| "ไม่พบอีเมลจาก Google UserInfo".to_string())?
        .to_string();
    let name = user_json["name"]
        .as_str()
        .map(|s| format!("{s} ({email})"))
        .unwrap_or_else(|| email.clone());

    Ok(Session {
        role: cfg.role_for_email(&email),
        display_name: name,
        username: email,
    })
}

fn extract_query_param(req: &str, key: &str) -> Option<String> {
    let line = req.lines().next()?;
    let path = line.split_whitespace().nth(1)?;
    let query = path.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut it = pair.splitn(2, '=');
        if it.next()? == key {
            return Some(url_decode(it.next().unwrap_or("")));
        }
    }
    None
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(val) = u8::from_str_radix(hex, 16) {
                    out.push(val);
                    i += 3;
                    continue;
                }
            }
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}
