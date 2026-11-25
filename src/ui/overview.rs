use crate::k8s::{EnvKind, Health, NamespaceSummary};
use crate::state::{AppState, OverviewFilter, ServiceTab, Tab};
use crate::ui::icons;
use crate::ui::pill::{
    PillSize, PillVariant, circle_action_btn, interactive, mini_pill_btn, pill_btn, pill_icon_btn,
    pill_icon_only,
    soft_badge,
};
use crate::ui::{alert_blink_color, health_color, theme};
use rune::{Color, Cursor, Drag, El, Phase, ScrollSpec, Shadow, TextStyle, col, el, row, text};
use rune::tok::motion;
use rune_kit::{Kit, TextInput};

pub fn view(s: &AppState, k: &Kit) -> El {
    let all_count = s.overview.len();
    let stg_count = s.overview.iter().filter(|n| n.env == EnvKind::Stg).count();
    let prod_count = s.overview.iter().filter(|n| n.env == EnvKind::Prod).count();

    let filtered: Vec<&NamespaceSummary> = s
        .overview
        .iter()
        .filter(|n| match s.overview_filter {
            OverviewFilter::All => true,
            OverviewFilter::Staging => n.env == EnvKind::Stg,
            OverviewFilter::Production => n.env == EnvKind::Prod,
        })
        .collect();

    let total_svc: usize = filtered.iter().map(|n| n.total_services).sum();
    let total_ok: usize = filtered.iter().map(|n| n.healthy).sum();
    let total_issues: usize = filtered.iter().map(|n| n.warning + n.error).sum();
    let total_ingress: usize = filtered.iter().map(|n| n.ingress_count).sum();
    let has_err = filtered.iter().any(|n| n.error > 0);
    let has_warn = filtered.iter().any(|n| n.warning > 0);

    let health_pct: usize = if total_svc == 0 {
        100
    } else {
        ((total_ok as f32 / total_svc as f32) * 100.0).round() as usize
    };

    // Quick-jump target: prefer active namespace if it's in the filtered list, else first filtered item.
    let jump_target: (EnvKind, String) = filtered
        .iter()
        .find(|n| n.env == s.env && n.namespace == s.namespace)
        .or_else(|| filtered.first())
        .map(|n| (n.env, n.namespace.clone()))
        .unwrap_or_else(|| (s.env, s.namespace.clone()));

    let filter_tab = |label: String, target: OverviewFilter| {
        let active = s.overview_filter == target;
        pill_btn(
            label,
            if active { PillVariant::Primary } else { PillVariant::Glass },
            PillSize::Md,
            move |s: &mut AppState| s.overview_filter = target,
        )
    };

    let top_bar = row()
        .key("overview-top-bar")
        .w_full()
        .center_y()
        .gap(14.0)
        .child(
            col()
                .gap(3.0)
                .grow(1.0)
                .child(
                    text("Cluster Environments")
                        .font(theme::heading_xl())
                        .color(theme::text_strong()),
                )
                .child(
                    text(format!(
                        "{} namespaces active · {total_ok}/{total_svc} services healthy",
                        filtered.len()
                    ))
                    .font(k.ty.text_small)
                    .color(theme::text_muted()),
                ),
        )
        .child(
            row()
                .gap(8.0)
                .center_y()
                .child(filter_tab(format!("All ({all_count})"), OverviewFilter::All))
                .child(filter_tab(format!("Staging ({stg_count})"), OverviewFilter::Staging))
                .child(filter_tab(format!("Production ({prod_count})"), OverviewFilter::Production))
                .child(el().w(1.0).h(20.0).fill(theme::border_glass()))
                .child(pill_icon_btn(
                    &icons::PLUS,
                    "Namespace",
                    PillVariant::Glass,
                    PillSize::Md,
                    |s: &mut AppState| s.open_add_namespace(),
                ))
                .child(pill_icon_btn(
                    &icons::REFRESH,
                    if s.overview_loading { crate::tr!("กำลังอัปเดต...", "Updating...") } else { "Refresh" },
                    PillVariant::Glass,
                    PillSize::Md,
                    |s: &mut AppState| s.refresh_overview(),
                )),
        );

    let orb_card = health_orb_card(health_pct, total_issues, has_err, has_warn, k);

    let (jump_env_topo, jump_ns_topo) = jump_target.clone();
    let (jump_env_svc, jump_ns_svc) = jump_target.clone();
    let (jump_env_logs, jump_ns_logs) = jump_target.clone();
    let (jump_env_env, jump_ns_env) = jump_target;

    let hero_right = col()
        .key("overview-hero-right")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .padding(18.0)
        .gap(14.0)
        .radius(22.0)
        .fill(theme::glass_card_soft())
        .border(1.0, theme::border_glass())
        .child(
            row()
                .w_full()
                .gap(10.0)
                .child(stat_pill("Services", total_svc.to_string(), theme::text_strong()))
                .child(stat_pill("Healthy", total_ok.to_string(), theme::ok_fg()))
                .child(stat_pill(
                    "Issues",
                    total_issues.to_string(),
                    if has_err {
                        theme::err_fg()
                    } else if total_issues > 0 {
                        theme::warn_fg()
                    } else {
                        theme::warn_fg()
                    },
                ))
                .child(stat_pill("Ingress", total_ingress.to_string(), theme::text_strong())),
        )
        .child(
            row()
                .w_full()
                .center_y()
                .gap(10.0)
                .child(
                    text(crate::tr!("มุมมองด่วน:", "Quick views:"))
                        .font(k.ty.small_medium)
                        .color(theme::text_muted())
                        .grow(1.0),
                )
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(pill_icon_btn(
                            &icons::TOPOLOGY,
                            "All Topology",
                            PillVariant::Primary,
                            PillSize::Md,
                            move |s: &mut AppState| {
                                s.drill_into_namespace_tab(jump_env_topo, &jump_ns_topo, Tab::Topology)
                            },
                        ))
                        .child(pill_icon_btn(
                            &icons::SERVICES,
                            "Services",
                            PillVariant::Glass,
                            PillSize::Md,
                            move |s: &mut AppState| {
                                s.drill_into_namespace_tab(jump_env_svc, &jump_ns_svc, Tab::Services)
                            },
                        ))
                        .child(pill_icon_btn(
                            &icons::LOGS,
                            "Live Logs",
                            PillVariant::Glass,
                            PillSize::Md,
                            move |s: &mut AppState| {
                                s.drill_into_namespace_service_tab(
                                    jump_env_logs,
                                    &jump_ns_logs,
                                    ServiceTab::Logs,
                                )
                            },
                        ))
                        .child(pill_icon_btn(
                            &icons::SLIDERS,
                            "ENV",
                            PillVariant::Glass,
                            PillSize::Md,
                            move |s: &mut AppState| {
                                s.drill_into_namespace_service_tab(
                                    jump_env_env,
                                    &jump_ns_env,
                                    ServiceTab::Env,
                                )
                            },
                        )),
                ),
        );

    let hero = row()
        .key("overview-hero")
        .w_full()
        .gap(14.0)
        .child(orb_card)
        .child(hero_right);

    // No mock data: while loading show the owl, if nothing could be read say so.
    let cards_grid = if s.overview.is_empty() && s.overview_loading {
        crate::ui::owl_loader::view(s.loader_t, crate::tr!("กำลังโหลดข้อมูลจาก cluster", "Loading cluster data"), k)
    } else if s.overview.is_empty() && s.overview_failed > 0 {
        // Every namespace failed: one clear card with the reason of the first failure.
        let raw = s.overview_errors.first().map(|(_, e)| e.clone()).unwrap_or_default();
        let names: Vec<&str> = s.overview_errors.iter().map(|(n, _)| n.as_str()).collect();
        crate::ui::error_panel::panel(
            &raw,
            Some(crate::tf!("อ่านไม่ได้ทั้งหมด: {}", "None could be read: {}", names.join(", "))),
            |s: &mut AppState| s.refresh_overview(),
            k,
        )
    } else if s.overview.is_empty() {
        empty_state(
            crate::tr!("ยังไม่มี Namespace", "No namespaces yet"),
            crate::tr!("กด \"เพิ่ม Namespace\" เพื่อเลือก namespace ที่ต้องการดู", "Press \"Add namespace\" to choose the namespaces to watch"),
            k,
        )
    } else {
        namespace_cards_grid(s, &filtered, s.pulse_on, s.cards_grid_w, k)
    };

    let content = col()
        .key("overview-content")
        .w_full()
        .padding(22.0)
        .gap(20.0)
        .child(top_bar)
        .children(if s.adding_ns { Some(add_namespace_panel(s, k)) } else { None })
        .child(hero)
        // Some namespaces loaded and some didn't: say which ones and why, above the cards.
        .children(if !s.overview.is_empty() && !s.overview_errors.is_empty() && !s.overview_loading {
            Some(crate::ui::error_panel::strip(&s.overview_errors, |s: &mut AppState| s.refresh_overview(), k))
        } else {
            None
        })
        .child(cards_grid);

    content.scroll(ScrollSpec { stick_bottom: false }).w_full().h_full()
}

