use crate::k8s::{EnvKind, Health};
use crate::state::{AppState, ServiceTab};
use crate::ui::icons;
use crate::ui::pill::{PillSize, PillVariant, pill_icon_btn, soft_badge};
use crate::ui::{alert_blink_color, health_color, theme};
use rune::{Color, El, ScrollSpec, col, el, row, text};
use rune::tok::motion;
use rune_kit::{Kit, TextInput};

pub fn view(s: &AppState, k: &Kit) -> El {
    let query = s.search.text.trim().to_ascii_lowercase();

    let total = s.snapshot.services.len();
    let healthy = s
        .snapshot
        .services
        .iter()
        .filter(|x| x.health == Health::Healthy)
        .count();
    let issues = total.saturating_sub(healthy);

    let stats_row = row()
        .key("svc-stats")
        .w_full()
        .gap(12.0)
        .center_y()
        .child(stat_card(
            crate::tr!("Services ทั้งหมด", "All services"),
            total.to_string(),
            theme::text_strong(),
        ))
        .child(stat_card(
            crate::tr!("ทำงานปกติ (Healthy)", "Healthy"),
            healthy.to_string(),
            health_color(Health::Healthy),
        ))
        .child(stat_card(
            crate::tr!("ต้องตรวจสอบ (Issues)", "Issues"),
            issues.to_string(),
            health_color(if issues > 0 {
                Health::Warning
            } else {
                Health::Healthy
            }),
        ))
        .child(el().grow(1.0))
        .child(
            TextInput::new(&s.search)
                .width(Some(280.0))
                .placeholder(crate::tr!("ค้นหาชื่อ service เช่น king-maker...", "Search services, e.g. king-maker..."))
                .on_edit(|s: &mut AppState| &mut s.search)
                .build(k),
        );

    let mut list = col().key("svc-list").gap(10.0);
    for svc in &s.snapshot.services {
        if !query.is_empty()
            && !svc.name.to_ascii_lowercase().contains(&query)
            && !svc
                .hosts
                .iter()
                .any(|h| h.to_ascii_lowercase().contains(&query))
        {
            continue;
        }

        let svc_name_card = svc.name.clone();
        let svc_name_env = svc.name.clone();
        let svc_name_log = svc.name.clone();

        let mut pod_btns = row().gap(6.0).center_y();
        pod_btns = pod_btns
            .child(pill_icon_btn(
                &icons::ARROW_RIGHT,
                crate::tr!("เจาะลึก", "Details"),
                PillVariant::Primary,
                PillSize::Sm,
                move |s: &mut AppState| {
                    s.drill_into_service(svc_name_card.clone(), ServiceTab::Info)
                },
            ))
            .child(pill_icon_btn(
                &icons::SLIDERS,
                "ENV",
                PillVariant::Glass,
                PillSize::Sm,
                move |s: &mut AppState| {
                    s.drill_into_service(svc_name_env.clone(), ServiceTab::Env)
                },
            ))
            .child(pill_icon_btn(
                &icons::LOGS,
                "Logs",
                PillVariant::Glass,
                PillSize::Sm,
                move |s: &mut AppState| {
                    s.drill_into_service(svc_name_log.clone(), ServiceTab::Logs)
                },
            ));

        if let Some(deploy) = &svc.deploy_name {
            let deploy_name = deploy.clone();
            pod_btns = pod_btns.child(pill_icon_btn(
                &icons::REFRESH,
                "Restart",
                if s.env == EnvKind::Prod {
                    PillVariant::Danger
                } else {
                    PillVariant::Gold
                },
                PillSize::Sm,
                move |s: &mut AppState| s.restart_deploy(deploy_name.clone()),
            ));
        }

        let has_issue = svc.health != Health::Healthy;
        let blink_col = alert_blink_color(svc.health, s.pulse_on);

        let card = row()
            .w_full()
            .padding(16.0)
            .gap(12.0)
            .center_y()
            .radius(20.0)
            .fill(if has_issue && s.pulse_on {
                theme::err_card_bg()
            } else {
                theme::glass_card()
            })
            .border(
                if has_issue { 1.4 } else { 1.0 },
                if has_issue {
                    blink_col
                } else {
                    theme::border_glass()
                },
            )
            .motion(motion::SNAPPY)
            .child(el().size(8.0, 8.0).radius(4.0).fill(blink_col))
            .child(
                col()
                    .gap(4.0)
                    .grow(1.0)
                    .shrink()
                    .min_w(0.0)
                    .child(
                        row()
                            .gap(8.0)
                            .center_y()
                            .child(
                                text(svc.name.clone())
                                    .font(k.ty.body_medium)
                                    .color(theme::text_strong()),
                            )
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
            .child(pod_btns);

        list = list.child(card);
    }

    col()
        .key("svc-main")
        .w_full()
        .h_full()
        .padding(20.0)
        .gap(16.0)
        .child(stats_row)
        .child(
            list.scroll(ScrollSpec { stick_bottom: false })
                .grow(1.0)
                .shrink()
                .min_h(0.0),
        )
}

fn stat_card(label: &'static str, value: String, val_color: Color) -> El {
    col()
        .px(14.0)
        .py(11.0)
        .gap(2.0)
        .radius(16.0)
        .fill(theme::metric_box_bg())
        .border(1.0, theme::border_subtle())
        .child(text(label).font(theme::label_xs()).color(theme::text_muted()))
        .child(text(value).font(theme::stat_val()).color(val_color))
}
