use crate::ui::icons::mono_icon;
use crate::ui::theme;
use rune::{Color, Cursor, El, Icon, Shadow, TextStyle, Tx, el, row, text};
use rune::tok::motion;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PillVariant {
    Chrome,
    Primary,
    Danger,
    Gold,
    Glass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PillSize {
    Sm,
    Md,
    Lg,
}

pub fn pill_btn<S: 'static>(
    label: impl Into<String>,
    variant: PillVariant,
    size: PillSize,
    on_click: impl Fn(&mut S) + 'static,
) -> El {
    build_pill(None, label.into(), variant, size).on_press(on_click)
}

pub fn pill_icon_btn<S: 'static>(
    ic: &'static Icon,
    label: impl Into<String>,
    variant: PillVariant,
    size: PillSize,
    on_click: impl Fn(&mut S) + 'static,
) -> El {
    build_pill(Some(ic), label.into(), variant, size).on_press(on_click)
}

pub fn pill_btn_tx<S: 'static>(
    label: impl Into<String>,
    variant: PillVariant,
    size: PillSize,
    on_click: impl Fn(&mut S, &mut Tx) + 'static,
) -> El {
    build_pill(None, label.into(), variant, size).on_press_tx(on_click)
}

pub fn pill_icon_only<S: 'static>(
    ic: &'static Icon,
    a11y_label: &'static str,
    variant: PillVariant,
    size: PillSize,
    on_click: impl Fn(&mut S) + 'static,
) -> El {
    build_icon_only(ic, a11y_label, variant, size).on_press(on_click)
}

pub fn pill_icon_only_tx<S: 'static>(
    ic: &'static Icon,
    a11y_label: &'static str,
    variant: PillVariant,
    size: PillSize,
    on_click: impl Fn(&mut S, &mut Tx) + 'static,
) -> El {
    build_icon_only(ic, a11y_label, variant, size).on_press_tx(on_click)
}

/// Circular warm off-white `→` action button matching `.go-circle` in the visual reference.
pub fn circle_action_btn<S: 'static>(
    ic: &'static Icon,
    a11y_label: impl Into<String>,
    on_click: impl Fn(&mut S) + 'static,
) -> El {
    el().role("button")
        .label(a11y_label.into())
        .hit()
        .cursor(Cursor::Pointer)
        .size(34.0, 34.0)
        .radius(999.0)
        .center()
        .fill(theme::accent_primary_bg())
        .hover(Color::BLACK.alpha(0.08))
        .press(Color::BLACK.alpha(0.16))
        .press_scale(PRESS_ICON)
        .hover_motion(motion::SNAPPY, motion::SNAPPY)
        .box_shadow(Shadow {
            x: 0.0,
            y: 4.0,
            blur: 12.0,
            spread: 0.0,
            color: Color::BLACK.alpha(0.32),
            inset: false,
        })
        .motion(motion::SNAPPY)
        .on_press(on_click)
        .child(mono_icon(ic, 15.0, theme::accent_primary_fg()))
}

/// Compact rounded pill tag matching `.mini-pill` in the visual reference.
pub fn mini_pill_btn<S: 'static>(
    label: impl Into<String>,
    on_click: impl Fn(&mut S) + 'static,
) -> El {
    let label = label.into();
    row()
        .role("button")
        .label(label.clone())
        .hit()
        .cursor(Cursor::Pointer)
        .h(25.0)
        .px(10.0)
        .center()
        .radius(999.0)
        .fill(Color::WHITE.alpha(0.05))
        .border(1.0, Color::WHITE.alpha(0.10))
        .hover(Color::WHITE.alpha(0.14))
        .press(Color::BLACK.alpha(0.18))
        .press_scale(PRESS_PILL)
        .hover_motion(motion::SNAPPY, motion::SNAPPY)
        .on_press(on_click)
        .child(
            text(label)
                .font(TextStyle::new(11.0, 520.0).tracking(-0.02))
                .color(theme::text_secondary())
                .a11y_hidden(),
        )
}

/// Rounded status / env / meta pill badge (replaces sharp 2px rectangles).
pub fn soft_badge(label: impl Into<String>, bg: Color, fg: Color, border: Option<Color>) -> El {
    let label = label.into();
    let e = row()
        .role("status")
        .label(label.clone())
        .h(20.0)
        .px(8.0)
        .center()
        .radius(999.0)
        .fill(bg)
        .child(
            text(label)
                .font(theme::badge_xs())
                .color(fg)
                .a11y_hidden(),
        );
    match border {
        Some(b) => e.border(1.0, b),
        None => e,
    }
}

// Press depths: Rune springs back with `motion::POP` on release, so a deeper press reads as a bounce.
const PRESS_PILL: f32 = 0.9;
const PRESS_ICON: f32 = 0.84;
const PRESS_CARD: f32 = 0.95;
const PRESS_ROW: f32 = 0.99;

/// Applies tactile spring hover + bouncy press feedback to any clickable card or tile.
pub fn interactive(e: El) -> El {
    interactive_scaled(e, PRESS_CARD)
}

