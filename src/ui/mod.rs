pub mod icons;
pub mod login;
pub mod overview;
pub mod pill;
pub mod project_picker;
pub mod service_detail;
pub mod setup;
pub mod services;
pub mod terminal;
pub mod theme;
pub mod tools_panel;
pub mod update_panel;
pub mod topology;
pub mod topology_g6;

use crate::k8s::{EnvKind, Health};
use crate::state::{AppState, Page, ServiceTab, Tab};
use crate::update::UpdateState;
use icons::mono_icon;
use pill::{
    PillSize, PillVariant, interactive, pill_icon_btn, pill_icon_only_tx, segmented_group,
    soft_badge,
};
use rune::{Color, El, Tx, col, el, row, text};
use rune::tok::motion;
use rune_kit::Kit;

pub fn health_color(h: Health) -> Color {
    match h {
        Health::Healthy => theme::ok_fg(),
        Health::Warning => theme::warn_fg(),
        Health::Error => theme::err_dot(),
    }
}

pub fn alert_blink_color(h: Health, pulse_on: bool) -> Color {
    if h == Health::Healthy {
        theme::ok_dot()
    } else if pulse_on {
        theme::err_pulse_hi()
    } else {
        theme::err_pulse_lo()
    }
}

pub fn env_color(env: EnvKind) -> Color {
    match env {
        EnvKind::Stg => theme::stg_dot(),
        EnvKind::Prod => theme::prod_dot(),
    }
}

/// Custom K9s Studio monochrome mark rendered cleanly on GPU.
pub fn app_icon() -> El {
    el().key("app-icon")
        .size(26.0, 26.0)
        .radius(8.0)
        .fill(theme::accent_primary_bg())
        .border(1.0, theme::accent_primary_bg())
        .center()
        .child(
            text("K9")
                .font(rune::TextStyle::new(11.0, 700.0).tracking(-0.2))
                .color(theme::accent_primary_fg()),
        )
        .a11y_hidden()
}

pub fn custom_titlebar(s: &AppState, k: &Kit) -> El {
    let logged_in = s.session.is_some();

    let breadcrumb = if !logged_in {
        "Sign In".to_string()
    } else if s.picking_project {
        "เลือก Project".to_string()
    } else {
        match s.page {
            Page::Overview => "Overview · STG + PROD".to_string(),
            Page::Namespace => format!("{}  ›  {}", s.env.label(), s.namespace),
            Page::ServiceDetail => format!(
                "{}  ›  {}  ›  {}",
                s.env.label(),
                s.namespace,
                s.selected_service.as_deref().unwrap_or("Service")
            ),
        }
    };

    let mut bar = row()
        .key("custom-titlebar")
        .w_full()
        .h(46.0)
        .px(14.0)
        .gap(10.0)
        .center_y()
        .fill(theme::bg_chrome())
        .border_sides([0.0, 0.0, 1.0, 0.0], theme::border_subtle())
        .window_drag()
        .child(app_icon())
        .child(
            text("K9s Studio")
                .font(k.ty.body_medium)
                .color(theme::text_strong()),
        )
        .child(soft_badge(
            breadcrumb,
            theme::glass_card(),
            theme::text_secondary(),
            Some(theme::border_subtle()),
        ))
        .child(el().grow(1.0));

    if let Some(u) = &s.session {
        bar = bar
            .child(soft_badge(
                u.display_name.clone(),
                theme::metric_box_soft(),
                theme::text_muted(),
                Some(theme::border_subtle()),
            ))
            .child(pill_icon_btn(
                &icons::SWITCH_ARROWS,
                match &s.active_project {
                    Some(p) => format!("Project: {p}"),
                    None => "เลือก Project".to_string(),
                },
                PillVariant::Glass,
                PillSize::Sm,
                |s: &mut AppState| s.begin_project_picker(),
            ))
            .child(pill_icon_btn(
                &icons::LOGOUT,
                "ออกจากระบบ",
                PillVariant::Glass,
                PillSize::Sm,
                |s: &mut AppState| s.logout(),
            ));
    }

    match &s.update {
        UpdateState::Available(version) => {
            bar = bar.child(pill_icon_btn(
                &icons::DOWNLOAD,
                format!("อัปเดตเป็น v{version}"),
                PillVariant::Primary,
                PillSize::Sm,
                |s: &mut AppState| s.start_update(),
            ));
        }
        UpdateState::Installing(_) => {
            bar = bar.child(soft_badge(
                "กำลังอัปเดต...",
                theme::metric_box_soft(),
                theme::text_muted(),
                Some(theme::border_subtle()),
            ));
        }
        UpdateState::Idle => {}
    }

    bar.child(
        row()
            .key("win-controls")
            .gap(6.0)
            .center_y()
            .child(pill_icon_only_tx(
                &icons::MINUS,
                "Minimize",
                PillVariant::Glass,
                PillSize::Sm,
                |_: &mut AppState, tx: &mut Tx| tx.minimize(),
            ))
            .child(pill_icon_only_tx(
                &icons::MAXIMIZE,
                "Maximize",
                PillVariant::Glass,
                PillSize::Sm,
                |_: &mut AppState, tx: &mut Tx| tx.toggle_maximized(),
            ))
            .child(pill_icon_only_tx(
                &icons::CLOSE,
                "Close",
                PillVariant::Danger,
                PillSize::Sm,
                |_: &mut AppState, tx: &mut Tx| tx.close_window(),
            )),
    )
}

