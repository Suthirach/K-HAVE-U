use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;
use std::sync::mpsc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnvKind {
    Stg,
    Prod,
}

impl EnvKind {
    pub fn label(self) -> &'static str {
        match self {
            EnvKind::Stg => "STG",
            EnvKind::Prod => "PROD",
        }
    }

    pub fn full_label(self) -> &'static str {
        match self {
            EnvKind::Stg => "Staging (STG)",
            EnvKind::Prod => "Production (PROD)",
        }
    }

    pub fn default_namespaces(self) -> Vec<&'static str> {
        match self {
            EnvKind::Stg => vec!["staging-api", "htai-staging"],
            EnvKind::Prod => vec!["prod-api", "htai-prod", "lotto-prod"],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Warning,
    Error,
}

impl Health {
    pub fn label(self) -> &'static str {
        match self {
            Health::Healthy => "Healthy",
            Health::Warning => "Warning",
            Health::Error => "Error",
        }
    }

    pub fn simple_thai(self) -> &'static str {
        match self {
            Health::Healthy => crate::tr!("ทำงานปกติ", "Healthy"),
            Health::Warning => crate::tr!("กำลังเริ่มต้น / มีเตือน", "Starting / warning"),
            Health::Error => crate::tr!("มีปัญหา / ล่ม", "Failing / down"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeRole {
    Ingress,
    Api,
    Worker,
    Cache,
}

impl NodeRole {
    pub fn label(self) -> &'static str {
        match self {
            NodeRole::Ingress => "Domain / Ingress",
            NodeRole::Api => "API / Service",
            NodeRole::Worker => "Worker / Cron",
            NodeRole::Cache => "Redis / Cache",
        }
    }
}

#[derive(Clone, Debug)]
pub struct IngressRoute {
    pub name: String,
    pub host: String,
    pub backend_service: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct PodInfo {
    pub name: String,
    pub phase: String,
    pub ready: bool,
    pub restarts: i64,
    pub reason: String,
    pub explanation: String,
    pub node_name: String,
    pub node_spec: String,
    pub cpu_spec: String,
    pub mem_spec: String,
    pub live_usage: String,
}

#[derive(Clone, Debug)]
pub struct ServiceInfo {
    pub name: String,
    pub deploy_name: Option<String>,
    pub namespace: String,
    pub role: NodeRole,
    pub health: Health,
    pub ready_replicas: i64,
    pub desired_replicas: i64,
    pub ports: String,
    pub hosts: Vec<String>,
    pub connects_to: Vec<String>,
    pub summary: String,
    pub pods: Vec<PodInfo>,
}

#[derive(Clone, Debug)]
pub struct ClusterSnapshot {
    pub env: EnvKind,
    pub namespace: String,
    pub context: String,
    pub services: Vec<ServiceInfo>,
    pub ingresses: Vec<IngressRoute>,
    pub mermaid_diagram: String,
    pub is_demo: bool,
    pub error: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvSourceKind {
    Inline,
    Secret(String),
    ConfigMap(String),
    Demo,
}

#[derive(Clone, Debug)]
pub struct EnvVarItem {
    pub name: String,
    pub value: String,
    pub is_secret: bool,
    pub source: String,
    pub source_kind: EnvSourceKind,
}

#[derive(Clone, Debug)]
pub struct NamespaceSummary {
    pub env: EnvKind,
    pub namespace: String,
    pub total_services: usize,
    pub healthy: usize,
    pub warning: usize,
    pub error: usize,
    pub ingress_count: usize,
    pub is_demo: bool,
}

pub enum K8sEvent {
    Snapshot(ClusterSnapshot),
    Overview(Vec<NamespaceSummary>),
    AllNamespaces(Vec<String>),
    Projects(Result<Vec<crate::gcp::GcpProject>, String>),
    Clusters {
        project: String,
        result: Result<Vec<crate::gcp::GkeCluster>, String>,
    },
    Credentials {
        project: String,
        cluster: String,
        result: Result<(), String>,
    },
    UpdateCheck(Result<Option<String>, String>),
    UpdateFailed(String),
    ToolsStatus(Vec<String>),
    ToolsProgress { message: String, percent: f32 },
    ToolsDone(Result<(), String>),
    Logs { pod: String, text: String },
    EnvVars { deploy: String, vars: Vec<EnvVarItem> },
    ActionResult { message: String, ok: bool },
}

pub fn spawn_list_all_namespaces(
    tx: mpsc::Sender<K8sEvent>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        let mut names = Vec::new();
        if let Ok(out) = Command::new("kubectl")
            .args(["get", "ns", "-o", "jsonpath={.items[*].metadata.name}", "--request-timeout=5s"])
            .output()
        {
            if out.status.success() {
                for s in String::from_utf8_lossy(&out.stdout).split_whitespace() {
                    names.push(s.to_string());
                }
            }
        }
        let _ = tx.send(K8sEvent::AllNamespaces(names));
        wake();
    });
}

pub fn spawn_fetch_overview(
    stg_ns: Vec<String>,
    prod_ns: Vec<String>,
    tx: mpsc::Sender<K8sEvent>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        let mut summaries = Vec::new();
        for (env, list) in [(EnvKind::Stg, stg_ns), (EnvKind::Prod, prod_ns)] {
            for ns in list {
                let snap = fetch_snapshot_blocking(env, &ns);
                let total_services = snap.services.len();
                let healthy = snap.services.iter().filter(|s| s.health == Health::Healthy).count();
                let warning = snap.services.iter().filter(|s| s.health == Health::Warning).count();
                let error = snap.services.iter().filter(|s| s.health == Health::Error).count();
                summaries.push(NamespaceSummary {
                    env,
                    namespace: ns,
                    total_services,
                    healthy,
                    warning,
                    error,
                    ingress_count: snap.ingresses.len(),
                    is_demo: snap.is_demo,
                });
            }
        }
        let _ = tx.send(K8sEvent::Overview(summaries));
        wake();
    });
}

