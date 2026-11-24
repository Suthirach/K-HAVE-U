use crate::k8s::{EnvKind, Health, NodeRole, ServiceInfo};
use crate::state::{AppState, ServiceTab, Tab};
use crate::ui::icons::{self, mono_icon};
use crate::ui::pill::{
    PillSize, PillVariant, interactive, pill_btn, pill_icon_btn, pill_icon_only, segmented_group,
    soft_badge,
};
use crate::ui::topology_g6::{G6Command, TopologyViewMode};
use crate::ui::{alert_blink_color, health_color, theme};
use rune::{
    Color, Cursor, Drag, El, LineCap, Path, Phase, ScrollSpec, Shadow, Shape, TextStyle,
    WheelEvent, col, el, row, text,
};
use rune::tok::motion;
use rune_kit::Kit;
use rune_mermaid::{DiagramStyle, Hover};

pub fn render_mermaid_zoomable(src: &str, zoom: f32, fullscreen: bool, k: &Kit) -> El {
    let mut style = DiagramStyle::neutral(true).min_scale(zoom);
    style.max_width = 920.0 * zoom;
    style.line = theme::warm_accent();
    style.node_border = theme::border_strong();
    style.node_fill = theme::bg_card_solid();
    style.surface = theme::bg_deep();
    style.surface_border = theme::border_glass();
    style.text = theme::text_strong();
    style.muted = theme::text_muted();

    let highlight = theme::warn_dot();
    let diagram = rune_mermaid::render_with_hover(src, &style, Hover::Neighbours, highlight)
        .unwrap_or_else(|| {
            text("ไม่สามารถวาดแผนภาพได้")
                .font(k.ty.text_small)
                .color(theme::text_muted())
        });

    let pct = format!("{}%", (zoom * 100.0).round() as i32);

    let zoom_canvas = col()
        .w_full()
        .padding(20.0)
        .center()
        .radius(if fullscreen { 0.0 } else { 16.0 })
        .fill(theme::bg_deep())
        .border(if fullscreen { 0.0 } else { 1.0 }, theme::border_subtle())
        .on_wheel(|s: &mut AppState, e: WheelEvent, _| {
            let factor = if e.dy > 0.0 {
                1.15
            } else if e.dy < 0.0 {
                1.0 / 1.15
            } else {
                1.0
            };
            if factor != 1.0 {
                let old_z = s.diagram_zoom;
                let new_z = (old_z * factor).clamp(0.4, 3.0);
                let ratio = new_z / old_z;
                s.diagram_pan_x = e.x - (e.x - s.diagram_pan_x) * ratio;
                s.diagram_pan_y = e.y - (e.y - s.diagram_pan_y) * ratio;
                s.diagram_zoom = new_z;
                return true;
            }
            false
        })
        .child(diagram)
        .scroll(ScrollSpec { stick_bottom: false });

    if fullscreen {
        return col()
            .key("true-fullscreen-mermaid")
            .w_full()
            .h_full()
            .fill(theme::bg_canvas())
            .child(zoom_canvas.grow(1.0).shrink().min_h(0.0))
            .child(floating_zoom_bar(pct, k));
    }

    col()
        .key("mermaid-card")
        .w_full()
        .padding(16.0)
        .gap(12.0)
        .radius(20.0)
        .fill(theme::glass_card())
        .border(1.0, theme::border_glass())
        .child(
            row()
                .w_full()
                .center_y()
                .gap(8.0)
                .child(mono_icon(&icons::TOPOLOGY, 15.0, theme::text_secondary()))
                .child(
                    text("Service Flow Diagram (Scroll เมาส์เพื่อ Zoom)")
                        .font(k.ty.body_medium)
                        .color(theme::text_strong()),
                )
                .child(el().grow(1.0))
                .child(zoom_toolbar(pct)),
        )
        .child(zoom_canvas.h(420.0))
}

fn zoom_toolbar(pct: String) -> El {
    row()
        .gap(6.0)
        .center_y()
        .child(pill_icon_only(
            &icons::MINUS,
            "Zoom Out",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.diagram_zoom = (s.diagram_zoom / 1.2).max(0.4),
        ))
        .child(soft_badge(
            pct,
            theme::metric_box_soft(),
            theme::text_secondary(),
            Some(theme::border_subtle()),
        ))
        .child(pill_icon_only(
            &icons::PLUS,
            "Zoom In",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.diagram_zoom = (s.diagram_zoom * 1.2).min(3.0),
        ))
        .child(pill_btn(
            "Reset",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| {
                s.diagram_zoom = 1.0;
                s.diagram_pan_x = 24.0;
                s.diagram_pan_y = 20.0;
            },
        ))
        .child(pill_icon_btn(
            &icons::FULLSCREEN,
            "เต็มจอ",
            PillVariant::Primary,
            PillSize::Sm,
            |s: &mut AppState| s.diagram_fullscreen = true,
        ))
}

