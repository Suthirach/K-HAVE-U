use rune::{Color, TextStyle};
use rune_kit::{Kit, Radius, StateColors, Type};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Plus Jakarta Sans (variable, OFL), the reference's geometric sans.
pub const SANS_FONT: &[u8] = include_bytes!("../../assets/fonts/PlusJakartaSans-Variable.ttf");

/// IBM Plex Sans Thai (OFL) for Thai text; Latin text stays in Plus Jakarta Sans.
pub const THAI_REGULAR: &[u8] = include_bytes!("../../assets/fonts/IBMPlexSansThai-Regular.ttf");
pub const THAI_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/IBMPlexSansThai-Medium.ttf");
pub const THAI_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/IBMPlexSansThai-SemiBold.ttf");
pub const THAI_BOLD: &[u8] = include_bytes!("../../assets/fonts/IBMPlexSansThai-Bold.ttf");

/// `AppSpec::fonts`: Plus Jakarta Sans for UI text, IBM Plex Sans Thai for Thai,
/// IBM Plex Mono kept for code and terminals.
pub const FONTS: &[&[u8]] = &[
    SANS_FONT,
    THAI_REGULAR,
    THAI_MEDIUM,
    THAI_SEMIBOLD,
    THAI_BOLD,
    rune_theme_opencode::FONTS[1],
    rune_theme_opencode::FONTS[2],
];

/// `AppSpec::families`: (sans, mono).
/// The sans entry is a CSS-style list: glyphs missing from Plus Jakarta Sans (all of Thai)
/// fall back to IBM Plex Sans Thai.
pub const FAMILIES: (&str, &str) = ("Plus Jakarta Sans, IBM Plex Sans Thai", "IBM Plex Mono");

/// Builds a translucent color from 8-bit sRGB channels and an alpha.
pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    hex(&format!("#{r:02x}{g:02x}{b:02x}")).alpha(a)
}

/// Helper to construct an opaque hex color cleanly at runtime.
pub fn hex(s: &str) -> Color {
    rune::color::hex(s).expect("valid hex color")
}

// ============================================================================
// Light / dark mode
// ============================================================================

static DARK: AtomicBool = AtomicBool::new(true);

/// Sets the colour mode used by every palette function below. Called once per frame from `view`.
pub fn set_dark(dark: bool) {
    DARK.store(dark, Ordering::Relaxed);
}

pub fn is_dark() -> bool {
    DARK.load(Ordering::Relaxed)
}

/// Picks the dark- or light-mode hex colour.
fn pick(dark: &str, light: &str) -> Color {
    hex(if is_dark() { dark } else { light })
}

/// Translucent "ink" for glass fills, hairlines and highlights: white on dark, black on light.
pub fn ink(alpha: f32) -> Color {
    if is_dark() { Color::WHITE.alpha(alpha) } else { Color::BLACK.alpha(alpha * 0.75) }
}

/// Logo colour: cream lines in dark mode, dark brown lines in light mode.
pub fn brand_mark() -> Color { pick("#F3EBDD", "#24170F") }

/// Boxes and chips inside a namespace card. Opaque in light mode so a red (issue) card
/// tints only the card itself, not the boxes inside it.
pub fn card_inner_bg() -> Color {
    if is_dark() { Color::BLACK.alpha(0.26) } else { hex("#ffffff") }
}

/// Workspace tab bar background.
pub fn bg_tabbar() -> Color { pick("#13110f", "#f3efe9") }

// ============================================================================
// Warm Dark Glass Minimal Palette (matching overview-ui-glass-minimal.html)
// ============================================================================

/// Deep warm obsidian page canvas (`#11100e`).
pub fn bg_canvas() -> Color { pick("#11100e", "#f7f4ef") }
/// Inner dark metric boxes, log viewer, terminal, segmented tracks (`#0e0d0b`).
pub fn bg_deep() -> Color { pick("#0e0d0b", "#efeae3") }
/// Chrome bars (titlebar, tab bar, bottom dock, drawer) (`#151311`).
pub fn bg_chrome() -> Color { pick("#151311", "#fbf9f6") }
/// Opaque fallback for warm dark glass surfaces (`#1c1a17`).
pub fn bg_card_solid() -> Color { pick("#1c1a17", "#ffffff") }
/// Subtle elevated surface (`#23201c`).
pub fn bg_elevated() -> Color { pick("#23201c", "#f2eee8") }

