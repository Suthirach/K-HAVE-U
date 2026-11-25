//! A clear error card: what failed, the likely cause, how to fix it, and the raw message.

use crate::diagnose::diagnose;
use crate::state::AppState;
use crate::ui::icons::{self, mono_icon};
use crate::ui::pill::{PillSize, PillVariant, pill_icon_btn};
use crate::ui::theme;
use rune::{El, col, row, text};
use rune_kit::Kit;

fn labeled(label: &str, body: String, k: &Kit) -> El {
    row()
        .w_full()
        .gap(10.0)
        .child(
            text(label.to_string())
                .font(k.ty.small_medium)
                .color(theme::text_muted())
                .w(78.0),
        )
        .child(
            text(body)
                .font(k.ty.text_small)
                .color(theme::text_strong())
                .wrap()
                .grow(1.0)
                .shrink()
                .min_w(0.0),
        )
}

/// The full card, for a page that has nothing else to show.
/// `scope` says what was being loaded (e.g. "namespace staging-api").
pub fn panel(
    raw: &str,
    scope: Option<String>,
    on_retry: impl Fn(&mut AppState) + 'static,
    k: &Kit,
) -> El {
    let d = diagnose(raw);
    let raw_copy = d.raw.clone();
    let mut head = col()
        .gap(3.0)
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .child(
            text(d.title.clone())
                .font(theme::heading_md())
                .color(theme::err_fg()),
        );
    if let Some(scope) = scope {
        head = head.child(text(scope).font(k.ty.small).color(theme::text_muted()));
    }

    col()
        .key("error-panel")
        .w_full()
        .max_w(760.0)
        .padding(20.0)
        .gap(14.0)
        .radius(20.0)
        .fill(theme::err_card_bg())
        .border(1.0, theme::err_border())
        .child(
            row()
                .w_full()
                .gap(12.0)
                .center_y()
                .child(mono_icon(&icons::WARNING, 22.0, theme::err_dot()))
                .child(head),
        )
        .child(labeled(crate::tr!("สาเหตุ", "Cause"), d.cause.clone(), k))
        .child(labeled(crate::tr!("วิธีแก้", "How to fix"), d.fix.clone(), k))
        .child(
            col()
                .w_full()
                .gap(6.0)
                .child(
                    text(crate::tr!("ข้อความจากระบบ", "System message"))
                        .font(k.ty.small_medium)
                        .color(theme::text_muted()),
                )
                .child(
                    col()
                        .w_full()
                        .padding(12.0)
                        .radius(12.0)
                        .fill(theme::ink(0.05))
                        .border(1.0, theme::border_subtle())
                        .child(
                            text(d.raw.clone())
                                .font(k.ty.mono)
                                .color(theme::text_secondary())
                                .wrap()
                                .w_full()
                                .shrink()
                                .min_w(0.0),
                        ),
                ),
        )
        .child(
            row()
                .gap(8.0)
                .child(pill_icon_btn(
                    &icons::REFRESH,
                    crate::tr!("ลองใหม่", "Retry"),
                    PillVariant::Primary,
                    PillSize::Sm,
                    on_retry,
                ))
                .child(pill_icon_btn(
                    &icons::COPY,
                    crate::tr!("คัดลอกข้อความ error", "Copy error"),
                    PillVariant::Glass,
                    PillSize::Sm,
                    move |s: &mut AppState| {
                        s.copy_to_clipboard(raw_copy.clone(), crate::tr!("ข้อความ error", "Error message"))
                    },
                )),
        )
}

/// A centred wrapper so the card sits nicely in an empty page.
pub fn page(raw: &str, scope: Option<String>, on_retry: impl Fn(&mut AppState) + 'static, k: &Kit) -> El {
    col()
        .w_full()
        .h_full()
        .padding(24.0)
        .center()
        .child(panel(raw, scope, on_retry, k))
}

/// One line for a few namespaces that failed while others loaded.
pub fn strip(failed: &[(String, String)], on_retry: impl Fn(&mut AppState) + 'static, k: &Kit) -> El {
    let mut lines = col().gap(4.0).grow(1.0).shrink().min_w(0.0).child(
        text(crate::tf!(
            "โหลดไม่ได้ {} namespace",
            "{} namespace(s) couldn't be loaded",
            failed.len()
        ))
        .font(k.ty.small_medium)
        .color(theme::err_fg()),
    );
    for (ns, raw) in failed.iter().take(4) {
        let d = diagnose(raw);
        lines = lines.child(
            text(format!("• {ns} — {}: {}", d.title, d.fix))
                .font(k.ty.text_small)
                .color(theme::text_secondary())
                .wrap()
                .w_full()
                .shrink()
                .min_w(0.0),
        );
    }
    row()
        .key("error-strip")
        .w_full()
        .padding(14.0)
        .gap(12.0)
        .radius(16.0)
        .fill(theme::err_card_bg())
        .border(1.0, theme::err_border())
        .child(mono_icon(&icons::WARNING, 18.0, theme::err_dot()))
        .child(lines)
        .child(pill_icon_btn(
            &icons::REFRESH,
            crate::tr!("ลองใหม่", "Retry"),
            PillVariant::Glass,
            PillSize::Sm,
            on_retry,
        ))
}
