use crate::auth::{self, AuthEvent, GoogleAuthConfig, Session, UserAccount, spawn_google_login};
use crate::k8s::{
    ClusterSnapshot, EnvKind, EnvSourceKind, EnvVarItem, K8sEvent, NamespaceSummary, ServiceInfo,
    default_overview, demo_snapshot, spawn_fetch_env, spawn_fetch_logs, spawn_fetch_overview,
    spawn_fetch_snapshot, spawn_list_all_namespaces, spawn_restart_deploy, spawn_update_env,
};
use crate::pty_k9s::{K9sPty, K9sScreen};
use crate::ui::topology_g6::{G6Command, TopologyViewMode, TopologyWebEvent};
use rune::runtime::Waker;
use rune::{Live, TextEdit};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Page {
    Overview,
    Namespace,
    ServiceDetail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Tab {
    Topology,
    Services,
    K9sCli,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ServiceTab {
    Info,
    Logs,
    Env,
    Diagram,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverviewFilter {
    #[default]
    All,
    Staging,
    Production,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceTab {
    pub id: usize,
    pub title: String,
    pub page: Page,
    pub ns_tab: Tab,
    pub service_tab: ServiceTab,
    pub env: EnvKind,
    pub namespace: String,
    pub service_name: Option<String>,
    #[serde(default)]
    pub env_search: String,
    #[serde(default)]
    pub svc_search: String,
}

pub struct AppState {
    // Auth
    pub accounts: Vec<UserAccount>,
    pub google_config: GoogleAuthConfig,
    pub google_loading: bool,
    pub session: Option<Session>,
    pub login_user: TextEdit,
    pub login_pass: TextEdit,
    pub login_error: Option<String>,

    // Navigation & Environment
    pub open_tabs: Vec<WorkspaceTab>,
    pub active_tab_id: usize,
    pub next_tab_id: usize,
    pub page: Page,
    pub tab: Tab,
    pub service_tab: ServiceTab,
    pub env: EnvKind,
    pub namespace: String,
    pub dark: bool,

    // Overview + Cluster Data
    pub overview_filter: OverviewFilter,
    pub pending_service_tab: Option<ServiceTab>,
    pub stg_namespaces: Vec<String>,
    pub prod_namespaces: Vec<String>,
    pub available_namespaces: Vec<String>,
    pub adding_ns: bool,
    pub new_ns_input: TextEdit,
    pub new_ns_env: EnvKind,
    pub overview: Vec<NamespaceSummary>,
    pub overview_loading: bool,
    pub snapshot: ClusterSnapshot,
    pub loading: bool,
    pub search: TextEdit,
    pub selected_service: Option<String>,
    pub diagram_zoom: f32,
    pub diagram_pan_x: f32,
    pub diagram_pan_y: f32,
    pub diagram_drag_start_x: f32,
    pub diagram_drag_start_y: f32,
    pub diagram_fullscreen: bool,
    pub topology_mode: TopologyViewMode,
    pub g6_viewport_rect: Live<[f32; 4]>,
    pub g6_window_visible: bool,
    pub g6_ready: Arc<AtomicBool>,
    pub g6_error: Option<String>,
    pub pulse_on: bool,
    pub logs_pod: Option<String>,
    pub logs_text: Option<String>,
    pub log_newest_first: bool,
    pub log_timestamps: bool,
    pub log_since: Option<String>,
    pub log_search: TextEdit,
    pub env_vars: Vec<EnvVarItem>,
    pub env_search: TextEdit,
    pub show_secrets: bool,
    pub editing_env_key: Option<String>,
    pub editing_env_input: TextEdit,
    pub adding_env: bool,
    pub new_env_key: TextEdit,
    pub new_env_val: TextEdit,
    pub banner: Option<(String, bool)>,

    // K9s PTY
    pub k9s_pty: K9sPty,
    pub k9s_screen: K9sScreen,

    // Async Channels
    pub waker: Option<Waker>,
    pub auth_tx: mpsc::Sender<AuthEvent>,
    pub auth_rx: mpsc::Receiver<AuthEvent>,
    pub k8s_tx: mpsc::Sender<K8sEvent>,
    pub k8s_rx: mpsc::Receiver<K8sEvent>,
    pub pty_tx: mpsc::Sender<K9sScreen>,
    pub pty_rx: mpsc::Receiver<K9sScreen>,
    pub g6_tx: mpsc::Sender<TopologyWebEvent>,
    pub g6_rx: mpsc::Receiver<TopologyWebEvent>,
    pub g6_cmd_tx: mpsc::Sender<G6Command>,
    pub g6_cmd_rx: mpsc::Receiver<G6Command>,
}

impl AppState {
    pub fn init() -> Self {
        let (auth_tx, auth_rx) = mpsc::channel();
        let (k8s_tx, k8s_rx) = mpsc::channel();
        let (pty_tx, pty_rx) = mpsc::channel();
        let (g6_tx, g6_rx) = mpsc::channel();
        let (g6_cmd_tx, g6_cmd_rx) = mpsc::channel();
        let (stg_namespaces, prod_namespaces) = load_saved_namespaces();
        let (open_tabs, active_tab_id, next_tab_id) = load_saved_tabs(&stg_namespaces);
        let active_tab = open_tabs
            .iter()
            .find(|t| t.id == active_tab_id)
            .cloned()
            .unwrap_or_else(|| open_tabs[0].clone());
        let env = active_tab.env;
        let namespace = active_tab.namespace.clone();
        let snapshot = demo_snapshot(env, &namespace, "gke-cluster", None);

        Self {
            accounts: auth::default_accounts(),
            google_config: GoogleAuthConfig::load(),
            google_loading: false,
            session: None,
            login_user: TextEdit::new(""),
            login_pass: TextEdit::new(""),
            login_error: None,
            open_tabs,
            active_tab_id,
            next_tab_id,
            page: active_tab.page,
            tab: active_tab.ns_tab,
            service_tab: active_tab.service_tab,
            env,
            namespace,
            dark: true,
            overview_filter: OverviewFilter::All,
            pending_service_tab: None,
            stg_namespaces,
            prod_namespaces,
            available_namespaces: Vec::new(),
            adding_ns: false,
            new_ns_input: TextEdit::new(""),
            new_ns_env: EnvKind::Stg,
            overview: default_overview(),
            overview_loading: false,
            snapshot,
            loading: false,
            search: TextEdit::new(active_tab.svc_search.clone()),
            selected_service: active_tab.service_name.clone(),
            diagram_zoom: 1.0,
            diagram_pan_x: 24.0,
            diagram_pan_y: 20.0,
            diagram_drag_start_x: 24.0,
            diagram_drag_start_y: 20.0,
            diagram_fullscreen: false,
            topology_mode: TopologyViewMode::G6,
            g6_viewport_rect: Live::new([0.0; 4]),
            g6_window_visible: true,
            g6_ready: Arc::new(AtomicBool::new(false)),
            g6_error: None,
            pulse_on: true,
            logs_pod: None,
            logs_text: None,
            log_newest_first: true,
            log_timestamps: false,
            log_since: None,
            log_search: TextEdit::new(""),
            env_vars: Vec::new(),
            env_search: TextEdit::new(active_tab.env_search),
            show_secrets: false,
            editing_env_key: None,
            editing_env_input: TextEdit::new(""),
            adding_env: false,
            new_env_key: TextEdit::new(""),
            new_env_val: TextEdit::new(""),
            banner: None,
            k9s_pty: K9sPty::new(),
            k9s_screen: K9sScreen::default(),
            waker: None,
            auth_tx,
            auth_rx,
            k8s_tx,
            k8s_rx,
            pty_tx,
            pty_rx,
            g6_tx,
            g6_rx,
            g6_cmd_tx,
            g6_cmd_rx,
        }
    }

    pub fn login_with_google(&mut self, force_browser: bool) {
        self.google_loading = true;
        self.login_error = None;
        let cfg = self.google_config.clone();
        let tx = self.auth_tx.clone();
        let waker = self.waker.clone();
        spawn_google_login(cfg, force_browser, tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    pub fn try_login(&mut self) {
        match auth::authenticate(&self.accounts, &self.login_user.text, &self.login_pass.text) {
            Ok(sess) => {
                self.session = Some(sess);
                self.login_error = None;
                self.login_pass = TextEdit::new("");
                self.refresh_overview();
                self.refresh_cluster();
            }
            Err(e) => {
                self.login_error = Some(e);
            }
        }
    }

    pub fn logout(&mut self) {
        self.k9s_pty.stop();
        self.k9s_screen = K9sScreen::default();
        self.session = None;
        self.banner = None;
        self.diagram_fullscreen = false;
        self.page = Page::Overview;
    }

    pub fn sync_active_tab(&mut self) {
        let title = match self.page {
            Page::Overview => "Overview".to_string(),
            Page::Namespace => format!("{} • {}", self.env.label(), self.namespace),
            Page::ServiceDetail => format!(
                "{} • {}",
                self.env.label(),
                self.selected_service.as_deref().unwrap_or("Service")
            ),
        };
        if let Some(t) = self.open_tabs.iter_mut().find(|t| t.id == self.active_tab_id) {
            t.title = title;
            t.page = self.page;
            t.ns_tab = self.tab;
            t.service_tab = self.service_tab;
            t.env = self.env;
            t.namespace = self.namespace.clone();
            t.service_name = self.selected_service.clone();
            t.env_search = self.env_search.text.clone();
            t.svc_search = self.search.text.clone();
        }
        save_tabs(&self.open_tabs, self.active_tab_id);
    }

    pub fn new_workspace_tab(&mut self) {
        self.sync_active_tab();
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let wt = WorkspaceTab {
            id,
            title: "Overview".into(),
            page: Page::Overview,
            ns_tab: Tab::Topology,
            service_tab: ServiceTab::Info,
            env: self.env,
            namespace: self.namespace.clone(),
            service_name: None,
            env_search: String::new(),
            svc_search: String::new(),
        };
        self.open_tabs.push(wt);
        self.active_tab_id = id;
        self.page = Page::Overview;
        self.selected_service = None;
        self.search = TextEdit::new("");
        self.env_search = TextEdit::new("");
        save_tabs(&self.open_tabs, self.active_tab_id);
    }

    pub fn switch_workspace_tab(&mut self, id: usize) {
        if self.active_tab_id == id {
            return;
        }
        self.sync_active_tab();
        let Some(target) = self.open_tabs.iter().find(|t| t.id == id).cloned() else {
            return;
        };
        let ns_changed = self.env != target.env || self.namespace != target.namespace;
        let svc_changed = self.selected_service != target.service_name;
        self.active_tab_id = id;
        self.page = target.page;
        self.tab = target.ns_tab;
        self.service_tab = target.service_tab;
        self.env = target.env;
        self.namespace = target.namespace;
        self.selected_service = target.service_name.clone();
        self.search = TextEdit::new(target.svc_search);
        self.env_search = TextEdit::new(target.env_search);
        save_tabs(&self.open_tabs, self.active_tab_id);
        if ns_changed {
            self.refresh_cluster();
        }
        if self.page == Page::ServiceDetail && (ns_changed || svc_changed || self.env_vars.is_empty()) {
            if let Some(svc_name) = target.service_name {
                self.load_service_data(&svc_name);
            }
        }
    }

    pub fn close_workspace_tab(&mut self, id: usize) {
        if self.open_tabs.len() <= 1 {
            return;
        }
        self.open_tabs.retain(|t| t.id != id);
        if self.active_tab_id == id {
            if let Some(last) = self.open_tabs.last().cloned() {
                self.active_tab_id = 0; // force switch
                self.switch_workspace_tab(last.id);
                return;
            }
        }
        save_tabs(&self.open_tabs, self.active_tab_id);
    }

    pub fn open_overview(&mut self) {
        self.page = Page::Overview;
        self.selected_service = None;
        self.sync_active_tab();
    }

    pub fn drill_into_namespace(&mut self, env: EnvKind, ns: &str) {
        self.pending_service_tab = None;
        let changed = self.env != env || self.namespace != ns;
        self.env = env;
        self.namespace = ns.to_string();
        self.page = Page::Namespace;
        self.selected_service = None;
        self.logs_pod = None;
        self.logs_text = None;
        self.env_vars.clear();
        self.sync_active_tab();
        if changed {
            self.refresh_cluster();
            if self.k9s_pty.is_running() {
                self.start_k9s();
            }
        }
    }

    pub fn drill_into_namespace_tab(&mut self, env: EnvKind, ns: &str, tab: Tab) {
        self.tab = tab;
        self.drill_into_namespace(env, ns);
        if tab == Tab::K9sCli && !self.k9s_pty.is_running() {
            self.start_k9s();
        }
    }

    /// Navigates into a namespace and opens its first service directly on the requested
    /// `ServiceTab` (e.g. `Logs` or `Env`). If the namespace's snapshot is not yet loaded,
    /// triggers a cluster fetch and automatically drills in once the snapshot arrives.
    pub fn drill_into_namespace_service_tab(&mut self, env: EnvKind, ns: &str, service_tab: ServiceTab) {
        let same_ns = self.env == env
            && self.namespace == ns
            && self.snapshot.namespace == ns
            && !self.snapshot.services.is_empty();
        if same_ns {
            let first_svc = self.snapshot.services[0].name.clone();
            self.drill_into_service(first_svc, service_tab);
            return;
        }
        self.tab = Tab::Services;
        self.drill_into_namespace(env, ns);
        self.pending_service_tab = Some(service_tab);
    }

    pub fn drill_into_service(&mut self, service_name: String, initial_tab: ServiceTab) {
        self.selected_service = Some(service_name.clone());
        self.service_tab = initial_tab;
        self.page = Page::ServiceDetail;
        self.show_secrets = false;
        self.env_search = TextEdit::new("");
        self.sync_active_tab();
        self.load_service_data(&service_name);
    }

    pub fn load_service_data(&mut self, service_name: &str) {
        if let Some(svc) = self.snapshot.services.iter().find(|s| s.name == service_name).cloned() {
            if let Some(pod) = svc.pods.first() {
                self.fetch_logs(pod.name.clone());
            } else {
                self.logs_pod = None;
                self.logs_text = Some("ยังไม่มี Pod ที่รันอยู่สำหรับ service นี้".into());
            }
            if let Some(deploy) = svc.deploy_name {
                self.fetch_env(deploy);
            } else {
                self.env_vars.clear();
            }
        }
    }

    pub fn open_add_namespace(&mut self) {
        self.adding_ns = !self.adding_ns;
        if self.adding_ns {
            let tx = self.k8s_tx.clone();
            let waker = self.waker.clone();
            spawn_list_all_namespaces(tx, move || {
                if let Some(w) = &waker {
                    w();
                }
            });
        }
    }

    pub fn add_namespace(&mut self, env: EnvKind, ns: String) {
        let ns = ns.trim().to_string();
        if ns.is_empty() {
            return;
        }
        let target = match env {
            EnvKind::Stg => &mut self.stg_namespaces,
            EnvKind::Prod => &mut self.prod_namespaces,
        };
        if !target.iter().any(|x| x == &ns) {
            target.push(ns.clone());
            save_namespaces(&self.stg_namespaces, &self.prod_namespaces);
            self.banner = Some((format!("เพิ่ม Namespace {ns} เข้าฝั่ง {} แล้ว", env.label()), true));
            self.refresh_overview();
        }
        self.new_ns_input = TextEdit::new("");
        self.adding_ns = false;
    }

    pub fn remove_namespace(&mut self, env: EnvKind, ns: &str) {
        let target = match env {
            EnvKind::Stg => &mut self.stg_namespaces,
            EnvKind::Prod => &mut self.prod_namespaces,
        };
        if target.len() > 1 {
            target.retain(|x| x != ns);
            save_namespaces(&self.stg_namespaces, &self.prod_namespaces);
            self.refresh_overview();
        }
    }

    pub fn refresh_overview(&mut self) {
        self.overview_loading = true;
        let tx = self.k8s_tx.clone();
        let stg = self.stg_namespaces.clone();
        let prod = self.prod_namespaces.clone();
        let waker = self.waker.clone();
        spawn_fetch_overview(stg, prod, tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    pub fn refresh_cluster(&mut self) {
        self.loading = true;
        let tx = self.k8s_tx.clone();
        let env = self.env;
        let ns = self.namespace.clone();
        let waker = self.waker.clone();
        spawn_fetch_snapshot(env, ns, tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    pub fn fetch_logs(&mut self, pod: String) {
        self.logs_pod = Some(pod.clone());
        self.logs_text = Some("กำลังโหลด logs...".into());
        let tx = self.k8s_tx.clone();
        let ns = self.namespace.clone();
        let since = self.log_since.clone();
        let timestamps = self.log_timestamps;
        let waker = self.waker.clone();
        spawn_fetch_logs(ns, pod, since, timestamps, tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    pub fn copy_to_clipboard(&mut self, text: String, label: &str) {
        use std::io::Write;
        #[cfg(target_os = "windows")]
        let mut cmd = std::process::Command::new("clip");
        #[cfg(target_os = "macos")]
        let mut cmd = std::process::Command::new("pbcopy");
        #[cfg(all(unix, not(target_os = "macos")))]
        let mut cmd = {
            let mut c = std::process::Command::new("xclip");
            c.args(["-selection", "clipboard"]);
            c
        };
        if let Ok(mut child) = cmd
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
            self.banner = Some((format!("คัดลอก {label} ไปยัง Clipboard แล้ว"), true));
        }
    }

    pub fn start_edit_env(&mut self, key: String, current_val: String) {
        self.editing_env_key = Some(key);
        self.editing_env_input = TextEdit::new(current_val);
        self.adding_env = false;
    }

    pub fn cancel_edit_env(&mut self) {
        self.editing_env_key = None;
        self.adding_env = false;
    }

    fn can_mutate_current_env(&mut self) -> bool {
        if let Some(sess) = &self.session {
            let allowed = match self.env {
                EnvKind::Prod => sess.role.can_mutate_prod(),
                EnvKind::Stg => sess.role.can_mutate_stg(),
            };
            if !allowed {
                self.banner = Some((
                    format!("สิทธิ์ {} ไม่สามารถแก้ไข ENV บน {} ได้", sess.role.label(), self.env.label()),
                    false,
                ));
                return false;
            }
        }
        true
    }

    pub fn save_editing_env(&mut self, restart_after: bool) {
        let Some(key) = self.editing_env_key.clone() else { return };
        if !self.can_mutate_current_env() {
            return;
        }
        let new_val = self.editing_env_input.text.clone();
        let source_kind = self
            .env_vars
            .iter()
            .find(|v| v.name == key)
            .map(|v| v.source_kind.clone())
            .unwrap_or(EnvSourceKind::Inline);
        let deploy = self
            .selected_service_info()
            .and_then(|s| s.deploy_name.clone())
            .unwrap_or_else(|| self.selected_service.clone().unwrap_or_default());

        // Optimistically update UI list
        if let Some(item) = self.env_vars.iter_mut().find(|v| v.name == key) {
            item.value = new_val.clone();
        }
        self.editing_env_key = None;

        let tx = self.k8s_tx.clone();
        let ns = self.namespace.clone();
        let waker = self.waker.clone();
        spawn_update_env(ns, deploy, source_kind, key, new_val, restart_after, tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    pub fn save_new_env(&mut self, restart_after: bool) {
        let key = self.new_env_key.text.trim().to_string();
        if key.is_empty() {
            self.banner = Some(("กรุณากรอกชื่อ Key ของ ENV".into(), false));
            return;
        }
        if !self.can_mutate_current_env() {
            return;
        }
        let new_val = self.new_env_val.text.clone();
        let source_kind = self
            .env_vars
            .first()
            .map(|v| v.source_kind.clone())
            .unwrap_or(EnvSourceKind::Inline);
        let source_label = match &source_kind {
            EnvSourceKind::Secret(s) => format!("Secret: {s}"),
            EnvSourceKind::ConfigMap(c) => format!("ConfigMap: {c}"),
            EnvSourceKind::Inline => "Inline".into(),
            EnvSourceKind::Demo => "Demo".into(),
        };
        let is_secret = matches!(source_kind, EnvSourceKind::Secret(_));
        let deploy = self
            .selected_service_info()
            .and_then(|s| s.deploy_name.clone())
            .unwrap_or_else(|| self.selected_service.clone().unwrap_or_default());

        if let Some(existing) = self.env_vars.iter_mut().find(|v| v.name == key) {
            existing.value = new_val.clone();
        } else {
            self.env_vars.push(EnvVarItem {
                name: key.clone(),
                value: new_val.clone(),
                is_secret,
                source: source_label,
                source_kind: source_kind.clone(),
            });
        }
        self.adding_env = false;
        self.new_env_key = TextEdit::new("");
        self.new_env_val = TextEdit::new("");

        let tx = self.k8s_tx.clone();
        let ns = self.namespace.clone();
        let waker = self.waker.clone();
        spawn_update_env(ns, deploy, source_kind, key, new_val, restart_after, tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    pub fn export_env_csv(&mut self) {
        let svc = self.selected_service.as_deref().unwrap_or("service");
        let query = self.env_search.text.trim().to_ascii_lowercase();
        let mut csv = String::from("Key,Value,Source,IsSecret\n");
        let mut count = 0usize;

        for ev in &self.env_vars {
            let match_key = ev.name.to_ascii_lowercase().contains(&query);
            let match_src = ev.source.to_ascii_lowercase().contains(&query);
            let match_val = ev.value.to_ascii_lowercase().contains(&query);
            if !query.is_empty() && !match_key && !match_src && !match_val {
                continue;
            }
            count += 1;
            csv.push_str(&format!(
                "{},{},{},{}\n",
                csv_escape(&ev.name),
                csv_escape(&ev.value),
                csv_escape(&ev.source),
                ev.is_secret
            ));
        }

        #[cfg(target_os = "windows")]
        let default_dir = std::env::var("USERPROFILE")
            .map(|p| format!("{p}\\Downloads"))
            .unwrap_or_else(|_| ".".into());
        #[cfg(not(target_os = "windows"))]
        let default_dir = std::env::var("HOME")
            .map(|p| format!("{p}/Downloads"))
            .unwrap_or_else(|_| ".".into());
        let file_name = format!("{}-{}-env.csv", svc, self.namespace);
        let tx = self.k8s_tx.clone();
        let waker = self.waker.clone();

        std::thread::spawn(move || {
            #[cfg(target_os = "windows")]
            let out = {
                let ps_script = format!(
                    "Add-Type -AssemblyName System.Windows.Forms; \
                     $d = New-Object System.Windows.Forms.SaveFileDialog; \
                     $d.Filter = 'CSV Files (*.csv)|*.csv|All Files (*.*)|*.*'; \
                     $d.InitialDirectory = '{}'; \
                     $d.FileName = '{}'; \
                     $d.Title = 'เลือกตำแหน่งบันทึกไฟล์ ENV CSV'; \
                     if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ Write-Output $d.FileName }}",
                    default_dir.replace('\'', "''"),
                    file_name.replace('\'', "''"),
                );
                std::process::Command::new("powershell")
                    .args(["-NoProfile", "-STA", "-Command", &ps_script])
                    .output()
            };
            #[cfg(target_os = "macos")]
            let out = {
                let osa_script = format!(
                    "POSIX path of (choose file name with prompt \"เลือกตำแหน่งบันทึกไฟล์ ENV CSV\" default name \"{}\" default location POSIX file \"{}\")",
                    file_name.replace('"', "\\\""),
                    default_dir.replace('"', "\\\""),
                );
                std::process::Command::new("osascript")
                    .args(["-e", &osa_script])
                    .output()
            };
            #[cfg(all(unix, not(target_os = "macos")))]
            let out: std::io::Result<std::process::Output> = Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Save dialog unsupported",
            ));

            if let Ok(o) = out {
                let chosen = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !chosen.is_empty() {
                    let msg = match std::fs::write(&chosen, csv) {
                        Ok(_) => (format!("Export ENV ({count} รายการ) ไปที่ {chosen} สำเร็จ"), true),
                        Err(e) => (format!("Export CSV ไม่สำเร็จ: {e}"), false),
                    };
                    let _ = tx.send(K8sEvent::ActionResult {
                        message: msg.0,
                        ok: msg.1,
                    });
                    if let Some(w) = &waker {
                        w();
                    }
                }
            }
        });
    }

    pub fn fetch_env(&mut self, deploy: String) {
        let tx = self.k8s_tx.clone();
        let ns = self.namespace.clone();
        let waker = self.waker.clone();
        spawn_fetch_env(ns, deploy, tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    pub fn restart_deploy(&mut self, deploy: String) {
        if let Some(sess) = &self.session {
            let allowed = match self.env {
                EnvKind::Prod => sess.role.can_mutate_prod(),
                EnvKind::Stg => sess.role.can_mutate_stg(),
            };
            if !allowed {
                self.banner = Some((
                    format!("สิทธิ์ {} ไม่สามารถสั่ง Restart บน {} ได้", sess.role.label(), self.env.label()),
                    false,
                ));
                return;
            }
        }
        let tx = self.k8s_tx.clone();
        let ns = self.namespace.clone();
        let waker = self.waker.clone();
        spawn_restart_deploy(ns, deploy, tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    pub fn start_k9s(&mut self) {
        let tx = self.pty_tx.clone();
        let ns = self.namespace.clone();
        let waker = self.waker.clone();
        let wake_fn: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            if let Some(w) = &waker {
                w();
            }
        });
        self.k9s_pty.start(&ns, tx, wake_fn);
    }

    pub fn has_any_issue(&self) -> bool {
        self.overview.iter().any(|o| o.warning > 0 || o.error > 0)
            || self.snapshot.services.iter().any(|svc| svc.health != crate::k8s::Health::Healthy)
    }

    pub fn send_g6_cmd(&self, cmd: G6Command) {
        let _ = self.g6_cmd_tx.send(cmd);
    }

    pub fn selected_service_info(&self) -> Option<&ServiceInfo> {
        let name = self.selected_service.as_deref()?;
        self.snapshot.services.iter().find(|s| s.name == name)
    }

    pub fn drain_events(&mut self) -> bool {
        let mut changed = false;
        while let Ok(ev) = self.auth_rx.try_recv() {
            changed = true;
            self.google_loading = false;
            match ev {
                AuthEvent::Success(sess) => {
                    self.session = Some(sess);
                    self.login_error = None;
                    self.refresh_overview();
                    self.refresh_cluster();
                }
                AuthEvent::Error(err) => {
                    self.login_error = Some(err);
                }
            }
        }
        while let Ok(ev) = self.k8s_rx.try_recv() {
            changed = true;
            match ev {
                K8sEvent::Snapshot(snap) => {
                    self.loading = false;
                    self.snapshot = snap;
                    if let Some(target_tab) = self.pending_service_tab.take() {
                        if let Some(first_svc) = self.snapshot.services.first().map(|s| s.name.clone()) {
                            self.drill_into_service(first_svc, target_tab);
                        }
                    } else if self.page == Page::ServiceDetail && self.env_vars.is_empty() {
                        if let Some(svc_name) = self.selected_service.clone() {
                            self.load_service_data(&svc_name);
                        }
                    }
                }
                K8sEvent::Overview(list) => {
                    self.overview_loading = false;
                    self.overview = list;
                }
                K8sEvent::AllNamespaces(list) => {
                    self.available_namespaces = list;
                }
                K8sEvent::Logs { pod, text } => {
                    if self.logs_pod.as_deref() == Some(pod.as_str()) {
                        self.logs_text = Some(text);
                    }
                }
                K8sEvent::EnvVars { vars, .. } => {
                    self.env_vars = vars;
                }
                K8sEvent::ActionResult { message, ok } => {
                    self.banner = Some((message, ok));
                    if ok {
                        self.refresh_cluster();
                        self.refresh_overview();
                    }
                }
            }
        }
        while let Ok(screen) = self.pty_rx.try_recv() {
            changed = true;
            self.k9s_screen = screen;
        }
        while let Ok(ev) = self.g6_rx.try_recv() {
            changed = true;
            match ev {
                TopologyWebEvent::Ready => {
                    self.g6_ready.store(true, Ordering::Release);
                    self.g6_error = None;
                }
                TopologyWebEvent::SelectService { name } => {
                    self.selected_service = Some(name);
                }
                TopologyWebEvent::DrillService { name } => {
                    self.diagram_fullscreen = false;
                    self.drill_into_service(name, ServiceTab::Info);
                }
                TopologyWebEvent::ClearSelection => {
                    self.selected_service = None;
                }
                TopologyWebEvent::ExitFullscreen => {
                    self.diagram_fullscreen = false;
                }
                TopologyWebEvent::LoadError { message } => {
                    self.g6_error = Some(message);
                }
            }
        }
        changed
    }
}

fn csv_escape(s: &str) -> String {
    let escaped = s.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedTabs {
    tabs: Vec<WorkspaceTab>,
    active_id: usize,
}

fn load_saved_tabs(stg_ns: &[String]) -> (Vec<WorkspaceTab>, usize, usize) {
    if let Ok(raw) = std::fs::read_to_string("k9s-ui-tabs.json") {
        if let Ok(mut parsed) = serde_json::from_str::<SavedTabs>(&raw) {
            if !parsed.tabs.is_empty() {
                for t in &mut parsed.tabs {
                    if t.title.starts_with("\u{1F3E0} ") {
                        t.title = t.title.trim_start_matches("\u{1F3E0} ").to_string();
                    }
                }
                let max_id = parsed.tabs.iter().map(|t| t.id).max().unwrap_or(1);
                return (parsed.tabs, parsed.active_id, max_id + 1);
            }
        }
    }
    let default_ns = stg_ns.first().cloned().unwrap_or_else(|| "staging-api".into());
    (
        vec![WorkspaceTab {
            id: 1,
            title: "Overview".into(),
            page: Page::Overview,
            ns_tab: Tab::Topology,
            service_tab: ServiceTab::Info,
            env: EnvKind::Stg,
            namespace: default_ns,
            service_name: None,
            env_search: String::new(),
            svc_search: String::new(),
        }],
        1,
        2,
    )
}

fn save_tabs(tabs: &[WorkspaceTab], active_id: usize) {
    let data = SavedTabs {
        tabs: tabs.to_vec(),
        active_id,
    };
    if let Ok(json) = serde_json::to_string_pretty(&data) {
        let _ = std::fs::write("k9s-ui-tabs.json", json);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedNamespaces {
    stg: Vec<String>,
    prod: Vec<String>,
}

fn load_saved_namespaces() -> (Vec<String>, Vec<String>) {
    if let Ok(raw) = std::fs::read_to_string("k9s-ui-namespaces.json") {
        if let Ok(parsed) = serde_json::from_str::<SavedNamespaces>(&raw) {
            if !parsed.stg.is_empty() && !parsed.prod.is_empty() {
                return (parsed.stg, parsed.prod);
            }
        }
    }
    (
        EnvKind::Stg.default_namespaces().into_iter().map(String::from).collect(),
        EnvKind::Prod.default_namespaces().into_iter().map(String::from).collect(),
    )
}

fn save_namespaces(stg: &[String], prod: &[String]) {
    let data = SavedNamespaces {
        stg: stg.to_vec(),
        prod: prod.to_vec(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&data) {
        let _ = std::fs::write("k9s-ui-namespaces.json", json);
    }
}
