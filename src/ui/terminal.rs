use crate::state::AppState;
use crate::ui::icons;
use crate::ui::pill::{PillSize, PillVariant, pill_btn, pill_icon_btn, pill_icon_only, soft_badge};
use crate::ui::theme;
use rune::{El, ScrollSpec, col, row, text};
use rune_kit::Kit;

pub fn view(s: &AppState, k: &Kit) -> El {
    let cmd_btn = |label: &'static str, bytes: &'static [u8]| {
        pill_btn(label, PillVariant::Glass, PillSize::Sm, move |s: &mut AppState| {
            s.k9s_pty.send_bytes(bytes)
        })
    };

    let toolbar = row()
        .key("k9s-toolbar")
        .w_full()
        .gap(6.0)
        .center_y()
        .child(pill_icon_btn(
            if s.k9s_pty.is_running() {
                &icons::REFRESH
            } else {
                &icons::PLAY
            },
            if s.k9s_pty.is_running() {
                "Restart K9s"
            } else {
                "Start K9s"
            },
            PillVariant::Primary,
            PillSize::Sm,
            |s: &mut AppState| s.start_k9s(),
        ))
        .child(cmd_btn("Pods", b":pods\r"))
        .child(cmd_btn("Deploy", b":deploy\r"))
        .child(cmd_btn("Ingress", b":ing\r"))
        .child(cmd_btn("Services", b":svc\r"))
        .child(cmd_btn("Logs (l)", b"l"))
        .child(cmd_btn("Describe (d)", b"d"))
        .child(pill_icon_only(
            &icons::ARROW_UP,
            "Arrow Up",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.k9s_pty.send_bytes(b"\x1b[A"),
        ))
        .child(pill_icon_only(
            &icons::ARROW_DOWN,
            "Arrow Down",
            PillVariant::Glass,
            PillSize::Sm,
            |s: &mut AppState| s.k9s_pty.send_bytes(b"\x1b[B"),
        ))
        .child(cmd_btn("Enter", b"\r"))
        .child(cmd_btn("Esc", b"\x1b"))
        .child(soft_badge(
            s.k9s_screen.status.clone(),
            if s.k9s_screen.running {
                theme::ok_bg()
            } else {
                theme::glass_card()
            },
            if s.k9s_screen.running {
                theme::ok_fg()
            } else {
                theme::text_muted()
            },
            Some(if s.k9s_screen.running {
                theme::ok_border()
            } else {
                theme::border_subtle()
            }),
        ));

    let screen_text = if s.k9s_screen.lines.is_empty() {
        crate::tr!("(รอ output จาก K9s...)", "(Waiting for K9s output...)").to_string()
    } else {
        s.k9s_screen.lines.join("\n")
    };

    let term_box = col()
        .key("k9s-screen")
        .w_full()
        .grow(1.0)
        .shrink()
        .min_h(0.0)
        .padding(16.0)
        .radius(20.0)
        .fill(theme::bg_deep())
        .border(1.0, theme::border_glass())
        .child(
            text(screen_text)
                .font(k.ty.mono)
                .color(theme::text_base()),
        )
        .scroll(ScrollSpec { stick_bottom: false });

    col()
        .key("k9s-cli-view")
        .w_full()
        .h_full()
        .padding(20.0)
        .gap(12.0)
        .child(
            row()
                .center_y()
                .gap(10.0)
                .child(icons::mono_icon(&icons::TERMINAL, 16.0, theme::text_strong()))
                .child(
                    text("K9s Live CLI Wrapper")
                        .font(theme::heading_lg())
                        .color(theme::text_strong()),
                )
                .child(
                    text(crate::tr!("พิมพ์คีย์บอร์ดหรือกดปุ่มลัดด้านล่างเพื่อควบคุม K9s โดยตรง", "Type or use the shortcut buttons below to control K9s"))
                        .font(k.ty.text_small)
                        .color(theme::text_muted()),
                ),
        )
        .child(toolbar)
        .child(term_box)
}