pub fn spawn_fetch_env(
    namespace: String,
    deploy: String,
    tx: mpsc::Sender<K8sEvent>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        let out = Command::new("kubectl")
            .args(["get", "deploy", &deploy, "-n", &namespace, "-o", "json", "--request-timeout=6s"])
            .output();
        let mut vars = Vec::new();
        if let Ok(o) = out {
            if o.status.success() {
                if let Ok(v) = serde_json::from_slice::<Value>(&o.stdout) {
                    if let Some(containers) = v["spec"]["template"]["spec"]["containers"].as_array() {
                        for c in containers {
                            if let Some(envs) = c["env"].as_array() {
                                for e in envs {
                                    let name = e["name"].as_str().unwrap_or("").to_string();
                                    let lower = name.to_ascii_lowercase();
                                    let is_secret = lower.contains("pass")
                                        || lower.contains("secret")
                                        || lower.contains("token")
                                        || lower.contains("key");
                                    let (value, source, source_kind) = if let Some(val) = e["value"].as_str() {
                                        (val.to_string(), "Inline".to_string(), EnvSourceKind::Inline)
                                    } else if let Some(sec) = e["valueFrom"]["secretKeyRef"]["name"].as_str() {
                                        let key = e["valueFrom"]["secretKeyRef"]["key"].as_str().unwrap_or("");
                                        let decoded = fetch_secret_key(&namespace, sec, key)
                                            .unwrap_or_else(|| format!("(Secret: {sec}/{key})"));
                                        (decoded, format!("Secret: {sec}"), EnvSourceKind::Secret(sec.to_string()))
                                    } else if let Some(cm) = e["valueFrom"]["configMapKeyRef"]["name"].as_str() {
                                        let key = e["valueFrom"]["configMapKeyRef"]["key"].as_str().unwrap_or("");
                                        let val = fetch_configmap_key(&namespace, cm, key)
                                            .unwrap_or_else(|| format!("(ConfigMap: {cm}/{key})"));
                                        (val, format!("ConfigMap: {cm}"), EnvSourceKind::ConfigMap(cm.to_string()))
                                    } else {
                                        ("(from valueFrom)".into(), "valueFrom".into(), EnvSourceKind::Inline)
                                    };
                                    if !name.is_empty() {
                                        vars.push(EnvVarItem { name, value, is_secret, source, source_kind });
                                    }
                                }
                            }
                            if let Some(env_from) = c["envFrom"].as_array() {
                                for ef in env_from {
                                    if let Some(cm) = ef["configMapRef"]["name"].as_str() {
                                        let mut cm_vars = fetch_all_configmap(&namespace, cm);
                                        if cm_vars.is_empty() {
                                            vars.push(EnvVarItem {
                                                name: "envFrom (ConfigMap)".into(),
                                                value: crate::tf!("{cm} (ว่างหรือไม่มีสิทธิ์อ่าน)", "{cm} (empty or no read access)"),
                                                is_secret: false,
                                                source: format!("ConfigMap: {cm}"),
                                                source_kind: EnvSourceKind::ConfigMap(cm.to_string()),
                                            });
                                        } else {
                                            vars.append(&mut cm_vars);
                                        }
                                    }
                                    if let Some(sec) = ef["secretRef"]["name"].as_str() {
                                        let mut sec_vars = fetch_all_secret(&namespace, sec);
                                        if sec_vars.is_empty() {
                                            vars.push(EnvVarItem {
                                                name: "envFrom (Secret)".into(),
                                                value: crate::tf!("{sec} (ว่างหรือไม่มีสิทธิ์อ่าน)", "{sec} (empty or no read access)"),
                                                is_secret: true,
                                                source: format!("Secret: {sec}"),
                                                source_kind: EnvSourceKind::Secret(sec.to_string()),
                                            });
                                        } else {
                                            vars.append(&mut sec_vars);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if vars.is_empty() {
            vars = vec![
                EnvVarItem { name: "ASPNETCORE_ENVIRONMENT".into(), value: if namespace.contains("prod") { "Production".into() } else { "Staging".into() }, is_secret: false, source: "Demo".into(), source_kind: EnvSourceKind::Demo },
                EnvVarItem { name: "REDIS_CONNECTION".into(), value: "redis-srv:6379".into(), is_secret: false, source: "Demo".into(), source_kind: EnvSourceKind::Demo },
                EnvVarItem { name: "JWT_SECRET_KEY".into(), value: "sk_live_demo_secret_value".into(), is_secret: true, source: "Demo".into(), source_kind: EnvSourceKind::Demo },
            ];
        }
        let _ = tx.send(K8sEvent::EnvVars { deploy, vars });
        wake();
    });
}

pub fn spawn_update_env(
    namespace: String,
    deploy: String,
    source_kind: EnvSourceKind,
    key: String,
    new_value: String,
    restart_after: bool,
    tx: mpsc::Sender<K8sEvent>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        let res = match &source_kind {
            EnvSourceKind::Secret(sec_name) => {
                let b64 = STANDARD.encode(new_value.as_bytes());
                let patch = serde_json::json!({ "data": { &key: b64 } }).to_string();
                Command::new("kubectl")
                    .args(["patch", "secret", sec_name, "-n", &namespace, "--type=merge", "-p", &patch, "--request-timeout=8s"])
                    .output()
            }
            EnvSourceKind::ConfigMap(cm_name) => {
                let patch = serde_json::json!({ "data": { &key: &new_value } }).to_string();
                Command::new("kubectl")
                    .args(["patch", "configmap", cm_name, "-n", &namespace, "--type=merge", "-p", &patch, "--request-timeout=8s"])
                    .output()
            }
            EnvSourceKind::Inline => {
                let kv = format!("{key}={new_value}");
                Command::new("kubectl")
                    .args(["set", "env", &format!("deploy/{deploy}"), &kv, "-n", &namespace, "--request-timeout=8s"])
                    .output()
            }
            EnvSourceKind::Demo => {
                let _ = tx.send(K8sEvent::ActionResult {
                    message: crate::tf!("(Demo) อัปเดต {key} = {new_value} แล้ว", "(Demo) Updated {key} = {new_value}"),
                    ok: true,
                });
                wake();
                return;
            }
        };

        match res {
            Ok(o) if o.status.success() => {
                if restart_after {
                    let _ = Command::new("kubectl")
                        .args(["rollout", "restart", &format!("deploy/{deploy}"), "-n", &namespace, "--request-timeout=8s"])
                        .output();
                }
                let msg = if restart_after {
                    crate::tf!("บันทึก ENV {key} และสั่ง Restart {deploy} สำเร็จ", "Saved ENV {key} and restarted {deploy}")
                } else {
                    crate::tf!("บันทึก ENV {key} สำเร็จ (Restart เพื่อให้ Pod รับค่าใหม่)", "Saved ENV {key} (restart for pods to pick it up)")
                };
                let _ = tx.send(K8sEvent::ActionResult { message: msg, ok: true });
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                let _ = tx.send(K8sEvent::ActionResult {
                    message: crate::tf!("บันทึก ENV {key} ไม่สำเร็จ: {err}", "Could not save ENV {key}: {err}"),
                    ok: false,
                });
            }
            Err(e) => {
                let _ = tx.send(K8sEvent::ActionResult {
                    message: crate::tf!("เรียก kubectl ไม่สำเร็จ: {e}", "Could not run kubectl: {e}"),
                    ok: false,
                });
            }
        }
        wake();
    });
}

fn fetch_all_secret(namespace: &str, name: &str) -> Vec<EnvVarItem> {
    let Ok(out) = Command::new("kubectl")
        .args(["get", "secret", name, "-n", namespace, "-o", "json", "--request-timeout=6s"])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let Ok(v) = serde_json::from_slice::<Value>(&out.stdout) else {
        return Vec::new();
    };
    let mut list = Vec::new();
    if let Some(obj) = v["data"].as_object() {
        for (k, val) in obj {
            let decoded = val
                .as_str()
                .and_then(|b64| STANDARD.decode(b64.trim()).ok())
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
                .unwrap_or_else(|| val.as_str().unwrap_or("").to_string());
            list.push(EnvVarItem {
                name: k.clone(),
                value: decoded,
                is_secret: true,
                source: format!("Secret: {name}"),
                source_kind: EnvSourceKind::Secret(name.to_string()),
            });
        }
    }
    list
}

fn fetch_secret_key(namespace: &str, name: &str, key: &str) -> Option<String> {
    fetch_all_secret(namespace, name)
        .into_iter()
        .find(|i| i.name == key)
        .map(|i| i.value)
}

fn fetch_all_configmap(namespace: &str, name: &str) -> Vec<EnvVarItem> {
    let Ok(out) = Command::new("kubectl")
        .args(["get", "configmap", name, "-n", namespace, "-o", "json", "--request-timeout=6s"])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let Ok(v) = serde_json::from_slice::<Value>(&out.stdout) else {
        return Vec::new();
    };
    let mut list = Vec::new();
    if let Some(obj) = v["data"].as_object() {
        for (k, val) in obj {
            list.push(EnvVarItem {
                name: k.clone(),
                value: val.as_str().unwrap_or("").to_string(),
                is_secret: false,
                source: format!("ConfigMap: {name}"),
                source_kind: EnvSourceKind::ConfigMap(name.to_string()),
            });
        }
    }
    list
}

fn fetch_configmap_key(namespace: &str, name: &str, key: &str) -> Option<String> {
    fetch_all_configmap(namespace, name)
        .into_iter()
        .find(|i| i.name == key)
        .map(|i| i.value)
}

pub fn spawn_fetch_snapshot(
    env: EnvKind,
    namespace: String,
    tx: mpsc::Sender<K8sEvent>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        let snap = fetch_snapshot_blocking(env, &namespace);
        let _ = tx.send(K8sEvent::Snapshot(snap));
        wake();
    });
}

pub fn spawn_fetch_logs(
    namespace: String,
    pod: String,
    since: Option<String>,
    timestamps: bool,
    tx: mpsc::Sender<K8sEvent>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        let mut args = vec![
            "logs".to_string(),
            "--tail=200".to_string(),
            pod.clone(),
            "-n".to_string(),
            namespace,
            "--request-timeout=6s".to_string(),
        ];
        if timestamps {
            args.push("--timestamps=true".to_string());
        }
        if let Some(s) = since {
            args.push(format!("--since={s}"));
        }
        let out = Command::new("kubectl").args(&args).output();
        let text = match out {
            Ok(o) if o.status.success() => {
                let s = String::from_utf8_lossy(&o.stdout).to_string();
                if s.trim().is_empty() {
                    crate::tr!("(ไม่มี log ในช่วงเวลาที่เลือก)", "(No logs in the selected time range)").to_string()
                } else {
                    s
                }
            }
            Ok(o) => crate::tf!("ไม่สามารถดึง log ได้:\n{}", "Could not fetch logs:\n{}", String::from_utf8_lossy(&o.stderr)),
            Err(e) => crate::tf!("เรียก kubectl ไม่สำเร็จ: {e}", "Could not run kubectl: {e}"),
        };
        let _ = tx.send(K8sEvent::Logs { pod, text });
        wake();
    });
}

pub fn spawn_restart_deploy(
    namespace: String,
    deploy: String,
    tx: mpsc::Sender<K8sEvent>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        let out = Command::new("kubectl")
            .args(["rollout", "restart", &format!("deploy/{deploy}"), "-n", &namespace, "--request-timeout=8s"])
            .output();
        let (ok, message) = match out {
            Ok(o) if o.status.success() => (true, crate::tf!("สั่ง Restart {deploy} ใน {namespace} สำเร็จ", "Restarted {deploy} in {namespace}")),
            Ok(o) => (false, crate::tf!("Restart ไม่สำเร็จ: {}", "Restart failed: {}", String::from_utf8_lossy(&o.stderr).trim())),
            Err(e) => (false, crate::tf!("เรียก kubectl ไม่สำเร็จ: {e}", "Could not run kubectl: {e}")),
        };
        let _ = tx.send(K8sEvent::ActionResult { message, ok });
        wake();
    });
}