fn floating_zoom_bar(pct: String, k: &Kit) -> El {
    let bar = row()
        .key("fs-floating-controls")
        .padding(8.0)
        .gap(6.0)
        .center_y()
        .radius(999.0)
        .fill(theme::bg_chrome().alpha(0.95))
        .border(1.0, theme::border_medium())
        .child(
            text("Scroll เมาส์เพื่อ Zoom • ลากเพื่อเลื่อน")
                .font(k.ty.small_medium)
                .color(theme::text_muted()),
        )
        .child(pill_icon_only(
            &icons::MINUS,
            "Zoom Out",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.diagram_zoom = (s.diagram_zoom / 1.2).max(0.4),
        ))
        .child(soft_badge(
            pct,
            theme::metric_box_soft(),
            theme::text_strong(),
            Some(theme::border_subtle()),
        ))
        .child(pill_icon_only(
            &icons::PLUS,
            "Zoom In",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.diagram_zoom = (s.diagram_zoom * 1.2).min(3.0),
        ))
        .child(pill_btn(
            "Reset",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| {
                s.diagram_zoom = 1.0;
                s.diagram_pan_x = 24.0;
                s.diagram_pan_y = 20.0;
            },
        ))
        .child(pill_icon_btn(
            &icons::CLOSE,
            "ออกจากเต็มจอ (Esc)",
            PillVariant::Danger,
            PillSize::Sm,
            |s: &mut AppState| s.diagram_fullscreen = false,
        ));

    row()
        .w_full()
        .px(20.0)
        .pb(16.0)
        .child(el().grow(1.0))
        .child(bar)
}