/// Translucent dark-warm card surface (`rgba(255,255,255,0.055)`).
/// An opaque card for modals that float over other content.
pub fn glass_card_solid() -> Color { pick("#1d1a16", "#ffffff") }
pub fn glass_card() -> Color { if is_dark() { Color::WHITE.alpha(0.055) } else { Color::WHITE.alpha(0.72) } }
/// Translucent soft card surface (`rgba(255,255,255,0.045)`).
pub fn glass_card_soft() -> Color { if is_dark() { Color::WHITE.alpha(0.045) } else { Color::WHITE.alpha(0.6) } }
/// Glass hover highlight (`rgba(255,255,255,0.10)`).
pub fn glass_hover() -> Color { if is_dark() { Color::WHITE.alpha(0.10) } else { Color::BLACK.alpha(0.05) } }
/// Inner dark metric box fill (`rgba(0,0,0,0.28)`).
pub fn metric_box_bg() -> Color { if is_dark() { Color::BLACK.alpha(0.28) } else { Color::BLACK.alpha(0.035) } }
/// Inner subtle metric box fill (`rgba(0,0,0,0.26)`).
pub fn metric_box_soft() -> Color { if is_dark() { Color::BLACK.alpha(0.26) } else { Color::BLACK.alpha(0.03) } }

/// Subtle glass borders.
pub fn border_subtle() -> Color { if is_dark() { Color::WHITE.alpha(0.08) } else { Color::BLACK.alpha(0.08) } }
pub fn border_glass() -> Color { if is_dark() { Color::WHITE.alpha(0.11) } else { Color::BLACK.alpha(0.11) } }
pub fn border_medium() -> Color { if is_dark() { Color::WHITE.alpha(0.15) } else { Color::BLACK.alpha(0.15) } }
pub fn border_strong() -> Color { if is_dark() { Color::WHITE.alpha(0.24) } else { Color::BLACK.alpha(0.22) } }

/// Crisp warm off-white primary accent (`#fafaf9`) & dark obsidian ink (`#11100e`).
pub fn accent_primary_bg() -> Color { pick("#fafaf9", "#24170f") }
pub fn accent_primary_fg() -> Color { pick("#11100e", "#f3ebdd") }

/// Warm ambient radial glow colors.
pub fn glow_warm() -> Color { if is_dark() { rgba(217, 179, 130, 0.14) } else { rgba(217, 179, 130, 0.22) } }
pub fn glow_slate() -> Color { if is_dark() { rgba(148, 163, 184, 0.11) } else { rgba(148, 163, 184, 0.14) } }

/// Warm stone typography hierarchy.
pub fn text_strong() -> Color { pick("#fafaf9", "#1c1917") }
pub fn text_base() -> Color { pick("#e7e5e4", "#292524") }
pub fn text_secondary() -> Color { pick("#d6d3d1", "#44403c") }
pub fn text_muted() -> Color { pick("#a8a29e", "#78716c") }
pub fn text_faint() -> Color { pick("#78716c", "#a8a29e") }

/// Semantic health colors cohesive with warm dark glass.
pub fn ok_fg() -> Color { pick("#86efac", "#15803d") }
pub fn ok_dot() -> Color { pick("#4ade80", "#16a34a") }
pub fn ok_bg() -> Color { if is_dark() { rgba(34, 197, 94, 0.14) } else { rgba(34, 197, 94, 0.12) } }
pub fn ok_border() -> Color { if is_dark() { rgba(134, 239, 172, 0.34) } else { rgba(22, 163, 74, 0.32) } }