fn health_orb_card(
    health_pct: usize,
    total_issues: usize,
    has_err: bool,
    has_warn: bool,
    k: &Kit,
) -> El {
    let (status_label, caption, mid_tint, outer_tint) = if has_err {
        (
            "Critical",
            crate::tf!("{total_issues} services มีปัญหา", "{total_issues} services failing"),
            theme::rgba(248, 113, 113, 0.24),
            theme::rgba(217, 179, 130, 0.14),
        )
    } else if has_warn || total_issues > 0 {
        (
            "Warning",
            crate::tf!("{total_issues} services ต้องตรวจสอบ", "{total_issues} services need attention"),
            theme::rgba(252, 211, 77, 0.24),
            theme::rgba(217, 179, 130, 0.16),
        )
    } else {
        (
            "Healthy",
            crate::tr!("ทุก service ทำงานปกติ", "All services healthy").to_string(),
            theme::rgba(167, 243, 208, 0.22),
            theme::rgba(217, 179, 130, 0.18),
        )
    };

    let orb = col()
        .key("health-orb")
        .size(112.0, 112.0)
        .radius(999.0)
        .center()
        .gap(2.0)
        .fill(theme::ink(0.05))
        .radial_gradient(
            [0.32, 0.30],
            84.0,
            [
                (0.0, theme::ink(0.26)),
                (0.42, mid_tint),
                (0.78, outer_tint),
                (1.0, theme::ink(0.04)),
            ],
        )
        .border(1.0, theme::ink(0.22))
        .box_shadow(Shadow {
            x: 0.0,
            y: 12.0,
            blur: 28.0,
            spread: 0.0,
            color: Color::BLACK.alpha(0.36),
            inset: false,
        })
        .box_shadow(Shadow {
            x: 0.0,
            y: 1.0,
            blur: 14.0,
            spread: 0.0,
            color: theme::ink(0.16),
            inset: true,
        })
        .child(
            text(format!("{health_pct}%"))
                .font(theme::orb_pct())
                .color(theme::text_strong()),
        )
        .child(
            text(status_label)
                .font(theme::label_xs())
                .color(theme::text_secondary()),
        );

    col()
        .key("overview-orb-card")
        .w(240.0)
        .padding(18.0)
        .gap(10.0)
        .center()
        .radius(22.0)
        .fill(theme::glass_card())
        .border(1.0, theme::border_glass())
        .child(orb)
        .child(
            text(caption)
                .font(k.ty.small)
                .color(theme::text_muted()),
        )
}

