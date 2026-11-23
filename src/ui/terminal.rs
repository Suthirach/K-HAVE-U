use crate::state::AppState;
use crate::ui::pill::{PillSize, PillVariant, pill_btn};
use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{Badge, BadgeVariant, Kit};

pub fn view(s: &AppState, k: &Kit) -> El {
    let c = &k.c;

    let cmd_btn = |label: &'static str, bytes: &'static [u8]| {
        pill_btn(label, PillVariant::Glass, PillSize::Sm, move |s: &mut AppState| s.k9s_pty.send_bytes(bytes))
    };

    let toolbar = row()
        .key("k9s-toolbar")
        .w_full()
        .gap(6.0)
        .center_y()
        .child(pill_btn(
            if s.k9s_pty.is_running() { "↻ Restart K9s" } else { "▶ Start K9s" },
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
        .child(cmd_btn("↑", b"\x1b[A"))
        .child(cmd_btn("↓", b"\x1b[B"))
        .child(cmd_btn("Enter", b"\r"))
        .child(cmd_btn("Esc", b"\x1b"))
        .child(
            Badge::new(s.k9s_screen.status.clone())
                .variant(if s.k9s_screen.running { BadgeVariant::Accent } else { BadgeVariant::Subtle })
                .build(k),
        );

    let screen_text = if s.k9s_screen.lines.is_empty() {
        "(รอ output จาก K9s...)".to_string()
    } else {
        s.k9s_screen.lines.join("\n")
    };

    let term_box = col()
        .key("k9s-screen")
        .w_full()
        .grow(1.0)
        .shrink()
        .min_h(0.0)
        .padding(14.0)
        .radius(k.radius.lg)
        .fill(rune::color::hex("#0d1117").unwrap())
        .border(1.0, c.border_base)
        .child(
            text(screen_text)
                .font(k.ty.mono)
                .color(rune::color::hex("#e6edf3").unwrap()),
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
                .gap(8.0)
                .child(text("K9s Live CLI Wrapper").font(rune::TextStyle::new(18.0, 600.0)).color(c.text_strong))
                .child(
                    text("พิมพ์คีย์บอร์ดหรือกดปุ่มลัดด้านล่างเพื่อควบคุม K9s โดยตรง")
                        .font(k.ty.text_small)
                        .color(c.text_muted),
                ),
        )
        .child(toolbar)
        .child(term_box)
}