/// Classic Server Tower Architecture Diagram unified with Warm Dark Glass Minimal palette.
fn infra_building_canvas(s: &AppState, fullscreen: bool) -> El {
    use std::collections::HashMap;

    let snap = &s.snapshot;
    let z = s.diagram_zoom;

    let apis: Vec<&ServiceInfo> = snap
        .services
        .iter()
        .filter(|x| x.role == NodeRole::Api)
        .collect();
    let workers: Vec<&ServiceInfo> = snap
        .services
        .iter()
        .filter(|x| x.role == NodeRole::Worker)
        .collect();
    let caches: Vec<&ServiceInfo> = snap
        .services
        .iter()
        .filter(|x| x.role == NodeRole::Cache)
        .collect();

    let cols = 6usize;
    let cell_w = 175.0 * z;
    let cell_h = 135.0 * z;
    let api_rows_count = apis.len().div_ceil(cols).max(1);
    let app_zone_h = 54.0 * z + api_rows_count as f32 * cell_h;
    let app_zone_w = (apis.len().min(cols).max(3) as f32) * cell_w + 36.0 * z;

    let bottom_y = 20.0 * z + app_zone_h + 70.0 * z;
    let total_w = app_zone_w.max(1060.0 * z) + 40.0 * z;
    let total_h = bottom_y + 300.0 * z;

    let mut svc_top_port: HashMap<String, [f32; 2]> = HashMap::new();
    let mut svc_bot_port: HashMap<String, [f32; 2]> = HashMap::new();
    let mut placed_nodes: Vec<El> = Vec::new();

    // Place Application Server Towers
    for (i, svc) in apis.iter().enumerate() {
        let r = i / cols;
        let c_idx = i % cols;
        let nx = 32.0 * z + c_idx as f32 * cell_w;
        let ny = 64.0 * z + r as f32 * cell_h;
        let selected = s.selected_service.as_deref() == Some(svc.name.as_str());
        let name = svc.name.clone();

        svc_top_port.insert(svc.name.clone(), [nx + 72.0 * z, ny + 8.0 * z]);
        svc_bot_port.insert(svc.name.clone(), [nx + 72.0 * z, ny + 106.0 * z]);

        placed_nodes.push(
            el().w(144.0 * z)
                .absolute(nx, ny)
                .center()
                .child(server_tower_tile(
                    svc.name.clone(),
                    format!("{}/{} Pods", svc.ready_replicas, svc.desired_replicas),
                    svc.health,
                    selected,
                    s.pulse_on,
                    z,
                    move |s: &mut AppState| s.selected_service = Some(name.clone()),
                )),
        );
    }

    // Place Network / Ingress Switch + Domain Pills (Bottom-Left)
    let switch_x = 46.0 * z;
    let switch_y = bottom_y + 72.0 * z;
    let switch_port = [switch_x + 48.0 * z, switch_y + 32.0 * z];
    placed_nodes.push(
        el().absolute(switch_x, switch_y)
            .child(switch_cluster_tile("Nginx Ingress", snap.ingresses.len(), z)),
    );

    let mut domain_ports: Vec<(String, [f32; 2], [f32; 2])> = Vec::new();
    for (i, ing) in snap.ingresses.iter().take(6).enumerate() {
        let dx = 185.0 * z;
        let dy = bottom_y + 42.0 * z + i as f32 * 34.0 * z;
        let backend = ing.backend_service.clone();
        let active_edge = s.selected_service.as_deref() == Some(ing.backend_service.as_str());

        domain_ports.push((
            ing.backend_service.clone(),
            [dx, dy + 13.0 * z],
            [dx + 250.0 * z, dy + 13.0 * z],
        ));

        placed_nodes.push(
            el().absolute(dx, dy).child(interactive(
                row()
                    .w(250.0 * z)
                    .h(26.0 * z)
                    .px(10.0 * z)
                    .gap(6.0 * z)
                    .center_y()
                    .radius(999.0)
                    .fill(if active_edge {
                        theme::accent_primary_bg()
                    } else {
                        theme::bg_card_solid()
                    })
                    .border(
                        1.0,
                        if active_edge {
                            theme::accent_primary_bg()
                        } else {
                            theme::border_medium()
                        },
                    )
                    .on_press(move |s: &mut AppState| s.selected_service = Some(backend.clone()))
                    .child(mono_icon(
                        &icons::GLOBE,
                        (11.0 * z).max(8.0),
                        if active_edge {
                            theme::accent_primary_fg()
                        } else {
                            theme::text_muted()
                        },
                    ))
                    .child(
                        text(ing.host.clone())
                            .font(TextStyle::new(10.0 * z, 600.0))
                            .color(if active_edge {
                                theme::accent_primary_fg()
                            } else {
                                theme::text_strong()
                            })
                            .grow(1.0)
                            .shrink()
                            .min_w(0.0),
                    ),
            )),
        );
    }

    // Place Storage (Redis) & Workers (Bottom-Right)
    let storage_start_x = 530.0 * z;
    let mut idx_right = 0usize;
    for svc in caches {
        let nx = storage_start_x + (idx_right % 4) as f32 * 135.0 * z;
        let ny = bottom_y + 44.0 * z + (idx_right / 4) as f32 * 115.0 * z;
        let selected = s.selected_service.as_deref() == Some(svc.name.as_str());
        let name = svc.name.clone();

        svc_top_port.insert(svc.name.clone(), [nx + 52.0 * z, ny + 4.0 * z]);
        placed_nodes.push(
            el().w(104.0 * z)
                .absolute(nx, ny)
                .center()
                .child(storage_cube_tile(
                    svc.name.clone(),
                    "Redis / DB".into(),
                    svc.health,
                    selected,
                    s.pulse_on,
                    z,
                    move |s: &mut AppState| s.selected_service = Some(name.clone()),
                )),
        );
        idx_right += 1;
    }
    for svc in workers.iter().take(4) {
        let nx = storage_start_x + (idx_right % 4) as f32 * 135.0 * z;
        let ny = bottom_y + 44.0 * z + (idx_right / 4) as f32 * 125.0 * z;
        let selected = s.selected_service.as_deref() == Some(svc.name.as_str());
        let name = svc.name.clone();

        svc_top_port.insert(svc.name.clone(), [nx + 56.0 * z, ny + 4.0 * z]);
        svc_bot_port.insert(svc.name.clone(), [nx + 56.0 * z, ny + 104.0 * z]);
        placed_nodes.push(
            el().w(112.0 * z)
                .absolute(nx, ny)
                .center()
                .child(server_tower_tile(
                    svc.name.clone(),
                    "Worker".into(),
                    svc.health,
                    selected,
                    s.pulse_on,
                    z,
                    move |s: &mut AppState| s.selected_service = Some(name.clone()),
                )),
        );
        idx_right += 1;
    }

    // Build Vector Connection Lines Layer using Rune Shape
    let mut lines_shape = Shape::new();
    let ingress_line = Color::WHITE.alpha(0.45);
    let dep_line = theme::warm_accent().alpha(0.55);
    let hl_line = theme::warn_dot();
    let dim_line = Color::WHITE.alpha(0.12);

    // 1. Switch -> Domain Pills -> Backend Service Towers
    for (backend, dom_left, dom_right) in &domain_ports {
        let is_hl = s.selected_service.as_deref() == Some(backend.as_str());
        let col = if is_hl {
            hl_line
        } else if s.selected_service.is_some() {
            dim_line
        } else {
            ingress_line
        };
        let stroke_w = if is_hl { 2.4 * z } else { 1.4 * z };

        let sw_curve = Path::new().move_to(switch_port).cubic_to(
            [switch_port[0] + 45.0 * z, switch_port[1]],
            [dom_left[0] - 45.0 * z, dom_left[1]],
            *dom_left,
        );
        lines_shape = lines_shape.stroke_path(&sw_curve, stroke_w, LineCap::Round, col);

        if let Some(target_port) = svc_bot_port
            .get(backend)
            .or_else(|| svc_top_port.get(backend))
        {
            let mid_y = (dom_right[1] + target_port[1]) * 0.5;
            let svc_curve = Path::new().move_to(*dom_right).cubic_to(
                [dom_right[0] + 40.0 * z, mid_y],
                [target_port[0], mid_y],
                *target_port,
            );
            lines_shape = lines_shape.stroke_path(&svc_curve, stroke_w, LineCap::Round, col);
            lines_shape = lines_shape.line(
                *target_port,
                [target_port[0], target_port[1] + 2.0],
                4.5 * z,
                LineCap::Round,
                col,
            );
        }
    }

    // 2. Service / Worker Towers -> Redis / Storage Cubes
    for svc in &snap.services {
        let Some(from_port) = svc_bot_port.get(&svc.name) else {
            continue;
        };
        for target in &svc.connects_to {
            let Some(to_port) = svc_top_port.get(target) else {
                continue;
            };
            let is_hl = s.selected_service.as_deref() == Some(svc.name.as_str())
                || s.selected_service.as_deref() == Some(target.as_str());
            let col = if is_hl {
                hl_line
            } else if s.selected_service.is_some() {
                dim_line
            } else {
                dep_line
            };
            let stroke_w = if is_hl { 2.4 * z } else { 1.4 * z };
            let mid_y = (from_port[1] + to_port[1]) * 0.5;
            let curve = Path::new().move_to(*from_port).cubic_to(
                [from_port[0], mid_y],
                [to_port[0], mid_y],
                *to_port,
            );
            lines_shape = lines_shape.stroke_path(&curve, stroke_w, LineCap::Round, col);
            lines_shape = lines_shape.line(
                *to_port,
                [to_port[0], to_port[1] - 2.0],
                4.5 * z,
                LineCap::Round,
                col,
            );
        }
    }

    let lines_layer = el()
        .size(total_w, total_h)
        .absolute(0.0, 0.0)
        .shape(lines_shape.view_box(total_w, total_h));

    // Warm Dark Glass Zone Frames
    let app_zone_box = col()
        .size(app_zone_w, app_zone_h)
        .absolute(20.0 * z, 20.0 * z)
        .radius(20.0 * z)
        .fill(Color::WHITE.alpha(0.035))
        .border(1.0, theme::border_glass())
        .padding(12.0 * z)
        .child(row().child(zone_pill(
            format!("APPLICATION SERVERS — {} ({})", snap.namespace, apis.len()),
            z,
        )));

    let net_zone_box = col()
        .size(450.0 * z, 240.0 * z)
        .absolute(20.0 * z, bottom_y)
        .radius(20.0 * z)
        .fill(Color::WHITE.alpha(0.035))
        .border(1.0, theme::border_glass())
        .padding(12.0 * z)
        .child(row().child(zone_pill(
            format!("NETWORK / INGRESS SWITCH ({})", snap.env.label()),
            z,
        )));

    let store_zone_box = col()
        .size(560.0 * z, 240.0 * z)
        .absolute(500.0 * z, bottom_y)
        .radius(20.0 * z)
        .fill(Color::WHITE.alpha(0.035))
        .border(1.0, theme::border_glass())
        .padding(12.0 * z)
        .child(row().child(zone_pill("STORAGE & WORKERS CLUSTER".to_string(), z)));

    let world = el()
        .key("infra-world")
        .size(total_w, total_h)
        .absolute(s.diagram_pan_x, s.diagram_pan_y)
        .child(app_zone_box)
        .child(net_zone_box)
        .child(store_zone_box)
        .child(lines_layer)
        .children(placed_nodes);

    let mut canvas = el()
        .key("infra-canvas")
        .w_full()
        .clip_rounded(if fullscreen { 0.0 } else { 20.0 })
        .cursor(Cursor::Grab)
        .fill(theme::bg_canvas())
        .radial_gradient(
            [0.16, 0.10],
            [620.0, 340.0],
            [
                (0.0, theme::glow_warm()),
                (0.65, Color::TRANSPARENT),
            ],
        )
        .on_wheel(|s: &mut AppState, e: WheelEvent, _| {
            let factor = if e.dy > 0.0 {
                1.15
            } else if e.dy < 0.0 {
                1.0 / 1.15
            } else {
                1.0
            };
            if factor != 1.0 {
                let old_z = s.diagram_zoom;
                let new_z = (old_z * factor).clamp(0.4, 3.0);
                let ratio = new_z / old_z;
                s.diagram_pan_x = e.x - (e.x - s.diagram_pan_x) * ratio;
                s.diagram_pan_y = e.y - (e.y - s.diagram_pan_y) * ratio;
                s.diagram_zoom = new_z;
                return true;
            }
            false
        })
        .on_drag(|s: &mut AppState, d: &mut Drag, _| match d.phase {
            Phase::Down => {
                s.diagram_drag_start_x = s.diagram_pan_x;
                s.diagram_drag_start_y = s.diagram_pan_y;
            }
            Phase::Move | Phase::Up => {
                s.diagram_pan_x = s.diagram_drag_start_x + d.dx;
                s.diagram_pan_y = s.diagram_drag_start_y + d.dy;
            }
        })
        .child(world);

    if fullscreen {
        canvas = canvas.grow(1.0).shrink().min_h(0.0);
    } else {
        canvas = canvas
            .h(560.0)
            .radius(20.0)
            .border(1.0, theme::border_glass());
    }

    canvas
}