/// [`interactive`] for full-width rows (log lines), where a deep bounce would distract.
pub fn interactive_row(e: El) -> El {
    interactive_scaled(e, PRESS_ROW)
}

fn interactive_scaled(e: El, scale: f32) -> El {
    e.cursor(Cursor::Pointer)
        .hover(Color::WHITE.alpha(0.06))
        .press(Color::BLACK.alpha(0.14))
        .press_scale(scale)
        .hover_motion(motion::SNAPPY, motion::SNAPPY)
}

fn variant_tokens(variant: PillVariant) -> (Color, Color, Color, Color, Color) {
    match variant {
        // Crisp warm off-white primary pill with obsidian ink (exact ref active style).
        PillVariant::Primary => (
            theme::accent_primary_bg(),
            theme::accent_primary_bg(),
            theme::accent_primary_fg(),
            Color::BLACK.alpha(0.24),
            Color::BLACK.alpha(0.08),
        ),
        // Elevated warm dark glass pill.
        PillVariant::Chrome => (
            Color::WHITE.alpha(0.10),
            Color::WHITE.alpha(0.18),
            theme::text_strong(),
            Color::BLACK.alpha(0.26),
            Color::WHITE.alpha(0.12),
        ),
        // Subtle warm dark glass pill.
        PillVariant::Glass => (
            Color::WHITE.alpha(0.055),
            Color::WHITE.alpha(0.13),
            theme::text_base(),
            Color::BLACK.alpha(0.20),
            Color::WHITE.alpha(0.10),
        ),
        // Warm amber warning / caution pill.
        PillVariant::Gold => (
            theme::warn_bg(),
            theme::rgba(252, 211, 77, 0.42),
            theme::warn_fg(),
            Color::BLACK.alpha(0.22),
            theme::rgba(252, 211, 77, 0.16),
        ),
        // Crisp dark-glass error / destructive pill.
        PillVariant::Danger => (
            theme::err_bg(),
            theme::err_border(),
            theme::err_fg(),
            Color::BLACK.alpha(0.24),
            theme::rgba(239, 68, 68, 0.24),
        ),
    }
}

fn build_pill(ic: Option<&'static Icon>, label: String, variant: PillVariant, size: PillSize) -> El {
    let (h, px, font_sz, icon_sz, gap) = match size {
        PillSize::Sm => (28.0, 12.0, 12.0, 13.0, 6.0),
        PillSize::Md => (34.0, 15.0, 12.5, 14.0, 7.0),
        PillSize::Lg => (40.0, 18.0, 13.5, 15.0, 8.0),
    };
    let (fill, border_col, fg, shadow_col, hover_col) = variant_tokens(variant);

    let mut r = row()
        .role("button")
        .label(label.clone())
        .hit()
        .cursor(Cursor::Pointer)
        .h(h)
        .px(px)
        .gap(gap)
        .center()
        .radius(999.0)
        .fill(fill)
        .border(1.0, border_col)
        .hover(hover_col)
        .press(Color::BLACK.alpha(0.18))
        .press_scale(PRESS_PILL)
        .hover_motion(motion::SNAPPY, motion::SNAPPY)
        .box_shadow(Shadow {
            x: 0.0,
            y: 3.0,
            blur: 10.0,
            spread: 0.0,
            color: shadow_col,
            inset: false,
        })
        .motion(motion::SNAPPY);

    if let Some(icon_ref) = ic {
        r = r.child(mono_icon(icon_ref, icon_sz, fg));
    }

    r.child(
        text(label)
            .font(TextStyle::new(font_sz, 600.0).tracking(-0.08))
            .color(fg)
            .a11y_hidden(),
    )
}

fn build_icon_only(ic: &'static Icon, a11y_label: &'static str, variant: PillVariant, size: PillSize) -> El {
    let (dim, icon_sz) = match size {
        PillSize::Sm => (28.0, 13.0),
        PillSize::Md => (34.0, 15.0),
        PillSize::Lg => (40.0, 16.0),
    };
    let (fill, border_col, fg, _shadow_col, hover_col) = variant_tokens(variant);

    el().role("button")
        .label(a11y_label)
        .hit()
        .cursor(Cursor::Pointer)
        .size(dim, dim)
        .center()
        .radius(999.0)
        .fill(fill)
        .border(1.0, border_col)
        .hover(hover_col)
        .press(Color::BLACK.alpha(0.18))
        .press_scale(PRESS_ICON)
        .hover_motion(motion::SNAPPY, motion::SNAPPY)
        .motion(motion::SNAPPY)
        .child(mono_icon(ic, icon_sz, fg))
}

pub fn segmented_group(children: impl IntoIterator<Item = El>) -> El {
    row()
        .padding(3.0)
        .gap(3.0)
        .center_y()
        .radius(999.0)
        .fill(theme::bg_deep())
        .border(1.0, theme::border_glass())
        .children(children)
}
