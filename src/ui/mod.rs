pub mod login;
pub mod overview;
pub mod pill;
pub mod service_detail;
pub mod services;
pub mod terminal;
pub mod topology;
pub mod topology_g6;

use crate::k8s::{EnvKind, Health};
use crate::state::{AppState, Page, ServiceTab, Tab};
use pill::{PillSize, PillVariant, interactive, pill_btn, pill_btn_tx, segmented_group};
use rune::{Color, El, Tx, col, el, row, text};
use rune::tok::motion;
use rune_kit::{Badge, BadgeVariant, Kit};

pub fn health_color(h: Health) -> Color {
    match h {
        Health::Healthy => rune::color::hex("#22c55e").unwrap(),
        Health::Warning => rune::color::hex("#f59e0b").unwrap(),
        Health::Error => rune::color::hex("#ef4444").unwrap(),
    }
}

pub fn alert_blink_color(h: Health, pulse_on: bool) -> Color {
    if h == Health::Healthy {
        health_color(h)
    } else if pulse_on {
        rune::color::hex("#ff2e4c").unwrap()
    } else {
        rune::color::hex("#7f1d1d").unwrap()
    }
}

pub fn env_color(env: EnvKind) -> Color {
    match env {
        EnvKind::Stg => rune::color::hex("#3b82f6").unwrap(),
        EnvKind::Prod => rune::color::hex("#ef4444").unwrap(),
    }
}

/// Custom K9s Studio App Icon rendered cleanly on GPU.
pub fn app_icon(k: &Kit) -> El {
    el().key("app-icon")
        .size(26.0, 26.0)
        .radius(7.0)
        .fill(rune::color::hex("#1e3a8a").unwrap())
        .border(1.5, rune::color::hex("#38bdf8").unwrap())
        .center()
        .child(
            text("K9")
                .font(rune::TextStyle::new(11.0, 700.0))
                .color(rune::color::hex("#e0f2fe").unwrap()),
        )
        .a11y_hidden()
        .label(k.c.text_strong.a.to_string()) // keeps unused param quiet if needed
}