fn zone_pill(title: String, z: f32) -> El {
    row()
        .px(12.0 * z)
        .py(4.0 * z)
        .radius(999.0)
        .fill(theme::bg_card_solid())
        .border(1.0, theme::border_medium())
        .child(
            text(title)
                .font(TextStyle::new(10.5 * z, 700.0))
                .color(theme::text_strong()),
        )
}

fn server_tower_tile(
    name: String,
    sub: String,
    health: Health,
    selected: bool,
    pulse_on: bool,
    z: f32,
    on_click: impl Fn(&mut AppState) + 'static,
) -> El {
    let has_issue = health != Health::Healthy;
    let led = alert_blink_color(health, pulse_on);
    let label_pill = row()
        .px(10.0 * z)
        .py(3.5 * z)
        .radius(999.0)
        .fill(if has_issue && pulse_on {
            theme::err_card_bg()
        } else if selected {
            theme::accent_primary_bg()
        } else {
            theme::bg_card_solid()
        })
        .border(
            1.0,
            if has_issue {
                led
            } else if selected {
                theme::accent_primary_bg()
            } else {
                theme::border_medium()
            },
        )
        .child(
            text(name)
                .font(TextStyle::new(10.5 * z, 600.0))
                .color(if selected {
                    theme::accent_primary_fg()
                } else {
                    theme::text_strong()
                }),
        );

    let rack_slot = |col_bar: Color| {
        row()
            .w(44.0 * z)
            .h(10.0 * z)
            .px(4.0 * z)
            .center_y()
            .radius(3.0 * z)
            .fill(theme::bg_deep())
            .border(1.0, theme::border_subtle())
            .child(el().size(4.5 * z, 4.5 * z).radius(3.0 * z).fill(col_bar))
            .child(el().grow(1.0))
            .child(el().w(18.0 * z).h(2.0 * z).radius(1.0).fill(Color::WHITE.alpha(0.32)))
    };

    let tower_body = col()
        .w(58.0 * z)
        .padding(5.0 * z)
        .gap(4.0 * z)
        .center()
        .radius(12.0 * z)
        .fill(theme::bg_card_solid())
        .border(
            if selected || has_issue { 1.8 } else { 1.0 },
            if has_issue {
                led
            } else if selected {
                theme::accent_primary_bg()
            } else {
                theme::border_medium()
            },
        )
        .box_shadow(Shadow {
            x: 0.0,
            y: 6.0 * z,
            blur: 14.0 * z,
            spread: 0.0,
            color: if has_issue {
                led.alpha(0.55)
            } else {
                Color::BLACK.alpha(0.45)
            },
            inset: false,
        })
        .child(rack_slot(led))
        .child(rack_slot(led))
        .child(rack_slot(theme::warm_accent()))
        .child(
            text(sub)
                .font(TextStyle::new(9.5 * z, 600.0))
                .color(theme::text_muted()),
        );

    interactive(
        col()
            .center()
            .gap(6.0 * z)
            .padding(4.0 * z)
            .radius(14.0 * z)
            .on_press(on_click)
            .child(label_pill)
            .child(tower_body),
    )
}