fn stat_pill(label: &'static str, value: String, color: Color) -> El {
    col()
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .px(13.0)
        .py(11.0)
        .gap(2.0)
        .radius(16.0)
        .fill(theme::metric_box_bg())
        .border(1.0, theme::border_subtle())
        .motion(motion::SNAPPY)
        .child(text(label).font(theme::label_xs()).color(theme::text_muted()))
        .child(text(value).font(theme::stat_val()).color(color))
}

fn empty_state(title: &str, body: &str, k: &Kit) -> El {
    col()
        .w_full()
        .padding(32.0)
        .gap(6.0)
        .center()
        .radius(20.0)
        .fill(theme::glass_card())
        .border(1.0, theme::border_glass())
        .child(text(title.to_string()).font(k.ty.body_medium).color(theme::text_strong()))
        .child(text(body.to_string()).font(k.ty.text_small).color(theme::text_muted()))
}

fn namespace_cards_grid(s: &AppState, items: &[&NamespaceSummary], pulse_on: bool, grid_w: f32, k: &Kit) -> El {
    if items.is_empty() {
        return col()
            .w_full()
            .padding(32.0)
            .center()
            .radius(20.0)
            .fill(theme::glass_card())
            .border(1.0, theme::border_glass())
            .child(
                text(crate::tr!("ไม่มี Namespace ในตัวกรองที่เลือก", "No namespaces match this filter"))
                    .font(k.ty.body_medium)
                    .color(theme::text_muted()),
            );
    }

    // Three equal cards per row: the grid measures its own width, then every card gets exactly
    // a third of it (minus the gaps). Until the first measurement, cards simply share the row.
    const GAP: f32 = 13.0;
    let card_w = if grid_w > 0.0 { Some(((grid_w - GAP * 2.0) / 3.0).floor().max(200.0)) } else { None };
    let mut grid = col()
        .key("overview-cards-grid")
        .w_full()
        .gap(GAP)
        .on_layout_tx(|s: &mut AppState, rect, tx| {
            if (s.cards_grid_w - rect[2]).abs() > 0.5 {
                s.cards_grid_w = rect[2];
            } else {
                tx.live_only();
            }
        });
    for chunk in items.chunks(3) {
        let mut r = row().w_full().gap(GAP);
        for item in chunk {
            r = r.child(namespace_card(s, item, pulse_on, card_w, k));
        }
        for _ in chunk.len()..3 {
            r = r.child(match card_w {
                Some(w) => el().w(w),
                None => el().grow(1.0).shrink().min_w(0.0),
            });
        }
        grid = grid.child(r);
    }
    grid
}

