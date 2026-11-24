use crate::state::AppState;
use crate::ui::icons;
use crate::ui::pill::{PillSize, PillVariant, pill_btn, pill_icon_btn, soft_badge};
use crate::ui::theme;
use rune::{Color, El, Shadow, TextEdit, col, el, row, text};
use rune_kit::{Kit, TextInput};

pub fn view(s: &AppState, k: &Kit) -> El {
    let c = &k.c;
    let has_oauth = s.google_config.google_client_id.is_some();

    let mut card = col()
        .key("login-card")
        .w(440.0)
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
                            text("Klar")
                                .font(theme::heading_xl())
                                .color(theme::text_strong()),
                        )
                        .child(soft_badge(
                            "STG + PROD",
                            theme::prod_bg(),
                            theme::prod_fg(),
                            None,
                        )),
                )
                // Two short lines so the description never runs past the card edge.
                .child(
                    text(crate::tr!("เข้าสู่ระบบด้วย Gmail / Google Account", "Sign in with Gmail / Google Account"))
                        .font(k.ty.text_small)
                        .color(theme::text_muted()),
                )
                .child(
                    text(crate::tr!("เพื่อดู Stack Diagram, สถานะ Service และ K9s CLI", "to see the stack diagram, service health and K9s CLI"))
                        .font(k.ty.text_small)
                        .color(theme::text_muted()),
                ),
        )
        .child(
            col()
                .gap(10.0)
                .padding(14.0)
                .radius(16.0)
                .fill(theme::metric_box_bg())
                .border(1.0, theme::border_subtle())
                .child(pill_icon_btn(
                    if s.google_loading {
                        &icons::CLOCK
                    } else {
                        &icons::GLOBE
                    },
                    if s.google_loading {
                        crate::tr!("กำลังเชื่อมต่อ Google...", "Connecting to Google...")
                    } else {
                        "Sign in with Google (Gmail)"
                    },
                    PillVariant::Primary,
                    PillSize::Lg,
                    |s: &mut AppState| s.login_with_google(false),
                ))
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(pill_icon_btn(
                            &icons::REFRESH,
                            crate::tr!("สลับบัญชี Gmail", "Switch Gmail account"),
                            PillVariant::Glass,
                            PillSize::Sm,
                            |s: &mut AppState| s.login_with_google(true),
                        ))
                        .child(
                            text(if has_oauth {
                                crate::tr!("OAuth2 PKCE พร้อมใช้งาน", "OAuth2 PKCE ready")
                            } else {
                                crate::tr!("เชื่อมผ่าน gcloud Google Auth", "Uses gcloud Google Auth")
                            })
                            .font(k.ty.small_medium)
                            .color(theme::text_muted()),
                        ),
                ),
        )
        .child(
            row()
                .gap(8.0)
                .center_y()
                .child(el().h(1.0).grow(1.0).fill(theme::border_subtle()))
                .child(
                    text(crate::tr!("หรือเข้าสู่ระบบด้วยบัญชีภายใน", "or sign in with a local account"))
                        .font(k.ty.small_medium)
                        .color(theme::text_faint()),
                )
                .child(el().h(1.0).grow(1.0).fill(theme::border_subtle())),
        )
        .child(
            col()
                .gap(6.0)
                .child(
                    text("Username")
                        .font(k.ty.text_small_medium)
                        .color(theme::text_base()),
                )
                .child(
                    TextInput::new(&s.login_user)
                        .large()
                        .width(None)
                        .placeholder(crate::tr!("เช่น admin หรือ dev", "admin or dev"))
                        .on_edit(|s: &mut AppState| &mut s.login_user)
                        .on_submit(|s: &mut AppState, _| s.try_login())
                        .build(k),
                ),
        )
        .child(
            col()
                .gap(6.0)
                .child(
                    text("Password")
                        .font(k.ty.text_small_medium)
                        .color(theme::text_base()),
                )
                .child(
                    TextInput::new(&s.login_pass)
                        .large()
                        .width(None)
                        .placeholder(crate::tr!("กรอกรหัสผ่าน", "Enter your password"))
                        .on_edit(|s: &mut AppState| &mut s.login_pass)
                        .on_submit(|s: &mut AppState, _| s.try_login())
                        .build(k),
                ),
        );

    if let Some(panel) = crate::ui::tools_panel::view(s, k) {
        card = card.child(panel);
    }
    card = card.child(crate::ui::update_panel::view(s, k));

    if let Some(err) = &s.login_error {
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

    card = card
        .child(pill_icon_btn(
            &icons::ARROW_RIGHT,
            crate::tr!("เข้าสู่ระบบ", "Sign in"),
            PillVariant::Chrome,
            PillSize::Lg,
            |s: &mut AppState| s.try_login(),
        ))
        .child(
            col()
                .gap(8.0)
                .pt(4.0)
                .child(
                    text(crate::tr!("บัญชีทดสอบด่วน:", "Test accounts:"))
                        .font(k.ty.small_medium)
                        .color(theme::text_muted()),
                )
                .child(
                    row()
                        .gap(6.0)
                        .child(pill_btn(
                            "Admin (STG+PROD)",
                            PillVariant::Glass,
                            PillSize::Sm,
                            |s: &mut AppState| {
                                s.login_user = TextEdit::new("admin");
                                s.login_pass = TextEdit::new("admin123");
                                s.try_login();
                            },
                        ))
                        .child(pill_btn(
                            "Dev (STG Write)",
                            PillVariant::Glass,
                            PillSize::Sm,
                            |s: &mut AppState| {
                                s.login_user = TextEdit::new("dev");
                                s.login_pass = TextEdit::new("dev123");
                                s.try_login();
                            },
                        ))
                        .child(pill_btn(
                            "Viewer",
                            PillVariant::Glass,
                            PillSize::Sm,
                            |s: &mut AppState| {
                                s.login_user = TextEdit::new("viewer");
                                s.login_pass = TextEdit::new("viewer123");
                                s.try_login();
                            },
                        )),
                ),
        );

    col()
        .key("login-page")
        .w_full()
        .h_full()
        .fill(theme::bg_canvas())
        .radial_gradient(
            [0.16, 0.10],
            [680.0, 380.0],
            [
                (0.0, theme::glow_warm()),
                (0.65, Color::TRANSPARENT),
            ],
        )
        .child(crate::ui::custom_titlebar(s, k))
        .child(el().grow(1.0).w_full().center().child(card))
}