fn switch_cluster_tile(title: &str, count: usize, z: f32) -> El {
    let cube = col()
        .size(64.0 * z, 64.0 * z)
        .center()
        .gap(4.0 * z)
        .radius(16.0 * z)
        .fill(theme::bg_card_solid())
        .border(1.2, theme::border_strong())
        .box_shadow(Shadow {
            x: 0.0,
            y: 6.0 * z,
            blur: 16.0 * z,
            spread: 0.0,
            color: Color::BLACK.alpha(0.45),
            inset: false,
        })
        .child(mono_icon(
            &icons::SWITCH_ARROWS,
            (20.0 * z).max(12.0),
            theme::text_strong(),
        ))
        .child(
            text(format!("{count} routes"))
                .font(TextStyle::new(9.5 * z, 600.0))
                .color(theme::text_muted()),
        );

    col()
        .center()
        .gap(6.0 * z)
        .child(cube)
        .child(zone_pill(title.to_string(), z))
}

fn storage_cube_tile(
    name: String,
    sub: String,
    health: Health,
    selected: bool,
    pulse_on: bool,
    z: f32,
    on_click: impl Fn(&mut AppState) + 'static,
) -> El {
    let has_issue = health != Health::Healthy;
    let led = alert_blink_color(health, pulse_on);
    let cube = col()
        .size(60.0 * z, 60.0 * z)
        .center()
        .gap(4.0 * z)
        .radius(16.0 * z)
        .fill(theme::bg_card_solid())
        .border(
            if selected || has_issue { 1.8 } else { 1.2 },
            if has_issue {
                led
            } else if selected {
                theme::accent_primary_bg()
            } else {
                theme::warm_accent().alpha(0.55)
            },
        )
        .box_shadow(Shadow {
            x: 0.0,
            y: 6.0 * z,
            blur: 14.0 * z,
            spread: 0.0,
            color: if has_issue {
                led.alpha(0.55)
            } else {
                Color::BLACK.alpha(0.45)
            },
            inset: false,
        })
        .child(mono_icon(
            &icons::DATABASE,
            (18.0 * z).max(11.0),
            theme::warm_accent(),
        ))
        .child(
            text(sub)
                .font(TextStyle::new(9.0 * z, 600.0))
                .color(theme::text_muted()),
        );

    interactive(
        col()
            .center()
            .gap(6.0 * z)
            .padding(4.0 * z)
            .on_press(on_click)
            .child(cube)
            .child(zone_pill(name, z)),
    )
}

