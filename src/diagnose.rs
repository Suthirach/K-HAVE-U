//! Turns raw kubectl / gcloud error text into a short, human explanation:
//! what went wrong, why, and what to do about it.

pub struct Diagnosis {
    /// One line: what failed.
    pub title: String,
    /// Why it most likely happened.
    pub cause: String,
    /// What the user can do.
    pub fix: String,
    /// The original message, trimmed, for the details box.
    pub raw: String,
}

fn has(hay: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| hay.contains(n))
}

/// Strips our own "kubectl error (showing sample data instead):" style prefixes.
fn clean_raw(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    for p in [
        "kubectl error (แสดงข้อมูลตัวอย่างแทน):",
        "kubectl error (showing sample data instead):",
        "เรียก kubectl ไม่ได้ (แสดงข้อมูลตัวอย่างแทน):",
        "Could not run kubectl (showing sample data instead):",
    ] {
        if let Some(rest) = s.strip_prefix(p) {
            s = rest.trim().to_string();
        }
    }
    if s.chars().count() > 600 {
        s = s.chars().take(600).collect::<String>() + "…";
    }
    s
}

pub fn diagnose(raw: &str) -> Diagnosis {
    let raw_clean = clean_raw(raw);
    let l = raw_clean.to_ascii_lowercase();
    let (title, cause, fix): (&str, &str, &str) = if has(&l, &["gke-gcloud-auth-plugin"]) {
        crate::i18n::pick3(
            ("ขาดตัวช่วยล็อกอิน GKE", "kubectl หา gke-gcloud-auth-plugin ไม่เจอ จึงยืนยันตัวตนกับ cluster ไม่ได้", "เปิดหน้า \"ตั้งค่าเครื่องมือ\" แล้วกดติดตั้ง หรือรัน: gcloud components install gke-gcloud-auth-plugin"),
            ("GKE sign-in helper missing", "kubectl can't find gke-gcloud-auth-plugin, so it can't authenticate to the cluster", "Open Tools setup and install it, or run: gcloud components install gke-gcloud-auth-plugin"),
        )
    } else if has(&l, &["executable file not found", "no such file or directory", "not found in $path", "os error 2"]) && has(&l, &["kubectl", "os error 2"]) {
        crate::i18n::pick3(
            ("ไม่พบ kubectl ในเครื่อง", "แอปเรียกคำสั่ง kubectl ไม่ได้ เพราะยังไม่ได้ติดตั้งหรือไม่อยู่ใน PATH", "เปิดหน้า \"ตั้งค่าเครื่องมือ\" แล้วกดติดตั้ง kubectl"),
            ("kubectl not found", "The app couldn't run kubectl: it isn't installed or isn't on PATH", "Open Tools setup and install kubectl"),
        )
    } else if has(&l, &["reauthentication", "refresh token", "invalid_grant", "token has been expired", "expired or revoked", "gcloud auth login", "you do not currently have an active account"]) {
        crate::i18n::pick3(
            ("การล็อกอิน Google หมดอายุ", "token ของ gcloud หมดอายุหรือถูกยกเลิก จึงคุยกับ cluster ไม่ได้", "ล็อกอินใหม่ (ออกจากระบบแล้วเข้าใหม่) หรือรัน: gcloud auth login"),
            ("Google sign-in expired", "Your gcloud token expired or was revoked, so the cluster rejects requests", "Sign in again (sign out and back in), or run: gcloud auth login"),
        )
    } else if has(&l, &["unauthorized", "you must be logged in", "provide credentials"]) {
        crate::i18n::pick3(
            ("Cluster ไม่ยอมรับการล็อกอิน", "ข้อมูลยืนยันตัวตนสำหรับ cluster นี้ใช้ไม่ได้", "เลือก Project/Cluster ใหม่อีกครั้งเพื่อดึง credentials ใหม่ แล้วกดลองใหม่"),
            ("The cluster rejected the sign-in", "The credentials for this cluster aren't valid", "Pick the project/cluster again to fetch fresh credentials, then retry"),
        )
    } else if has(&l, &["forbidden", "cannot list resource", "permission denied", "does not have permission", "is not authorized"]) {
        crate::i18n::pick3(
            ("บัญชีนี้ไม่มีสิทธิ์", "บัญชี Google ที่ล็อกอินอยู่ไม่มีสิทธิ์อ่านข้อมูลใน namespace/cluster นี้", "ขอสิทธิ์ (เช่น Kubernetes Engine Viewer) จากผู้ดูแล project หรือเปลี่ยนไปใช้บัญชีที่มีสิทธิ์"),
            ("This account lacks permission", "The signed-in Google account can't read this namespace or cluster", "Ask the project admin for access (e.g. Kubernetes Engine Viewer) or switch to an account that has it"),
        )
    } else if has(&l, &["namespaces \"", "namespace \""]) && has(&l, &["not found"]) {
        crate::i18n::pick3(
            ("ไม่มี Namespace นี้ใน cluster", "namespace ที่เลือกไม่มีอยู่ใน cluster ที่เชื่อมต่ออยู่ (อาจถูกลบหรืออยู่คนละ cluster)", "กดค้างที่การ์ดเพื่อเลิกดู namespace นี้ หรือตรวจว่าเลือก cluster ถูกตัว"),
            ("Namespace not found", "This namespace doesn't exist in the connected cluster (deleted, or in another cluster)", "Press and hold its card to stop watching it, or check you picked the right cluster"),
        )
    } else if has(&l, &["no such host", "dial tcp", "i/o timeout", "connection refused", "network is unreachable", "timeout awaiting", "context deadline exceeded", "tls handshake timeout", "unable to connect to the server", "request canceled"]) {
        crate::i18n::pick3(
            ("เชื่อมต่อ cluster ไม่ได้", "เครือข่ายเข้าไม่ถึง API ของ cluster (เน็ตหลุด, ต้องต่อ VPN, หรือ cluster ปิดอยู่)", "ตรวจอินเทอร์เน็ต/VPN แล้วกดลองใหม่ ถ้า cluster เป็น private ต้องอยู่ในเครือข่ายที่อนุญาต"),
            ("Can't reach the cluster", "The network can't reach the cluster API (offline, VPN needed, or the cluster is down)", "Check your internet/VPN and retry. Private clusters need an allowed network"),
        )
    } else if has(&l, &["current-context is not set", "no configuration has been provided", "context was not found", "invalid configuration"]) {
        crate::i18n::pick3(
            ("ยังไม่ได้เลือก Cluster", "kubectl ไม่รู้ว่าจะเชื่อมต่อ cluster ไหน (ไม่มี context)", "กดปุ่ม Project ด้านบนแล้วเลือก Project และ Cluster อีกครั้ง"),
            ("No cluster selected", "kubectl doesn't know which cluster to use (no context)", "Use the Project button at the top to pick a project and cluster again"),
        )
    } else if has(&l, &["parse json"]) {
        crate::i18n::pick3(
            ("อ่านข้อมูลที่ได้จาก cluster ไม่ออก", "ผลลัพธ์จาก kubectl อยู่ในรูปแบบที่แอปไม่รู้จัก", "กดลองใหม่ ถ้ายังเป็นอยู่ ส่งรายละเอียดด้านล่างให้ผู้พัฒนา"),
            ("Couldn't read the cluster's reply", "kubectl returned data in a format the app doesn't understand", "Retry; if it keeps happening, send the details below to the developer"),
        )
    } else {
        crate::i18n::pick3(
            ("โหลดข้อมูลไม่สำเร็จ", "เกิดข้อผิดพลาดที่แอปยังไม่รู้จัก ดูข้อความจากระบบด้านล่าง", "กดลองใหม่ ถ้ายังไม่ได้ ส่งรายละเอียดด้านล่างให้ผู้พัฒนา"),
            ("Loading failed", "An error the app doesn't recognise yet; see the system message below", "Retry; if it persists, send the details below to the developer"),
        )
    };
    Diagnosis { title: title.into(), cause: cause.into(), fix: fix.into(), raw: raw_clean }
}
