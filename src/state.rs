use crate::auth::{self, AuthEvent, GoogleAuthConfig, Session, UserAccount, spawn_google_login};
use crate::k8s::{
    ClusterSnapshot, EnvKind, EnvSourceKind, EnvVarItem, K8sEvent, NamespaceSummary, ServiceInfo,
    demo_snapshot, spawn_fetch_env, spawn_fetch_logs, spawn_fetch_overview,
    spawn_fetch_snapshot, spawn_list_all_namespaces, spawn_restart_deploy, spawn_update_env,
};
use crate::gcp::{
    GcpProject, GkeCluster, spawn_get_credentials, spawn_list_clusters, spawn_list_projects,
};
use crate::pty_k9s::{K9sPty, K9sScreen};
use crate::update::UpdateState;
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

/// One step in the back-button history: which page (and namespace / service) was on screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavEntry {
    pub page: Page,
    pub env: EnvKind,
    pub namespace: String,
    pub service: Option<String>,
    pub tab: Tab,
    pub service_tab: ServiceTab,
}

impl NavEntry {
    /// Two entries are the same "page" when page, namespace and service match (sub-tabs ignored).
    fn same_place(&self, other: &NavEntry) -> bool {
        self.page == other.page
            && self.env == other.env
            && self.namespace == other.namespace
            && self.service == other.service
    }
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

    // GCP project / GKE cluster picker (shown right after login)
    pub picking_project: bool,
    pub projects: Vec<GcpProject>,
    pub projects_loading: bool,
    pub selected_project: Option<GcpProject>,
    pub clusters: Vec<GkeCluster>,
    pub clusters_loading: bool,
    pub connecting_cluster: Option<String>,
    pub gcp_error: Option<String>,
    pub active_project: Option<String>,
    pub active_cluster: Option<String>,

    // Runtime tools (kubectl / k9s / gcloud) and app updates
    pub tools_missing: Vec<String>,
    pub tools_installing: bool,
    pub tools_log: String,
    pub tools_error: Option<String>,
    pub update: UpdateState,
    pub update_token: Option<String>,
    pub update_token_input: TextEdit,
    pub update_note: Option<String>,
    pub show_setup: bool,
    /// UI language: true = Thai, false = English.
    pub lang_th: bool,
    /// Measured width of the Overview card grid, used to give every card the same width.
    pub cards_grid_w: f32,
    // Back-button history (per workspace tab)
    pub nav_history: Vec<NavEntry>,
    pub nav_last: Option<NavEntry>,
    pub nav_tab_id: usize,
    pub nav_going_back: bool,
    pub tools_checked: bool,
    pub tools_percent: f32,
    /// "ตกลง" was pressed on the setup screen: install what is missing, then go to sign-in.
    pub setup_pending: bool,

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
    /// Namespaces whose data could not be read on the last overview refresh (no mock data is shown).
    pub overview_failed: usize,
    /// ("STG / name", raw error) for each namespace the Overview couldn't read.
    pub overview_errors: Vec<(String, String)>,
    /// Seconds since a loading animation appeared (drives the owl loader frames).
    pub loader_t: f32,
    /// Press-and-hold on a namespace card (to stop watching it).
    pub ns_hold: Option<NsHold>,
    pub ns_hold_progress: f32,
    pub ns_press_block_until: Option<std::time::Instant>,
    /// The namespace card that is asking "stop watching?".
    pub confirm_unwatch: Option<(EnvKind, String)>,
    /// The "switch service" modal on the service page.
    pub svc_picker_open: bool,
    pub svc_picker_search: TextEdit,
    pub loader_since: Option<f64>,
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
        let show_setup = !crate::update::setup_done();
        let update_token = crate::update::load_token();