pub fn app_shell(s: &AppState, k: &Kit) -> El {
    let c = &k.c;

    // True fullscreen diagram mode: hide titlebar, tab bar, subbar, and bottom dock completely.
    if s.diagram_fullscreen {
        if s.page == Page::Namespace && s.tab == Tab::Topology {
            return topology::view(s, k);
        }
        if s.page == Page::ServiceDetail && s.service_tab == ServiceTab::Diagram {
            return service_detail::view(s, k);
        }
    }

    let body = match s.page {
        Page::Overview => overview::view(s, k),
        Page::Namespace => {
            let sub_bar = namespace_subbar(s, k);
            let inner = match s.tab {
                Tab::Topology => topology::view(s, k),
                Tab::Services => services::view(s, k),
                Tab::K9sCli => terminal::view(s, k),
            };
            col()
                .w_full()
                .h_full()
                .child(sub_bar)
                .child(inner.grow(1.0).shrink().min_h(0.0))
        }
        Page::ServiceDetail => service_detail::view(s, k),
    };

    let mut root_col = col()
        .key("shell")
        .w_full()
        .h_full()
        .fill(theme::bg_canvas())
        .radial_gradient(
            [0.14, 0.08],
            [680.0, 380.0],
            [
                (0.0, theme::glow_warm()),
                (0.65, Color::TRANSPARENT),
            ],
        )
        .child(custom_titlebar(s, k))
        .child(workspace_tab_bar(s, k));

    if let Some((msg, ok)) = &s.banner {
        let bg = if *ok { c.success.bg } else { c.danger.bg };
        let fg = if *ok { c.success.fg } else { c.danger.fg };
        let ic = if *ok { &icons::CHECK } else { &icons::WARNING };
        root_col = root_col.child(
            row()
                .key("banner")
                .w_full()
                .px(20.0)
                .py(8.0)
                .fill(bg)
                .border_sides([0.0, 0.0, 1.0, 0.0], if *ok { c.success.border } else { c.danger.border })
                .center_y()
                .gap(10.0)
                .child(mono_icon(ic, 14.0, fg))
                .child(text(msg.clone()).font(k.ty.text_small_medium).color(fg).grow(1.0))
                .child(pill_icon_btn(
                    &icons::CLOSE,
                    "ปิด",
                    PillVariant::Glass,
                    PillSize::Sm,
                    |s: &mut AppState| s.banner = None,
                )),
        );
    }

    if s.page != Page::Overview {
        if let Some(err) = &s.snapshot.error {
            root_col = root_col.child(
                row()
                    .key("k8s-warn")
                    .w_full()
                    .px(20.0)
                    .py(7.0)
                    .fill(c.warning.bg)
                    .border_sides([0.0, 0.0, 1.0, 0.0], c.warning.border)
                    .center_y()
                    .gap(8.0)
                    .child(mono_icon(&icons::WARNING, 14.0, c.warning.fg))
                    .child(
                        text(err.clone())
                            .font(k.ty.small_medium)
                            .color(c.warning.fg)
                            .grow(1.0),
                    ),
            );
        }
    }

    root_col
        .child(body.grow(1.0).shrink().min_h(0.0))
        .child(bottom_dock(s, k))
}