fn g6_zoom_toolbar() -> El {
    row()
        .gap(6.0)
        .center_y()
        .child(pill_icon_only(
            &icons::MINUS,
            "Zoom Out",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.send_g6_cmd(G6Command::ZoomOut),
        ))
        .child(pill_icon_only(
            &icons::PLUS,
            "Zoom In",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.send_g6_cmd(G6Command::ZoomIn),
        ))
        .child(pill_btn(
            "Fit View",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.send_g6_cmd(G6Command::FitView),
        ))
        .child(pill_icon_btn(
            &icons::FULLSCREEN,
            "เต็มจอ",
            PillVariant::Primary,
            PillSize::Sm,
            |s: &mut AppState| s.diagram_fullscreen = true,
        ))
}

fn topology_mode_switch(mode: TopologyViewMode) -> El {
    let g6_active = mode == TopologyViewMode::G6;
    segmented_group([
        pill_icon_btn(
            &icons::TOPOLOGY,
            "AntV G6",
            if g6_active {
                PillVariant::Primary
            } else {
                PillVariant::Glass
            },
            PillSize::Sm,
            |s: &mut AppState| s.topology_mode = TopologyViewMode::G6,
        ),
        pill_icon_btn(
            &icons::CUBE,
            "Classic",
            if !g6_active {
                PillVariant::Primary
            } else {
                PillVariant::Glass
            },
            PillSize::Sm,
            |s: &mut AppState| s.topology_mode = TopologyViewMode::Classic,
        ),
    ])
}

fn g6_viewport_host(s: &AppState, fullscreen: bool, k: &Kit) -> El {
    let body = if let Some(err) = &s.g6_error {
        col()
            .center()
            .gap(10.0)
            .padding(24.0)
            .child(
                text("AntV G6 WebView Unavailable")
                    .font(theme::heading_md())
                    .color(theme::text_strong()),
            )
            .child(
                text(err.clone())
                    .font(k.ty.text_small)
                    .color(theme::text_muted()),
            )
            .child(pill_icon_btn(
                &icons::CUBE,
                "Switch to Classic View",
                PillVariant::Primary,
                PillSize::Md,
                |s: &mut AppState| s.topology_mode = TopologyViewMode::Classic,
            ))
    } else {
        col()
            .center()
            .gap(6.0)
            .child(
                text("AntV G6 v5 — Interactive Cluster Topology")
                    .font(k.ty.body_medium)
                    .color(theme::text_strong()),
            )
            .child(
                text("Initializing embedded WebView canvas…")
                    .font(k.ty.text_small)
                    .color(theme::text_muted()),
            )
    };

    let mut host = col()
        .key("g6-topology-viewport")
        .w_full()
        .center()
        .clip_rounded(if fullscreen { 0.0 } else { 20.0 })
        .fill(theme::bg_canvas())
        .radial_gradient(
            [0.14, 0.08],
            [620.0, 340.0],
            [
                (0.0, theme::glow_warm()),
                (0.65, Color::TRANSPARENT),
            ],
        )
        .drawn_rect_into(&s.g6_viewport_rect)
        .child(body);

    if fullscreen {
        host = host.grow(1.0).shrink().min_h(0.0);
    } else {
        host = host
            .grow(1.0)
            .shrink()
            .min_h(420.0)
            .radius(20.0)
            .border(1.0, theme::border_glass());
    }

    host
}