pub fn warn_fg() -> Color { pick("#fcd34d", "#a16207") }
pub fn warn_dot() -> Color { pick("#fbbf24", "#d97706") }
pub fn warn_bg() -> Color { if is_dark() { rgba(251, 191, 36, 0.14) } else { rgba(245, 158, 11, 0.14) } }
pub fn warn_border() -> Color { if is_dark() { rgba(252, 211, 77, 0.40) } else { rgba(217, 119, 6, 0.36) } }

pub fn err_fg() -> Color { pick("#fca5a5", "#b91c1c") }
pub fn err_dot() -> Color { pick("#f87171", "#dc2626") }
pub fn err_pulse_hi() -> Color { pick("#ef4444", "#ef4444") }
pub fn err_pulse_lo() -> Color { pick("#7f1d1d", "#fecaca") }
pub fn err_bg() -> Color { if is_dark() { rgba(239, 68, 68, 0.16) } else { rgba(239, 68, 68, 0.10) } }
pub fn err_border() -> Color { if is_dark() { rgba(248, 113, 113, 0.42) } else { rgba(220, 38, 38, 0.34) } }
pub fn err_card_bg() -> Color { pick("#251414", "#fdecec") }

/// Environment badge colors (`STG` neutral glass, `PROD` warm amber-orange tint).
pub fn stg_bg() -> Color { if is_dark() { Color::WHITE.alpha(0.08) } else { Color::BLACK.alpha(0.06) } }
pub fn stg_fg() -> Color { pick("#d6d3d1", "#44403c") }
pub fn stg_dot() -> Color { pick("#d6d3d1", "#57534e") }

pub fn prod_bg() -> Color { if is_dark() { rgba(251, 146, 60, 0.16) } else { rgba(234, 88, 12, 0.12) } }
pub fn prod_fg() -> Color { pick("#fdba74", "#c2410c") }
pub fn prod_dot() -> Color { pick("#fb923c", "#ea580c") }

/// Warm sand/gold accent for highlights and ingress/domain links.
pub fn warm_accent() -> Color { pick("#e5c59c", "#8a5a2b") }

// ============================================================================
// Typography helpers (Inter variable + tight heading tracking -0.02em)
// ============================================================================

pub fn heading_xl() -> TextStyle {
    TextStyle::new(22.0, 600.0).lh(28.0).tracking(-0.44)
}

pub fn heading_lg() -> TextStyle {
    TextStyle::new(18.0, 600.0).lh(24.0).tracking(-0.36)
}

pub fn heading_md() -> TextStyle {
    TextStyle::new(16.0, 600.0).lh(22.0).tracking(-0.16)
}

pub fn orb_pct() -> TextStyle {
    TextStyle::new(22.0, 700.0).lh(26.0).tracking(-0.44)
}

pub fn stat_val() -> TextStyle {
    TextStyle::new(20.0, 700.0).lh(25.0).tracking(-0.30)
}

pub fn metric_val() -> TextStyle {
    TextStyle::new(15.0, 700.0).lh(20.0).tracking(-0.15)
}

pub fn label_xs() -> TextStyle {
    TextStyle::new(11.0, 500.0).lh(15.0).tracking(0.05)
}

pub fn badge_xs() -> TextStyle {
    TextStyle::new(10.0, 700.0).lh(12.0).tracking(0.20)
}

