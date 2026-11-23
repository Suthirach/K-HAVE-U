use crate::k8s::{EnvKind, Health, NamespaceSummary};
use crate::state::AppState;
use crate::ui::{alert_blink_color, env_color, health_color};
use crate::ui::pill::{PillSize, PillVariant, interactive, pill_btn};
use rune::{El, ScrollSpec, Spring, col, el, row, text};
use rune::tok::motion;
use rune_kit::{Badge, BadgeVariant, Kit, TextInput};

pub fn view(s: &AppState, k: &Kit) -> El {
    let c = &k.c;

    let stg_items: Vec<&NamespaceSummary> = s.overview.iter().filter(|n| n.env == EnvKind::Stg).collect();
    let prod_items: Vec<&NamespaceSummary> = s.overview.iter().filter(|n| n.env == EnvKind::Prod).collect();

    let total_svc: usize = s.overview.iter().map(|n| n.total_services).sum();
    let total_ok: usize = s.overview.iter().map(|n| n.healthy).sum();
    let total_warn: usize = s.overview.iter().map(|n| n.warning + n.error).sum();

    let hero = row()
        .key("overview-hero")
        .w_full()
        .gap(14.0)
        .child(hero_stat("Namespaces ทั้งหมด", format!("{}", s.overview.len()), "STG + PROD", c.text_strong, k))
        .child(hero_stat("Services ทั้งหมด", format!("{total_svc}"), "พร้อมทำงานใน Cluster", c.text_accent, k))
        .child(hero_stat("Healthy (ปกติ)", format!("{total_ok}"), "พร้อมรับ Traffic", health_color(Health::Healthy), k))
        .child(hero_stat(
            "ต้องตรวจสอบ",
            format!("{total_warn}"),
            if total_warn == 0 { "ทุกระบบปกติ" } else { "มีบางตัวรีสตาร์ท/เตือน" },
            health_color(if total_warn > 0 { Health::Warning } else { Health::Healthy }),
            k,
        ));

    let content = col()
        .key("overview-content")
        .w_full()
        .padding(22.0)
        .gap(20.0)
        .child(
            row()
                .w_full()
                .center_y()
                .child(
                    col()
                        .gap(4.0)
                        .grow(1.0)
                        .child(
                            text("Cluster Overview — ภาพรวมทุก Namespace")
                                .font(rune::TextStyle::new(20.0, 600.0))
                                .color(c.text_strong),
                        )
                        .child(
                            text("เลือก Namespace ด้านล่าง หรือคลิกที่การ์ดเพื่อเจาะลึกดู Stack Diagram, Service, ENV และ Logs")
                                .font(k.ty.text_small)
                                .color(c.text_muted),
                        ),
                )
                .child(
                    row()
                        .gap(8.0)
                        .child(pill_btn(
                            "+ เพิ่ม Namespace",
                            PillVariant::Primary,
                            PillSize::Md,
                            |s: &mut AppState| s.open_add_namespace(),
                        ))
                        .child(pill_btn(
                            if s.overview_loading { "⏳ กำลังอัปเดต..." } else { "↻ อัปเดตภาพรวม" },
                            PillVariant::Chrome,
                            PillSize::Md,
                            |s: &mut AppState| s.refresh_overview(),
                        )),
                ),
        )
        .children(if s.adding_ns { Some(add_namespace_panel(s, k)) } else { None })
        .child(hero)
        .child(env_section("Staging Environments (STG)", EnvKind::Stg, &stg_items, s.pulse_on, k))
        .child(env_section("Production Environments (PROD)", EnvKind::Prod, &prod_items, s.pulse_on, k));

    content.scroll(ScrollSpec { stick_bottom: false }).w_full().h_full()
}

