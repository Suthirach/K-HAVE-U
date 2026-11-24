use crate::state::AppState;
use crate::ui::pill::{PillSize, PillVariant, pill_btn};
use crate::ui::theme;
use rune::{El, col, row, text};
use rune_kit::{Kit, TextInput};

fn short(msg: &str) -> String {
    let line = msg.lines().next().unwrap_or("").trim();
    if line.chars().count() > 90 {
        let cut: String = line.chars().take(90).collect();
        format!("{cut}…")
    } else {
        line.to_string()
    }
}

/// Optional: paste a GitHub token so the app can check for updates in a private repo.
pub fn view(s: &AppState, k: &Kit) -> El {
    let placeholder = if s.update_token.is_some() {
        "มี GitHub token แล้ว (วางอันใหม่เพื่อเปลี่ยน)"
    } else {
        "วาง GitHub token (สิทธิ์อ่านอย่างเดียว)"
    };
    let mut panel = col()
        .key("update-panel")
        .gap(6.0)
        .child(
            text("อัปเดตแอปจาก repo private (ไม่บังคับ)")
                .font(k.ty.small_medium)
                .color(theme::text_muted()),
        )
        .child(
            row()
                .gap(8.0)
                .center_y()
                .child(
                    TextInput::new(&s.update_token_input)
                        .width(None)
                        .placeholder(placeholder)
                        .on_edit(|s: &mut AppState| &mut s.update_token_input)
                        .on_submit(|s: &mut AppState, _| s.save_update_token())
                        .build(k),
                )
                .child(pill_btn(
                    "บันทึกและเช็ก",
                    PillVariant::Glass,
                    PillSize::Sm,
                    |s: &mut AppState| s.save_update_token(),
                )),
        );
    if let Some(note) = &s.update_note {
        panel = panel.child(
            text(short(note))
                .font(k.ty.small_medium)
                .color(theme::text_faint()),
        );
    }
    panel
}