fn namespace_card(s: &AppState, ns: &NamespaceSummary, pulse_on: bool, card_w: Option<f32>, k: &Kit) -> El {
    let env = ns.env;
    let ns_hold = ns.namespace.clone();
    let hold_p = s.hold_progress_for(env, &ns.namespace);
    let confirming = s
        .confirm_unwatch
        .as_ref()
        .map_or(false, |(e, n)| *e == env && n == &ns.namespace);
    let ns_card_click = ns.namespace.clone();
    let ns_arrow = ns.namespace.clone();
    let ns_diag = ns.namespace.clone();
    let ns_svc = ns.namespace.clone();
    let ns_logs = ns.namespace.clone();
    let ns_env = ns.namespace.clone();

    let overall_health = if ns.error > 0 {
        Health::Error
    } else if ns.warning > 0 {
        Health::Warning
    } else {
        Health::Healthy
    };
    let has_issue = overall_health != Health::Healthy;
    let blink_col = alert_blink_color(overall_health, pulse_on);

    let env_pill = if env == EnvKind::Prod {
        soft_badge("PROD", theme::prod_bg(), theme::prod_fg(), None)
    } else {
        soft_badge("STG", theme::stg_bg(), theme::stg_fg(), None)
    };

    let source_pill = soft_badge(
        if ns.is_demo { "DEMO" } else { "LIVE" },
        theme::ink(0.05),
        theme::text_faint(),
        Some(theme::border_subtle()),
    );

    let card = col().key(format!("ns-card-{}", ns.namespace));
    let card = match card_w {
        Some(w) => card.w(w),
        None => card.grow(1.0).shrink().min_w(0.0),
    };
    interactive(
        card
            .padding(16.0)
            .gap(14.0)
            .radius(20.0)
            .fill(if confirming || (has_issue && pulse_on) {
                theme::err_card_bg()
            } else {
                theme::glass_card()
            })
            // Issue cards show it through the red background only; the border stays neutral.
            .border(1.0, if confirming { theme::err_border() } else { theme::border_glass() })
            .motion(motion::SNAPPY)
            .on_press(move |s: &mut AppState| {
                // A press that completed a hold (or a card asking to stop watching) is not a click.
                if s.ns_press_blocked() || s.confirm_unwatch.is_some() {
                    return;
                }
                s.drill_into_namespace(env, &ns_card_click)
            })
            // Press and hold the card to stop watching this namespace.
            .on_drag(move |s: &mut AppState, d: &mut Drag, tx| match d.phase {
                Phase::Down => {
                    if s.confirm_unwatch.is_none() {
                        s.hold_begin(env, &ns_hold);
                        tx.wake_after(std::time::Duration::from_millis(16));
                    }
                }
                Phase::Move => {
                    if d.dx * d.dx + d.dy * d.dy > 64.0 {
                        s.hold_cancel();
                    }
                }
                Phase::Up => s.hold_cancel(),
            })
            .child(
                row()
                    .w_full()
                    .gap(10.0)
                    .child(
                        col()
                            .gap(5.0)
                            .grow(1.0)
                            .shrink()
                            .min_w(0.0)
                            .child(
                                text(ns.namespace.clone())
                                    .font(theme::heading_md())
                                    .color(theme::text_strong()),
                            )
                            .child(
                                row()
                                    .gap(6.0)
                                    .center_y()
                                    .child(env_pill)
                                    .child(el().size(6.0, 6.0).radius(3.0).fill(blink_col))
                                    .child(
                                        text(format!("{}/{} Ready", ns.healthy, ns.total_services))
                                            .font(k.ty.small)
                                            .color(theme::text_muted()),
                                    )
                                    .child(source_pill),
                            ),
                    )
                    .child(circle_action_btn(
                        &icons::ARROW_RIGHT,
                        crate::tf!("เปิด {}", "Open {}", ns.namespace),
                        move |s: &mut AppState| s.drill_into_namespace(env, &ns_arrow),
                    )),
            )
            .child(
                row()
                    .w_full()
                    .gap(8.0)
                    .child(card_metric_box(
                        "Services",
                        ns.total_services.to_string(),
                        theme::text_strong(),
                    ))
                    .child(card_metric_box(
                        "Healthy",
                        ns.healthy.to_string(),
                        health_color(overall_health),
                    ))
                    .child(card_metric_box(
                        "Ingress",
                        ns.ingress_count.to_string(),
                        theme::text_strong(),
                    )),
            )
            .child(if confirming {
                unwatch_confirm_row(&ns.namespace, k)
            } else {
                row()
                    .w_full()
                    .gap(6.0)
                    .center_y()
                    .child(mini_pill_btn("Diagram", move |s: &mut AppState| {
                        s.drill_into_namespace_tab(env, &ns_diag, Tab::Topology)
                    }))
                    .child(mini_pill_btn("Services", move |s: &mut AppState| {
                        s.drill_into_namespace_tab(env, &ns_svc, Tab::Services)
                    }))
                    .child(mini_pill_btn("Logs", move |s: &mut AppState| {
                        s.drill_into_namespace_service_tab(env, &ns_logs, ServiceTab::Logs)
                    }))
                    .child(mini_pill_btn("ENV", move |s: &mut AppState| {
                        s.drill_into_namespace_service_tab(env, &ns_env, ServiceTab::Env)
                    }))
            })
            .children(hold_bar(hold_p, card_w)),
    )
}

