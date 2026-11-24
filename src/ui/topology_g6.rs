use crate::k8s::{ClusterSnapshot, Health, NodeRole, ServiceInfo};
use crate::state::{AppState, Page, Tab};
use rune::WindowHandle;
use rune::runtime::Waker;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::sync::mpsc;

pub const TOPOLOGY_G6_HTML: &str = include_str!("../../assets/topology_g6.html");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopologyViewMode {
    #[default]
    G6,
    Classic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum G6Command {
    FitView,
    ZoomIn,
    ZoomOut,
    ResetView,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TopologyWebEvent {
    Ready,
    SelectService { name: String },
    DrillService { name: String },
    ClearSelection,
    ExitFullscreen,
    LoadError { message: String },
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct G6GraphPayload {
    pub meta: G6TopologyMeta,
    pub selected_service: Option<String>,
    pub data: G6GraphData,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct G6TopologyMeta {
    pub env_label: String,
    pub namespace: String,
    pub context: String,
    pub service_count: usize,
    pub ingress_count: usize,
    pub edge_count: usize,
    pub is_demo: bool,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct G6GraphData {
    pub combos: Vec<G6Combo>,
    pub nodes: Vec<G6Node>,
    pub edges: Vec<G6Edge>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct G6Combo {
    pub id: String,
    pub data: G6ComboData,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct G6ComboData {
    pub is_combo: bool,
    pub label: String,
    pub description: String,
    pub fill: String,
    pub stroke: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct G6Node {
    pub id: String,
    pub combo: String,
    pub x: f32,
    pub y: f32,
    pub data: G6NodeData,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct G6NodeData {
    pub kind: String,
    pub label: String,
    pub full_name: String,
    pub service_name: Option<String>,
    pub role_label: String,
    pub health: String,
    pub pod_badge: Option<String>,
    pub badge: Option<String>,
    pub ports: Option<String>,
    pub hosts: Vec<String>,
    pub connects_to: Vec<String>,
    pub summary: String,
    pub fan_in: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct G6Edge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub data: G6EdgeData,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct G6EdgeData {
    pub kind: String,
    pub label: String,
    pub source_label: String,
    pub target_label: String,
    pub path: Option<String>,
    pub source_fan_out: usize,
    pub source_lane_index: usize,
    pub target_fan_in: usize,
    pub target_lane_index: usize,
    pub curve_offset: f32,
}

fn health_key(health: Health) -> &'static str {
    match health {
        Health::Healthy => "healthy",
        Health::Warning => "warning",
        Health::Error => "error",
    }
}

fn service_node_id(name: &str) -> String {
    format!("svc:{name}")
}

/// Serializes `ClusterSnapshot` into AntV G6 v5 `{ combos, nodes, edges }` with deterministic,
/// collision-free zone coordinates and fan-in-aware curve offsets.
pub fn build_g6_topology(
    snap: &ClusterSnapshot,
    selected_service: Option<&str>,
) -> G6GraphPayload {
    let apis: Vec<&ServiceInfo> = snap.services.iter().filter(|s| s.role == NodeRole::Api).collect();
    let caches: Vec<&ServiceInfo> = snap.services.iter().filter(|s| s.role == NodeRole::Cache).collect();
    let workers: Vec<&ServiceInfo> = snap.services.iter().filter(|s| s.role == NodeRole::Worker).collect();

    // Collect any `connects_to` targets or ingress backends not present in `snap.services`
    // so G6 never receives dangling edge endpoints.
    let known_services: BTreeSet<&str> = snap.services.iter().map(|s| s.name.as_str()).collect();
    let mut external_caches: Vec<String> = Vec::new();
    let mut external_seen: BTreeSet<String> = BTreeSet::new();
    for svc in &snap.services {
        for target in &svc.connects_to {
            if !known_services.contains(target.as_str()) && external_seen.insert(target.clone()) {
                external_caches.push(target.clone());
            }
        }
    }

    // Count inbound dependency links per target service (crucial for 20+ edges -> Redis).
    let mut dep_fan_in: HashMap<String, usize> = HashMap::new();
    for svc in &snap.services {
        let mut uniq_targets = BTreeSet::new();
        for target in &svc.connects_to {
            if target != &svc.name && uniq_targets.insert(target.as_str()) {
                *dep_fan_in.entry(target.clone()).or_insert(0) += 1;
            }
        }
    }

    let has_storage_nodes = !caches.is_empty() || !external_caches.is_empty() || !workers.is_empty();
    let mut combos = Vec::with_capacity(3);
    if !apis.is_empty() {
        combos.push(G6Combo {
            id: "combo-apps".into(),
            data: G6ComboData {
                is_combo: true,
                label: format!("APPLICATION SERVERS — {} ({})", snap.namespace, apis.len()),
                description: format!("{} API/service deployments in {}", apis.len(), snap.namespace),
                fill: "rgba(99, 102, 241, 0.06)".into(),
                stroke: "rgba(129, 140, 248, 0.42)".into(),
            },
        });
    }
    combos.push(G6Combo {
        id: "combo-network".into(),
        data: G6ComboData {
            is_combo: true,
            label: format!(
                "NETWORK / INGRESS SWITCH ({}) • {} routes",
                snap.env.label(),
                snap.ingresses.len()
            ),
            description: "Nginx Ingress controller and public domain routes".into(),
            fill: "rgba(56, 189, 248, 0.055)".into(),
            stroke: "rgba(56, 189, 248, 0.40)".into(),
        },
    });
    if has_storage_nodes {
        combos.push(G6Combo {
            id: "combo-storage".into(),
            data: G6ComboData {
                is_combo: true,
                label: format!(
                    "STORAGE & WORKERS CLUSTER ({} cache • {} worker)",
                    caches.len() + external_caches.len(),
                    workers.len()
                ),
                description: "Redis / Database caches and background worker deployments".into(),
                fill: "rgba(192, 132, 252, 0.06)".into(),
                stroke: "rgba(192, 132, 252, 0.42)".into(),
            },
        });
    }

    let mut nodes: Vec<G6Node> = Vec::new();
    let mut node_labels: HashMap<String, String> = HashMap::new();

    // Card dimensions & combo padding used for exact zero-overlap bounding-box layout:
    // - API / Cache / Worker card: 224 x 68 (half: 112 x 34)
    // - Switch card:               196 x 68 (half: 98 x 34)
    // - Domain pill:               244 x 46 (half: 122 x 23)
    // - Combo padding in G6 v5:    [top=68, right=36, bottom=36, left=36]
    const CARD_HALF_W: f32 = 112.0;
    const CARD_HALF_H: f32 = 34.0;
    const SWITCH_HALF_W: f32 = 98.0;
    const DOMAIN_HALF_W: f32 = 122.0;
    const COMBO_PAD_TOP: f32 = 68.0;
    const COMBO_PAD_SIDE: f32 = 36.0;
    const COMBO_PAD_BOT: f32 = 36.0;
    const ZONE_V_GAP: f32 = 220.0;
    const ZONE_H_GAP: f32 = 190.0;

    // 1. Application Servers (Top zone, spacious 284x126 pitch -> 60px H-gap, 58px V-gap)
    let api_cols = if apis.len() <= 4 {
        apis.len().max(1)
    } else if apis.len() <= 10 {
        5
    } else {
        6
    };
    let api_col_pitch = 284.0_f32;
    let api_row_pitch = 126.0_f32;
    let api_rows = apis.len().div_ceil(api_cols).max(1);
    let api_start_x = 156.0_f32;
    let api_start_y = 120.0_f32;

    for (idx, svc) in apis.iter().enumerate() {
        let r = idx / api_cols;
        let c = idx % api_cols;
        let id = service_node_id(&svc.name);
        node_labels.insert(id.clone(), svc.name.clone());
        nodes.push(G6Node {
            id,
            combo: "combo-apps".into(),
            x: api_start_x + c as f32 * api_col_pitch,
            y: api_start_y + r as f32 * api_row_pitch,
            data: G6NodeData {
                kind: "api".into(),
                label: svc.name.clone(),
                full_name: svc.name.clone(),
                service_name: Some(svc.name.clone()),
                role_label: svc.role.label().into(),
                health: health_key(svc.health).into(),
                pod_badge: Some(format!("{}/{} Pods", svc.ready_replicas, svc.desired_replicas)),
                badge: None,
                ports: Some(svc.ports.clone()),
                hosts: svc.hosts.clone(),
                connects_to: svc.connects_to.clone(),
                summary: svc.summary.clone(),
                fan_in: dep_fan_in.get(&svc.name).copied().unwrap_or(0),
            },
        });
    }

    // Exact outer bottom of `combo-apps` + generous vertical channel + `combo-network/storage` top pad
    let app_last_row_y = api_start_y + (api_rows.saturating_sub(1) as f32) * api_row_pitch;
    let app_combo_bottom = app_last_row_y + CARD_HALF_H + COMBO_PAD_BOT;
    let bottom_zone_y = app_combo_bottom + ZONE_V_GAP + COMBO_PAD_TOP + CARD_HALF_H;

    // 2. Network / Ingress Switch (Bottom-left zone — spread across 2 columns if >4 ingresses)
    let ingress_count = snap.ingresses.len();
    let domain_cols = if ingress_count > 4 { 2usize } else { 1usize };
    let domain_rows = ingress_count.div_ceil(domain_cols).max(1);
    let domain_col_pitch = 290.0_f32;
    let domain_row_pitch = 78.0_f32;

    let switch_x = 148.0_f32;
    let switch_id = "node-ingress-switch".to_string();
    let switch_y = bottom_zone_y
        + if domain_rows > 1 {
            ((domain_rows - 1) as f32 * domain_row_pitch) * 0.5
        } else {
            0.0
        };
    node_labels.insert(switch_id.clone(), "Nginx Ingress".into());
    nodes.push(G6Node {
        id: switch_id.clone(),
        combo: "combo-network".into(),
        x: switch_x,
        y: switch_y,
        data: G6NodeData {
            kind: "switch".into(),
            label: "Nginx Ingress".into(),
            full_name: format!("Nginx Ingress Controller ({})", snap.env.full_label()),
            service_name: None,
            role_label: "Ingress Controller".into(),
            health: "healthy".into(),
            pod_badge: None,
            badge: Some(format!("{ingress_count} routes")),
            ports: Some("80 / 443".into()),
            hosts: snap.ingresses.iter().map(|i| i.host.clone()).collect(),
            connects_to: vec![],
            summary: format!("Routes external HTTPS traffic to {} ingress endpoints", ingress_count),
            fan_in: 0,
        },
    });

    let dom_start_x = switch_x + SWITCH_HALF_W + 96.0 + DOMAIN_HALF_W;
    let mut edges: Vec<G6Edge> = Vec::new();
    let mut edge_counter = 0usize;

    // Count inbound ingress routes per backend service for accurate fan-in and lane indices.
    let mut ingress_target_totals: HashMap<&str, usize> = HashMap::new();
    for ing in &snap.ingresses {
        if known_services.contains(ing.backend_service.as_str()) {
            *ingress_target_totals.entry(ing.backend_service.as_str()).or_insert(0) += 1;
        }
    }
    let mut ingress_target_seen: HashMap<&str, usize> = HashMap::new();

    for (idx, ing) in snap.ingresses.iter().enumerate() {
        let col = idx / domain_rows;
        let row = idx % domain_rows;
        let dom_id = format!("domain:{idx}:{}", ing.host);
        let dom_x = dom_start_x + col as f32 * domain_col_pitch;
        let dom_y = bottom_zone_y + row as f32 * domain_row_pitch;
        node_labels.insert(dom_id.clone(), ing.host.clone());

        nodes.push(G6Node {
            id: dom_id.clone(),
            combo: "combo-network".into(),
            x: dom_x,
            y: dom_y,
            data: G6NodeData {
                kind: "domain".into(),
                label: ing.host.clone(),
                full_name: ing.host.clone(),
                service_name: known_services
                    .contains(ing.backend_service.as_str())
                    .then(|| ing.backend_service.clone()),
                role_label: NodeRole::Ingress.label().into(),
                health: "healthy".into(),
                pod_badge: None,
                badge: None,
                ports: Some(ing.path.clone()),
                hosts: vec![ing.host.clone()],
                connects_to: vec![ing.backend_service.clone()],
                summary: format!("Ingress {} ({}) → {}", ing.name, ing.path, ing.backend_service),
                fan_in: 1,
            },
        });

        // Switch -> Domain
        edge_counter += 1;
        edges.push(G6Edge {
            id: format!("e-sw-{edge_counter}"),
            source: switch_id.clone(),
            target: dom_id.clone(),
            data: G6EdgeData {
                kind: "ingress-switch".into(),
                label: "Ingress Route".into(),
                source_label: "Nginx Ingress".into(),
                target_label: ing.host.clone(),
                path: Some(ing.path.clone()),
                source_fan_out: ingress_count,
                source_lane_index: idx,
                target_fan_in: 1,
                target_lane_index: 0,
                curve_offset: 0.0,
            },
        });

        // Domain -> Backend Service
        let target_id = service_node_id(&ing.backend_service);
        if known_services.contains(ing.backend_service.as_str()) {
            edge_counter += 1;
            let tgt_total = ingress_target_totals
                .get(ing.backend_service.as_str())
                .copied()
                .unwrap_or(1);
            let tgt_lane = {
                let entry = ingress_target_seen
                    .entry(ing.backend_service.as_str())
                    .or_insert(0);
                let cur = *entry;
                *entry += 1;
                cur
            };
            let offset = if tgt_total > 1 {
                let centered = tgt_lane as f32 - (tgt_total.saturating_sub(1) as f32) * 0.5;
                (centered * 14.0).clamp(-64.0, 64.0)
            } else {
                -24.0 - (idx % 4) as f32 * 10.0
            };
            edges.push(G6Edge {
                id: format!("e-ing-{edge_counter}"),
                source: dom_id,
                target: target_id,
                data: G6EdgeData {
                    kind: "ingress-route".into(),
                    label: format!("HTTP {}", ing.path),
                    source_label: ing.host.clone(),
                    target_label: ing.backend_service.clone(),
                    path: Some(ing.path.clone()),
                    source_fan_out: 1,
                    source_lane_index: 0,
                    target_fan_in: tgt_total,
                    target_lane_index: tgt_lane,
                    curve_offset: offset,
                },
            });
        }
    }

    // Exact outer right of `combo-network` + generous horizontal gap -> start of `combo-storage`
    let net_rightmost_edge = if ingress_count == 0 {
        switch_x + SWITCH_HALF_W
    } else {
        let last_dom_col = (ingress_count - 1) / domain_rows;
        dom_start_x + last_dom_col as f32 * domain_col_pitch + DOMAIN_HALF_W
    };
    let net_combo_right = net_rightmost_edge + COMBO_PAD_SIDE;

    // 3. Storage (Caches / Redis) & Workers Cluster (Bottom-right zone — guaranteed non-overlapping!)
    let storage_start_x = net_combo_right + ZONE_H_GAP + COMBO_PAD_SIDE + CARD_HALF_W;
    let storage_cols = 3usize;
    let storage_col_pitch = 284.0_f32;
    let storage_row_pitch = 126.0_f32;

    let mut storage_slot = 0usize;
    for svc in &caches {
        let r = storage_slot / storage_cols;
        let c = storage_slot % storage_cols;
        let id = service_node_id(&svc.name);
        node_labels.insert(id.clone(), svc.name.clone());
        nodes.push(G6Node {
            id,
            combo: "combo-storage".into(),
            x: storage_start_x + c as f32 * storage_col_pitch,
            y: bottom_zone_y + r as f32 * storage_row_pitch,
            data: G6NodeData {
                kind: "cache".into(),
                label: svc.name.clone(),
                full_name: svc.name.clone(),
                service_name: Some(svc.name.clone()),
                role_label: svc.role.label().into(),
                health: health_key(svc.health).into(),
                pod_badge: Some(format!("{}/{} Pods", svc.ready_replicas, svc.desired_replicas)),
                badge: None,
                ports: Some(svc.ports.clone()),
                hosts: svc.hosts.clone(),
                connects_to: svc.connects_to.clone(),
                summary: svc.summary.clone(),
                fan_in: dep_fan_in.get(&svc.name).copied().unwrap_or(0),
            },
        });
        storage_slot += 1;
    }

    for ext_name in &external_caches {
        let r = storage_slot / storage_cols;
        let c = storage_slot % storage_cols;
        let id = service_node_id(ext_name);
        node_labels.insert(id.clone(), ext_name.clone());
        nodes.push(G6Node {
            id,
            combo: "combo-storage".into(),
            x: storage_start_x + c as f32 * storage_col_pitch,
            y: bottom_zone_y + r as f32 * storage_row_pitch,
            data: G6NodeData {
                kind: "cache".into(),
                label: ext_name.clone(),
                full_name: ext_name.clone(),
                service_name: None,
                role_label: "Referenced Cache / DB".into(),
                health: "external".into(),
                pod_badge: Some("External".into()),
                badge: None,
                ports: Some("6379".into()),
                hosts: vec![],
                connects_to: vec![],
                summary: "Referenced dependency target in cluster topology".into(),
                fan_in: dep_fan_in.get(ext_name).copied().unwrap_or(0),
            },
        });
        storage_slot += 1;
    }

    // Place workers on a fresh row beneath caches with extra vertical breathing room.
    let worker_start_row = if storage_slot == 0 {
        0
    } else {
        storage_slot.div_ceil(storage_cols)
    };
    let worker_extra_y = if storage_slot > 0 { 28.0_f32 } else { 0.0_f32 };
    for (w_idx, svc) in workers.iter().enumerate() {
        let r = worker_start_row + w_idx / storage_cols;
        let c = w_idx % storage_cols;
        let id = service_node_id(&svc.name);
        node_labels.insert(id.clone(), svc.name.clone());
        nodes.push(G6Node {
            id,
            combo: "combo-storage".into(),
            x: storage_start_x + c as f32 * storage_col_pitch,
            y: bottom_zone_y + r as f32 * storage_row_pitch + worker_extra_y,
            data: G6NodeData {
                kind: "worker".into(),
                label: svc.name.clone(),
                full_name: svc.name.clone(),
                service_name: Some(svc.name.clone()),
                role_label: svc.role.label().into(),
                health: health_key(svc.health).into(),
                pod_badge: Some(format!("{}/{} Pods", svc.ready_replicas, svc.desired_replicas)),
                badge: None,
                ports: Some(svc.ports.clone()),
                hosts: svc.hosts.clone(),
                connects_to: svc.connects_to.clone(),
                summary: svc.summary.clone(),
                fan_in: dep_fan_in.get(&svc.name).copied().unwrap_or(0),
            },
        });
    }

    // 4. Service / Worker -> Cache / Dependency edges (deduplicated with smooth fan-out curves)
    let mut target_edge_seen: HashMap<String, usize> = HashMap::new();
    let mut seen_pairs: BTreeSet<(String, String)> = BTreeSet::new();
    for svc in &snap.services {
        let src_id = service_node_id(&svc.name);
        let mut uniq_targets_for_svc: Vec<&String> = Vec::new();
        let mut uniq_set: BTreeSet<&str> = BTreeSet::new();
        for target in &svc.connects_to {
            let dst_id = service_node_id(target);
            if src_id != dst_id && node_labels.contains_key(&dst_id) && uniq_set.insert(target.as_str()) {
                uniq_targets_for_svc.push(target);
            }
        }
        let src_fan_out = uniq_targets_for_svc.len();

        for (src_lane_idx, target) in uniq_targets_for_svc.into_iter().enumerate() {
            let dst_id = service_node_id(target);
            if !seen_pairs.insert((src_id.clone(), dst_id.clone())) {
                continue;
            }
            let total_for_target = dep_fan_in.get(target).copied().unwrap_or(1);
            let idx_for_target = {
                let entry = target_edge_seen.entry(target.clone()).or_insert(0);
                let cur = *entry;
                *entry += 1;
                cur
            };
            // Symmetric fan spread around 0 so 20+ edges converging into one Redis separate cleanly.
            let centered = idx_for_target as f32 - (total_for_target.saturating_sub(1) as f32) * 0.5;
            let curve_offset = (centered * 11.0).clamp(-78.0, 78.0);

            edge_counter += 1;
            edges.push(G6Edge {
                id: format!("e-dep-{edge_counter}"),
                source: src_id.clone(),
                target: dst_id.clone(),
                data: G6EdgeData {
                    kind: "service-dep".into(),
                    label: "Service → Cache/DB".into(),
                    source_label: svc.name.clone(),
                    target_label: target.clone(),
                    path: None,
                    source_fan_out: src_fan_out,
                    source_lane_index: src_lane_idx,
                    target_fan_in: total_for_target,
                    target_lane_index: idx_for_target,
                    curve_offset,
                },
            });
        }
    }

    let edge_count = edges.len();
    G6GraphPayload {
        meta: G6TopologyMeta {
            env_label: snap.env.label().into(),
            namespace: snap.namespace.clone(),
            context: snap.context.clone(),
            service_count: snap.services.len(),
            ingress_count: snap.ingresses.len(),
            edge_count,
            is_demo: snap.is_demo,
            updated_at: snap.updated_at.clone(),
        },
        selected_service: selected_service.map(str::to_string),
        data: G6GraphData { combos, nodes, edges },
    }
}

/// Computes a lightweight signature for detecting when the graph data needs to be re-pushed to JS.
fn snapshot_signature(snap: &ClusterSnapshot) -> String {
    let mut sig = format!(
        "{:?}|{}|{}|{}|{}|{}",
        snap.env,
        snap.namespace,
        snap.updated_at,
        snap.is_demo,
        snap.services.len(),
        snap.ingresses.len()
    );
    for svc in &snap.services {
        sig.push('|');
        sig.push_str(&svc.name);
        sig.push(':');
        sig.push_str(health_key(svc.health));
        sig.push(':');
        sig.push_str(&svc.ready_replicas.to_string());
        sig.push('/');
        sig.push_str(&svc.desired_replicas.to_string());
        sig.push(':');
        sig.push_str(&svc.ports);
        for target in &svc.connects_to {
            sig.push('~');
            sig.push_str(target);
        }
    }
    for ing in &snap.ingresses {
        sig.push('|');
        sig.push_str(&ing.host);
        sig.push('@');
        sig.push_str(&ing.path);
        sig.push('>');
        sig.push_str(&ing.backend_service);
    }
    sig
}

/// Owns the child `wry::WebView` on the UI thread, keeping visibility, bounds, and G6 data synced
/// with the Rune window without panicking or leaving orphan overlays.
pub struct G6WebViewHost {
    window: Option<WindowHandle>,
    webview: Option<wry::WebView>,
    visible: bool,
    last_bounds: Option<[f32; 4]>,
    last_sig: Option<String>,
    last_selected: Option<Option<String>>,
    failed_init: bool,
}

impl Default for G6WebViewHost {
    fn default() -> Self {
        Self::new()
    }
}

impl G6WebViewHost {
    pub fn new() -> Self {
        Self {
            window: None,
            webview: None,
            visible: false,
            last_bounds: None,
            last_sig: None,
            last_selected: None,
            failed_init: false,
        }
    }

    pub fn attach_window(&mut self, handle: WindowHandle) {
        // If native window was recreated, drop any stale child WebView first.
        self.webview = None;
        self.visible = false;
        self.last_bounds = None;
        self.last_sig = None;
        self.last_selected = None;
        self.failed_init = false;
        self.window = Some(handle);
    }

    pub fn destroy(&mut self) {
        if let Some(wv) = self.webview.take() {
            let _ = wv.set_visible(false);
            drop(wv);
        }
        self.visible = false;
        self.last_bounds = None;
        self.last_sig = None;
        self.last_selected = None;
    }

    pub fn hide(&mut self) {
        if self.visible {
            if let Some(wv) = &self.webview {
                let _ = wv.set_visible(false);
            }
            self.visible = false;
        }
    }

    fn should_show(state: &AppState) -> bool {
        state.session.is_some()
            && state.g6_window_visible
            && state.page == Page::Namespace
            && state.tab == Tab::Topology
            && state.topology_mode == TopologyViewMode::G6
    }

    /// Called on the main UI thread after each drawn Rune frame (`WindowSpec::on_frame_drawn`).
    pub fn sync(&mut self, state: &AppState) {
        let wants_visible = Self::should_show(state);
        let rect = state.g6_viewport_rect.get();
        let valid_rect = rect[2] >= 24.0 && rect[3] >= 24.0;

        if !wants_visible || !valid_rect {
            if self.visible {
                if let Some(wv) = &self.webview {
                    let _ = wv.set_visible(false);
                }
                self.visible = false;
            }
            return;
        }

        // Lazily create child WebView the first time the G6 topology viewport is actually visible.
        if self.webview.is_none() && !self.failed_init {
            let Some(win) = &self.window else { return };
            state
                .g6_ready
                .store(false, std::sync::atomic::Ordering::Release);
            let tx = state.g6_tx.clone();
            let waker = state.waker.clone();
            match self.create_child_webview(win, rect, tx, waker) {
                Ok(wv) => {
                    self.webview = Some(wv);
                    self.visible = true;
                    self.last_bounds = Some(rect);
                }
                Err(err) => {
                    eprintln!("k9s-ui: failed to initialize wry WebView for G6 topology: {err}");
                    self.failed_init = true;
                    let _ = state
                        .g6_tx
                        .send(TopologyWebEvent::LoadError { message: err });
                    if let Some(w) = &state.waker {
                        w();
                    }
                    return;
                }
            }
        }

        let Some(wv) = &self.webview else { return };

        if !self.visible {
            let _ = wv.set_visible(true);
            self.visible = true;
        }

        if bounds_changed(self.last_bounds, rect) {
            let bounds = wry_rect(rect);
            if wv.set_bounds(bounds).is_ok() {
                self.last_bounds = Some(rect);
            }
        }

        // Only push JS payloads once the WebView signals `ready` (avoids lost calls during boot).
        if !state.g6_ready.load(std::sync::atomic::Ordering::Acquire) {
            return;
        }

        let sig = snapshot_signature(&state.snapshot);
        if self.last_sig.as_deref() != Some(sig.as_str()) {
            let payload = build_g6_topology(&state.snapshot, state.selected_service.as_deref());
            if let Ok(json) = serde_json::to_string(&payload) {
                let js = format!("window.__k9sTopology && window.__k9sTopology.setData({json});");
                if wv.evaluate_script(&js).is_ok() {
                    self.last_sig = Some(sig);
                    self.last_selected = Some(state.selected_service.clone());
                }
            }
        } else if self.last_selected.as_ref() != Some(&state.selected_service) {
            if let Ok(sel_json) = serde_json::to_string(&state.selected_service) {
                let js = format!(
                    "window.__k9sTopology && window.__k9sTopology.setSelected({sel_json});"
                );
                if wv.evaluate_script(&js).is_ok() {
                    self.last_selected = Some(state.selected_service.clone());
                }
            }
        }

        // Drain any toolbar commands queued by Rune buttons (Fit, Zoom +/-, Reset).
        while let Ok(cmd) = state.g6_cmd_rx.try_recv() {
            let js = match cmd {
                G6Command::FitView => "window.__k9sTopology && window.__k9sTopology.fitView();",
                G6Command::ZoomIn => "window.__k9sTopology && window.__k9sTopology.zoomBy(1.2);",
                G6Command::ZoomOut => "window.__k9sTopology && window.__k9sTopology.zoomBy(1/1.2);",
                G6Command::ResetView => "window.__k9sTopology && window.__k9sTopology.resetView();",
            };
            let _ = wv.evaluate_script(js);
        }
    }

    fn create_child_webview(
        &self,
        win: &WindowHandle,
        rect: [f32; 4],
        tx: mpsc::Sender<TopologyWebEvent>,
        waker: Option<Waker>,
    ) -> Result<wry::WebView, String> {
        let bounds = wry_rect(rect);
        win.with_raw_handle(move |raw_handle| {
            wry::WebViewBuilder::new()
                .with_bounds(bounds)
                .with_visible(true)
                .with_background_color((9, 11, 20, 255))
                .with_accept_first_mouse(true)
                .with_html(TOPOLOGY_G6_HTML)
                .with_ipc_handler(move |req| {
                    if let Ok(ev) = serde_json::from_str::<TopologyWebEvent>(req.body()) {
                        let _ = tx.send(ev);
                        if let Some(w) = &waker {
                            w();
                        }
                    }
                })
                .build_as_child(&raw_handle)
                .map_err(|e| e.to_string())
        })?
    }
}

fn wry_rect([x, y, w, h]: [f32; 4]) -> wry::Rect {
    wry::Rect {
        position: wry::dpi::LogicalPosition::new(x.max(0.0) as f64, y.max(0.0) as f64).into(),
        size: wry::dpi::LogicalSize::new(w.max(1.0) as f64, h.max(1.0) as f64).into(),
    }
}

fn bounds_changed(prev: Option<[f32; 4]>, next: [f32; 4]) -> bool {
    let Some(p) = prev else { return true };
    (p[0] - next[0]).abs() > 0.5
        || (p[1] - next[1]).abs() > 0.5
        || (p[2] - next[2]).abs() > 0.5
        || (p[3] - next[3]).abs() > 0.5
}

thread_local! {
    static G6_HOST: std::cell::RefCell<G6WebViewHost> = std::cell::RefCell::new(G6WebViewHost::new());
}

pub fn attach_window(handle: WindowHandle) {
    G6_HOST.with(|host| host.borrow_mut().attach_window(handle));
}

pub fn sync_webview(state: &AppState) {
    G6_HOST.with(|host| host.borrow_mut().sync(state));
}

pub fn hide_webview() {
    G6_HOST.with(|host| host.borrow_mut().hide());
}

pub fn destroy_webview() {
    G6_HOST.with(|host| host.borrow_mut().destroy());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::k8s::{EnvKind, IngressRoute, PodInfo, demo_snapshot};

    #[test]
    fn serializes_demo_snapshot_without_dangling_edges() {
        let snap = demo_snapshot(EnvKind::Stg, "staging-api", "gke-cluster", None);
        let payload = build_g6_topology(&snap, Some("staging-pg-srv"));

        assert_eq!(payload.data.combos.len(), 3);
        assert_eq!(payload.selected_service.as_deref(), Some("staging-pg-srv"));
        assert!(payload.data.nodes.iter().any(|n| n.id == "node-ingress-switch"));
        // External target `staging-hangfire-redis-srv` referenced in demo_snapshot should be synthesized
        assert!(
            payload
                .data
                .nodes
                .iter()
                .any(|n| n.id == "svc:staging-hangfire-redis-srv" && n.combo == "combo-storage")
        );

        let node_ids: BTreeSet<&str> = payload.data.nodes.iter().map(|n| n.id.as_str()).collect();
        for edge in &payload.data.edges {
            assert!(
                node_ids.contains(edge.source.as_str()),
                "dangling edge source: {}",
                edge.source
            );
            assert!(
                node_ids.contains(edge.target.as_str()),
                "dangling edge target: {}",
                edge.target
            );
        }

        let json = serde_json::to_string(&payload).expect("payload serializes cleanly to JSON");
        assert!(json.contains("\"combos\""));
        assert!(json.contains("\"nodes\""));
        assert!(json.contains("\"edges\""));
    }

    #[test]
    fn handles_many_services_and_high_redis_fan_in_without_truncation() {
        let mut snap = demo_snapshot(EnvKind::Prod, "prod-api", "gke-prod", None);
        snap.services.clear();
        snap.ingresses.clear();

        snap.services.push(ServiceInfo {
            name: "prod-shared-redis-cluster-srv".into(),
            deploy_name: Some("prod-shared-redis-depl".into()),
            namespace: "prod-api".into(),
            role: NodeRole::Cache,
            health: Health::Healthy,
            ready_replicas: 3,
            desired_replicas: 3,
            ports: "6379".into(),
            hosts: vec![],
            connects_to: vec![],
            summary: "Shared Redis".into(),
            pods: vec![],
        });

        for i in 0..24 {
            let name = format!("prod-very-long-microservice-handler-{i:02}-srv");
            snap.services.push(ServiceInfo {
                name: name.clone(),
                deploy_name: Some(format!("prod-deploy-{i:02}")),
                namespace: "prod-api".into(),
                role: NodeRole::Api,
                health: if i == 3 { Health::Error } else { Health::Healthy },
                ready_replicas: if i == 3 { 0 } else { 2 },
                desired_replicas: 2,
                ports: "3010".into(),
                hosts: vec![format!("svc-{i:02}.handler.aombet.net")],
                connects_to: vec!["prod-shared-redis-cluster-srv".into()],
                summary: "API Service".into(),
                pods: vec![PodInfo {
                    name: format!("pod-{i}"),
                    phase: "Running".into(),
                    ready: i != 3,
                    restarts: 0,
                    reason: "Running".into(),
                    explanation: "ok".into(),
                    node_name: "node-1".into(),
                    node_spec: "e2".into(),
                    cpu_spec: "100m".into(),
                    mem_spec: "128Mi".into(),
                    live_usage: "10m".into(),
                }],
            });
            if i < 10 {
                snap.ingresses.push(IngressRoute {
                    name: format!("ing-{i}"),
                    host: format!("svc-{i:02}.handler.aombet.net"),
                    backend_service: name,
                    path: "/".into(),
                });
            }
        }

        // 6 workers (previously truncated at 4 in classic view)
        for i in 0..6 {
            snap.services.push(ServiceInfo {
                name: format!("prod-hangfire-worker-{i}"),
                deploy_name: None,
                namespace: "prod-api".into(),
                role: NodeRole::Worker,
                health: Health::Healthy,
                ready_replicas: 1,
                desired_replicas: 1,
                ports: "-".into(),
                hosts: vec![],
                connects_to: vec!["prod-shared-redis-cluster-srv".into()],
                summary: "Worker".into(),
                pods: vec![],
            });
        }

        let payload = build_g6_topology(&snap, None);
        // 1 switch + 10 domains + 24 apis + 1 redis + 6 workers = 42 nodes (zero silent truncation)
        assert_eq!(payload.data.nodes.len(), 42);
        let redis_node = payload
            .data
            .nodes
            .iter()
            .find(|n| n.id == "svc:prod-shared-redis-cluster-srv")
            .unwrap();
        assert_eq!(redis_node.data.fan_in, 30);

        let dep_edges: Vec<&G6Edge> = payload
            .data
            .edges
            .iter()
            .filter(|e| e.data.kind == "service-dep")
            .collect();
        assert_eq!(dep_edges.len(), 30);
        // Curve offsets should spread smoothly across negative and positive values
        assert!(dep_edges.iter().any(|e| e.data.curve_offset < -20.0));
        assert!(dep_edges.iter().any(|e| e.data.curve_offset > 20.0));

        // Verify ZERO node-to-node bounding-box overlap and ZERO combo-to-combo overlap!
        let node_box = |n: &G6Node| -> (f32, f32, f32, f32) {
            let (hw, hh) = match n.data.kind.as_str() {
                "domain" => (122.0_f32, 23.0_f32),
                "switch" => (98.0_f32, 34.0_f32),
                _ => (112.0_f32, 34.0_f32),
            };
            (n.x - hw, n.y - hh, n.x + hw, n.y + hh)
        };

        for i in 0..payload.data.nodes.len() {
            let a = node_box(&payload.data.nodes[i]);
            for j in (i + 1)..payload.data.nodes.len() {
                let b = node_box(&payload.data.nodes[j]);
                let overlaps = a.0 < b.2 && a.2 > b.0 && a.1 < b.3 && a.3 > b.1;
                assert!(
                    !overlaps,
                    "nodes {} and {} overlap: {:?} vs {:?}",
                    payload.data.nodes[i].id, payload.data.nodes[j].id, a, b
                );
            }
        }

        let combo_bounds = |cid: &str| -> (f32, f32, f32, f32) {
            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            for n in payload.data.nodes.iter().filter(|n| n.combo == cid) {
                let b = node_box(n);
                min_x = min_x.min(b.0);
                min_y = min_y.min(b.1);
                max_x = max_x.max(b.2);
                max_y = max_y.max(b.3);
            }
            (min_x - 36.0, min_y - 68.0, max_x + 36.0, max_y + 36.0)
        };
        let apps_b = combo_bounds("combo-apps");
        let net_b = combo_bounds("combo-network");
        let store_b = combo_bounds("combo-storage");

        // Top apps combo sits well above both bottom combos (>=180px gap)
        assert!(net_b.1 - apps_b.3 >= 180.0, "apps/net vertical gap too small: {:?}", (apps_b, net_b));
        assert!(store_b.1 - apps_b.3 >= 180.0, "apps/store vertical gap too small: {:?}", (apps_b, store_b));
        // Network combo sits well left of storage combo (>=150px gap)
        assert!(store_b.0 - net_b.2 >= 150.0, "net/store horizontal gap too small: {:?}", (net_b, store_b));
    }

    #[test]
    fn omits_empty_combos_and_ignores_self_loop_fan_in() {
        let mut snap = demo_snapshot(EnvKind::Stg, "empty-ish", "gke-stg", None);
        snap.ingresses.clear();
        snap.services = vec![ServiceInfo {
            name: "solo-cache".into(),
            deploy_name: None,
            namespace: "empty-ish".into(),
            role: NodeRole::Cache,
            health: Health::Healthy,
            ready_replicas: 1,
            desired_replicas: 1,
            ports: "6379".into(),
            hosts: vec![],
            connects_to: vec!["solo-cache".into()],
            summary: "Self-referencing cache".into(),
            pods: vec![],
        }];

        let payload = build_g6_topology(&snap, None);
        // No APIs -> combo-apps omitted; switch + cache present -> combo-network & combo-storage present
        let combo_ids: Vec<&str> = payload.data.combos.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(combo_ids, vec!["combo-network", "combo-storage"]);
        assert!(payload.data.edges.is_empty());
        let cache = payload
            .data
            .nodes
            .iter()
            .find(|n| n.id == "svc:solo-cache")
            .unwrap();
        assert_eq!(cache.data.fan_in, 0);
        assert_eq!(cache.data.pod_badge.as_deref(), Some("1/1 Pods"));
        assert_eq!(cache.data.ports.as_deref(), Some("6379"));
    }

    #[test]
    fn omits_storage_combo_when_no_cache_or_workers() {
        let mut snap = demo_snapshot(EnvKind::Stg, "web-only", "gke-stg", None);
        snap.services.retain(|s| s.role == NodeRole::Api);
        for s in &mut snap.services {
            s.connects_to.clear();
        }

        let payload = build_g6_topology(&snap, None);
        let combo_ids: Vec<&str> = payload.data.combos.iter().map(|c| c.id.as_str()).collect();
        // Since no caches, external caches, or workers exist, combo-storage must be omitted
        assert_eq!(combo_ids, vec!["combo-apps", "combo-network"]);
        assert!(payload.data.combos.iter().all(|c| c.id != "combo-storage"));
    }

    #[test]
    fn verifies_multi_port_and_domain_columns_layout() {
        let mut snap = demo_snapshot(EnvKind::Prod, "multi-port-test", "gke-prod", None);
        snap.services.clear();
        snap.ingresses.clear();

        // Multi-port API service
        snap.services.push(ServiceInfo {
            name: "gateway-api".into(),
            deploy_name: Some("gateway-api-depl".into()),
            namespace: "multi-port-test".into(),
            role: NodeRole::Api,
            health: Health::Healthy,
            ready_replicas: 2,
            desired_replicas: 2,
            ports: "80, 443, 8080".into(),
            hosts: vec!["gateway.aombet.net".into()],
            connects_to: vec![],
            summary: "Multi-port API Gateway".into(),
            pods: vec![],
        });

        // 8 ingress routes (>4 triggers 2-column domain spread)
        for i in 0..8 {
            snap.ingresses.push(IngressRoute {
                name: format!("ing-{i}"),
                host: format!("route-{i}.aombet.net"),
                backend_service: "gateway-api".into(),
                path: format!("/api/v1/sub-{i}"),
            });
        }

        let payload = build_g6_topology(&snap, None);
        let domain_nodes: Vec<&G6Node> = payload
            .data
            .nodes
            .iter()
            .filter(|n| n.data.kind == "domain")
            .collect();
        assert_eq!(domain_nodes.len(), 8);

        // Check that domains are spread across 2 columns (different x coordinates)
        let unique_x: BTreeSet<i32> = domain_nodes.iter().map(|n| n.x.round() as i32).collect();
        assert_eq!(unique_x.len(), 2, "8 domains should be distributed across 2 distinct columns");

        let gw_node = payload
            .data
            .nodes
            .iter()
            .find(|n| n.id == "svc:gateway-api")
            .unwrap();
        assert_eq!(gw_node.data.ports.as_deref(), Some("80, 443, 8080"));

        // Verify switch -> domain fan-out lanes and domain -> gateway-api fan-in lanes are strictly distinct
        let sw_edges: Vec<&G6Edge> = payload
            .data
            .edges
            .iter()
            .filter(|e| e.data.kind == "ingress-switch")
            .collect();
        assert_eq!(sw_edges.len(), 8);
        let sw_lanes: BTreeSet<usize> = sw_edges.iter().map(|e| e.data.source_lane_index).collect();
        assert_eq!(sw_lanes.len(), 8, "every switch->domain edge must have a unique source_lane_index");
        assert!(sw_edges.iter().all(|e| e.data.source_fan_out == 8));

        let ing_edges: Vec<&G6Edge> = payload
            .data
            .edges
            .iter()
            .filter(|e| e.data.kind == "ingress-route")
            .collect();
        assert_eq!(ing_edges.len(), 8);
        let ing_lanes: BTreeSet<usize> = ing_edges.iter().map(|e| e.data.target_lane_index).collect();
        assert_eq!(ing_lanes.len(), 8, "every domain->gateway-api edge must have a unique target_lane_index");
        assert!(ing_edges.iter().all(|e| e.data.target_fan_in == 8));
    }
}
