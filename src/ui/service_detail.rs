use crate::k8s::{EnvKind, ServiceInfo, build_service_mermaid};
use crate::state::{AppState, Page, ServiceTab, Tab};
use crate::ui::icons::{self, mono_icon};
use crate::ui::pill::{
    PillSize, PillVariant, interactive_row, pill_btn, pill_icon_btn, segmented_group, soft_badge,
};
use crate::ui::topology::render_mermaid_zoomable;
use crate::ui::{health_color, theme};
use rune::{Color, El, ScrollSpec, col, el, row, text};
use rune::tok::motion;
use rune_kit::{Kit, TextInput};

pub fn view(s: &AppState, k: &Kit) -> El {
    let Some(svc) = s.selected_service_info() else {
        return col()
            .w_full()
            .h_full()
            .center()
            .gap(12.0)
            .child(
                text(crate::tr!("ไม่พบข้อมูล Service ที่เลือก", "Selected service not found"))
                    .font(k.ty.body_medium)
                    .color(theme::text_muted()),
            )
            .child(pill_icon_btn(
                &icons::ARROW_LEFT,
                crate::tr!("กลับไปหน้า Namespace", "Back to namespace"),
                PillVariant::Chrome,
                PillSize::Md,
                |s: &mut AppState| s.page = Page::Namespace,
            ));
    };

    if s.service_tab == ServiceTab::Diagram && s.diagram_fullscreen {
        let src = build_service_mermaid(svc);
        return col()
            .w_full()
            .h_full()
            .padding(14.0)
            .fill(theme::bg_canvas())
            .child(render_mermaid_zoomable(&src, s.diagram_zoom, true, k));
    }

    let tab_btn = |ic: &'static rune::Icon, label: &'static str, target: ServiceTab| {
        let active = s.service_tab == target;
        pill_icon_btn(
            ic,
            label,
            if active {
                PillVariant::Primary
            } else {
                PillVariant::Glass
            },
            PillSize::Sm,
            move |s: &mut AppState| {
                s.service_tab = target;
                s.sync_active_tab();
            },
        )
    };

    let header = row()
        .key("svc-detail-header")
        .w_full()
        .padding(16.0)
        .gap(12.0)
        .center_y()
        .radius(20.0)
        .fill(theme::glass_card())
        .border(1.0, theme::border_glass())
        .child(pill_icon_btn(
            &icons::ARROW_LEFT,
            crate::tr!("กลับ", "Back"),
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| {
                s.page = Page::Namespace;
                s.sync_active_tab();
            },
        ))
        .child(el().size(10.0, 10.0).radius(5.0).fill(health_color(svc.health)))
        .child(
            col()
                .gap(3.0)
                .grow(1.0)
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        // The name opens a picker of the namespace's other services.
                        .child(interactive_row(
                            row()
                                .key("svc-switcher")
                                .role("button")
                                .label(crate::tr!("เปลี่ยน Service", "Switch service"))
                                .hit()
                                .gap(6.0)
                                .center_y()
                                .px(8.0)
                                .py(2.0)
                                .radius(10.0)
                                .fill(if s.svc_picker_open { theme::ink(0.08) } else { Color::TRANSPARENT })
                                .on_press(|s: &mut AppState| {
                                    if s.svc_picker_open {
                                        s.close_svc_picker()
                                    } else {
                                        s.open_svc_picker()
                                    }
                                })
                                .child(
                                    text(svc.name.clone())
                                        .font(theme::heading_lg())
                                        .color(theme::text_strong()),
                                )
                                .child(mono_icon(&icons::CHEVRON_DOWN, 16.0, theme::text_muted())),
                        ))
                        .child(soft_badge(
                            svc.role.label(),
                            theme::ink(0.06),
                            theme::text_secondary(),
                            Some(theme::border_subtle()),
                        ))
                        .child(soft_badge(
                            format!("{}/{} Pods", svc.ready_replicas, svc.desired_replicas),
                            theme::metric_box_soft(),
                            health_color(svc.health),
                            Some(theme::border_subtle()),
                        )),
                )
                .child(
                    text(svc.summary.clone())
                        .font(k.ty.text_small)
                        .color(theme::text_muted()),
                ),
        )
        .child(
            row()
                .gap(8.0)
                .child(pill_icon_btn(
                    &icons::TERMINAL,
                    crate::tr!("เปิดใน K9s", "Open in K9s"),
                    PillVariant::Glass,
                    PillSize::Sm,
                    |s: &mut AppState| {
                        s.page = Page::Namespace;
                        s.tab = Tab::K9sCli;
                        if !s.k9s_pty.is_running() {
                            s.start_k9s();
                        }
                    },
                ))
                .children(svc.deploy_name.clone().map(|deploy| {
                    pill_icon_btn(
                        &icons::REFRESH,
                        "Restart",
                        if s.env == EnvKind::Prod {
                            PillVariant::Danger
                        } else {
                            PillVariant::Primary
                        },
                        PillSize::Sm,
                        move |s: &mut AppState| s.restart_deploy(deploy.clone()),
                    )
                })),
        );

    let sub_tabs = row().child(segmented_group([
        tab_btn(&icons::INFO_CIRCLE, "Overview & Pods", ServiceTab::Info),
        tab_btn(&icons::LOGS, "Logs", ServiceTab::Logs),
        tab_btn(&icons::SLIDERS, "ENV & Config", ServiceTab::Env),
        tab_btn(&icons::TOPOLOGY, "Service Diagram", ServiceTab::Diagram),
    ]));

    let body = match s.service_tab {
        ServiceTab::Info => info_view(svc, k),
        ServiceTab::Logs => logs_view(s, svc, k),
        ServiceTab::Env => env_view(s, svc, k),
        ServiceTab::Diagram => diagram_view(s, svc, k),
    };

    col()
        .key("svc-detail-page")
        .w_full()
        .h_full()
        .padding(20.0)
        .gap(14.0)
        .child(header)
        .child(sub_tabs)
        .child(body.grow(1.0).shrink().min_h(0.0))
}