        Self {
            accounts: auth::default_accounts(),
            google_config: GoogleAuthConfig::load(),
            google_loading: false,
            session: None,
            login_user: TextEdit::new(""),
            login_pass: TextEdit::new(""),
            login_error: None,
            picking_project: false,
            projects: Vec::new(),
            projects_loading: false,
            selected_project: None,
            clusters: Vec::new(),
            clusters_loading: false,
            connecting_cluster: None,
            gcp_error: None,
            active_project: None,
            active_cluster: None,
            tools_missing: Vec::new(),
            tools_installing: false,
            tools_log: String::new(),
            tools_error: None,
            update: UpdateState::Idle,
            // The key was already entered in the installer: skip the form and install right away.
            setup_pending: show_setup && update_token.is_some(),
            update_token,
            update_token_input: TextEdit::new(""),
            update_note: None,
            show_setup,
            lang_th: load_lang_thai(),
            cards_grid_w: 0.0,
            nav_history: Vec::new(),
            nav_last: None,
            nav_tab_id: 0,
            nav_going_back: false,
            tools_checked: false,
            tools_percent: 0.0,
            open_tabs,
            active_tab_id,
            next_tab_id,
            page: active_tab.page,
            tab: active_tab.ns_tab,
            service_tab: active_tab.service_tab,
            env,
            namespace,
            dark: load_theme_dark(),
            overview_filter: OverviewFilter::All,
            pending_service_tab: None,
            stg_namespaces,
            prod_namespaces,
            available_namespaces: Vec::new(),
            adding_ns: false,
            new_ns_input: TextEdit::new(""),
            new_ns_env: EnvKind::Stg,
            overview: Vec::new(),
            overview_loading: false,
            overview_failed: 0,
            overview_errors: Vec::new(),
            loader_t: 0.0,
            loader_since: None,
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
            ns_hold: None,
            ns_hold_progress: 0.0,
            ns_press_block_until: None,
            confirm_unwatch: None,
            svc_picker_open: false,
            svc_picker_search: TextEdit::new(""),
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
                self.begin_project_picker();
            }
            Err(e) => {
                self.login_error = Some(e);
            }
        }
    }

    fn nav_entry(&self) -> NavEntry {
        NavEntry {
            page: self.page,
            env: self.env,
            namespace: self.namespace.clone(),
            service: if self.page == Page::ServiceDetail { self.selected_service.clone() } else { None },
            tab: self.tab,
            service_tab: self.service_tab,
        }
    }

    /// Records page changes for the back button. Called every frame from `poll`; returns true
    /// when the history changed (so the back button can appear or disappear).
    pub fn track_navigation(&mut self) -> bool {
        if self.session.is_none() || self.picking_project || self.show_setup {
            return false;
        }
        let cur = self.nav_entry();
        if self.nav_tab_id != self.active_tab_id {
            // Each workspace tab has its own trail.
            self.nav_tab_id = self.active_tab_id;
            let had = !self.nav_history.is_empty();
            self.nav_history.clear();
            self.nav_last = Some(cur);
            return had;
        }
        let Some(last) = self.nav_last.clone() else {
            self.nav_last = Some(cur);
            return false;
        };
        if last.same_place(&cur) {
            self.nav_last = Some(cur); // keep sub-tab choices up to date
            return false;
        }
        let pushed = !self.nav_going_back;
        if pushed {
            self.nav_history.push(last);
            if self.nav_history.len() > 50 {
                self.nav_history.remove(0);
            }
        }
        self.nav_going_back = false;
        self.nav_last = Some(cur);
        true
    }

    pub fn can_go_back(&self) -> bool {
        !self.nav_history.is_empty()
    }

    /// Returns to the previous page in this workspace tab.
    pub fn go_back(&mut self) {
        let Some(prev) = self.nav_history.pop() else {
            return;
        };
        self.nav_going_back = true;
        self.diagram_fullscreen = false;
        match prev.page {
            Page::Overview => self.open_overview(),
            Page::Namespace => {
                self.tab = prev.tab;
                self.drill_into_namespace(prev.env, &prev.namespace);
            }
            Page::ServiceDetail => {
                let same_ns = self.env == prev.env && self.namespace == prev.namespace;
                if !same_ns {
                    // Load the namespace first; the snapshot handler then loads this service.
                    self.drill_into_namespace(prev.env, &prev.namespace);
                }
                match prev.service {
                    Some(name) if same_ns => self.drill_into_service(name, prev.service_tab),
                    Some(name) => {
                        self.selected_service = Some(name);
                        self.service_tab = prev.service_tab;
                        self.page = Page::ServiceDetail;
                        self.sync_active_tab();
                    }
                    None => {}
                }
            }
        }
    }

    /// The Namespace page is still fetching the cluster (or only has fallback data for another one).
    pub fn namespace_loading(&self) -> bool {
        self.page == Page::Namespace
            && self.loading
            && (self.snapshot.namespace != self.namespace || self.snapshot.is_demo)
    }

    /// Is any owl loading animation on screen right now?
    pub fn loader_visible(&self) -> bool {
        if self.session.is_none() {
            return false;
        }
        if self.picking_project {
            return self.projects_loading || self.clusters_loading || self.connecting_cluster.is_some();
        }
        (self.page == Page::Overview && self.overview_loading && self.overview.is_empty())
            || self.namespace_loading()
    }

    /// Advances the loader clock. Returns true while a loader is visible (a frame is needed).
    /// How long a card has to be held before it asks to stop watching it.
    pub const NS_HOLD_SECS: f32 = 0.6;

    pub fn hold_begin(&mut self, env: EnvKind, ns: &str) {
        self.ns_hold = Some(NsHold { env, ns: ns.to_string(), start: std::time::Instant::now(), fired: false });
        self.ns_hold_progress = 0.0;
    }

    /// The pointer moved too far or the press ended before the hold completed.
    pub fn hold_cancel(&mut self) {
        if let Some(h) = self.ns_hold.take() {
            if h.fired {
                // The release that ends a completed hold must not also open the namespace.
                self.ns_press_block_until = Some(std::time::Instant::now() + std::time::Duration::from_millis(350));
            }
        }
        self.ns_hold_progress = 0.0;
    }

    /// Advances the hold; true while it still needs frames.
    pub fn tick_hold(&mut self) -> bool {
        let Some(h) = self.ns_hold.as_mut() else { return false };
        if h.fired {
            return false;
        }
        let p = (h.start.elapsed().as_secs_f32() / Self::NS_HOLD_SECS).min(1.0);
        self.ns_hold_progress = p;
        if p >= 1.0 {
            h.fired = true;
            self.confirm_unwatch = Some((h.env, h.ns.clone()));
            self.ns_hold_progress = 0.0;
        }
        true
    }

    pub fn hold_progress_for(&self, env: EnvKind, ns: &str) -> f32 {
        match &self.ns_hold {
            Some(h) if !h.fired && h.env == env && h.ns == ns => self.ns_hold_progress,
            _ => 0.0,
        }
    }

    /// A card press that ends a hold (or lands right after one) is not a click.
    pub fn ns_press_blocked(&mut self) -> bool {
        if self.ns_hold.as_ref().map_or(false, |h| h.fired) {
            return true;
        }
        match self.ns_press_block_until {
            Some(t) if std::time::Instant::now() < t => true,
            _ => {
                self.ns_press_block_until = None;
                false
            }
        }
    }

    pub fn cancel_unwatch(&mut self) {
        self.confirm_unwatch = None;
    }

    /// Stops watching a namespace: it leaves the Overview and the saved list.
    pub fn confirm_unwatch_namespace(&mut self) {
        let Some((env, ns)) = self.confirm_unwatch.take() else { return };
        let list = match env {
            EnvKind::Stg => &self.stg_namespaces,
            EnvKind::Prod => &self.prod_namespaces,
        };
        if list.len() <= 1 {
            self.banner = Some((crate::tf!(
                "ต้องมีอย่างน้อย 1 Namespace ในฝั่ง {} จึงเลิกดู {ns} ไม่ได้",
                "{} needs at least one namespace, so {ns} stays",
                env.label()
            ), false));
            return;
        }
        self.overview.retain(|x| !(x.env == env && x.namespace == ns));
        self.remove_namespace(env, &ns);
        self.banner = Some((crate::tf!("เลิกดู {ns} แล้ว (เพิ่มกลับได้ที่ปุ่มเพิ่ม Namespace)", "Stopped watching {ns} (add it back with Add namespace)"), true));
    }

    pub fn tick_loader(&mut self, now: f64) -> bool {
        if self.loader_visible() {
            let since = *self.loader_since.get_or_insert(now);
            self.loader_t = (now - since) as f32;
            true
        } else {
            self.loader_since = None;
            self.loader_t = 0.0;
            false
        }
    }

    /// Switches the UI language (Thai / English) and remembers the choice.
    pub fn toggle_lang(&mut self) {
        self.lang_th = !self.lang_th;
        crate::i18n::set_thai(self.lang_th);
        let _ = std::fs::write("klar-lang.txt", if self.lang_th { "th" } else { "en" });
    }

    /// Switches between the light and dark theme and remembers the choice.
    pub fn toggle_theme(&mut self) {
        self.dark = !self.dark;
        let _ = std::fs::write("klar-theme.txt", if self.dark { "dark" } else { "light" });
    }

    /// Looks (in the background) for kubectl / k9s / gcloud on PATH.
    pub fn check_tools(&mut self) {
        let tx = self.k8s_tx.clone();
        let waker = self.waker.clone();
        crate::tools::spawn_check(tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    /// Downloads whatever tool is missing into the app's own folder.
    pub fn install_tools(&mut self) {
        if self.tools_installing || self.tools_missing.is_empty() {
            return;
        }
        self.tools_installing = true;
        self.tools_percent = 0.0;
        self.tools_error = None;
        self.tools_log = crate::tr!("เริ่มดาวน์โหลด...", "Starting download...").to_string();
        let tx = self.k8s_tx.clone();
        let waker = self.waker.clone();
        crate::tools::spawn_install(self.tools_missing.clone(), tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    pub fn check_for_update(&mut self) {
        let tx = self.k8s_tx.clone();
        let waker = self.waker.clone();
        crate::update::spawn_check(self.update_token.clone(), tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    /// The single "ตกลง" button on the setup screen: save the key (if any), then install
    /// whatever is missing without asking again, then go to sign-in.
    pub fn setup_confirm(&mut self) {
        if !self.update_token_input.text.trim().is_empty() {
            self.save_update_token();
        }
        self.tools_error = None;
        self.setup_pending = true;
        if self.tools_checked {
            self.continue_setup();
        }
    }

    fn continue_setup(&mut self) {
        if self.tools_missing.is_empty() {
            self.setup_pending = false;
            self.finish_setup();
        } else {
            self.install_tools();
        }
    }

    /// Closes the first-run setup screen and remembers it was done.
    pub fn finish_setup(&mut self) {
        self.show_setup = false;
        if let Err(e) = crate::update::mark_setup_done() {
            self.login_error = Some(e);
        }
    }

    /// Saves the GitHub token typed on the login screen and re-checks for updates.
    pub fn save_update_token(&mut self) {
        let token = self.update_token_input.text.trim().to_string();
        if token.is_empty() {
            return;
        }
        match crate::update::save_token(&token) {
            Ok(()) => {
                self.update_token = Some(token);
                self.update_token_input = TextEdit::new("");
                self.update_note = Some(crate::tr!("บันทึก token แล้ว กำลังเช็กอัปเดต...", "Token saved, checking for updates...").to_string());
                self.check_for_update();
            }
            Err(e) => self.update_note = Some(e),
        }
    }

    /// Downloads the newest release, replaces this app and restarts it.
    pub fn start_update(&mut self) {
        let version = match &self.update {
            UpdateState::Available(v) => v.clone(),
            _ => return,
        };
        self.update = UpdateState::Installing(version);
        let tx = self.k8s_tx.clone();
        let waker = self.waker.clone();
        crate::update::spawn_install(self.update_token.clone(), tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    /// Opens the project picker and loads the user's GCP projects via gcloud.
    pub fn begin_project_picker(&mut self) {
        self.picking_project = true;
        self.selected_project = None;
        self.clusters.clear();
        self.connecting_cluster = None;
        self.gcp_error = None;
        self.load_projects();
    }

    pub fn load_projects(&mut self) {
        self.projects_loading = true;
        self.gcp_error = None;
        let tx = self.k8s_tx.clone();
        let waker = self.waker.clone();
        spawn_list_projects(tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    pub fn select_project(&mut self, project: GcpProject) {
        self.selected_project = Some(project.clone());
        self.clusters.clear();
        self.clusters_loading = true;
        self.connecting_cluster = None;
        self.gcp_error = None;
        let tx = self.k8s_tx.clone();
        let waker = self.waker.clone();
        spawn_list_clusters(project.id, tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    pub fn back_to_projects(&mut self) {
        self.selected_project = None;
        self.clusters.clear();
        self.clusters_loading = false;
        self.connecting_cluster = None;
        self.gcp_error = None;
    }

    pub fn connect_cluster(&mut self, cluster: GkeCluster) {
        let Some(project) = self.selected_project.clone() else {
            return;
        };
        if self.connecting_cluster.is_some() {
            return;
        }
        self.connecting_cluster = Some(cluster.name.clone());
        self.gcp_error = None;
        let tx = self.k8s_tx.clone();
        let waker = self.waker.clone();
        spawn_get_credentials(project.id, cluster, tx, move || {
            if let Some(w) = &waker {
                w();
            }
        });
    }

    /// Leaves the picker and keeps whatever kubectl context is already active.
    pub fn skip_project_picker(&mut self) {
        self.picking_project = false;
        self.gcp_error = None;
        self.refresh_overview();
        self.refresh_cluster();
    }

    pub fn logout(&mut self) {
        self.k9s_pty.stop();
        self.k9s_screen = K9sScreen::default();
        self.session = None;
        self.picking_project = false;
        self.selected_project = None;
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

    pub fn open_svc_picker(&mut self) {
        self.svc_picker_open = true;
        self.svc_picker_search = TextEdit::new("");
    }

    pub fn close_svc_picker(&mut self) {
        self.svc_picker_open = false;
    }

    /// Jumps to another service of the same namespace, staying on the same tab.
    pub fn switch_service(&mut self, name: &str) {
        self.svc_picker_open = false;
        if self.selected_service.as_deref() == Some(name) {
            return;
        }
        let tab = self.service_tab;
        self.drill_into_service(name.to_string(), tab);
    }

    /// Services of the current namespace that match the picker's search.
    pub fn svc_picker_matches(&self) -> Vec<&ServiceInfo> {
        let q = self.svc_picker_search.text.trim().to_ascii_lowercase();
        let mut v: Vec<&ServiceInfo> = self
            .snapshot
            .services
            .iter()
            .filter(|x| {
                q.is_empty()
                    || x.name.to_ascii_lowercase().contains(&q)
                    || x.role.label().to_ascii_lowercase().contains(&q)
                    || x.hosts.iter().any(|h| h.to_ascii_lowercase().contains(&q))
            })
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    pub fn load_service_data(&mut self, service_name: &str) {
        if let Some(svc) = self.snapshot.services.iter().find(|s| s.name == service_name).cloned() {
            if let Some(pod) = svc.pods.first() {
                self.fetch_logs(pod.name.clone());
            } else {
                self.logs_pod = None;
                self.logs_text = Some(crate::tr!("ยังไม่มี Pod ที่รันอยู่สำหรับ service นี้", "No running pods for this service yet").into());
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
            self.banner = Some((crate::tf!("เพิ่ม Namespace {ns} เข้าฝั่ง {} แล้ว", "Added namespace {ns} to {}", env.label()), true));
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
        self.logs_text = Some(crate::tr!("กำลังโหลด logs...", "Loading logs...").into());
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
            self.banner = Some((crate::tf!("คัดลอก {label} ไปยัง Clipboard แล้ว", "Copied {label} to the clipboard"), true));
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
                    crate::tf!("สิทธิ์ {} ไม่สามารถแก้ไข ENV บน {} ได้", "{} cannot edit ENV on {}", sess.role.label(), self.env.label()),
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
            self.banner = Some((crate::tr!("กรุณากรอกชื่อ Key ของ ENV", "Enter an ENV key name").into(), false));
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
                let ps_script = crate::tf!(
                    "Add-Type -AssemblyName System.Windows.Forms; \
                     $d = New-Object System.Windows.Forms.SaveFileDialog; \
                     $d.Filter = 'CSV Files (*.csv)|*.csv|All Files (*.*)|*.*'; \
                     $d.InitialDirectory = '{}'; \
                     $d.FileName = '{}'; \
                     $d.Title = 'เลือกตำแหน่งบันทึกไฟล์ ENV CSV'; \
                     if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ Write-Output $d.FileName }}", "Add-Type -AssemblyName System.Windows.Forms; \
                     $d = New-Object System.Windows.Forms.SaveFileDialog; \
                     $d.Filter = 'CSV Files (*.csv)|*.csv|All Files (*.*)|*.*'; \
                     $d.InitialDirectory = '{}'; \
                     $d.FileName = '{}'; \
                     $d.Title = 'Choose where to save the ENV CSV'; \
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
                let osa_script = crate::tf!(
                    "POSIX path of (choose file name with prompt \"เลือกตำแหน่งบันทึกไฟล์ ENV CSV\" default name \"{}\" default location POSIX file \"{}\")", "POSIX path of (choose file name with prompt \"Choose where to save the ENV CSV\" default name \"{}\" default location POSIX file \"{}\")",
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
                        Ok(_) => (crate::tf!("Export ENV ({count} รายการ) ไปที่ {chosen} สำเร็จ", "Exported {count} ENV entries to {chosen}"), true),
                        Err(e) => (crate::tf!("Export CSV ไม่สำเร็จ: {e}", "CSV export failed: {e}"), false),
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
                    crate::tf!("สิทธิ์ {} ไม่สามารถสั่ง Restart บน {} ได้", "{} cannot restart on {}", sess.role.label(), self.env.label()),
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
                    self.begin_project_picker();
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
                    // Only real cluster data on the Overview: drop the demo fallbacks.
                    self.overview_failed = list.iter().filter(|n| n.is_demo).count();
                    self.overview_errors = list
                        .iter()
                        .filter(|n| n.is_demo)
                        .map(|n| {
                            (
                                format!("{} / {}", n.env.label(), n.namespace),
                                n.fetch_error.clone().unwrap_or_else(|| "unknown error".into()),
                            )
                        })
                        .collect();
                    let list: Vec<NamespaceSummary> = list.into_iter().filter(|n| !n.is_demo).collect();
                    self.overview = list;
                }
                K8sEvent::AllNamespaces(list) => {
                    self.available_namespaces = list;
                }
                K8sEvent::Projects(result) => {
                    self.projects_loading = false;
                    match result {
                        Ok(list) => {
                            self.projects = list;
                            self.gcp_error = None;
                        }
                        Err(e) => {
                            self.projects.clear();
                            self.gcp_error = Some(e);
                        }
                    }
                }
                K8sEvent::Clusters { project, result } => {
                    // Ignore a stale answer if the user already picked another project.
                    if self.selected_project.as_ref().map(|p| p.id.as_str()) == Some(project.as_str()) {
                        self.clusters_loading = false;
                        match result {
                            Ok(list) => {
                                self.clusters = list;
                                self.gcp_error = None;
                            }
                            Err(e) => {
                                self.clusters.clear();
                                self.gcp_error = Some(e);
                            }
                        }
                    }
                }
                K8sEvent::Credentials { project, cluster, result } => {
                    self.connecting_cluster = None;
                    match result {
                        Ok(()) => {
                            self.banner = Some((
                                crate::tf!("เชื่อมต่อ cluster {cluster} ({project}) แล้ว", "Connected to cluster {cluster} ({project})"),
                                true,
                            ));
                            self.active_project = Some(project);
                            self.active_cluster = Some(cluster);
                            self.picking_project = false;
                            self.gcp_error = None;
                            self.refresh_overview();
                            self.refresh_cluster();
                        }
                        Err(e) => {
                            self.gcp_error = Some(e);
                        }
                    }
                }
                K8sEvent::UpdateCheck(result) => match result {
                    Ok(found) => {
                        if self.update_token.is_some() {
                            self.update_note = Some(match &found {
                                Some(v) => crate::tf!("พบเวอร์ชันใหม่ v{v}", "New version v{v} available"),
                                None => crate::tr!("ใช้เวอร์ชันล่าสุดอยู่แล้ว", "You have the latest version").to_string(),
                            });
                        }
                        if let Some(version) = found {
                            self.update = UpdateState::Available(version);
                        }
                    }
                    Err(e) => {
                        if self.update_token.is_some() {
                            self.update_note = Some(e);
                        }
                    }
                },
                K8sEvent::UpdateFailed(err) => {
                    let previous = self.update.clone();
                    if let UpdateState::Installing(version) = previous {
                        self.update = UpdateState::Available(version);
                    }
                    let msg = crate::tf!("อัปเดตไม่สำเร็จ: {err}", "Update failed: {err}");
                    if self.session.is_none() {
                        self.login_error = Some(msg.clone());
                    }
                    self.banner = Some((msg, false));
                }
                K8sEvent::ToolsStatus(list) => {
                    self.tools_checked = true;
                    self.tools_missing = list;
                    if self.setup_pending && !self.tools_installing {
                        self.continue_setup();
                    }
                }
                K8sEvent::ToolsProgress { message, percent } => {
                    self.tools_log = message;
                    self.tools_percent = percent;
                }
                K8sEvent::ToolsDone(result) => {
                    self.tools_installing = false;
                    match result {
                        Ok(()) => {
                            self.tools_error = None;
                            self.tools_log.clear();
                            self.tools_missing.clear();
                            if self.setup_pending {
                                self.setup_pending = false;
                                self.finish_setup();
                            }
                            self.check_tools();
                        }
                        Err(e) => {
                            self.setup_pending = false;
                            self.tools_error = Some(e);
                        }
                    }
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
    if let Ok(raw) = std::fs::read_to_string("klar-tabs.json") {
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
        let _ = std::fs::write("klar-tabs.json", json);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedNamespaces {
    stg: Vec<String>,
    prod: Vec<String>,
}

/// A press being held on a namespace card.
pub struct NsHold {
    pub env: EnvKind,
    pub ns: String,
    pub start: std::time::Instant,
    pub fired: bool,
}

fn load_saved_namespaces() -> (Vec<String>, Vec<String>) {
    if let Ok(raw) = std::fs::read_to_string("klar-namespaces.json") {
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
        let _ = std::fs::write("klar-namespaces.json", json);
    }
}

/// Saved theme choice (dark by default).
fn load_theme_dark() -> bool {
    std::fs::read_to_string("klar-theme.txt")
        .map(|t| t.trim() != "light")
        .unwrap_or(true)
}

/// Saved UI language (Thai by default).
pub fn load_lang_thai() -> bool {
    std::fs::read_to_string("klar-lang.txt")
        .map(|t| t.trim() != "en")
        .unwrap_or(true)
}