fn hero_stat(title: &'static str, value: String, sub: &'static str, color: rune::Color, k: &Kit) -> El {
    let c = &k.c;
    col()
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .padding(16.0)
        .gap(4.0)
        .radius(k.radius.lg)
        .fill(c.bg_base)
        .border(1.0, c.border_weak)
        .motion(motion::SNAPPY)
        .child(text(title).font(k.ty.small_medium).color(c.text_muted))
        .child(text(value).font(rune::TextStyle::new(26.0, 700.0)).color(color))
        .child(text(sub).font(k.ty.small).color(c.text_faint))
}

fn env_section(title: &'static str, env: EnvKind, items: &[&NamespaceSummary], pulse_on: bool, k: &Kit) -> El {
    let c = &k.c;
    let mut grid = row().w_full().gap(14.0);
    for item in items {
        grid = grid.child(namespace_card(item, pulse_on, k));
    }

    col()
        .w_full()
        .gap(10.0)
        .child(
            row()
                .gap(8.0)
                .center_y()
                .child(el().size(10.0, 10.0).radius(5.0).fill(env_color(env)))
                .child(text(title).font(k.ty.body_medium).color(c.text_strong))
                .child(Badge::new(env.label()).variant(if env == EnvKind::Prod { BadgeVariant::Accent } else { BadgeVariant::Neutral }).build(k)),
        )
        .child(grid)
}

fn namespace_card(ns: &NamespaceSummary, pulse_on: bool, k: &Kit) -> El {
    let c = &k.c;
    let env = ns.env;
    let ns_name = ns.namespace.clone();
    let total = ns.total_services.max(1) as f32;
    let healthy_ratio = (ns.healthy as f32 / total).clamp(0.0, 1.0);
    let warn_ratio = ((ns.warning + ns.error) as f32 / total).clamp(0.0, 1.0);

    let overall_health = if ns.error > 0 {
        Health::Error
    } else if ns.warning > 0 {
        Health::Warning
    } else {
        Health::Healthy
    };
    let has_issue = overall_health != Health::Healthy;
    let blink_col = alert_blink_color(overall_health, pulse_on);

    // Animated visual health bar
    let bar = row()
        .w_full()
        .h(8.0)
        .radius(4.0)
        .fill(c.layer_03)
        .clip_rounded(4.0)
        .child(
            el().h(8.0)
                .w(240.0 * healthy_ratio)
                .fill(health_color(Health::Healthy))
                .motion(Spring::new(0.45, 0.22)),
        )
        .child(
            el().h(8.0)
                .w(240.0 * warn_ratio)
                .fill(blink_col)
                .motion(Spring::new(0.45, 0.22)),
        );

    interactive(
        col()
            .key(format!("ns-card-{}", ns.namespace))
            .grow(1.0)
            .shrink()
            .min_w(0.0)
            .padding(18.0)
            .gap(12.0)
            .radius(k.radius.lg)
            .fill(if has_issue && pulse_on {
                rune::color::hex("#2a0f18").unwrap()
            } else {
                rune::color::hex("#111624").unwrap()
            })
            .border(
                if has_issue { 1.8 } else { 1.0 },
                if has_issue {
                    blink_col
                } else {
                    rune::color::hex("#6366f1").unwrap().alpha(0.32)
                },
            )
            .motion(motion::SNAPPY)
            .on_press(move |s: &mut AppState| s.drill_into_namespace(env, &ns_name))
            .child(
                row()
                    .w_full()
                    .center_y()
                    .gap(8.0)
                    .child(el().size(10.0, 10.0).radius(5.0).fill(blink_col))
                    .child(text(ns.namespace.clone()).font(rune::TextStyle::new(16.0, 600.0)).color(c.text_strong).grow(1.0))
                    .child(
                        Badge::new(if ns.is_demo { "DEMO" } else { "LIVE" })
                            .variant(BadgeVariant::Subtle)
                            .build(k),
                    ),
            )
            .child(bar)
            .child(
                row()
                    .w_full()
                    .gap(12.0)
                    .child(mini_metric("Services", format!("{}", ns.total_services), c.text_strong, k))
                    .child(mini_metric("Healthy", format!("{}", ns.healthy), health_color(Health::Healthy), k))
                    .child(mini_metric("Issues", format!("{}", ns.warning + ns.error), health_color(Health::Warning), k))
                    .child(mini_metric("Ingress", format!("{}", ns.ingress_count), c.text_accent, k)),
            )
            .child(
                row()
                    .w_full()
                    .center_y()
                    .child(text("คลิกเพื่อเจาะลึก Services, Diagram, ENV & Logs").font(k.ty.small).color(c.text_muted).grow(1.0))
                    .child(text("เจาะลึก →").font(k.ty.small_medium).color(c.text_accent)),
            ),
    )
}