fn info_view(svc: &ServiceInfo, k: &Kit) -> El {
    let mut pods_col = col().gap(8.0);
    if svc.pods.is_empty() {
        pods_col = pods_col.child(
            text(crate::tr!("ไม่มี Pod ที่กำลังรันอยู่", "No running pods"))
                .font(k.ty.text_small)
                .color(theme::text_muted()),
        );
    } else {
        for p in &svc.pods {
            let pod_name = p.name.clone();
            let mut spec_row = row()
                .gap(6.0)
                .center_y()
                .child(
                    row()
                        .h(20.0)
                        .px(8.0)
                        .gap(5.0)
                        .center_y()
                        .radius(999.0)
                        .fill(theme::ink(0.06))
                        .border(1.0, theme::border_subtle())
                        .child(mono_icon(&icons::SERVER, 10.5, theme::text_muted()))
                        .child(
                            text(p.node_name.clone())
                                .font(theme::badge_xs())
                                .color(theme::text_secondary()),
                        ),
                )
                .child(soft_badge(
                    p.node_spec.clone(),
                    theme::metric_box_soft(),
                    theme::text_muted(),
                    Some(theme::border_subtle()),
                ))
                .child(soft_badge(
                    p.cpu_spec.clone(),
                    theme::metric_box_soft(),
                    theme::text_muted(),
                    Some(theme::border_subtle()),
                ))
                .child(soft_badge(
                    p.mem_spec.clone(),
                    theme::metric_box_soft(),
                    theme::text_muted(),
                    Some(theme::border_subtle()),
                ));
            if !p.live_usage.is_empty() {
                spec_row = spec_row.child(soft_badge(
                    p.live_usage.clone(),
                    theme::prod_bg(),
                    theme::prod_fg(),
                    None,
                ));
            }
            pods_col = pods_col.child(
                row()
                    .padding(14.0)
                    .gap(10.0)
                    .center_y()
                    .radius(16.0)
                    .fill(theme::metric_box_soft())
                    .border(1.0, theme::border_subtle())
                    .motion(motion::SNAPPY)
                    .child(
                        el().size(8.0, 8.0).radius(4.0).fill(health_color(if p.ready {
                            crate::k8s::Health::Healthy
                        } else {
                            crate::k8s::Health::Warning
                        })),
                    )
                    .child(
                        col()
                            .gap(4.0)
                            .grow(1.0)
                            .child(
                                text(p.name.clone())
                                    .font(k.ty.body_medium)
                                    .color(theme::text_strong()),
                            )
                            .child(
                                text(p.explanation.clone())
                                    .font(k.ty.text_small)
                                    .color(theme::text_muted()),
                            )
                            .child(spec_row),
                    )
                    .child(soft_badge(
                        format!("Restarts: {}", p.restarts),
                        theme::ink(0.05),
                        theme::text_muted(),
                        Some(theme::border_subtle()),
                    ))
                    .child(pill_icon_btn(
                        &icons::LOGS,
                        crate::tr!("ดู Log", "View logs"),
                        PillVariant::Glass,
                        PillSize::Sm,
                        move |s: &mut AppState| {
                            s.service_tab = ServiceTab::Logs;
                            s.fetch_logs(pod_name.clone());
                        },
                    )),
            );
        }
    }

    col()
        .gap(14.0)
        .child(
            row()
                .gap(14.0)
                .child(
                    col()
                        .grow(1.0)
                        .padding(16.0)
                        .gap(8.0)
                        .radius(20.0)
                        .fill(theme::glass_card())
                        .border(1.0, theme::border_glass())
                        .child(
                            row()
                                .gap(8.0)
                                .center_y()
                                .child(mono_icon(&icons::GLOBE, 14.5, theme::text_secondary()))
                                .child(
                                    text("Domain / Ingress")
                                        .font(k.ty.body_medium)
                                        .color(theme::text_strong()),
                                ),
                        )
                        .child(
                            text(if svc.hosts.is_empty() {
                                crate::tr!("Internal Service (ไม่เปิดออกนอก Cluster)", "Internal service (not exposed outside the cluster)").into()
                            } else {
                                svc.hosts.join("\n")
                            })
                            .font(k.ty.text_small)
                            .color(theme::warm_accent()),
                        ),
                )
                .child(
                    col()
                        .grow(1.0)
                        .padding(16.0)
                        .gap(8.0)
                        .radius(20.0)
                        .fill(theme::glass_card())
                        .border(1.0, theme::border_glass())
                        .child(
                            row()
                                .gap(8.0)
                                .center_y()
                                .child(mono_icon(&icons::LINK, 14.5, theme::text_secondary()))
                                .child(
                                    text(crate::tr!("เชื่อมต่อไปยัง", "Connects to"))
                                        .font(k.ty.body_medium)
                                        .color(theme::text_strong()),
                                ),
                        )
                        .child(
                            text(if svc.connects_to.is_empty() {
                                crate::tr!("ไม่มีปลายทางภายในที่ตรวจพบ", "No internal targets detected").into()
                            } else {
                                svc.connects_to.join(", ")
                            })
                            .font(k.ty.text_small)
                            .color(theme::text_base()),
                        ),
                ),
        )
        .child(
            col()
                .padding(16.0)
                .gap(10.0)
                .radius(20.0)
                .fill(theme::glass_card())
                .border(1.0, theme::border_glass())
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(mono_icon(&icons::SERVICES, 14.5, theme::text_secondary()))
                        .child(
                            text(format!("Pods ({})", svc.pods.len()))
                                .font(k.ty.body_medium)
                                .color(theme::text_strong()),
                        ),
                )
                .child(pods_col)
                .scroll(ScrollSpec { stick_bottom: false })
                .grow(1.0)
                .shrink()
                .min_h(0.0),
        )
}

