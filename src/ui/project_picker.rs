use crate::state::AppState;
use crate::ui::icons;
use crate::ui::pill::{PillSize, PillVariant, pill_btn, pill_icon_btn, soft_badge};
use crate::ui::theme;
use rune::{Color, El, ScrollSpec, Shadow, col, el, row, text};
use rune_kit::Kit;

/// Up to this many rows are laid out directly; longer lists get a scroll area.
const MAX_PLAIN_ROWS: usize = 5;

fn note(msg: impl Into<String>, k: &Kit) -> El {
    text(msg.into())
        .font(k.ty.text_small)
        .color(theme::text_muted())
}

/// Shown right after login: pick a GCP project, then a GKE cluster to connect kubectl to.
pub fn view(s: &AppState, k: &Kit) -> El {
    let c = &k.c;

    let (title, subtitle) = match &s.selected_project {
        None => (
            "เลือก Project",
            "เลือก GCP Project ที่ต้องการดู แล้วเลือก GKE cluster ในขั้นตอนถัดไป".to_string(),
        ),
        Some(p) => (
            "เลือก Cluster",
            format!("Project: {} ({})", p.name, p.id),
        ),
    };

    let mut header = row()
        .gap(10.0)
        .center_y()
        .child(crate::ui::app_icon())
        .child(
            text(title)
                .font(theme::heading_xl())
                .color(theme::text_strong()),
        );
    if let Some(user) = &s.session {
        header = header.child(soft_badge(
            user.display_name.clone(),
            theme::metric_box_soft(),
            theme::text_muted(),
            Some(theme::border_subtle()),
        ));
    }

    let mut list = col().gap(8.0).w_full();
    let rows: usize;

    if s.selected_project.is_none() {
        rows = s.projects.len();
        if s.projects_loading {
            list = list.child(note("กำลังโหลดรายชื่อ Project จาก gcloud...", k));
        } else if s.projects.is_empty() && s.gcp_error.is_none() {
            list = list.child(note("ไม่พบ Project ในบัญชีนี้", k));
        }
        for p in &s.projects {
            let picked = p.clone();
            list = list.child(pill_btn(
                format!("{}  ({})", p.name, p.id),
                PillVariant::Glass,
                PillSize::Lg,
                move |s: &mut AppState| s.select_project(picked.clone()),
            ));
        }
    } else {
        rows = s.clusters.len();
        if s.clusters_loading {
            list = list.child(note("กำลังโหลดรายชื่อ GKE cluster...", k));
        } else if s.clusters.is_empty() && s.gcp_error.is_none() {
            list = list.child(note(
                "ไม่พบ GKE cluster ใน Project นี้ (ตรวจว่าเปิด Kubernetes Engine API แล้ว)",
                k,
            ));
        }
        for cl in &s.clusters {
            let picked = cl.clone();
            let busy = s.connecting_cluster.as_deref() == Some(cl.name.as_str());
            let label = if busy {
                format!("กำลังเชื่อมต่อ {} ...", cl.name)
            } else {
                format!("{}  ·  {}  ·  {}", cl.name, cl.location, cl.status)
            };
            list = list.child(pill_btn(
                label,
                PillVariant::Glass,
                PillSize::Lg,
                move |s: &mut AppState| s.connect_cluster(picked.clone()),
            ));
        }
    }

    let mut card = col()
        .key("project-card")
        .w(520.0)
        .padding(28.0)
        .gap(16.0)
        .radius(22.0)
        .fill(theme::glass_card())
        .border(1.0, theme::border_glass())
        .box_shadow(Shadow {
            x: 0.0,
            y: 20.0,
            blur: 48.0,
            spread: 0.0,
            color: Color::BLACK.alpha(0.45),
            inset: false,
        })
        .child(col().gap(6.0).child(header).child(note(subtitle, k)))
        .child(if rows > MAX_PLAIN_ROWS {
            // A scroll viewport clips its children, so pad it to leave room for the pills' hover "pop".
            list.padding(16.0)
                .scroll(ScrollSpec { stick_bottom: false })
                .w_full()
                .h(340.0)
        } else {
            list
        });

    if let Some(err) = &s.gcp_error {
        card = card.child(
            row()
                .padding(10.0)
                .gap(8.0)
                .center_y()
                .radius(14.0)
                .fill(c.danger.bg)
                .border(1.0, c.danger.border)
                .child(icons::mono_icon(&icons::WARNING, 14.0, c.danger.fg))
                .child(
                    text(err.clone())
                        .font(k.ty.text_small_medium)
                        .color(c.danger.fg),
                ),
        );
    }

    if let Some(panel) = crate::ui::tools_panel::view(s, k) {
        card = card.child(panel);
    }

    let mut actions = row().gap(8.0).center_y();
    if s.selected_project.is_some() {
        actions = actions.child(pill_icon_btn(
            &icons::ARROW_LEFT,
            "ย้อนกลับ",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.back_to_projects(),
        ));
    } else {
        actions = actions.child(pill_icon_btn(
            &icons::REFRESH,
            "โหลดใหม่",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.load_projects(),
        ));
    }
    actions = actions
        .child(el().grow(1.0))
        .child(pill_btn(
            "ข้าม (ใช้ kubectl context ปัจจุบัน)",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.skip_project_picker(),
        ));
    card = card.child(actions);

    col()
        .key("project-page")
        .w_full()
        .h_full()
        .fill(theme::bg_canvas())
        .radial_gradient(
            [0.16, 0.10],
            [680.0, 380.0],
            [(0.0, theme::glow_warm()), (0.65, Color::TRANSPARENT)],
        )
        .child(crate::ui::custom_titlebar(s, k))
        .child(el().grow(1.0).w_full().center().child(card))
}