fn floating_g6_fullscreen_bar(k: &Kit) -> El {
    let bar = row()
        .key("fs-g6-floating-controls")
        .padding(8.0)
        .gap(6.0)
        .center_y()
        .radius(999.0)
        .fill(theme::bg_chrome().alpha(0.95))
        .border(1.0, theme::border_medium())
        .child(
            text("AntV G6 v5 • Scroll Zoom • Drag Pan • Hover Trace")
                .font(k.ty.small_medium)
                .color(theme::text_muted()),
        )
        .child(pill_icon_only(
            &icons::MINUS,
            "Zoom Out",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.send_g6_cmd(G6Command::ZoomOut),
        ))
        .child(pill_icon_only(
            &icons::PLUS,
            "Zoom In",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.send_g6_cmd(G6Command::ZoomIn),
        ))
        .child(pill_btn(
            "Fit",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.send_g6_cmd(G6Command::FitView),
        ))
        .child(pill_btn(
            "Reset",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.send_g6_cmd(G6Command::ResetView),
        ))
        .child(pill_icon_btn(
            &icons::CUBE,
            "Classic",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.topology_mode = TopologyViewMode::Classic,
        ))
        .child(pill_icon_btn(
            &icons::CLOSE,
            "ออกจากเต็มจอ (Esc)",
            PillVariant::Danger,
            PillSize::Sm,
            |s: &mut AppState| s.diagram_fullscreen = false,
        ));

    row()
        .w_full()
        .px(20.0)
        .py(10.0)
        .fill(theme::bg_canvas())
        .child(el().grow(1.0))
        .child(bar)
}

pub fn view(s: &AppState, k: &Kit) -> El {
    let snap = &s.snapshot;
    let pct = format!("{}%", (s.diagram_zoom * 100.0).round() as i32);
    let use_g6 = s.topology_mode == TopologyViewMode::G6;

    if s.diagram_fullscreen {
        if use_g6 {
            return col()
                .key("true-fullscreen-g6")
                .w_full()
                .h_full()
                .fill(theme::bg_canvas())
                .child(g6_viewport_host(s, true, k))
                .child(floating_g6_fullscreen_bar(k));
        }
        return col()
            .key("true-fullscreen-building")
            .w_full()
            .h_full()
            .fill(theme::bg_canvas())
            .child(infra_building_canvas(s, true))
            .child(floating_zoom_bar(pct, k));
    }

    let env_desc = match (snap.env, use_g6) {
        (EnvKind::Stg, true) => {
            "STG • AntV G6 v5 — Zones layout, hover edge highlight, click node เพื่อเปิด drawer, double-click เพื่อเจาะลึก"
        }
        (EnvKind::Prod, true) => {
            "PROD • AntV G6 v5 — Zones layout, hover edge highlight, click node เพื่อเปิด drawer, double-click เพื่อเจาะลึก"
        }
        (EnvKind::Stg, false) => {
            "สภาพแวดล้อมทดสอบ (STG) — Scroll เมาส์เพื่อ Zoom ตรงจุดที่ชี้ • ลากเพื่อเลื่อน • คลิก node เพื่อเจาะลึก"
        }
        (EnvKind::Prod, false) => {
            "สภาพแวดล้อมจริง (PROD) — Scroll เมาส์เพื่อ Zoom ตรงจุดที่ชี้ • ลากเพื่อเลื่อน • คลิก node เพื่อเจาะลึก"
        }
    };

    let header = row()
        .key("topo-header")
        .w_full()
        .center_y()
        .gap(12.0)
        .child(
            col()
                .gap(3.0)
                .grow(1.0)
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(
                            text(format!(
                                "Infrastructure Topology — {} / {}",
                                snap.env.full_label(),
                                snap.namespace
                            ))
                            .font(theme::heading_lg())
                            .color(theme::text_strong()),
                        )
                        .child(soft_badge(
                            format!("{} services", snap.services.len()),
                            theme::glass_card(),
                            theme::text_secondary(),
                            Some(theme::border_subtle()),
                        )),
                )
                .child(
                    text(env_desc)
                        .font(k.ty.text_small)
                        .color(theme::text_muted()),
                ),
        )
        .child(topology_mode_switch(s.topology_mode))
        .child(if use_g6 {
            g6_zoom_toolbar()
        } else {
            zoom_toolbar(pct)
        });

    let canvas = if use_g6 {
        g6_viewport_host(s, false, k)
    } else {
        infra_building_canvas(s, false)
            .grow(1.0)
            .shrink()
            .min_h(0.0)
    };

    let main_col = col()
        .key("topo-main")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .h_full()
        .padding(18.0)
        .gap(14.0)
        .child(header)
        .child(canvas);

    if let Some(svc) = s.selected_service_info() {
        return row()
            .key("topo-split")
            .w_full()
            .h_full()
            .child(main_col)
            .child(detail_drawer(s, svc, k));
    }

    main_col.w_full()
}