fn current_context() -> String {
    Command::new("kubectl")
        .args(["config", "current-context"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "default-context".into())
}

fn fetch_snapshot_blocking(env: EnvKind, namespace: &str) -> ClusterSnapshot {
    let context = current_context();
    let out = Command::new("kubectl")
        .args([
            "get",
            "ingress,svc,deploy,pods",
            "-n",
            namespace,
            "-o",
            "json",
            "--request-timeout=6s",
        ])
        .output();

    match out {
        Ok(o) if o.status.success() => {
            let raw = String::from_utf8_lossy(&o.stdout);
            let node_specs = fetch_node_specs();
            let pod_top = fetch_pod_top(namespace);
            match parse_k8s_list(env, namespace, &context, &raw, &node_specs, &pod_top) {
                Ok(snap) => snap,
                Err(e) => demo_snapshot(env, namespace, &context, Some(format!("Parse JSON error: {e}"))),
            }
        }
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
            demo_snapshot(env, namespace, &context, Some(crate::tf!("kubectl error (แสดงข้อมูลตัวอย่างแทน): {err}", "kubectl error (showing sample data instead): {err}")))
        }
        Err(e) => demo_snapshot(env, namespace, &context, Some(crate::tf!("เรียก kubectl ไม่ได้ (แสดงข้อมูลตัวอย่างแทน): {e}", "Could not run kubectl (showing sample data instead): {e}"))),
    }
}