fn workspace_tab_bar(s: &AppState, k: &Kit) -> El {
    let can_close = s.open_tabs.len() > 1;
    let mut tabs_row = row().gap(6.0).center_y();

    for wt in &s.open_tabs {
        let active = wt.id == s.active_tab_id;
        let id_switch = wt.id;
        let id_close = wt.id;
        let tab_icon = match wt.page {
            Page::Overview => &icons::OVERVIEW_GRID,
            Page::Namespace => match wt.ns_tab {
                Tab::Topology => &icons::TOPOLOGY,
                Tab::Services => &icons::SERVICES,
                Tab::K9sCli => &icons::TERMINAL,
            },
            Page::ServiceDetail => &icons::SERVER,
        };
        let fg = if active {
            theme::accent_primary_fg()
        } else {
            theme::text_secondary()
        };

        let mut pill = row()
            .key(format!("wtab-{}", wt.id))
            .h(28.0)
            .px(12.0)
            .gap(7.0)
            .center_y()
            .radius(999.0)
            .fill(if active {
                theme::accent_primary_bg()
            } else {
                theme::glass_card()
            })
            .border(
                1.0,
                if active {
                    theme::accent_primary_bg()
                } else {
                    theme::border_glass()
                },
            )
            .on_press(move |s: &mut AppState| s.switch_workspace_tab(id_switch))
            .child(mono_icon(tab_icon, 12.5, fg))
            .child(
                text(wt.title.clone())
                    .font(k.ty.small_medium)
                    .color(fg),
            );

        if can_close {
            pill = pill.child(interactive(
                el().size(16.0, 16.0)
                    .radius(999.0)
                    .center()
                    .on_press(move |s: &mut AppState| s.close_workspace_tab(id_close))
                    .child(mono_icon(
                        &icons::CLOSE,
                        10.0,
                        if active {
                            theme::accent_primary_fg().alpha(0.75)
                        } else {
                            theme::text_muted()
                        },
                    )),
            ));
        }
        tabs_row = tabs_row.child(interactive(pill));
    }

    row()
        .key("workspace-tab-bar")
        .w_full()
        .h(40.0)
        .px(16.0)
        .gap(8.0)
        .center_y()
        .fill(theme::hex("#13110f"))
        .border_sides([0.0, 0.0, 1.0, 0.0], theme::border_subtle())
        .child(tabs_row)
        .child(pill_icon_btn(
            &icons::PLUS,
            "Tab ใหม่",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.new_workspace_tab(),
        ))
}