/// Returns a cached Warm Dark Glass Minimal [`Kit`] built on top of `rune_theme_opencode`
/// so all `rune_kit` components (`TextInput`, `Badge`, `Divider`, etc.) inherit the
/// obsidian palette, generous radii, and Inter variable typography cleanly.
pub fn warm_dark_kit() -> &'static Kit {
    // One cached kit per (sRGB blending, dark mode). The palette functions read the current mode,
    // so each kit is built while that mode is active.
    static KITS: [OnceLock<Kit>; 4] = [const { OnceLock::new() }; 4];
    let srgb = rune::theme::blends_srgb();
    let dark = is_dark();
    KITS[(srgb as usize) * 2 + dark as usize].get_or_init(|| {
        let mut k = rune_theme_opencode::kit(dark).clone();
        k.dark = dark;

        let c = &mut k.c;
        c.page = bg_canvas();
        c.bg_base = bg_card_solid();
        c.bg_deep = bg_deep();
        c.layer_01 = bg_chrome();
        c.layer_02 = pick("#191714", "#f6f2ec");
        c.layer_03 = bg_elevated();
        c.layer_04 = pick("#2b2722", "#ebe5dd");
        c.bg_inverse = accent_primary_bg();
        c.bg_contrast = accent_primary_bg();
        c.bg_button_neutral = glass_card();
        c.bg_icon_button_contrast = accent_primary_bg();
        c.bg_accent = prod_bg();

        c.text_strong = text_strong();
        c.text_base = text_base();
        c.text_weak = text_secondary();
        c.text_muted = text_muted();
        c.text_faint = text_faint();
        c.text_inverse = accent_primary_fg();
        c.text_contrast = prod_fg();
        c.text_accent = warm_accent();
        c.text_invert_strong = accent_primary_fg();
        c.std_text = text_base();

        c.icon_base = text_base();
        c.icon_muted = text_muted();
        c.icon_faint = text_faint();
        c.icon_inverse = accent_primary_fg();
        c.icon_contrast = prod_fg();
        c.icon_accent = accent_primary_bg();
        c.std_icon = text_base();
        c.std_icon_strong = text_strong();
        c.std_icon_invert = accent_primary_fg();

        c.border_weak = border_subtle();
        c.border_muted = border_subtle();
        c.border_base = border_glass();
        c.border_strong = border_strong();
        c.border_focus = ink(0.45);
        c.border_hover = border_strong();
        c.border_active = accent_primary_bg();
        c.std_border = border_glass();

        c.surface_base = bg_card_solid();
        c.surface_weak = bg_chrome();
        c.surface_raised = bg_elevated();
        c.surface_raised_base = pick("#201d19", "#ffffff");
        c.surface_raised_hover = pick("#2a2621", "#f4f0ea");
        c.surface_float = pick("#1b1916", "#ffffff");
        c.hover = glass_hover();
        c.pressed = Color::BLACK.alpha(if dark { 0.22 } else { 0.08 });

        c.success = StateColors {
            bg: ok_bg(),
            fg: ok_fg(),
            border: ok_border(),
            icon: ok_fg(),
        };
        c.warning = StateColors {
            bg: warn_bg(),
            fg: warn_fg(),
            border: warn_border(),
            icon: warn_fg(),
        };
        c.danger = StateColors {
            bg: err_bg(),
            fg: err_fg(),
            border: err_border(),
            icon: err_fg(),
        };
        c.info = StateColors {
            bg: glass_card(),
            fg: text_base(),
            border: border_glass(),
            icon: text_secondary(),
        };

        k.radius = Radius {
            xs: 8.0,
            sm: 10.0,
            md: 14.0,
            lg: 20.0,
            xl: 22.0,
        };

        k.ty = Type {
            ui: TextStyle::new(13.0, 450.0).lh(19.0).tracking(-0.08),
            ui_medium: TextStyle::new(13.0, 560.0).lh(19.0).tracking(-0.08),
            ui_compact: TextStyle::new(12.5, 450.0).lh(16.0).tracking(-0.06),
            small: TextStyle::new(11.5, 440.0).lh(15.0).tracking(0.02),
            small_medium: TextStyle::new(11.5, 560.0).lh(15.0).tracking(0.02),
            body: TextStyle::new(14.0, 420.0).lh(20.0).tracking(-0.10),
            body_medium: TextStyle::new(14.0, 600.0).lh(20.0).tracking(-0.14),
            text_small: TextStyle::new(12.5, 420.0).lh(18.0).tracking(-0.04),
            text_small_medium: TextStyle::new(12.5, 560.0).lh(18.0).tracking(-0.04),
            text_xsmall: TextStyle::new(11.5, 420.0).lh(16.0).tracking(0.0),
            text_xsmall_medium: TextStyle::new(11.5, 560.0).lh(16.0).tracking(0.0),
            mono: TextStyle::new(12.0, 420.0).mono().lh(18.0),
        };

        k
    })
}
