use crate::state::AppState;
use crate::ui::icons;
use crate::ui::pill::{PillSize, PillVariant, pill_icon_btn, soft_badge};
use crate::ui::theme;
use rune::{Color, El, Shadow, col, el, row, text};
use rune_kit::Kit;

fn step_title(label: &'static str, k: &Kit) -> El {
    text(label)
        .font(k.ty.text_small_medium)
        .color(theme::text_base())
}

/// First launch only: paste the update key, install missing tools, then go on to sign-in.
pub fn view(s: &AppState, k: &Kit) -> El {
    let tools_state: El = if let Some(panel) = crate::ui::tools_panel::view(s, k) {
        panel
    } else if s.tools_checked {
        text("พร้อมแล้ว: พบ kubectl, k9s และ gcloud ครบ")
            .font(k.ty.small_medium)
            .color(theme::ok_fg())
    } else {
        text("กำลังตรวจสอบเครื่องมือ...")
            .font(k.ty.small_medium)
            .color(theme::text_muted())
    };

    let card = col()
        .key("setup-card")
        .w(500.0)
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
        .child(
            col()
                .gap(6.0)
                .child(
                    row()
                        .gap(10.0)
                        .center_y()
                        .child(crate::ui::app_icon())
                        .child(
                            text("ตั้งค่าเริ่มต้น")
                                .font(theme::heading_xl())
                                .color(theme::text_strong()),
                        )
                        .child(soft_badge(
                            "ครั้งแรกเท่านั้น",
                            theme::metric_box_soft(),
                            theme::text_muted(),
                            Some(theme::border_subtle()),
                        )),
                )
                .child(
                    text("ทำสองขั้นตอนนี้ครั้งเดียว ข้ามได้ทั้งคู่ แล้วค่อยเข้าสู่ระบบ")
                        .font(k.ty.text_small)
                        .color(theme::text_muted()),
                ),
        )
        .child(step_title("1. Key สำหรับอัปเดตแอป", k))
        .child(crate::ui::update_panel::view(s, k))
        .child(step_title("2. เครื่องมือที่ต้องใช้", k))
        .child(tools_state)
        .child(pill_icon_btn(
            &icons::ARROW_RIGHT,
            "ถัดไป: เข้าสู่ระบบ",
            PillVariant::Chrome,
            PillSize::Lg,
            |s: &mut AppState| s.finish_setup(),
        ));

    col()
        .key("setup-page")
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