fn logs_view(s: &AppState, svc: &ServiceInfo, k: &Kit) -> El {
    let mut pod_bar = row().gap(6.0).center_y();
    for p in &svc.pods {
        let p_name = p.name.clone();
        let active = s.logs_pod.as_deref() == Some(p.name.as_str());
        pod_bar = pod_bar.child(pill_icon_btn(
            &icons::SERVER,
            p.name.clone(),
            if active {
                PillVariant::Primary
            } else {
                PillVariant::Glass
            },
            PillSize::Sm,
            move |s: &mut AppState| s.fetch_logs(p_name.clone()),
        ));
    }

    let since_btn = |label: &'static str, val: Option<&'static str>| {
        let active = s.log_since.as_deref() == val;
        pill_btn(
            label,
            if active {
                PillVariant::Primary
            } else {
                PillVariant::Glass
            },
            PillSize::Sm,
            move |s: &mut AppState| {
                s.log_since = val.map(String::from);
                if let Some(pod) = s.logs_pod.clone() {
                    s.fetch_logs(pod);
                }
            },
        )
    };

    let raw_logs = s
        .logs_text
        .clone()
        .unwrap_or_else(|| crate::tr!("(เลือก Pod เพื่อดู Log)", "(Choose a pod to view logs)").into());
    let query = s.log_search.text.trim().to_ascii_lowercase();

    let mut lines: Vec<String> = raw_logs.lines().map(String::from).collect();
    if !query.is_empty() {
        lines.retain(|l| strip_ansi(l).to_ascii_lowercase().contains(&query));
    }
    if s.log_newest_first {
        lines.reverse();
    }

    let clean_all = lines
        .iter()
        .map(|l| strip_ansi(l))
        .collect::<Vec<_>>()
        .join("\n");

    let controls_bar = row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(pill_icon_btn(
            if s.log_newest_first {
                &icons::ARROW_UP
            } else {
                &icons::ARROW_DOWN
            },
            if s.log_newest_first {
                crate::tr!("ใหม่สุดอยู่บน", "Newest on top")
            } else {
                crate::tr!("ใหม่สุดอยู่ล่าง", "Newest at bottom")
            },
            PillVariant::Gold,
            PillSize::Sm,
            |s: &mut AppState| s.log_newest_first = !s.log_newest_first,
        ))
        .child(segmented_group([
            since_btn(crate::tr!("ทั้งหมด", "All"), None),
            since_btn(crate::tr!("5 นาที", "5 min"), Some("5m")),
            since_btn(crate::tr!("15 นาที", "15 min"), Some("15m")),
            since_btn(crate::tr!("1 ชม.", "1 hr"), Some("1h")),
            since_btn(crate::tr!("6 ชม.", "6 hr"), Some("6h")),
        ]))
        .child(pill_icon_btn(
            &icons::CLOCK,
            if s.log_timestamps {
                crate::tr!("เวลา: เปิด", "Time: on")
            } else {
                crate::tr!("เวลา: ปิด", "Time: off")
            },
            if s.log_timestamps {
                PillVariant::Chrome
            } else {
                PillVariant::Glass
            },
            PillSize::Sm,
            |s: &mut AppState| {
                s.log_timestamps = !s.log_timestamps;
                if let Some(pod) = s.logs_pod.clone() {
                    s.fetch_logs(pod);
                }
            },
        ))
        .child(
            TextInput::new(&s.log_search)
                .width(Some(190.0))
                .placeholder(crate::tr!("ค้นหาใน Log...", "Search logs..."))
                .on_edit(|s: &mut AppState| &mut s.log_search)
                .build(k),
        )
        .child(el().grow(1.0))
        .child(pill_icon_btn(
            &icons::REFRESH,
            "Refresh",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| {
                if let Some(pod) = s.logs_pod.clone() {
                    s.fetch_logs(pod);
                }
            },
        ))
        .child(pill_icon_btn(
            &icons::COPY,
            crate::tr!("Copy ทั้งหมด", "Copy all"),
            PillVariant::Primary,
            PillSize::Sm,
            move |s: &mut AppState| {
                s.copy_to_clipboard(clean_all.clone(), crate::tr!("Logs ทั้งหมด", "All logs"));
            },
        ));

    let mut log_rows = col().gap(2.0);
    for (idx, line) in lines.iter().take(250).enumerate() {
        let clean_line = strip_ansi(line);
        let spans = parse_ansi_spans(line);
        let mut line_row = row().gap(0.0).center_y();
        for (col_fg, seg) in spans {
            line_row = line_row.child(text(seg).font(k.ty.mono).color(col_fg));
        }
        let line_widget = interactive_row(
            row()
                .key(format!("log-line-{idx}"))
                .w_full()
                .px(6.0)
                .py(2.0)
                .radius(6.0)
                .on_press(move |s: &mut AppState| {
                    s.copy_to_clipboard(clean_line.clone(), crate::tr!("บรรทัด Log", "Log line"));
                })
                .child(line_row),
        );
        log_rows = log_rows.child(line_widget);
    }

    col()
        .gap(10.0)
        .child(pod_bar)
        .child(controls_bar)
        .child(
            row()
                .gap(8.0)
                .center_y()
                .child(soft_badge(
                    if s.log_newest_first {
                        crate::tr!("เรียง: ใหม่ล่าสุดอยู่ด้านบน", "Order: newest on top")
                    } else {
                        crate::tr!("เรียง: ใหม่ล่าสุดอยู่ด้านล่าง", "Order: newest at bottom")
                    },
                    theme::prod_bg(),
                    theme::prod_fg(),
                    None,
                ))
                .child(
                    text(crate::tr!("คลิกที่บรรทัดใดก็ได้เพื่อ Copy บรรทัดนั้น", "Click any line to copy it"))
                        .font(k.ty.small)
                        .color(theme::text_muted()),
                ),
        )
        .child(
            col()
                .padding(14.0)
                .radius(20.0)
                .fill(theme::bg_deep())
                .border(1.0, theme::border_glass())
                .child(log_rows)
                .scroll(ScrollSpec {
                    stick_bottom: !s.log_newest_first,
                })
                .grow(1.0)
                .shrink()
                .min_h(0.0),
        )
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn parse_ansi_spans(input: &str) -> Vec<(Color, String)> {
    let default_fg = theme::text_base();
    let mut current_color = default_fg;
    let mut spans = Vec::new();
    let mut buf = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            if !buf.is_empty() {
                spans.push((current_color, std::mem::take(&mut buf)));
            }
            chars.next(); // consume '['
            let mut code_str = String::new();
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    if c == 'm' {
                        current_color = ansi_code_to_color(&code_str, current_color, default_fg);
                    }
                    break;
                }
                code_str.push(c);
            }
        } else {
            buf.push(ch);
        }
    }
    if !buf.is_empty() {
        spans.push((current_color, buf));
    }
    if spans.is_empty() {
        spans.push((default_fg, String::new()));
    }
    spans
}