fn fetch_node_specs() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Ok(out) = Command::new("kubectl")
        .args(["get", "nodes", "-o", "json", "--request-timeout=4s"])
        .output()
    else {
        return map;
    };
    if !out.status.success() {
        return map;
    }
    if let Ok(v) = serde_json::from_slice::<Value>(&out.stdout) {
        if let Some(items) = v["items"].as_array() {
            for item in items {
                let name = item["metadata"]["name"].as_str().unwrap_or("").to_string();
                let cpu = item["status"]["capacity"]["cpu"].as_str().unwrap_or("?");
                let mem_raw = item["status"]["capacity"]["memory"].as_str().unwrap_or("?");
                let mem_gb = format_k8s_mem(mem_raw);
                let machine = item["metadata"]["labels"]["node.kubernetes.io/instance-type"]
                    .as_str()
                    .unwrap_or("node");
                if !name.is_empty() {
                    map.insert(name, format!("{machine} • {cpu} vCPU / {mem_gb}"));
                }
            }
        }
    }
    map
}

fn format_k8s_mem(raw: &str) -> String {
    if let Some(ki) = raw.strip_suffix("Ki") {
        if let Ok(kb) = ki.parse::<f64>() {
            return format!("{:.1} GB RAM", kb / (1024.0 * 1024.0));
        }
    }
    raw.to_string()
}

fn fetch_pod_top(namespace: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Ok(out) = Command::new("kubectl")
        .args(["top", "pods", "-n", namespace, "--no-headers", "--request-timeout=4s"])
        .output()
    else {
        return map;
    };
    if !out.status.success() {
        return map;
    }
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            map.insert(parts[0].to_string(), crate::tf!("ใช้จริง: CPU {} • RAM {}", "Usage: CPU {} • RAM {}", parts[1], parts[2]));
        }
    }
    map
}

