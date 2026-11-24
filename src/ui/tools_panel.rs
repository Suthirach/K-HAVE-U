use crate::state::AppState;
use crate::ui::icons;
use crate::ui::pill::{PillSize, PillVariant, pill_icon_btn};
use crate::ui::theme;
use rune::{El, col, text};
use rune_kit::Kit;

/// First line only, capped, so a long error never overflows the card.
fn short(msg: &str) -> String {
    let line = msg.lines().next().unwrap_or("").trim();
    if line.chars().count() > 90 {
        let cut: String = line.chars().take(90).collect();
        format!("{cut}…")
    } else {
        line.to_string()
    }
}

/// Shows which required tools (kubectl, k9s, gcloud) are missing and installs them on one click.
/// Returns `None` when everything is in place.
pub fn view(s: &AppState, k: &Kit) -> Option<El> {
    if s.tools_missing.is_empty() && !s.tools_installing && s.tools_error.is_none() {
        return None;
    }
    let c = &k.c;
    let mut panel = col()
        .key("tools-panel")
        .gap(8.0)
        .padding(14.0)
        .radius(16.0)
        .fill(theme::metric_box_bg())
        .border(1.0, theme::border_subtle());

    if s.tools_installing {
        panel = panel
            .child(
                text("กำลังติดตั้งเครื่องมือ...")
                    .font(k.ty.text_small_medium)
                    .color(theme::text_base()),
            )
            .child(
                text(format!("{:.0}%  ·  {}", s.tools_percent, short(&s.tools_log)))
                    .font(k.ty.small_medium)
                    .color(theme::text_muted()),
            );
    } else {
        if !s.tools_missing.is_empty() {
            panel = panel
                .child(
                    text(format!(
                        "ยังไม่พบเครื่องมือที่ต้องใช้: {}",
                        s.tools_missing.join(", ")
                    ))
                    .font(k.ty.text_small_medium)
                    .color(theme::warn_fg()),
                )
                .child(pill_icon_btn(
                    &icons::DOWNLOAD,
                    "ติดตั้งให้อัตโนมัติ",
                    PillVariant::Primary,
                    PillSize::Sm,
                    |s: &mut AppState| s.install_tools(),
                ));
        }
        if let Some(err) = &s.tools_error {
            panel = panel.child(
                text(short(err))
                    .font(k.ty.small_medium)
                    .color(c.danger.fg),
            );
        }
    }
    Some(panel)
}