/// The thin bar that fills while a card is held down.
fn hold_bar(p: f32, card_w: Option<f32>) -> Option<El> {
    if p <= 0.02 {
        return None;
    }
    let track = card_w.map(|w| w - 32.0).unwrap_or(260.0).max(40.0);
    // Ease out so the bar leaps forward first and settles into the finish.
    let e = 1.0 - (1.0 - p).powi(3);
    Some(
        col()
            .w_full()
            .gap(6.0)
            .child(
                text(crate::tr!("กดค้างไว้เพื่อเลิกดู Namespace นี้…", "Keep holding to stop watching…"))
                    .font(TextStyle::new(11.0, 500.0))
                    .color(theme::err_fg().alpha(0.35 + 0.65 * p)),
            )
            .child(
                row()
                    .w(track)
                    .h(3.0)
                    .radius(2.0)
                    .fill(theme::ink(0.08))
                    .child(el().w(track * e).h(3.0).radius(2.0).fill(theme::err_dot())),
            ),
    )
}

/// Replaces the card's buttons with "stop watching?" once the hold completes.
fn unwatch_confirm_row(ns: &str, k: &Kit) -> El {
    row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(
            text(crate::tf!("เลิกดู {ns} ?", "Stop watching {ns}?"))
                .font(k.ty.small)
                .color(theme::err_fg())
                .grow(1.0)
                .shrink()
                .min_w(0.0),
        )
        .child(mini_pill_btn(crate::tr!("ยกเลิก", "Cancel"), |s: &mut AppState| s.cancel_unwatch()))
        .child(danger_pill_btn(crate::tr!("เลิกดู", "Stop watching"), |s: &mut AppState| {
            s.confirm_unwatch_namespace()
        }))
}

fn danger_pill_btn(label: &str, on_click: impl Fn(&mut AppState) + 'static) -> El {
    let label = label.to_string();
    interactive(
        row()
            .role("button")
            .label(label.clone())
            .hit()
            .cursor(Cursor::Pointer)
            .h(25.0)
            .px(12.0)
            .center()
            .radius(999.0)
            .fill(theme::err_bg())
            .border(1.0, theme::err_border())
            .hover(theme::err_pulse_hi().alpha(0.28))
            .press(Color::BLACK.alpha(0.18))
            .on_press(on_click)
            .child(
                text(label)
                    .font(TextStyle::new(11.0, 600.0))
                    .color(theme::err_fg())
                    .a11y_hidden(),
            ),
    )
}