fn parse_k8s_list(
    env: EnvKind,
    namespace: &str,
    context: &str,
    raw: &str,
    node_specs: &BTreeMap<String, String>,
    pod_top: &BTreeMap<String, String>,
) -> Result<ClusterSnapshot, String> {
    let root: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let items = root["items"].as_array().ok_or("missing items array")?;

    let mut ingresses = Vec::new();
    let mut svc_ports: BTreeMap<String, String> = BTreeMap::new();
    let mut svc_selectors: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut deploys: Vec<(String, i64, i64, BTreeMap<String, String>)> = Vec::new();
    let mut pods: Vec<(PodInfo, BTreeMap<String, String>)> = Vec::new();

    for item in items {
        let kind = item["kind"].as_str().unwrap_or("");
        let name = item["metadata"]["name"].as_str().unwrap_or("").to_string();
        match kind {
            "Ingress" => {
                if let Some(rules) = item["spec"]["rules"].as_array() {
                    for r in rules {
                        let host = r["host"].as_str().unwrap_or("*").to_string();
                        if let Some(paths) = r["http"]["paths"].as_array() {
                            for p in paths {
                                let path = p["path"].as_str().unwrap_or("/").to_string();
                                let backend = p["backend"]["service"]["name"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string();
                                if !backend.is_empty() {
                                    ingresses.push(IngressRoute {
                                        name: name.clone(),
                                        host: host.clone(),
                                        backend_service: backend,
                                        path,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            "Service" => {
                let ports = item["spec"]["ports"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|p| p["port"].as_i64().map(|n| n.to_string()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                let mut sel = BTreeMap::new();
                if let Some(obj) = item["spec"]["selector"].as_object() {
                    for (k, v) in obj {
                        if let Some(vs) = v.as_str() {
                            sel.insert(k.clone(), vs.to_string());
                        }
                    }
                }
                svc_ports.insert(name.clone(), ports);
                svc_selectors.insert(name, sel);
            }
            "Deployment" => {
                let desired = item["spec"]["replicas"].as_i64().unwrap_or(1);
                let ready = item["status"]["readyReplicas"].as_i64().unwrap_or(0);
                let mut labels = BTreeMap::new();
                if let Some(obj) = item["spec"]["template"]["metadata"]["labels"].as_object() {
                    for (k, v) in obj {
                        if let Some(vs) = v.as_str() {
                            labels.insert(k.clone(), vs.to_string());
                        }
                    }
                }
                deploys.push((name, ready, desired, labels));
            }
            "Pod" => {
                let phase = item["status"]["phase"].as_str().unwrap_or("Unknown").to_string();
                let mut restarts = 0;
                let mut ready = true;
                let mut reason = phase.clone();
                if let Some(cs) = item["status"]["containerStatuses"].as_array() {
                    for c in cs {
                        restarts += c["restartCount"].as_i64().unwrap_or(0);
                        if !c["ready"].as_bool().unwrap_or(false) {
                            ready = false;
                        }
                        if let Some(w) = c["state"]["waiting"]["reason"].as_str() {
                            reason = w.to_string();
                        } else if let Some(t) = c["state"]["terminated"]["reason"].as_str() {
                            reason = t.to_string();
                        }
                    }
                } else {
                    ready = phase == "Running";
                }
                let explanation = explain_pod(&phase, ready, restarts, &reason);
                let mut labels = BTreeMap::new();
                if let Some(obj) = item["metadata"]["labels"].as_object() {
                    for (k, v) in obj {
                        if let Some(vs) = v.as_str() {
                            labels.insert(k.clone(), vs.to_string());
                        }
                    }
                }
                let node_name = item["spec"]["nodeName"].as_str().unwrap_or("unscheduled").to_string();
                let node_spec = node_specs
                    .get(&node_name)
                    .cloned()
                    .unwrap_or_else(|| "Standard Node".into());
                let (cpu_spec, mem_spec) = parse_pod_resources(&item["spec"]["containers"]);
                let live_usage = pod_top.get(&name).cloned().unwrap_or_default();

                pods.push((
                    PodInfo {
                        name,
                        phase,
                        ready,
                        restarts,
                        reason,
                        explanation,
                        node_name,
                        node_spec,
                        cpu_spec,
                        mem_spec,
                        live_usage,
                    },
                    labels,
                ));
            }
            _ => {}
        }
    }

    let mut services = Vec::new();
    let mut cache_names = Vec::new();

    for (svc_name, _) in &svc_ports {
        if svc_name.contains("redis") || svc_name.contains("db") || svc_name.contains("mongo") {
            cache_names.push(svc_name.clone());
        }
    }

    let mut matched_deploys = BTreeSet::new();
    for (svc_name, ports) in &svc_ports {
        let sel = svc_selectors.get(svc_name).cloned().unwrap_or_default();
        let matched_deploy = deploys.iter().find(|(_, _, _, labels)| {
            !sel.is_empty() && sel.iter().all(|(k, v)| labels.get(k) == Some(v))
        });

        let (deploy_name, ready_replicas, desired_replicas) = match matched_deploy {
            Some((dn, r, d, _)) => {
                matched_deploys.insert(dn.clone());
                (Some(dn.clone()), *r, *d)
            }
            None => (None, 1, 1),
        };

        let matching_pods: Vec<PodInfo> = pods
            .iter()
            .filter(|(_, labels)| !sel.is_empty() && sel.iter().all(|(k, v)| labels.get(k) == Some(v)))
            .map(|(p, _)| p.clone())
            .collect();

        let hosts: Vec<String> = ingresses
            .iter()
            .filter(|i| &i.backend_service == svc_name)
            .map(|i| i.host.clone())
            .collect();

        let role = classify_role(svc_name);
        let health = compute_health(ready_replicas, desired_replicas, &matching_pods);
        let mut connects_to = Vec::new();
        if role == NodeRole::Api {
            for c in &cache_names {
                if svc_name.contains("hangfire") || c.contains("redis") {
                    connects_to.push(c.clone());
                }
            }
        }

        let summary = build_summary(role, health, ready_replicas, desired_replicas, &hosts);
        services.push(ServiceInfo {
            name: svc_name.clone(),
            deploy_name,
            namespace: namespace.to_string(),
            role,
            health,
            ready_replicas,
            desired_replicas,
            ports: ports.clone(),
            hosts,
            connects_to,
            summary,
            pods: matching_pods,
        });
    }

    // Include deployments without a Service (e.g. workers, hangfire, schedulers)
    for (dn, ready, desired, d_labels) in &deploys {
        if matched_deploys.contains(dn) {
            continue;
        }
        let matching_pods: Vec<PodInfo> = pods
            .iter()
            .filter(|(_, p_labels)| {
                d_labels.get("app").is_some() && d_labels.get("app") == p_labels.get("app")
                    || d_labels.iter().all(|(k, v)| p_labels.get(k) == Some(v))
            })
            .map(|(p, _)| p.clone())
            .collect();

        let role = classify_role(dn);
        let health = compute_health(*ready, *desired, &matching_pods);
        let mut connects_to = Vec::new();
        for c in &cache_names {
            if (dn.contains("hangfire") && c.contains("hangfire")) || c.contains("redis") {
                connects_to.push(c.clone());
            }
        }
        let summary = build_summary(role, health, *ready, *desired, &[]);
        services.push(ServiceInfo {
            name: dn.clone(),
            deploy_name: Some(dn.clone()),
            namespace: namespace.to_string(),
            role,
            health,
            ready_replicas: *ready,
            desired_replicas: *desired,
            ports: "-".into(),
            hosts: vec![],
            connects_to,
            summary,
            pods: matching_pods,
        });
    }

    let mermaid_diagram = build_mermaid(env, namespace, &ingresses, &services);
    Ok(ClusterSnapshot {
        env,
        namespace: namespace.to_string(),
        context: context.to_string(),
        services,
        ingresses,
        mermaid_diagram,
        is_demo: false,
        error: None,
        updated_at: crate::tr!("เมื่อสักครู่", "just now").into(),
    })
}

fn classify_role(name: &str) -> NodeRole {
    let lower = name.to_ascii_lowercase();
    if lower.contains("redis") || lower.contains("mongo") || lower.contains("postgres") || lower.contains("mysql") {
        NodeRole::Cache
    } else if lower.contains("worker") || lower.contains("scheduler") || lower.contains("hangfire") || lower.contains("cron") {
        NodeRole::Worker
    } else {
        NodeRole::Api
    }
}

fn compute_health(ready: i64, desired: i64, pods: &[PodInfo]) -> Health {
    if desired > 0 && ready == 0 {
        return Health::Error;
    }
    if pods.iter().any(|p| !p.ready && (p.reason.contains("Crash") || p.reason.contains("Err"))) {
        return Health::Error;
    }
    if ready < desired || pods.iter().any(|p| !p.ready || p.restarts > 3) {
        return Health::Warning;
    }
    Health::Healthy
}

fn explain_pod(phase: &str, ready: bool, restarts: i64, reason: &str) -> String {
    if ready && restarts == 0 {
        return crate::tr!("ทำงานปกติ พร้อมรับ traffic", "Healthy, serving traffic").into();
    }
    if reason.contains("CrashLoopBackOff") {
        return crate::tr!("แอปเปิดไม่ขึ้นแล้วรีสตาร์ทซ้ำ — แนะนำกดดู Log เพื่อเช็ค error", "App keeps crashing and restarting — open the logs to check the error").into();
    }
    if reason.contains("ImagePull") || reason.contains("ErrImage") {
        return crate::tr!("ดึง Docker Image ไม่สำเร็จ — เช็คชื่อ image หรือสิทธิ์ registry", "Could not pull the image — check the image name or registry access").into();
    }
    if reason.contains("ContainerCreating") || reason.contains("PodInitializing") {
        return crate::tr!("กำลังสร้าง Container และเริ่มต้นทำงาน...", "Creating the container and starting...").into();
    }
    if reason.contains("Pending") || phase == "Pending" {
        return crate::tr!("รอเครื่องว่าง (CPU/Memory อาจไม่พอ)", "Waiting for a node (CPU/memory may be short)").into();
    }
    if restarts > 0 {
        return crate::tf!("เคยรีสตาร์ทไป {restarts} ครั้ง (สถานะล่าสุด: {reason})", "Restarted {restarts} times (last state: {reason})");
    }
    crate::tf!("สถานะ: {phase} ({reason})", "Status: {phase} ({reason})")
}

fn build_summary(role: NodeRole, health: Health, ready: i64, desired: i64, hosts: &[String]) -> String {
    let host_note = if hosts.is_empty() {
        String::new()
    } else {
        format!(" • {}", hosts[0])
    };
    crate::tf!("{} ({ready}/{desired} พร้อมใช้){host_note} — {}", "{} ({ready}/{desired} ready){host_note} — {}", role.label(), health.simple_thai())
}

pub fn build_mermaid(
    env: EnvKind,
    namespace: &str,
    ingresses: &[IngressRoute],
    services: &[ServiceInfo],
) -> String {
    let mut lines = vec![
        "flowchart TD".to_string(),
        format!("    INET([\"Internet / Users\"]) --> INGRESS[\"Nginx Ingress ({}) \"]", env.label()),
    ];

    let mut linked_services = BTreeSet::new();
    for (idx, ing) in ingresses.iter().take(8).enumerate() {
        let host_node = format!("HOST_{idx}");
        let svc_id = sanitize_id(&ing.backend_service);
        lines.push(format!("    INGRESS --> {host_node}[\"{}\"]", ing.host));
        lines.push(format!("    {host_node} --> {svc_id}[\"{}\"]", ing.backend_service));
        linked_services.insert(ing.backend_service.clone());
    }

    // Add remaining services (workers / caches) up to a readable cap
    let mut shown = 0;
    for svc in services {
        let svc_id = sanitize_id(&svc.name);
        if !linked_services.contains(&svc.name) && svc.role != NodeRole::Cache && shown < 6 {
            let tag = if svc.role == NodeRole::Worker { "[Worker]" } else { "[API]" };
            lines.push(format!("    INGRESS -.-> {svc_id}[\"{tag} {}\"]", svc.name));
            shown += 1;
        }
        for target in svc.connects_to.iter().take(2) {
            let t_id = sanitize_id(target);
            lines.push(format!("    {svc_id} --> {t_id}[(\"{target}\")]"));
        }
    }

    if ingresses.is_empty() && services.is_empty() {
        lines.push(format!("    INGRESS --> EMPTY[\"No services in {namespace}\"]"));
    }

    lines.join("\n")
}

fn parse_pod_resources(containers: &Value) -> (String, String) {
    let mut cpu_reqs = Vec::new();
    let mut cpu_lims = Vec::new();
    let mut mem_reqs = Vec::new();
    let mut mem_lims = Vec::new();
    if let Some(arr) = containers.as_array() {
        for c in arr {
            if let Some(v) = c["resources"]["requests"]["cpu"].as_str() { cpu_reqs.push(v.to_string()); }
            if let Some(v) = c["resources"]["limits"]["cpu"].as_str() { cpu_lims.push(v.to_string()); }
            if let Some(v) = c["resources"]["requests"]["memory"].as_str() { mem_reqs.push(v.to_string()); }
            if let Some(v) = c["resources"]["limits"]["memory"].as_str() { mem_lims.push(v.to_string()); }
        }
    }
    let cpu = match (cpu_reqs.first(), cpu_lims.first()) {
        (Some(r), Some(l)) => format!("CPU Req {r} / Lim {l}"),
        (Some(r), None) => format!("CPU Req {r}"),
        (None, Some(l)) => format!("CPU Lim {l}"),
        (None, None) => crate::tr!("CPU: ไม่จำกัด", "CPU: unlimited").into(),
    };
    let mem = match (mem_reqs.first(), mem_lims.first()) {
        (Some(r), Some(l)) => format!("RAM Req {r} / Lim {l}"),
        (Some(r), None) => format!("RAM Req {r}"),
        (None, Some(l)) => format!("RAM Lim {l}"),
        (None, None) => crate::tr!("RAM: ไม่จำกัด", "RAM: unlimited").into(),
    };
    (cpu, mem)
}

fn sanitize_id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

pub fn default_overview() -> Vec<NamespaceSummary> {
    let mut out = Vec::new();
    for env in [EnvKind::Stg, EnvKind::Prod] {
        for ns in env.default_namespaces() {
            let snap = demo_snapshot(env, ns, "gke-cluster", None);
            let total_services = snap.services.len();
            let healthy = snap.services.iter().filter(|s| s.health == Health::Healthy).count();
            let warning = snap.services.iter().filter(|s| s.health == Health::Warning).count();
            let error = snap.services.iter().filter(|s| s.health == Health::Error).count();
            out.push(NamespaceSummary {
                env,
                namespace: ns.to_string(),
                total_services,
                healthy,
                warning,
                error,
                ingress_count: snap.ingresses.len(),
                is_demo: true,
            });
        }
    }
    out
}

pub fn build_service_mermaid(svc: &ServiceInfo) -> String {
    let mut lines = vec!["flowchart TD".to_string()];
    let svc_id = sanitize_id(&svc.name);
    if svc.hosts.is_empty() {
        lines.push(format!("    CLUSTER([\"Internal Cluster\"]) --> {svc_id}[\"{}\"]", svc.name));
    } else {
        for (i, h) in svc.hosts.iter().enumerate() {
            lines.push(format!("    INET([\"Internet\"]) --> H_{i}[\"{h}\"]"));
            lines.push(format!("    H_{i} --> {svc_id}[\"{}\"]", svc.name));
        }
    }
    for (i, p) in svc.pods.iter().take(4).enumerate() {
        lines.push(format!("    {svc_id} --> POD_{i}[\"Pod: {}\"]", p.name));
    }
    for t in &svc.connects_to {
        let tid = sanitize_id(t);
        lines.push(format!("    {svc_id} --> {tid}[(\"{t}\")]"));
    }
    lines.join("\n")
}

pub fn demo_snapshot(env: EnvKind, namespace: &str, context: &str, error: Option<String>) -> ClusterSnapshot {
    let prefix = if env == EnvKind::Prod { "prod" } else { "staging" };
    let domain_suffix = if env == EnvKind::Prod { "aombet.net" } else { "staging.aombet.net" };

    let ingresses = vec![
        IngressRoute {
            name: format!("ingress-pg-service"),
            host: format!("pg.handler.{domain_suffix}"),
            backend_service: format!("{prefix}-pg-srv"),
            path: "/".into(),
        },
        IngressRoute {
            name: format!("ingress-game-service"),
            host: format!("game-service.handler.{domain_suffix}"),
            backend_service: format!("{prefix}-game-service-srv"),
            path: "/".into(),
        },
        IngressRoute {
            name: format!("ingress-jili-service"),
            host: format!("jili.handler.{domain_suffix}"),
            backend_service: format!("{prefix}-jili-srv"),
            path: "/".into(),
        },
    ];

    let services = vec![
        ServiceInfo {
            name: format!("{prefix}-game-service-srv"),
            deploy_name: Some(format!("{prefix}-game-service-depl")),
            namespace: namespace.into(),
            role: NodeRole::Api,
            health: Health::Healthy,
            ready_replicas: 1,
            desired_replicas: 1,
            ports: "3010".into(),
            hosts: vec![format!("game-service.handler.{domain_suffix}")],
            connects_to: vec![format!("{prefix}-redis-srv")],
            summary: crate::tr!("API / Service (1/1 พร้อมใช้) — ทำงานปกติ", "API / Service (1/1 ready) — healthy").into(),
            pods: vec![PodInfo {
                name: format!("{prefix}-game-service-depl-7f9b8c"),
                phase: "Running".into(),
                ready: true,
                restarts: 0,
                reason: "Running".into(),
                explanation: crate::tr!("ทำงานปกติ พร้อมรับ traffic", "Healthy, serving traffic").into(),
                node_name: "gke-aombet-pool-1".into(),
                node_spec: "e2-standard-4 • 4 vCPU / 16.0 GB RAM".into(),
                cpu_spec: "CPU Req 100m / Lim 500m".into(),
                mem_spec: "RAM Req 128Mi / Lim 512Mi".into(),
                live_usage: crate::tr!("ใช้จริง: CPU 18m • RAM 164Mi", "Usage: CPU 18m • RAM 164Mi").into(),
            }],
        },
        ServiceInfo {
            name: format!("{prefix}-pg-srv"),
            deploy_name: Some(format!("{prefix}-pg-depl")),
            namespace: namespace.into(),
            role: NodeRole::Api,
            health: Health::Healthy,
            ready_replicas: 1,
            desired_replicas: 1,
            ports: "3010".into(),
            hosts: vec![format!("pg.handler.{domain_suffix}")],
            connects_to: vec![format!("{prefix}-redis-srv")],
            summary: crate::tr!("API / Service (1/1 พร้อมใช้) — ทำงานปกติ", "API / Service (1/1 ready) — healthy").into(),
            pods: vec![PodInfo {
                name: format!("{prefix}-pg-depl-5d4a2b"),
                phase: "Running".into(),
                ready: true,
                restarts: 0,
                reason: "Running".into(),
                explanation: crate::tr!("ทำงานปกติ พร้อมรับ traffic", "Healthy, serving traffic").into(),
                node_name: "gke-aombet-pool-1".into(),
                node_spec: "e2-standard-4 • 4 vCPU / 16.0 GB RAM".into(),
                cpu_spec: "CPU Req 100m / Lim 500m".into(),
                mem_spec: "RAM Req 128Mi / Lim 512Mi".into(),
                live_usage: crate::tr!("ใช้จริง: CPU 12m • RAM 142Mi", "Usage: CPU 12m • RAM 142Mi").into(),
            }],
        },
        ServiceInfo {
            name: format!("{prefix}-jili-srv"),
            deploy_name: Some(format!("{prefix}-jili-depl")),
            namespace: namespace.into(),
            role: NodeRole::Api,
            health: if env == EnvKind::Stg { Health::Warning } else { Health::Healthy },
            ready_replicas: 1,
            desired_replicas: 1,
            ports: "3010".into(),
            hosts: vec![format!("jili.handler.{domain_suffix}")],
            connects_to: vec![format!("{prefix}-hangfire-redis-srv")],
            summary: if env == EnvKind::Stg {
                crate::tr!("API / Service (1/1 พร้อมใช้) — มีรีสตาร์ท 2 ครั้ง", "API / Service (1/1 ready) — restarted 2 times").into()
            } else {
                crate::tr!("API / Service (1/1 พร้อมใช้) — ทำงานปกติ", "API / Service (1/1 ready) — healthy").into()
            },
            pods: vec![PodInfo {
                name: format!("{prefix}-jili-depl-9c1e3f"),
                phase: "Running".into(),
                ready: true,
                restarts: if env == EnvKind::Stg { 2 } else { 0 },
                reason: "Running".into(),
                explanation: if env == EnvKind::Stg {
                    crate::tr!("เคยรีสตาร์ทไป 2 ครั้ง", "Restarted 2 times").into()
                } else {
                    crate::tr!("ทำงานปกติ พร้อมรับ traffic", "Healthy, serving traffic").into()
                },
                node_name: "gke-aombet-pool-2".into(),
                node_spec: "e2-standard-4 • 4 vCPU / 16.0 GB RAM".into(),
                cpu_spec: "CPU Req 100m / Lim 500m".into(),
                mem_spec: "RAM Req 128Mi / Lim 512Mi".into(),
                live_usage: crate::tr!("ใช้จริง: CPU 24m • RAM 189Mi", "Usage: CPU 24m • RAM 189Mi").into(),
            }],
        },
        ServiceInfo {
            name: format!("{prefix}-hangfire-worker"),
            deploy_name: Some(format!("{prefix}-psg-hangfire-depl")),
            namespace: namespace.into(),
            role: NodeRole::Worker,
            health: Health::Healthy,
            ready_replicas: 1,
            desired_replicas: 1,
            ports: "-".into(),
            hosts: vec![],
            connects_to: vec![format!("{prefix}-hangfire-redis-srv")],
            summary: crate::tr!("Worker / Cron (1/1 พร้อมใช้) — ทำงานปกติ", "Worker / Cron (1/1 ready) — healthy").into(),
            pods: vec![],
        },
        ServiceInfo {
            name: format!("{prefix}-redis-srv"),
            deploy_name: Some(format!("{prefix}-redis-depl")),
            namespace: namespace.into(),
            role: NodeRole::Cache,
            health: Health::Healthy,
            ready_replicas: 1,
            desired_replicas: 1,
            ports: "6379".into(),
            hosts: vec![],
            connects_to: vec![],
            summary: crate::tr!("Redis / Cache (1/1 พร้อมใช้) — ทำงานปกติ", "Redis / Cache (1/1 ready) — healthy").into(),
            pods: vec![],
        },
    ];

    let mermaid_diagram = build_mermaid(env, namespace, &ingresses, &services);
    ClusterSnapshot {
        env,
        namespace: namespace.to_string(),
        context: context.to_string(),
        services,
        ingresses,
        mermaid_diagram,
        is_demo: true,
        error,
        updated_at: "Demo Mode".into(),
    }
}
