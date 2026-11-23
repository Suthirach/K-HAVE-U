use rune::{Color, Cursor, El, Shadow, TextStyle, Tx, row, text};
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
    build_pill(label.into(), variant, size).on_press(on_click)
}

pub fn pill_btn_tx<S: 'static>(
    label: impl Into<String>,
    variant: PillVariant,
    size: PillSize,
    on_click: impl Fn(&mut S, &mut Tx) + 'static,
) -> El {
    build_pill(label.into(), variant, size).on_press_tx(on_click)
}

/// Applies tactile spring hover + press feedback to any clickable card or tile.
pub fn interactive(e: El) -> El {
    e.cursor(Cursor::Pointer)
        .hover(Color::WHITE.alpha(0.08))
        .press(Color::BLACK.alpha(0.16))
        .press_scale(0.98)
        .hover_motion(motion::SNAPPY, motion::SNAPPY)
}

fn build_pill(label: String, variant: PillVariant, size: PillSize) -> El {
    let (h, px, font_sz) = match size {
        PillSize::Sm => (28.0, 12.0, 12.0),
        PillSize::Md => (34.0, 16.0, 13.0),
        PillSize::Lg => (40.0, 20.0, 14.0),
    };

    let (top, bot, border_col, fg, shadow_col, hover_col) = match variant {
        PillVariant::Chrome => (
            rune::color::hex("#1e2433").unwrap(),
            rune::color::hex("#11151f").unwrap(),
            rune::color::hex("#94a3b8").unwrap(),
            rune::color::hex("#f8fafc").unwrap(),
            rune::color::hex("#38bdf8").unwrap().alpha(0.20),
            Color::WHITE.alpha(0.14),
        ),
        PillVariant::Primary => (
            rune::color::hex("#6366f1").unwrap(),
            rune::color::hex("#4338ca").unwrap(),
            rune::color::hex("#a5b4fc").unwrap(),
            Color::WHITE,
            rune::color::hex("#6366f1").unwrap().alpha(0.42),
            Color::WHITE.alpha(0.18),
        ),
        PillVariant::Danger => (
            rune::color::hex("#ef4444").unwrap(),
            rune::color::hex("#b91c1c").unwrap(),
            rune::color::hex("#fca5a5").unwrap(),
            Color::WHITE,
            rune::color::hex("#ef4444").unwrap().alpha(0.38),
            Color::WHITE.alpha(0.18),
        ),
        PillVariant::Gold => (
            rune::color::hex("#2e220f").unwrap(),
            rune::color::hex("#1b1408").unwrap(),
            rune::color::hex("#f59e0b").unwrap(),
            rune::color::hex("#fde68a").unwrap(),
            rune::color::hex("#f59e0b").unwrap().alpha(0.28),
            rune::color::hex("#fbbf24").unwrap().alpha(0.16),
        ),
        PillVariant::Glass => (
            Color::WHITE.alpha(0.08),
            Color::WHITE.alpha(0.04),
            Color::WHITE.alpha(0.18),
            rune::color::hex("#e2e8f0").unwrap(),
            Color::BLACK.alpha(0.25),
            Color::WHITE.alpha(0.14),
        ),
    };

    row()
        .role("button")
        .label(label.clone())
        .hit()
        .cursor(Cursor::Pointer)
        .h(h)
        .px(px)
        .center()
        .radius(999.0)
        .gradient(top, bot)
        .border(1.25, border_col)
        .hover(hover_col)
        .press(Color::BLACK.alpha(0.22))
        .press_scale(0.95)
        .hover_motion(motion::SNAPPY, motion::SNAPPY)
        .box_shadow(Shadow {
            x: 0.0,
            y: 4.0,
            blur: 14.0,
            spread: 0.0,
            color: shadow_col,
            inset: false,
        })
        .motion(motion::SNAPPY)
        .child(
            text(label)
                .font(TextStyle::new(font_sz, 600.0))
                .color(fg)
                .a11y_hidden(),
        )
}

pub fn segmented_group(children: impl IntoIterator<Item = El>) -> El {
    row()
        .padding(3.0)
        .gap(3.0)
        .center_y()
        .radius(999.0)
        .fill(rune::color::hex("#0c0f17").unwrap())
        .border(1.0, Color::WHITE.alpha(0.10))
        .children(children)
}