fn ansi_code_to_color(codes: &str, cur: Color, default_fg: Color) -> Color {
    let mut col = cur;
    for part in codes.split(';') {
        match part.trim() {
            "0" | "" => col = default_fg,
            "30" | "90" => col = theme::text_muted(),
            "31" | "91" => col = theme::err_dot(),
            "32" | "92" => col = theme::ok_fg(),
            "33" | "93" => col = theme::warn_fg(),
            "34" | "94" => col = theme::hex("#93c5fd"),
            "35" | "95" => col = theme::hex("#d8b4fe"),
            "36" | "96" => col = theme::hex("#67e8f9"),
            "37" | "97" => col = theme::text_strong(),
            _ => {}
        }
    }
    col
}

fn env_view(s: &AppState, svc: &ServiceInfo, k: &Kit) -> El {
    let query = s.env_search.text.trim().to_ascii_lowercase();
    let mut rows = col().gap(8.0);

    if s.adding_env {
        rows = rows.child(
            col()
                .padding(14.0)
                .gap(10.0)
                .radius(16.0)
                .fill(theme::metric_box_bg())
                .border(1.0, theme::border_strong())
                .child(
                    text(crate::tr!("เพิ่ม Environment Variable ใหม่", "Add an environment variable"))
                        .font(k.ty.body_medium)
                        .color(theme::text_strong()),
                )
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(
                            TextInput::new(&s.new_env_key)
                                .width(Some(240.0))
                                .placeholder(crate::tr!("KEY (เช่น REDIS_HOST)", "KEY (e.g. REDIS_HOST)"))
                                .on_edit(|s: &mut AppState| &mut s.new_env_key)
                                .build(k),
                        )
                        .child(
                            TextInput::new(&s.new_env_val)
                                .width(None)
                                .placeholder("VALUE")
                                .on_edit(|s: &mut AppState| &mut s.new_env_val)
                                .on_submit(|s: &mut AppState, _| s.save_new_env(false))
                                .build(k),
                        ),
                )
                .child(
                    row()
                        .gap(8.0)
                        .child(pill_icon_btn(
                            &icons::CHECK,
                            crate::tr!("บันทึก", "Save"),
                            PillVariant::Primary,
                            PillSize::Sm,
                            |s: &mut AppState| s.save_new_env(false),
                        ))
                        .child(pill_icon_btn(
                            &icons::REFRESH,
                            crate::tr!("บันทึก + Restart Pod", "Save + restart pods"),
                            PillVariant::Gold,
                            PillSize::Sm,
                            |s: &mut AppState| s.save_new_env(true),
                        ))
                        .child(pill_btn(
                            crate::tr!("ยกเลิก", "Cancel"),
                            PillVariant::Glass,
                            PillSize::Sm,
                            |s: &mut AppState| s.cancel_edit_env(),
                        )),
                ),
        );
    }

    let mut matched_count = 0usize;
    if s.env_vars.is_empty() {
        rows = rows.child(
            text(crate::tr!("กำลังโหลดหรือไม่มี ENV variables ใน Deployment", "Loading, or this deployment has no ENV variables"))
                .font(k.ty.text_small)
                .color(theme::text_muted()),
        );
    } else {
        for ev in &s.env_vars {
            let match_key = ev.name.to_ascii_lowercase().contains(&query);
            let match_src = ev.source.to_ascii_lowercase().contains(&query);
            let match_val = ev.value.to_ascii_lowercase().contains(&query);
            if !query.is_empty() && !match_key && !match_src && !match_val {
                continue;
            }
            matched_count += 1;
            let is_editing = s.editing_env_key.as_deref() == Some(ev.name.as_str());
            if is_editing {
                rows = rows.child(
                    col()
                        .padding(14.0)
                        .gap(10.0)
                        .radius(16.0)
                        .fill(theme::metric_box_bg())
                        .border(1.0, theme::warn_border())
                        .child(
                            row()
                                .gap(8.0)
                                .center_y()
                                .child(
                                    text(ev.name.clone())
                                        .font(k.ty.mono)
                                        .color(theme::warm_accent()),
                                )
                                .child(soft_badge(
                                    ev.source.clone(),
                                    theme::ink(0.06),
                                    theme::text_secondary(),
                                    Some(theme::border_subtle()),
                                )),
                        )
                        .child(
                            TextInput::new(&s.editing_env_input)
                                .large()
                                .width(None)
                                .placeholder(crate::tr!("กรอกค่า ENV ใหม่...", "New value..."))
                                .on_edit(|s: &mut AppState| &mut s.editing_env_input)
                                .on_submit(|s: &mut AppState, _| s.save_editing_env(false))
                                .build(k),
                        )
                        .child(
                            row()
                                .gap(8.0)
                                .child(pill_icon_btn(
                                    &icons::CHECK,
                                    crate::tr!("บันทึก", "Save"),
                                    PillVariant::Primary,
                                    PillSize::Sm,
                                    |s: &mut AppState| s.save_editing_env(false),
                                ))
                                .child(pill_icon_btn(
                                    &icons::REFRESH,
                                    crate::tr!("บันทึก + Restart Pod", "Save + restart pods"),
                                    PillVariant::Danger,
                                    PillSize::Sm,
                                    |s: &mut AppState| s.save_editing_env(true),
                                ))
                                .child(pill_btn(
                                    crate::tr!("ยกเลิก", "Cancel"),
                                    PillVariant::Glass,
                                    PillSize::Sm,
                                    |s: &mut AppState| s.cancel_edit_env(),
                                )),
                        ),
                );
            } else {
                let display_val = if ev.is_secret && !s.show_secrets {
                    crate::tr!("•••••••••••• (ซ่อนอยู่)", "•••••••••••• (hidden)").to_string()
                } else {
                    ev.value.clone()
                };
                let key_edit = ev.name.clone();
                let val_edit = ev.value.clone();
                let matched_hidden_val =
                    !query.is_empty() && match_val && !match_key && ev.is_secret && !s.show_secrets;
                rows = rows.child(
                    row()
                        .padding(11.0)
                        .gap(12.0)
                        .center_y()
                        .radius(14.0)
                        .fill(theme::metric_box_soft())
                        .border(1.0, theme::border_subtle())
                        .child(
                            text(ev.name.clone())
                                .font(k.ty.mono)
                                .color(theme::warm_accent())
                                .w(240.0),
                        )
                        .child(
                            text(display_val)
                                .font(k.ty.mono)
                                .color(theme::text_base())
                                .grow(1.0)
                                .shrink()
                                .min_w(0.0),
                        )
                        .children(if matched_hidden_val {
                            Some(soft_badge(
                                crate::tr!("ตรงกับ Value", "Matches value"),
                                theme::warn_bg(),
                                theme::warn_fg(),
                                None,
                            ))
                        } else {
                            None
                        })
                        .child(soft_badge(
                            ev.source.clone(),
                            theme::ink(0.06),
                            theme::text_muted(),
                            Some(theme::border_subtle()),
                        ))
                        .children(if ev.is_secret {
                            Some(soft_badge(
                                "SECRET",
                                theme::prod_bg(),
                                theme::prod_fg(),
                                None,
                            ))
                        } else {
                            None
                        })
                        .child(pill_icon_btn(
                            &icons::EDIT,
                            crate::tr!("แก้ไข", "Edit"),
                            PillVariant::Glass,
                            PillSize::Sm,
                            move |s: &mut AppState| {
                                s.start_edit_env(key_edit.clone(), val_edit.clone());
                            },
                        )),
                );
            }
        }
    }

    col()
        .gap(10.0)
        .child(
            row()
                .center_y()
                .gap(8.0)
                .child(
                    text(format!(
                        "Environment Variables ({}) — {}/{}",
                        svc.name,
                        matched_count,
                        s.env_vars.len()
                    ))
                    .font(k.ty.body_medium)
                    .color(theme::text_strong())
                    .grow(1.0),
                )
                .child(
                    TextInput::new(&s.env_search)
                        .width(Some(250.0))
                        .placeholder(crate::tr!("ค้นหา Key, Value หรือ Source...", "Search key, value or source..."))
                        .on_edit(|s: &mut AppState| &mut s.env_search)
                        .build(k),
                )
                .child(pill_icon_btn(
                    &icons::DOWNLOAD,
                    "Export CSV",
                    PillVariant::Glass,
                    PillSize::Sm,
                    |s: &mut AppState| s.export_env_csv(),
                ))
                .child(pill_icon_btn(
                    &icons::PLUS,
                    crate::tr!("เพิ่ม ENV ใหม่", "Add ENV"),
                    PillVariant::Primary,
                    PillSize::Sm,
                    |s: &mut AppState| {
                        s.adding_env = !s.adding_env;
                        s.editing_env_key = None;
                    },
                ))
                .child(pill_icon_btn(
                    if s.show_secrets {
                        &icons::EYE_OFF
                    } else {
                        &icons::EYE
                    },
                    if s.show_secrets {
                        crate::tr!("ซ่อนค่า Secret", "Hide secret values")
                    } else {
                        crate::tr!("แสดงค่า Secret", "Show secret values")
                    },
                    if s.show_secrets {
                        PillVariant::Gold
                    } else {
                        PillVariant::Glass
                    },
                    PillSize::Sm,
                    |s: &mut AppState| s.show_secrets = !s.show_secrets,
                )),
        )
        .child(
            col()
                .padding(14.0)
                .radius(20.0)
                .fill(theme::glass_card())
                .border(1.0, theme::border_glass())
                .child(rows)
                .scroll(ScrollSpec { stick_bottom: false })
                .grow(1.0)
                .shrink()
                .min_h(0.0),
        )
}