fn namespace_subbar(s: &AppState, k: &Kit) -> El {
    let tab_btn = |ic: &'static rune::Icon, label: &'static str, target: Tab| {
        let active = s.tab == target;
        pill_icon_btn(
            ic,
            label,
            if active { PillVariant::Primary } else { PillVariant::Glass },
            PillSize::Sm,
            move |s: &mut AppState| {
                s.tab = target;
                s.sync_active_tab();
                if target == Tab::K9sCli && !s.k9s_pty.is_running() {
                    s.start_k9s();
                }
            },
        )
    };

    let env_pill = if s.env == EnvKind::Prod {
        soft_badge("PROD", theme::prod_bg(), theme::prod_fg(), None)
    } else {
        soft_badge("STG", theme::stg_bg(), theme::stg_fg(), None)
    };

    row()
        .key("ns-subbar")
        .w_full()
        .h(52.0)
        .px(20.0)
        .gap(12.0)
        .center_y()
        .fill(theme::bg_chrome())
        .border_sides([0.0, 0.0, 1.0, 0.0], theme::border_subtle())
        .child(pill_icon_btn(
            &icons::ARROW_LEFT,
            "Overview",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.open_overview(),
        ))
        .child(env_pill)
        .child(
            text(s.namespace.clone())
                .font(k.ty.body_medium)
                .color(theme::text_strong()),
        )
        .child(segmented_group([
            tab_btn(&icons::TOPOLOGY, "Stack Diagram", Tab::Topology),
            tab_btn(&icons::SERVICES, "Services", Tab::Services),
            tab_btn(&icons::TERMINAL, "K9s CLI", Tab::K9sCli),
        ]))
        .child(el().grow(1.0))
        .child(pill_icon_btn(
            &icons::REFRESH,
            if s.loading { "กำลังโหลด..." } else { "Refresh" },
            PillVariant::Chrome,
            PillSize::Sm,
            |s: &mut AppState| s.refresh_cluster(),
        ))
}

fn bottom_dock(s: &AppState, k: &Kit) -> El {
    let mut ns_pills = row().gap(6.0).center_y();
    for (env, list) in [
        (EnvKind::Stg, &s.stg_namespaces),
        (EnvKind::Prod, &s.prod_namespaces),
    ] {
        ns_pills = ns_pills.child(if env == EnvKind::Prod {
            soft_badge("PROD", theme::prod_bg(), theme::prod_fg(), None)
        } else {
            soft_badge("STG", theme::stg_bg(), theme::stg_fg(), None)
        });
        for ns in list {
            let active = s.page != Page::Overview && s.env == env && s.namespace == *ns;
            let ns_owned = ns.clone();
            let health = s
                .overview
                .iter()
                .find(|o| o.env == env && o.namespace == *ns)
                .map(|o| {
                    if o.error > 0 {
                        Health::Error
                    } else if o.warning > 0 {
                        Health::Warning
                    } else {
                        Health::Healthy
                    }
                })
                .unwrap_or(Health::Healthy);

            let has_issue = health != Health::Healthy;
            let blink_col = alert_blink_color(health, s.pulse_on);
            ns_pills = ns_pills.child(interactive(
                row()
                    .key(format!("dock-{ns}"))
                    .px(12.0)
                    .h(28.0)
                    .gap(6.0)
                    .center_y()
                    .radius(999.0)
                    .fill(if active {
                        theme::accent_primary_bg()
                    } else {
                        theme::glass_card()
                    })
                    .border(
                        if has_issue { 1.4 } else { 1.0 },
                        if has_issue {
                            blink_col
                        } else if active {
                            theme::accent_primary_bg()
                        } else {
                            theme::border_glass()
                        },
                    )
                    .motion(motion::SNAPPY)
                    .on_press(move |s: &mut AppState| s.drill_into_namespace(env, &ns_owned))
                    .child(el().size(6.0, 6.0).radius(3.0).fill(blink_col))
                    .child(
                        text(ns.clone())
                            .font(k.ty.small_medium)
                            .color(if active {
                                theme::accent_primary_fg()
                            } else {
                                theme::text_secondary()
                            }),
                    ),
            ));
        }
    }

    row()
        .key("bottom-dock")
        .w_full()
        .h(52.0)
        .px(16.0)
        .gap(10.0)
        .center_y()
        .fill(theme::bg_chrome())
        .border_sides([1.0, 0.0, 0.0, 0.0], theme::border_subtle())
        .child(pill_icon_btn(
            &icons::OVERVIEW_GRID,
            "Overview",
            if s.page == Page::Overview {
                PillVariant::Primary
            } else {
                PillVariant::Glass
            },
            PillSize::Sm,
            |s: &mut AppState| s.open_overview(),
        ))
        .child(ns_pills)
        .child(el().grow(1.0))
        .child(pill_icon_btn(
            &icons::TERMINAL,
            "K9s CLI",
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
}
