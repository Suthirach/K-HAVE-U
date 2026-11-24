use crate::state::AppState;
use crate::ui::icons;
use crate::ui::pill::{PillSize, PillVariant, pill_btn, pill_icon_btn, soft_badge};
use crate::ui::theme;
use rune::{Color, El, Shadow, col, el, row, text};
use rune_kit::{Kit, TextInput};

const CARD_W: f32 = 500.0;
const CARD_PAD: f32 = 28.0;

fn short(msg: &str) -> String {
    let line = msg.lines().next().unwrap_or("").trim();
    if line.chars().count() > 90 {
        let cut: String = line.chars().take(90).collect();
        format!("{cut}…")
    } else {
        line.to_string()
    }
}

fn card() -> El {
    col()
        .key("setup-card")
        .w(CARD_W)
        .padding(CARD_PAD)
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
}

fn header(title: &'static str, subtitle: &'static str, k: &Kit) -> El {
    col()
        .gap(6.0)
        .child(
            row()
                .gap(10.0)
                .center_y()
                .child(crate::ui::app_icon())
                .child(
                    text(title)
                        .font(theme::heading_xl())
                        .color(theme::text_strong()),
                ),
        )
        .child(
            text(subtitle)
                .font(k.ty.text_small)
                .color(theme::text_muted()),
        )
}

/// Loading screen while tools download and install: percentage + bar + current step.
fn loading(s: &AppState, k: &Kit) -> El {
    let bar_w = CARD_W - CARD_PAD * 2.0;
    let frac = (s.tools_percent / 100.0).clamp(0.0, 1.0);
    let message = if s.tools_installing {
        short(&s.tools_log)
    } else {
        crate::tr!("กำลังตรวจสอบเครื่องมือ...", "Checking tools...").to_string()
    };
    card()
        .child(header(
            crate::tr!("กำลังติดตั้ง", "Installing"),
            crate::tr!("ไม่ต้องกดอะไร เสร็จแล้วจะไปหน้าเข้าสู่ระบบเอง", "Nothing to do here; you'll go to sign-in when it's done"),
            k,
        ))
        .child(
            text(format!("{:.0}%", s.tools_percent))
                .font(theme::stat_val())
                .color(theme::text_strong()),
        )
        .child(
            row()
                .w(bar_w)
                .h(10.0)
                .radius(999.0)
                .fill(theme::metric_box_bg())
                .border(1.0, theme::border_subtle())
                .child(
                    el().w((bar_w * frac).max(10.0))
                        .h(10.0)
                        .radius(999.0)
                        .fill(theme::accent_primary_bg()),
                ),
        )
        .child(
            text(message)
                .font(k.ty.small_medium)
                .color(theme::text_muted()),
        )
}

/// First launch only: paste the update key and press OK; missing tools install automatically.
fn form(s: &AppState, k: &Kit) -> El {
    let c = &k.c;
    let tools_line: El = if !s.tools_checked {
        text(crate::tr!("กำลังตรวจสอบเครื่องมือ...", "Checking tools..."))
            .font(k.ty.small_medium)
            .color(theme::text_muted())
    } else if s.tools_missing.is_empty() {
        text(crate::tr!("พร้อมแล้ว: พบ kubectl, k9s และ gcloud ครบ", "Ready: kubectl, k9s and gcloud found"))
            .font(k.ty.small_medium)
            .color(theme::ok_fg())
    } else {
        text(crate::tf!(
            "จะติดตั้งให้อัตโนมัติเมื่อกดตกลง: {}", "Will be installed when you press OK: {}",
            s.tools_missing.join(", ")
        ))
        .font(k.ty.small_medium)
        .color(theme::warn_fg())
    };

    let placeholder = if s.update_token.is_some() {
        crate::tr!("มี key แล้ว (วางอันใหม่เพื่อเปลี่ยน หรือเว้นว่าง)", "Key saved (paste a new one to replace it, or leave empty)")
    } else {
        crate::tr!("วาง GitHub token (ไม่บังคับ)", "Paste a GitHub token (optional)")
    };

    let mut body = card()
        .child(
            row()
                .gap(10.0)
                .center_y()
                .child(header(
                    crate::tr!("ตั้งค่าเริ่มต้น", "Setup"),
                    crate::tr!("วาง key แล้วกดตกลง ที่เหลือระบบทำให้เอง", "Paste the key and press OK; the rest is automatic"),
                    k,
                ))
                .child(el().grow(1.0))
                .child(soft_badge(
                    crate::tr!("ครั้งแรกเท่านั้น", "First run only"),
                    theme::metric_box_soft(),
                    theme::text_muted(),
                    Some(theme::border_subtle()),
                )),
        )
        .child(
            col()
                .gap(6.0)
                .child(
                    text(crate::tr!("Key สำหรับอัปเดตแอป", "App update key"))
                        .font(k.ty.text_small_medium)
                        .color(theme::text_base()),
                )
                .child(
                    TextInput::new(&s.update_token_input)
                        .large()
                        .width(None)
                        .placeholder(placeholder)
                        .on_edit(|s: &mut AppState| &mut s.update_token_input)
                        .on_submit(|s: &mut AppState, _| s.setup_confirm())
                        .build(k),
                ),
        )
        .child(tools_line);

    if let Some(err) = &s.tools_error {
        body = body.child(
            text(crate::tf!("ติดตั้งไม่สำเร็จ: {}", "Install failed: {}", short(err)))
                .font(k.ty.small_medium)
                .color(c.danger.fg),
        );
    }

    body = body.child(pill_icon_btn(
        &icons::ARROW_RIGHT,
        if s.tools_error.is_some() { crate::tr!("ลองใหม่", "Try again") } else { crate::tr!("ตกลง", "OK") },
        PillVariant::Primary,
        PillSize::Lg,
        |s: &mut AppState| s.setup_confirm(),
    ));
    if s.tools_error.is_some() {
        body = body.child(pill_btn(
            crate::tr!("ข้ามไปก่อน", "Skip for now"),
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.finish_setup(),
        ));
    }
    body
}

pub fn view(s: &AppState, k: &Kit) -> El {
    let content = if s.tools_installing || s.setup_pending {
        loading(s, k)
    } else {
        form(s, k)
    };
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
        .child(el().grow(1.0).w_full().center().child(content))
}
