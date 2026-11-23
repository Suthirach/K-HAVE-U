use crate::k8s::{EnvKind, Health};
use crate::state::{AppState, ServiceTab};
use crate::ui::{alert_blink_color, health_color};
use crate::ui::pill::{PillSize, PillVariant, pill_btn};
use rune::{El, ScrollSpec, col, el, row, text};
use rune::tok::motion;
use rune_kit::{Badge, BadgeVariant, Kit, TextInput};

pub fn view(s: &AppState, k: &Kit) -> El {
    let c = &k.c;
    let query = s.search.text.trim().to_ascii_lowercase();

    let total = s.snapshot.services.len();
    let healthy = s.snapshot.services.iter().filter(|x| x.health == Health::Healthy).count();
    let issues = total.saturating_sub(healthy);

    let stats_row = row()
        .key("svc-stats")
        .w_full()
        .gap(12.0)
        .center_y()
        .child(stat_card("Services ทั้งหมด", total.to_string(), c.text_strong, k))
        .child(stat_card("ทำงานปกติ (Healthy)", healthy.to_string(), health_color(Health::Healthy), k))
        .child(stat_card("ต้องตรวจสอบ (Issues)", issues.to_string(), health_color(if issues > 0 { Health::Warning } else { Health::Healthy }), k))
        .child(el().grow(1.0))
        .child(
            TextInput::new(&s.search)
                .width(Some(280.0))
                .placeholder("ค้นหาชื่อ service เช่น king-maker...")
                .on_edit(|s: &mut AppState| &mut s.search)
                .build(k),
        );

    let mut list = col().key("svc-list").gap(8.0);
    for svc in &s.snapshot.services {
        if !query.is_empty()
            && !svc.name.to_ascii_lowercase().contains(&query)
            && !svc.hosts.iter().any(|h| h.to_ascii_lowercase().contains(&query))
        {
            continue;
        }

        let svc_name_card = svc.name.clone();
        let svc_name_env = svc.name.clone();
        let svc_name_log = svc.name.clone();

        let mut pod_btns = row().gap(6.0);
        pod_btns = pod_btns
            .child(pill_btn("🔍 เจาะลึก", PillVariant::Primary, PillSize::Sm, move |s: &mut AppState| {
                s.drill_into_service(svc_name_card.clone(), ServiceTab::Info)
            }))
            .child(pill_btn("🔐 ENV", PillVariant::Chrome, PillSize::Sm, move |s: &mut AppState| {
                s.drill_into_service(svc_name_env.clone(), ServiceTab::Env)
            }))
            .child(pill_btn("📄 Log", PillVariant::Glass, PillSize::Sm, move |s: &mut AppState| {
                s.drill_into_service(svc_name_log.clone(), ServiceTab::Logs)
            }));

        if let Some(deploy) = &svc.deploy_name {
            let deploy_name = deploy.clone();
            pod_btns = pod_btns.child(pill_btn(
                "↻ Restart",
                if s.env == EnvKind::Prod { PillVariant::Danger } else { PillVariant::Gold },
                PillSize::Sm,
                move |s: &mut AppState| s.restart_deploy(deploy_name.clone()),
            ));
        }

        let has_issue = svc.health != Health::Healthy;
        let blink_col = alert_blink_color(svc.health, s.pulse_on);

        let card = row()
            .w_full()
            .padding(14.0)
            .gap(12.0)
            .center_y()
            .radius(k.radius.lg)
            .fill(if has_issue && s.pulse_on {
                rune::color::hex("#2a0f18").unwrap()
            } else {
                c.bg_base
            })
            .border(if has_issue { 1.8 } else { 1.0 }, if has_issue { blink_col } else { c.border_weak })
            .motion(motion::SNAPPY)
            .child(el().size(10.0, 10.0).radius(5.0).fill(blink_col))
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
                            .child(text(svc.name.clone()).font(k.ty.body_medium).color(c.text_strong))
                            .child(Badge::new(svc.role.label()).variant(BadgeVariant::Subtle).build(k))
                            .child(
                                Badge::new(format!("{}/{}", svc.ready_replicas, svc.desired_replicas))
                                    .variant(BadgeVariant::Neutral)
                                    .build(k),
                            ),
                    )
                    .child(text(svc.summary.clone()).font(k.ty.text_small).color(c.text_muted)),
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
        .child(list.scroll(ScrollSpec { stick_bottom: false }).grow(1.0).shrink().min_h(0.0))
}

fn stat_card(label: &'static str, value: String, val_color: rune::Color, k: &Kit) -> El {
    let c = &k.c;
    col()
        .padding(12.0)
        .gap(2.0)
        .radius(k.radius.md)
        .fill(c.bg_base)
        .border(1.0, c.border_weak)
        .child(text(label).font(k.ty.small_medium).color(c.text_muted))
        .child(text(value).font(rune::TextStyle::new(20.0, 600.0)).color(val_color))
}