fn diagram_view(s: &AppState, svc: &ServiceInfo, k: &Kit) -> El {
    let src = build_service_mermaid(svc);
    render_mermaid_zoomable(&src, s.diagram_zoom, s.diagram_fullscreen, k)
}


/// The "switch service" modal: every service of the current namespace, searchable.
pub fn service_picker(s: &AppState, k: &Kit) -> El {
    let matches = s.svc_picker_matches();
    let total = s.snapshot.services.len();
    let current = s.selected_service.clone().unwrap_or_default();

    let mut list = col().w_full().gap(4.0);
    if matches.is_empty() {
        list = list.child(
            col().w_full().padding(24.0).center().child(
                text(crate::tr!("ไม่พบ Service ที่ตรงกับคำค้น", "No services match"))
                    .font(k.ty.small)
                    .color(theme::text_muted()),
            ),
        );
    }
    for svc in &matches {
        let name = svc.name.clone();
        let is_current = svc.name == current;
        list = list.child(interactive_row(
            row()
                .key(format!("svc-pick-{}", svc.name))
                .role("button")
                .label(svc.name.clone())
                .hit()
                .w_full()
                .gap(10.0)
                .center_y()
                .px(12.0)
                .py(9.0)
                .radius(12.0)
                .fill(if is_current { theme::ink(0.07) } else { Color::TRANSPARENT })
                .on_press(move |s: &mut AppState| s.switch_service(&name))
                .child(el().size(8.0, 8.0).radius(4.0).fill(health_color(svc.health)))
                .child(
                    text(svc.name.clone())
                        .font(k.ty.body_medium)
                        .color(theme::text_strong())
                        .grow(1.0)
                        .shrink()
                        .min_w(0.0),
                )
                .child(soft_badge(
                    svc.role.label(),
                    theme::ink(0.05),
                    theme::text_secondary(),
                    Some(theme::border_subtle()),
                ))
                .child(
                    text(format!("{}/{}", svc.ready_replicas, svc.desired_replicas))
                        .font(k.ty.small)
                        .color(health_color(svc.health)),
                )
                .children(if is_current {
                    Some(mono_icon(&icons::CHECK, 14.0, theme::text_strong()))
                } else {
                    None
                }),
        ));
    }

    let panel = col()
        .key("svc-picker-panel")
        .hit()
        .w(520.0)
        .padding(18.0)
        .gap(12.0)
        .radius(22.0)
        .fill(theme::glass_card_solid())
        .border(1.0, theme::border_glass())
        .shadow(24.0, 60.0, Color::BLACK.alpha(0.45))
        .child(
            row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(
                    col()
                        .gap(2.0)
                        .grow(1.0)
                        .child(
                            text(crate::tr!("เลือก Service", "Choose a service"))
                                .font(theme::heading_md())
                                .color(theme::text_strong()),
                        )
                        .child(
                            text(crate::tf!(
                                "{} • {} services",
                                "{} • {} services",
                                s.snapshot.namespace,
                                total
                            ))
                            .font(k.ty.small)
                            .color(theme::text_muted()),
                        ),
                )
                .child(crate::ui::pill::pill_icon_only(
                    &icons::CLOSE,
                    crate::tr!("ปิด", "Close"),
                    PillVariant::Glass,
                    PillSize::Sm,
                    |s: &mut AppState| s.close_svc_picker(),
                )),
        )
        .child(
            TextInput::new(&s.svc_picker_search)
                .width(Some(484.0))
                .placeholder(crate::tr!("ค้นหาชื่อ service, ประเภท หรือโดเมน…", "Search name, type or domain…"))
                .on_edit(|s: &mut AppState| &mut s.svc_picker_search)
                .on_submit(|s: &mut AppState, _| {
                    // Enter opens the first match.
                    let first = s.svc_picker_matches().first().map(|x| x.name.clone());
                    if let Some(n) = first {
                        s.switch_service(&n);
                    }
                })
                .build(k),
        )
        .child(if matches.len() > 7 {
            list.scroll(ScrollSpec { stick_bottom: false }).w_full().h(380.0)
        } else {
            list
        });

    // Drops down from the service name; presses outside reach the scrim under it.
    rune_kit::popover::anchored(k, panel, "svc-switcher", rune_kit::Placement::BottomStart, 8.0)
}