pub fn custom_titlebar(s: &AppState, k: &Kit) -> El {
    let c = &k.c;
    let logged_in = s.session.is_some();

    let breadcrumb = if !logged_in {
        "Sign In".to_string()
    } else {
        match s.page {
            Page::Overview => "Overview (STG + PROD)".to_string(),
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
        .h(44.0)
        .px(14.0)
        .gap(10.0)
        .center_y()
        .fill(c.bg_base)
        .border(1.0, c.border_weak)
        .window_drag()
        .child(
            el().size(26.0, 26.0)
                .radius(7.0)
                .fill(rune::color::hex("#1e3a8a").unwrap())
                .border(1.5, rune::color::hex("#38bdf8").unwrap())
                .center()
                .child(
                    text("K9")
                        .font(rune::TextStyle::new(11.0, 700.0))
                        .color(rune::color::hex("#e0f2fe").unwrap()),
                ),
        )
        .child(text("K9s Studio").font(k.ty.body_medium).color(c.text_strong))
        .child(Badge::new(breadcrumb).variant(BadgeVariant::Subtle).standard().build(k))
        .child(el().grow(1.0));

    if let Some(u) = &s.session {
        bar = bar
            .child(Badge::new(u.display_name.clone()).variant(BadgeVariant::Neutral).standard().build(k))
            .child(pill_btn("ออกจากระบบ", PillVariant::Glass, PillSize::Sm, |s: &mut AppState| s.logout()));
    }

    bar.child(
        row()
            .key("win-controls")
            .gap(6.0)
            .center_y()
            .child(pill_btn_tx("—", PillVariant::Glass, PillSize::Sm, |_: &mut AppState, tx: &mut Tx| tx.minimize()))
            .child(pill_btn_tx("□", PillVariant::Chrome, PillSize::Sm, |_: &mut AppState, tx: &mut Tx| tx.toggle_maximized()))
            .child(pill_btn_tx("✕", PillVariant::Danger, PillSize::Sm, |_: &mut AppState, tx: &mut Tx| tx.close_window())),
    )
}

pub fn app_shell(s: &AppState, k: &Kit) -> El {
    let c = &k.c;

    // True fullscreen diagram mode: hide titlebar, rail, subbar, and bottom dock completely.
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
        .fill(rune::color::hex("#080b12").unwrap())
        .child(custom_titlebar(s, k))
        .child(workspace_tab_bar(s, k));

    if let Some((msg, ok)) = &s.banner {
        let bg = if *ok { c.success.bg } else { c.danger.bg };
        let fg = if *ok { c.success.fg } else { c.danger.fg };
        root_col = root_col.child(
            row()
                .key("banner")
                .w_full()
                .px(20.0)
                .py(8.0)
                .fill(bg)
                .center_y()
                .gap(12.0)
                .child(text(msg.clone()).font(k.ty.text_small_medium).color(fg).grow(1.0))
                .child(pill_btn("ปิด", PillVariant::Glass, PillSize::Sm, |s: &mut AppState| s.banner = None)),
        );
    }

    if s.page != Page::Overview {
        if let Some(err) = &s.snapshot.error {
            root_col = root_col.child(
                row()
                    .key("k8s-warn")
                    .w_full()
                    .px(20.0)
                    .py(6.0)
                    .fill(c.warning.bg)
                    .center_y()
                    .gap(8.0)
                    .child(
                        text(format!("⚠ {err}"))
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
    let c = &k.c;
    let can_close = s.open_tabs.len() > 1;
    let mut tabs_row = row().gap(6.0).center_y();

    for wt in &s.open_tabs {
        let active = wt.id == s.active_tab_id;
        let id_switch = wt.id;
        let id_close = wt.id;
        let mut pill = row()
            .key(format!("wtab-{}", wt.id))
            .h(28.0)
            .px(12.0)
            .gap(8.0)
            .center_y()
            .radius(999.0)
            .fill(if active {
                rune::color::hex("#1e293b").unwrap()
            } else {
                Color::WHITE.alpha(0.04)
            })
            .border(
                1.2,
                if active {
                    rune::color::hex("#38bdf8").unwrap()
                } else {
                    Color::WHITE.alpha(0.10)
                },
            )
            .on_press(move |s: &mut AppState| s.switch_workspace_tab(id_switch))
            .child(
                text(wt.title.clone())
                    .font(k.ty.small_medium)
                    .color(if active { Color::WHITE } else { c.text_muted }),
            );

        if can_close {
            pill = pill.child(interactive(
                el().size(16.0, 16.0)
                    .radius(8.0)
                    .center()
                    .on_press(move |s: &mut AppState| s.close_workspace_tab(id_close))
                    .child(text("✕").font(rune::TextStyle::new(9.5, 700.0)).color(c.text_muted)),
            ));
        }
        tabs_row = tabs_row.child(interactive(pill));
    }

    row()
        .key("workspace-tab-bar")
        .w_full()
        .h(38.0)
        .px(16.0)
        .gap(8.0)
        .center_y()
        .fill(rune::color::hex("#0c101a").unwrap())
        .border(1.0, c.border_weak)
        .child(tabs_row)
        .child(pill_btn("+ เปิด Tab ใหม่", PillVariant::Glass, PillSize::Sm, |s: &mut AppState| {
            s.new_workspace_tab();
        }))
}

fn namespace_subbar(s: &AppState, k: &Kit) -> El {
    let c = &k.c;
    let tab_btn = |label: &'static str, target: Tab| {
        let active = s.tab == target;
        pill_btn(
            label,
            if active { PillVariant::Chrome } else { PillVariant::Glass },
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

    row()
        .key("ns-subbar")
        .w_full()
        .h(52.0)
        .px(20.0)
        .gap(12.0)
        .center_y()
        .fill(c.layer_01)
        .border(1.0, c.border_weak)
        .child(pill_btn("← Overview", PillVariant::Glass, PillSize::Sm, |s: &mut AppState| s.open_overview()))
        .child(el().size(10.0, 10.0).radius(5.0).fill(env_color(s.env)))
        .child(text(format!("{} / {}", s.env.label(), s.namespace)).font(k.ty.body_medium).color(c.text_strong))
        .child(segmented_group([
            tab_btn("🗺 Stack Diagram", Tab::Topology),
            tab_btn("📦 Services", Tab::Services),
            tab_btn("⌨ K9s CLI", Tab::K9sCli),
        ]))
        .child(el().grow(1.0))
        .child(pill_btn(
            if s.loading { "⏳ กำลังโหลด..." } else { "↻ Refresh" },
            PillVariant::Chrome,
            PillSize::Sm,
            |s: &mut AppState| s.refresh_cluster(),
        ))
}

fn bottom_dock(s: &AppState, k: &Kit) -> El {
    let c = &k.c;

    let mut ns_pills = row().gap(6.0).center_y();
    for (env, list) in [(EnvKind::Stg, &s.stg_namespaces), (EnvKind::Prod, &s.prod_namespaces)] {
        ns_pills = ns_pills.child(
            Badge::new(env.label())
                .variant(if env == EnvKind::Prod { BadgeVariant::Accent } else { BadgeVariant::Neutral })
                .build(k),
        );
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
                    .h(30.0)
                    .gap(6.0)
                    .center_y()
                    .radius(999.0)
                    .fill(if active { rune::color::hex("#1e293b").unwrap() } else { Color::WHITE.alpha(0.05) })
                    .border(
                        if has_issue { 1.6 } else { 1.25 },
                        if has_issue {
                            blink_col
                        } else if active {
                            rune::color::hex("#38bdf8").unwrap()
                        } else {
                            Color::WHITE.alpha(0.12)
                        },
                    )
                    .motion(motion::SNAPPY)
                    .on_press(move |s: &mut AppState| s.drill_into_namespace(env, &ns_owned))
                    .child(el().size(7.0, 7.0).radius(4.0).fill(blink_col))
                    .child(text(ns.clone()).font(k.ty.small_medium).color(if active { Color::WHITE } else { c.text_base })),
            ));
        }
    }

    row()
        .key("bottom-dock")
        .w_full()
        .h(54.0)
        .px(16.0)
        .gap(10.0)
        .center_y()
        .fill(c.bg_base)
        .border(1.0, c.border_base)
        .child(pill_btn(
            "🏠 Overview",
            if s.page == Page::Overview { PillVariant::Primary } else { PillVariant::Chrome },
            PillSize::Sm,
            |s: &mut AppState| s.open_overview(),
        ))
        .child(ns_pills)
        .child(el().grow(1.0))
        .child(pill_btn("⌨ K9s CLI", PillVariant::Chrome, PillSize::Sm, |s: &mut AppState| {
            s.page = Page::Namespace;
            s.tab = Tab::K9sCli;
            if !s.k9s_pty.is_running() {
                s.start_k9s();
            }
        }))
}
