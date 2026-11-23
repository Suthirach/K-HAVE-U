use crate::state::AppState;
use crate::ui::pill::{PillSize, PillVariant, pill_btn};
use rune::{El, TextEdit, col, el, row, text};
use rune_kit::{Badge, BadgeVariant, Kit, TextInput};

pub fn view(s: &AppState, k: &Kit) -> El {
    let c = &k.c;
    let has_oauth = s.google_config.google_client_id.is_some();

    let mut card = col()
        .key("login-card")
        .w(440.0)
        .padding(28.0)
        .gap(16.0)
        .radius(k.radius.lg)
        .fill(c.bg_base)
        .border(1.0, c.border_base)
        .child(
            col()
                .gap(6.0)
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(text("K9s Studio").font(rune::TextStyle::new(22.0, 600.0)).color(c.text_strong))
                        .child(Badge::new("STG + PROD").variant(BadgeVariant::Accent).build(k)),
                )
                .child(
                    text("เข้าสู่ระบบด้วย Gmail / Google Account เพื่อดู Stack Diagram, สถานะ Service และ K9s CLI")
                        .font(k.ty.text_small)
                        .color(c.text_muted),
                ),
        )
        .child(
            col()
                .gap(8.0)
                .padding(14.0)
                .radius(k.radius.md)
                .fill(c.layer_02)
                .border(1.0, c.border_weak)
                .child(pill_btn(
                    if s.google_loading {
                        "⏳ กำลังเชื่อมต่อ Google..."
                    } else {
                        "G  Sign in with Google (Gmail)"
                    },
                    PillVariant::Primary,
                    PillSize::Lg,
                    |s: &mut AppState| s.login_with_google(false),
                ))
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(pill_btn(
                            "🔄 สลับบัญชี Gmail (เปิด Browser)",
                            PillVariant::Chrome,
                            PillSize::Sm,
                            |s: &mut AppState| s.login_with_google(true),
                        ))
                        .child(
                            text(if has_oauth {
                                "OAuth2 PKCE พร้อมใช้งาน"
                            } else {
                                "เชื่อมผ่าน gcloud Google Auth"
                            })
                            .font(k.ty.small_medium)
                            .color(c.text_muted),
                        ),
                ),
        )
        .child(
            row()
                .gap(8.0)
                .center_y()
                .child(el().h(1.0).grow(1.0).fill(c.border_weak))
                .child(text("หรือเข้าสู่ระบบด้วยบัญชีภายใน").font(k.ty.small_medium).color(c.text_faint))
                .child(el().h(1.0).grow(1.0).fill(c.border_weak)),
        )
        .child(
            col()
                .gap(6.0)
                .child(text("Username").font(k.ty.text_small_medium).color(c.text_base))
                .child(
                    TextInput::new(&s.login_user)
                        .large()
                        .width(None)
                        .placeholder("เช่น admin หรือ dev")
                        .on_edit(|s: &mut AppState| &mut s.login_user)
                        .on_submit(|s: &mut AppState, _| s.try_login())
                        .build(k),
                ),
        )
        .child(
            col()
                .gap(6.0)
                .child(text("Password").font(k.ty.text_small_medium).color(c.text_base))
                .child(
                    TextInput::new(&s.login_pass)
                        .large()
                        .width(None)
                        .placeholder("กรอกรหัสผ่าน")
                        .on_edit(|s: &mut AppState| &mut s.login_pass)
                        .on_submit(|s: &mut AppState, _| s.try_login())
                        .build(k),
                ),
        );

    if let Some(err) = &s.login_error {
        card = card.child(
            row()
                .padding(10.0)
                .radius(k.radius.md)
                .fill(c.danger.bg)
                .child(text(err.clone()).font(k.ty.text_small_medium).color(c.danger.fg)),
        );
    }

    card = card
        .child(pill_btn("เข้าสู่ระบบ", PillVariant::Chrome, PillSize::Lg, |s: &mut AppState| s.try_login()))
        .child(
            col()
                .gap(8.0)
                .pt(4.0)
                .child(
                    text("บัญชีทดสอบด่วน:")
                        .font(k.ty.small_medium)
                        .color(c.text_muted),
                )
                .child(
                    row()
                        .gap(6.0)
                        .child(pill_btn("Admin (STG+PROD)", PillVariant::Glass, PillSize::Sm, |s: &mut AppState| {
                            s.login_user = TextEdit::new("admin");
                            s.login_pass = TextEdit::new("admin123");
                            s.try_login();
                        }))
                        .child(pill_btn("Dev (STG Write)", PillVariant::Glass, PillSize::Sm, |s: &mut AppState| {
                            s.login_user = TextEdit::new("dev");
                            s.login_pass = TextEdit::new("dev123");
                            s.try_login();
                        }))
                        .child(pill_btn("Viewer", PillVariant::Glass, PillSize::Sm, |s: &mut AppState| {
                            s.login_user = TextEdit::new("viewer");
                            s.login_pass = TextEdit::new("viewer123");
                            s.try_login();
                        })),
                ),
        );

    col()
        .key("login-page")
        .w_full()
        .h_full()
        .fill(c.page)
        .child(crate::ui::custom_titlebar(s, k))
        .child(el().grow(1.0).w_full().center().child(card))
}