fn card_metric_box(label: &'static str, val: String, color: Color) -> El {
    col()
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .px(10.0)
        .py(9.0)
        .gap(1.0)
        .radius(14.0)
        .fill(theme::card_inner_bg())
        .border(1.0, theme::border_subtle())
        .child(text(label).font(theme::label_xs()).color(theme::text_muted()))
        .child(text(val).font(theme::metric_val()).color(color))
}

fn add_namespace_panel(s: &AppState, k: &Kit) -> El {
    // Namespace chips flow onto new lines instead of running off the panel. Rows are filled
    // using an estimate of each chip's width; the limit fits the narrowest window.
    const MAX_ROW_W: f32 = 860.0;
    const GAP: f32 = 6.0;
    let chip_w = |name: &str| name.chars().count() as f32 * 7.4 + 46.0;
    let mut avail_rows = col().gap(6.0);
    let mut line = row().gap(GAP).center_y();
    let mut line_w = 0.0_f32;
    let mut line_has = false;
    for ns in s.available_namespaces.iter() {
        let already = s.stg_namespaces.contains(ns) || s.prod_namespaces.contains(ns);
        if already {
            continue;
        }
        let w = chip_w(ns);
        if line_has && line_w + GAP + w > MAX_ROW_W {
            avail_rows = avail_rows.child(line);
            line = row().gap(GAP).center_y();
            line_w = 0.0;
            line_has = false;
        }
        let ns_click = ns.clone();
        let target_env = s.new_ns_env;
        line = line.child(pill_icon_btn(
            &icons::PLUS,
            ns.clone(),
            PillVariant::Glass,
            PillSize::Sm,
            move |s: &mut AppState| s.add_namespace(target_env, ns_click.clone()),
        ));
        line_w += if line_has { GAP + w } else { w };
        line_has = true;
    }
    if line_has {
        avail_rows = avail_rows.child(line);
    }

    col()
        .key("add-ns-panel")
        .w_full()
        .padding(16.0)
        .gap(10.0)
        .radius(20.0)
        .fill(theme::glass_card())
        .border(1.0, theme::border_strong())
        .child(
            row()
                .center_y()
                .gap(8.0)
                .child(
                    text(crate::tr!("เพิ่ม Namespace เข้า Overview & Dock ด้านล่าง", "Add a namespace to the Overview and the dock"))
                        .font(k.ty.body_medium)
                        .color(theme::text_strong())
                        .grow(1.0),
                )
                .child(pill_btn(
                    "STG",
                    if s.new_ns_env == EnvKind::Stg {
                        PillVariant::Primary
                    } else {
                        PillVariant::Glass
                    },
                    PillSize::Sm,
                    |s: &mut AppState| s.new_ns_env = EnvKind::Stg,
                ))
                .child(pill_btn(
                    "PROD",
                    if s.new_ns_env == EnvKind::Prod {
                        PillVariant::Gold
                    } else {
                        PillVariant::Glass
                    },
                    PillSize::Sm,
                    |s: &mut AppState| s.new_ns_env = EnvKind::Prod,
                ))
                .child(pill_icon_only(
                    &icons::CLOSE,
                    crate::tr!("ปิดแผงเพิ่ม Namespace", "Close the add-namespace panel"),
                    PillVariant::Glass,
                    PillSize::Sm,
                    |s: &mut AppState| s.adding_ns = false,
                )),
        )
        .child(
            row()
                .gap(8.0)
                .center_y()
                .child(
                    TextInput::new(&s.new_ns_input)
                        .width(Some(280.0))
                        .placeholder(crate::tr!("พิมพ์ชื่อ namespace เช่น finance หรือ monitor", "Namespace name, e.g. finance or monitor"))
                        .on_edit(|s: &mut AppState| &mut s.new_ns_input)
                        .on_submit(|s: &mut AppState, _| {
                            let val = s.new_ns_input.text.clone();
                            s.add_namespace(s.new_ns_env, val);
                        })
                        .build(k),
                )
                .child(pill_icon_btn(
                    &icons::PLUS,
                    crate::tr!("เพิ่ม", "Add"),
                    PillVariant::Primary,
                    PillSize::Sm,
                    |s: &mut AppState| {
                        let val = s.new_ns_input.text.clone();
                        s.add_namespace(s.new_ns_env, val);
                    },
                )),
        )
        .child(
            col()
                .gap(8.0)
                .child(text(crate::tr!("ใน Cluster:", "In cluster:")).font(k.ty.small_medium).color(theme::text_muted()))
                .child(avail_rows),
        )
}