fn detail_drawer(s: &AppState, svc: &ServiceInfo, k: &Kit) -> El {
    let svc_name_drill = svc.name.clone();
    let mut col_detail = col()
        .key("topo-drawer")
        .w(360.0)
        .h_full()
        .padding(18.0)
        .gap(12.0)
        .fill(theme::bg_chrome())
        .border_sides([0.0, 0.0, 0.0, 1.0], theme::border_glass())
        .motion(motion::SNAPPY)
        .child(
            row()
                .center_y()
                .gap(8.0)
                .child(el().size(9.0, 9.0).radius(5.0).fill(health_color(svc.health)))
                .child(
                    text(svc.name.clone())
                        .font(theme::heading_md())
                        .color(theme::text_strong())
                        .grow(1.0),
                )
                .child(pill_icon_only(
                    &icons::CLOSE,
                    "Close Drawer",
                    PillVariant::Glass,
                    PillSize::Sm,
                    |s: &mut AppState| s.selected_service = None,
                )),
        )
        .child(
            text(svc.summary.clone())
                .font(k.ty.text_small)
                .color(theme::text_muted()),
        )
        .child(pill_icon_btn(
            &icons::ARROW_RIGHT,
            "เจาะลึก Service นี้ (ENV / Logs / Spec)",
            PillVariant::Primary,
            PillSize::Md,
            move |s: &mut AppState| {
                s.drill_into_service(svc_name_drill.clone(), ServiceTab::Info)
            },
        ))
        .child(
            row()
                .gap(6.0)
                .child(soft_badge(
                    svc.role.label(),
                    Color::WHITE.alpha(0.06),
                    theme::text_secondary(),
                    Some(theme::border_subtle()),
                ))
                .child(soft_badge(
                    format!("Port: {}", svc.ports),
                    theme::metric_box_soft(),
                    theme::warm_accent(),
                    Some(theme::border_subtle()),
                )),
        );

    if !svc.hosts.is_empty() {
        col_detail = col_detail.child(
            col()
                .gap(4.0)
                .padding(12.0)
                .radius(14.0)
                .fill(theme::metric_box_soft())
                .border(1.0, theme::border_subtle())
                .child(
                    row()
                        .gap(6.0)
                        .center_y()
                        .child(mono_icon(&icons::GLOBE, 12.0, theme::text_muted()))
                        .child(
                            text("รับ Traffic จาก Domain:")
                                .font(k.ty.small_medium)
                                .color(theme::text_muted()),
                        ),
                )
                .child(
                    text(svc.hosts.join("\n"))
                        .font(k.ty.small_medium)
                        .color(theme::warm_accent()),
                ),
        );
    }

    if !svc.connects_to.is_empty() {
        col_detail = col_detail.child(
            col()
                .gap(4.0)
                .padding(12.0)
                .radius(14.0)
                .fill(theme::metric_box_soft())
                .border(1.0, theme::border_subtle())
                .child(
                    row()
                        .gap(6.0)
                        .center_y()
                        .child(mono_icon(&icons::LINK, 12.0, theme::text_muted()))
                        .child(
                            text("เชื่อมต่อไปยัง:")
                                .font(k.ty.small_medium)
                                .color(theme::text_muted()),
                        ),
                )
                .child(
                    text(svc.connects_to.join(", "))
                        .font(k.ty.small_medium)
                        .color(theme::text_base()),
                ),
        );
    }

    if let Some(deploy) = &svc.deploy_name {
        let deploy_clone = deploy.clone();
        col_detail = col_detail.child(
            row()
                .gap(8.0)
                .child(pill_icon_btn(
                    &icons::REFRESH,
                    "Restart Service",
                    if s.env == EnvKind::Prod {
                        PillVariant::Danger
                    } else {
                        PillVariant::Gold
                    },
                    PillSize::Sm,
                    move |s: &mut AppState| s.restart_deploy(deploy_clone.clone()),
                ))
                .child(pill_icon_btn(
                    &icons::TERMINAL,
                    "เปิดใน K9s",
                    PillVariant::Glass,
                    PillSize::Sm,
                    |s: &mut AppState| {
                        s.tab = Tab::K9sCli;
                        if !s.k9s_pty.is_running() {
                            s.start_k9s();
                        }
                    },
                )),
        );
    }

    col_detail = col_detail.child(
        row()
            .gap(6.0)
            .center_y()
            .child(mono_icon(&icons::SERVER, 13.0, theme::text_muted()))
            .child(
                text(format!("Pods ({})", svc.pods.len()))
                    .font(k.ty.text_small_medium)
                    .color(theme::text_strong()),
            ),
    );

    let mut pods_list = col().gap(8.0);
    for pod in &svc.pods {
        let svc_name_log = svc.name.clone();
        pods_list = pods_list.child(
            col()
                .padding(11.0)
                .gap(5.0)
                .radius(14.0)
                .fill(theme::metric_box_soft())
                .border(1.0, theme::border_subtle())
                .child(
                    text(pod.name.clone())
                        .font(k.ty.small_medium)
                        .color(theme::text_strong()),
                )
                .child(
                    text(pod.explanation.clone())
                        .font(k.ty.small)
                        .color(theme::text_muted()),
                )
                .child(pill_icon_btn(
                    &icons::LOGS,
                    "ดู Log",
                    PillVariant::Glass,
                    PillSize::Sm,
                    move |s: &mut AppState| {
                        s.drill_into_service(svc_name_log.clone(), ServiceTab::Logs);
                    },
                )),
        );
    }

    col_detail.child(
        pods_list
            .scroll(ScrollSpec { stick_bottom: false })
            .grow(1.0)
            .shrink()
            .min_h(0.0),
    )
}
