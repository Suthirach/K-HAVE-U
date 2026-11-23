use crate::k8s::{EnvKind, ServiceInfo, build_service_mermaid};
use crate::state::{AppState, Page, ServiceTab, Tab};
use crate::ui::health_color;
use crate::ui::pill::{PillSize, PillVariant, interactive, pill_btn, segmented_group};
use crate::ui::topology::render_mermaid_zoomable;
use rune::{El, ScrollSpec, col, el, row, text};
use rune::tok::motion;
use rune_kit::{Badge, BadgeVariant, Kit, TextInput};

pub fn view(s: &AppState, k: &Kit) -> El {
    let c = &k.c;
    let Some(svc) = s.selected_service_info() else {
        return col()
            .w_full()
            .h_full()
            .center()
            .gap(12.0)
            .child(text("ไม่พบข้อมูล Service ที่เลือก").font(k.ty.body_medium).color(c.text_muted))
            .child(pill_btn("← กลับไปหน้า Namespace", PillVariant::Chrome, PillSize::Md, |s: &mut AppState| s.page = Page::Namespace));
    };

    if s.service_tab == ServiceTab::Diagram && s.diagram_fullscreen {
        let src = build_service_mermaid(svc);
        return col()
            .w_full()
            .h_full()
            .padding(14.0)
            .child(render_mermaid_zoomable(&src, s.diagram_zoom, true, k));
    }

    let tab_btn = |label: &'static str, target: ServiceTab| {
        let active = s.service_tab == target;
        pill_btn(
            label,
            if active { PillVariant::Chrome } else { PillVariant::Glass },
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
        .radius(k.radius.lg)
        .fill(c.bg_base)
        .border(1.0, c.border_weak)
        .child(pill_btn("← กลับ", PillVariant::Glass, PillSize::Sm, |s: &mut AppState| {
            s.page = Page::Namespace;
            s.sync_active_tab();
        }))
        .child(el().size(12.0, 12.0).radius(6.0).fill(health_color(svc.health)))
        .child(
            col()
                .gap(2.0)
                .grow(1.0)
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(text(svc.name.clone()).font(rune::TextStyle::new(18.0, 600.0)).color(c.text_strong))
                        .child(Badge::new(svc.role.label()).variant(BadgeVariant::Subtle).build(k))
                        .child(Badge::new(format!("{}/{} Pods", svc.ready_replicas, svc.desired_replicas)).variant(BadgeVariant::Neutral).build(k)),
                )
                .child(text(svc.summary.clone()).font(k.ty.text_small).color(c.text_muted)),
        )
        .child(
            row()
                .gap(8.0)
                .child(pill_btn("⌨ เปิดใน K9s", PillVariant::Chrome, PillSize::Sm, |s: &mut AppState| {
                    s.page = Page::Namespace;
                    s.tab = Tab::K9sCli;
                    if !s.k9s_pty.is_running() {
                        s.start_k9s();
                    }
                }))
                .children(svc.deploy_name.clone().map(|deploy| {
                    pill_btn(
                        "↻ Restart",
                        if s.env == EnvKind::Prod { PillVariant::Danger } else { PillVariant::Primary },
                        PillSize::Sm,
                        move |s: &mut AppState| s.restart_deploy(deploy.clone()),
                    )
                })),
        );

    let sub_tabs = segmented_group([
        tab_btn("📋 Overview & Pods", ServiceTab::Info),
        tab_btn("📄 Logs", ServiceTab::Logs),
        tab_btn("🔐 ENV & Config", ServiceTab::Env),
        tab_btn("🗺 Service Diagram", ServiceTab::Diagram),
    ]);

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
    let c = &k.c;
    let mut pods_col = col().gap(8.0);
    if svc.pods.is_empty() {
        pods_col = pods_col.child(text("ไม่มี Pod ที่กำลังรันอยู่").font(k.ty.text_small).color(c.text_muted));
    } else {
        for p in &svc.pods {
            let pod_name = p.name.clone();
            let mut spec_row = row()
                .gap(6.0)
                .center_y()
                .child(Badge::new(format!("🖥 {}", p.node_name)).variant(BadgeVariant::Neutral).build(k))
                .child(Badge::new(p.node_spec.clone()).variant(BadgeVariant::Subtle).build(k))
                .child(Badge::new(p.cpu_spec.clone()).variant(BadgeVariant::Subtle).build(k))
                .child(Badge::new(p.mem_spec.clone()).variant(BadgeVariant::Subtle).build(k));
            if !p.live_usage.is_empty() {
                spec_row = spec_row.child(Badge::new(p.live_usage.clone()).variant(BadgeVariant::Accent).build(k));
            }
            pods_col = pods_col.child(
                row()
                    .padding(12.0)
                    .gap(10.0)
                    .center_y()
                    .radius(k.radius.md)
                    .fill(c.layer_02)
                    .border(1.0, c.border_weak)
                    .motion(motion::SNAPPY)
                    .child(el().size(8.0, 8.0).radius(4.0).fill(health_color(if p.ready { crate::k8s::Health::Healthy } else { crate::k8s::Health::Warning })))
                    .child(
                        col()
                            .gap(4.0)
                            .grow(1.0)
                            .child(text(p.name.clone()).font(k.ty.body_medium).color(c.text_strong))
                            .child(text(p.explanation.clone()).font(k.ty.text_small).color(c.text_muted))
                            .child(spec_row),
                    )
                    .child(Badge::new(format!("Restarts: {}", p.restarts)).variant(BadgeVariant::Subtle).build(k))
                    .child(pill_btn("📄 ดู Log", PillVariant::Chrome, PillSize::Sm, move |s: &mut AppState| {
                        s.service_tab = ServiceTab::Logs;
                        s.fetch_logs(pod_name.clone());
                    })),
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
                        .gap(6.0)
                        .radius(k.radius.lg)
                        .fill(c.bg_base)
                        .border(1.0, c.border_weak)
                        .child(text("🌐 Domain / Ingress").font(k.ty.body_medium).color(c.text_strong))
                        .child(
                            text(if svc.hosts.is_empty() {
                                "Internal Service (ไม่เปิดออกนอก Cluster)".into()
                            } else {
                                svc.hosts.join("\n")
                            })
                            .font(k.ty.text_small)
                            .color(c.text_accent),
                        ),
                )
                .child(
                    col()
                        .grow(1.0)
                        .padding(16.0)
                        .gap(6.0)
                        .radius(k.radius.lg)
                        .fill(c.bg_base)
                        .border(1.0, c.border_weak)
                        .child(text("🔗 เชื่อมต่อไปยัง").font(k.ty.body_medium).color(c.text_strong))
                        .child(
                            text(if svc.connects_to.is_empty() {
                                "ไม่มีปลายทางภายในที่ตรวจพบ".into()
                            } else {
                                svc.connects_to.join(", ")
                            })
                            .font(k.ty.text_small)
                            .color(c.text_base),
                        ),
                ),
        )
        .child(
            col()
                .padding(16.0)
                .gap(10.0)
                .radius(k.radius.lg)
                .fill(c.bg_base)
                .border(1.0, c.border_weak)
                .child(text(format!("📦 Pods ({})", svc.pods.len())).font(k.ty.body_medium).color(c.text_strong))
                .child(pods_col)
                .scroll(ScrollSpec { stick_bottom: false })
                .grow(1.0)
                .shrink()
                .min_h(0.0),
        )
}

fn logs_view(s: &AppState, svc: &ServiceInfo, k: &Kit) -> El {
    let c = &k.c;
    let mut pod_bar = row().gap(6.0).center_y();
    for p in &svc.pods {
        let p_name = p.name.clone();
        let active = s.logs_pod.as_deref() == Some(p.name.as_str());
        pod_bar = pod_bar.child(pill_btn(
            p.name.clone(),
            if active { PillVariant::Primary } else { PillVariant::Chrome },
            PillSize::Sm,
            move |s: &mut AppState| s.fetch_logs(p_name.clone()),
        ));
    }

    let since_btn = |label: &'static str, val: Option<&'static str>| {
        let active = s.log_since.as_deref() == val;
        pill_btn(
            label,
            if active { PillVariant::Primary } else { PillVariant::Glass },
            PillSize::Sm,
            move |s: &mut AppState| {
                s.log_since = val.map(String::from);
                if let Some(pod) = s.logs_pod.clone() {
                    s.fetch_logs(pod);
                }
            },
        )
    };

    let raw_logs = s.logs_text.clone().unwrap_or_else(|| "(เลือก Pod เพื่อดู Log)".into());
    let query = s.log_search.text.trim().to_ascii_lowercase();

    let mut lines: Vec<String> = raw_logs.lines().map(String::from).collect();
    if !query.is_empty() {
        lines.retain(|l| strip_ansi(l).to_ascii_lowercase().contains(&query));
    }
    if s.log_newest_first {
        lines.reverse();
    }

    let clean_all = lines.iter().map(|l| strip_ansi(l)).collect::<Vec<_>>().join("\n");

    let controls_bar = row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(
            pill_btn(
                if s.log_newest_first { "⬆ ใหม่สุดอยู่บน" } else { "⬇ ใหม่สุดอยู่ล่าง" },
                PillVariant::Gold,
                PillSize::Sm,
                |s: &mut AppState| s.log_newest_first = !s.log_newest_first,
            ),
        )
        .child(segmented_group([
            since_btn("ทั้งหมด", None),
            since_btn("5 นาที", Some("5m")),
            since_btn("15 นาที", Some("15m")),
            since_btn("1 ชม.", Some("1h")),
            since_btn("6 ชม.", Some("6h")),
        ]))
        .child(pill_btn(
            if s.log_timestamps { "🕒 เวลา: เปิด" } else { "🕒 เวลา: ปิด" },
            if s.log_timestamps { PillVariant::Chrome } else { PillVariant::Glass },
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
                .placeholder("ค้นหาใน Log...")
                .on_edit(|s: &mut AppState| &mut s.log_search)
                .build(k),
        )
        .child(el().grow(1.0))
        .child(pill_btn("↻ Refresh", PillVariant::Glass, PillSize::Sm, |s: &mut AppState| {
            if let Some(pod) = s.logs_pod.clone() {
                s.fetch_logs(pod);
            }
        }))
        .child(pill_btn("📋 Copy ทั้งหมด", PillVariant::Primary, PillSize::Sm, move |s: &mut AppState| {
            s.copy_to_clipboard(clean_all.clone(), "Logs ทั้งหมด");
        }));

    let mut log_rows = col().gap(2.0);
    for (idx, line) in lines.iter().take(250).enumerate() {
        let clean_line = strip_ansi(line);
        let spans = parse_ansi_spans(line);
        let mut line_row = row().gap(0.0).center_y();
        for (col_fg, seg) in spans {
            line_row = line_row.child(text(seg).font(k.ty.mono).color(col_fg));
        }
        let line_widget = interactive(
            row()
                .key(format!("log-line-{idx}"))
                .w_full()
                .px(6.0)
                .py(2.0)
                .radius(4.0)
                .on_press(move |s: &mut AppState| {
                    s.copy_to_clipboard(clean_line.clone(), "บรรทัด Log");
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
                .child(Badge::new(if s.log_newest_first { "เรียง: ใหม่ล่าสุดอยู่ด้านบน ↑" } else { "เรียง: ใหม่ล่าสุดอยู่ด้านล่าง ↓" }).variant(BadgeVariant::Accent).build(k))
                .child(text("คลิกที่บรรทัดใดก็ได้เพื่อ Copy บรรทัดนั้น").font(k.ty.small).color(c.text_muted)),
        )
        .child(
            col()
                .padding(12.0)
                .radius(k.radius.lg)
                .fill(rune::color::hex("#0d1117").unwrap())
                .border(1.0, c.border_base)
                .child(log_rows)
                .scroll(ScrollSpec { stick_bottom: !s.log_newest_first })
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

fn parse_ansi_spans(input: &str) -> Vec<(rune::Color, String)> {
    let default_fg = rune::color::hex("#e6edf3").unwrap();
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

fn ansi_code_to_color(codes: &str, cur: rune::Color, default_fg: rune::Color) -> rune::Color {
    let mut col = cur;
    for part in codes.split(';') {
        match part.trim() {
            "0" | "" => col = default_fg,
            "30" | "90" => col = rune::color::hex("#94a3b8").unwrap(),
            "31" | "91" => col = rune::color::hex("#f87171").unwrap(),
            "32" | "92" => col = rune::color::hex("#4ade80").unwrap(),
            "33" | "93" => col = rune::color::hex("#facc15").unwrap(),
            "34" | "94" => col = rune::color::hex("#60a5fa").unwrap(),
            "35" | "95" => col = rune::color::hex("#e879f9").unwrap(),
            "36" | "96" => col = rune::color::hex("#22d3ee").unwrap(),
            "37" | "97" => col = rune::color::hex("#f8fafc").unwrap(),
            _ => {}
        }
    }
    col
}

fn env_view(s: &AppState, svc: &ServiceInfo, k: &Kit) -> El {
    let c = &k.c;
    let query = s.env_search.text.trim().to_ascii_lowercase();
    let mut rows = col().gap(8.0);

    if s.adding_env {
        rows = rows.child(
            col()
                .padding(12.0)
                .gap(8.0)
                .radius(k.radius.md)
                .fill(rune::color::hex("#161e2e").unwrap())
                .border(1.25, rune::color::hex("#38bdf8").unwrap())
                .child(text("เพิ่ม Environment Variable ใหม่").font(k.ty.body_medium).color(c.text_strong))
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(
                            TextInput::new(&s.new_env_key)
                                .width(Some(240.0))
                                .placeholder("KEY (เช่น REDIS_HOST)")
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
                        .child(pill_btn("💾 บันทึก", PillVariant::Primary, PillSize::Sm, |s: &mut AppState| s.save_new_env(false)))
                        .child(pill_btn("💾 บันทึก + Restart Pod", PillVariant::Gold, PillSize::Sm, |s: &mut AppState| s.save_new_env(true)))
                        .child(pill_btn("ยกเลิก", PillVariant::Glass, PillSize::Sm, |s: &mut AppState| s.cancel_edit_env())),
                ),
        );
    }

    let mut matched_count = 0usize;
    if s.env_vars.is_empty() {
        rows = rows.child(text("กำลังโหลดหรือไม่มี ENV variables ใน Deployment").font(k.ty.text_small).color(c.text_muted));
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
                        .padding(12.0)
                        .gap(8.0)
                        .radius(k.radius.md)
                        .fill(rune::color::hex("#161e2e").unwrap())
                        .border(1.25, rune::color::hex("#f59e0b").unwrap())
                        .child(
                            row()
                                .gap(8.0)
                                .center_y()
                                .child(text(ev.name.clone()).font(k.ty.mono).color(c.text_accent))
                                .child(Badge::new(ev.source.clone()).variant(BadgeVariant::Neutral).build(k)),
                        )
                        .child(
                            TextInput::new(&s.editing_env_input)
                                .large()
                                .width(None)
                                .placeholder("กรอกค่า ENV ใหม่...")
                                .on_edit(|s: &mut AppState| &mut s.editing_env_input)
                                .on_submit(|s: &mut AppState, _| s.save_editing_env(false))
                                .build(k),
                        )
                        .child(
                            row()
                                .gap(8.0)
                                .child(pill_btn("💾 บันทึก", PillVariant::Primary, PillSize::Sm, |s: &mut AppState| s.save_editing_env(false)))
                                .child(pill_btn("💾 บันทึก + Restart Pod", PillVariant::Danger, PillSize::Sm, |s: &mut AppState| s.save_editing_env(true)))
                                .child(pill_btn("ยกเลิก", PillVariant::Glass, PillSize::Sm, |s: &mut AppState| s.cancel_edit_env())),
                        ),
                );
            } else {
                let display_val = if ev.is_secret && !s.show_secrets {
                    "•••••••••••• (ซ่อนอยู่)".to_string()
                } else {
                    ev.value.clone()
                };
                let key_edit = ev.name.clone();
                let val_edit = ev.value.clone();
                let matched_hidden_val = !query.is_empty() && match_val && !match_key && ev.is_secret && !s.show_secrets;
                rows = rows.child(
                    row()
                        .padding(10.0)
                        .gap(12.0)
                        .center_y()
                        .radius(k.radius.md)
                        .fill(c.layer_02)
                        .child(text(ev.name.clone()).font(k.ty.mono).color(c.text_accent).w(240.0))
                        .child(text(display_val).font(k.ty.mono).color(c.text_base).grow(1.0).shrink().min_w(0.0))
                        .children(if matched_hidden_val {
                            Some(Badge::new("ตรงกับ Value").variant(BadgeVariant::Accent).build(k))
                        } else {
                            None
                        })
                        .child(Badge::new(ev.source.clone()).variant(BadgeVariant::Neutral).build(k))
                        .children(if ev.is_secret {
                            Some(Badge::new("SECRET").variant(BadgeVariant::Accent).build(k))
                        } else {
                            None
                        })
                        .child(pill_btn("✏ แก้ไข", PillVariant::Chrome, PillSize::Sm, move |s: &mut AppState| {
                            s.start_edit_env(key_edit.clone(), val_edit.clone());
                        })),
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
                    .color(c.text_strong)
                    .grow(1.0),
                )
                .child(
                    TextInput::new(&s.env_search)
                        .width(Some(250.0))
                        .placeholder("ค้นหา Key, Value หรือ Source...")
                        .on_edit(|s: &mut AppState| &mut s.env_search)
                        .build(k),
                )
                .child(pill_btn("📥 Export CSV", PillVariant::Chrome, PillSize::Sm, |s: &mut AppState| {
                    s.export_env_csv();
                }))
                .child(pill_btn("+ เพิ่ม ENV ใหม่", PillVariant::Primary, PillSize::Sm, |s: &mut AppState| {
                    s.adding_env = !s.adding_env;
                    s.editing_env_key = None;
                }))
                .child(pill_btn(
                    if s.show_secrets { "🙈 ซ่อนค่า Secret" } else { "👁 แสดงค่า Secret" },
                    if s.show_secrets { PillVariant::Gold } else { PillVariant::Chrome },
                    PillSize::Sm,
                    |s: &mut AppState| s.show_secrets = !s.show_secrets,
                )),
        )
        .child(
            col()
                .padding(12.0)
                .radius(k.radius.lg)
                .fill(c.bg_base)
                .border(1.0, c.border_weak)
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