fn add_namespace_panel(s: &AppState, k: &Kit) -> El {
    let c = &k.c;
    let mut avail_row = row().gap(6.0).center_y();
    for ns in s.available_namespaces.iter().take(8) {
        let already = s.stg_namespaces.contains(ns) || s.prod_namespaces.contains(ns);
        if already {
            continue;
        }
        let ns_click = ns.clone();
        let target_env = s.new_ns_env;
        avail_row = avail_row.child(pill_btn(
            format!("+ {ns}"),
            PillVariant::Glass,
            PillSize::Sm,
            move |s: &mut AppState| s.add_namespace(target_env, ns_click.clone()),
        ));
    }

    col()
        .key("add-ns-panel")
        .w_full()
        .padding(16.0)
        .gap(10.0)
        .radius(k.radius.lg)
        .fill(rune::color::hex("#131a2a").unwrap())
        .border(1.25, rune::color::hex("#38bdf8").unwrap())
        .child(
            row()
                .center_y()
                .gap(8.0)
                .child(text("เพิ่ม Namespace เข้า Overview & Dock ด้านล่าง").font(k.ty.body_medium).color(c.text_strong).grow(1.0))
                .child(pill_btn(
                    "STG",
                    if s.new_ns_env == EnvKind::Stg { PillVariant::Primary } else { PillVariant::Glass },
                    PillSize::Sm,
                    |s: &mut AppState| s.new_ns_env = EnvKind::Stg,
                ))
                .child(pill_btn(
                    "PROD",
                    if s.new_ns_env == EnvKind::Prod { PillVariant::Danger } else { PillVariant::Glass },
                    PillSize::Sm,
                    |s: &mut AppState| s.new_ns_env = EnvKind::Prod,
                )),
        )
        .child(
            row()
                .gap(8.0)
                .center_y()
                .child(
                    TextInput::new(&s.new_ns_input)
                        .width(Some(280.0))
                        .placeholder("พิมพ์ชื่อ namespace เช่น finance หรือ monitor")
                        .on_edit(|s: &mut AppState| &mut s.new_ns_input)
                        .on_submit(|s: &mut AppState, _| {
                            let val = s.new_ns_input.text.clone();
                            s.add_namespace(s.new_ns_env, val);
                        })
                        .build(k),
                )
                .child(pill_btn("เพิ่ม", PillVariant::Primary, PillSize::Sm, |s: &mut AppState| {
                    let val = s.new_ns_input.text.clone();
                    s.add_namespace(s.new_ns_env, val);
                }))
                .child(pill_btn("ปิด", PillVariant::Glass, PillSize::Sm, |s: &mut AppState| s.adding_ns = false)),
        )
        .child(
            row()
                .gap(8.0)
                .center_y()
                .child(text("ใน Cluster:").font(k.ty.small_medium).color(c.text_muted))
                .child(avail_row),
        )
}

fn mini_metric(label: &'static str, val: String, color: rune::Color, k: &Kit) -> El {
    col()
        .gap(2.0)
        .child(text(label).font(k.ty.small).color(k.c.text_faint))
        .child(text(val).font(k.ty.body_medium).color(color))
}
