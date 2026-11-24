use rune::{Color, TextStyle};
use rune_kit::{Kit, Radius, StateColors, Type};
use std::sync::OnceLock;

/// Plus Jakarta Sans (variable, OFL), the reference's geometric sans.
pub const SANS_FONT: &[u8] = include_bytes!("../../assets/fonts/PlusJakartaSans-Variable.ttf");

/// `AppSpec::fonts`: Plus Jakarta Sans for UI text, IBM Plex Mono kept for code and terminals.
pub const FONTS: &[&[u8]] = &[
    SANS_FONT,
    rune_theme_opencode::FONTS[1],
    rune_theme_opencode::FONTS[2],
];

/// `AppSpec::families`: (sans, mono).
pub const FAMILIES: (&str, &str) = ("Plus Jakarta Sans", "IBM Plex Mono");

/// Builds a translucent color from 8-bit sRGB channels and an alpha.
pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    hex(&format!("#{r:02x}{g:02x}{b:02x}")).alpha(a)
}

/// Helper to construct an opaque hex color cleanly at runtime.
pub fn hex(s: &str) -> Color {
    rune::color::hex(s).expect("valid hex color")
}

// ============================================================================
// Warm Dark Glass Minimal Palette (matching overview-ui-glass-minimal.html)
// ============================================================================

/// Deep warm obsidian page canvas (`#11100e`).
pub fn bg_canvas() -> Color { hex("#11100e") }
/// Inner dark metric boxes, log viewer, terminal, segmented tracks (`#0e0d0b`).
pub fn bg_deep() -> Color { hex("#0e0d0b") }
/// Chrome bars (titlebar, tab bar, bottom dock, drawer) (`#151311`).
pub fn bg_chrome() -> Color { hex("#151311") }
/// Opaque fallback for warm dark glass surfaces (`#1c1a17`).
pub fn bg_card_solid() -> Color { hex("#1c1a17") }
/// Subtle elevated surface (`#23201c`).
pub fn bg_elevated() -> Color { hex("#23201c") }

/// Translucent dark-warm card surface (`rgba(255,255,255,0.055)`).
pub fn glass_card() -> Color { Color::WHITE.alpha(0.055) }
/// Translucent soft card surface (`rgba(255,255,255,0.045)`).
pub fn glass_card_soft() -> Color { Color::WHITE.alpha(0.045) }
/// Glass hover highlight (`rgba(255,255,255,0.10)`).
pub fn glass_hover() -> Color { Color::WHITE.alpha(0.10) }
/// Inner dark metric box fill (`rgba(0,0,0,0.28)`).
pub fn metric_box_bg() -> Color { Color::BLACK.alpha(0.28) }
/// Inner subtle metric box fill (`rgba(0,0,0,0.26)`).
pub fn metric_box_soft() -> Color { Color::BLACK.alpha(0.26) }

/// Subtle glass borders.
pub fn border_subtle() -> Color { Color::WHITE.alpha(0.08) }
pub fn border_glass() -> Color { Color::WHITE.alpha(0.11) }
pub fn border_medium() -> Color { Color::WHITE.alpha(0.15) }
pub fn border_strong() -> Color { Color::WHITE.alpha(0.24) }

/// Crisp warm off-white primary accent (`#fafaf9`) & dark obsidian ink (`#11100e`).
pub fn accent_primary_bg() -> Color { hex("#fafaf9") }
pub fn accent_primary_fg() -> Color { hex("#11100e") }

/// Warm ambient radial glow colors.
pub fn glow_warm() -> Color { rgba(217, 179, 130, 0.14) }
pub fn glow_slate() -> Color { rgba(148, 163, 184, 0.11) }

/// Warm stone typography hierarchy.
pub fn text_strong() -> Color { hex("#fafaf9") }
pub fn text_base() -> Color { hex("#e7e5e4") }
pub fn text_secondary() -> Color { hex("#d6d3d1") }
pub fn text_muted() -> Color { hex("#a8a29e") }
pub fn text_faint() -> Color { hex("#78716c") }

/// Semantic health colors cohesive with warm dark glass.
pub fn ok_fg() -> Color { hex("#86efac") }
pub fn ok_dot() -> Color { hex("#4ade80") }
pub fn ok_bg() -> Color { rgba(34, 197, 94, 0.14) }
pub fn ok_border() -> Color { rgba(134, 239, 172, 0.34) }

pub fn warn_fg() -> Color { hex("#fcd34d") }
pub fn warn_dot() -> Color { hex("#fbbf24") }
pub fn warn_bg() -> Color { rgba(251, 191, 36, 0.14) }
pub fn warn_border() -> Color { rgba(252, 211, 77, 0.40) }

pub fn err_fg() -> Color { hex("#fca5a5") }
pub fn err_dot() -> Color { hex("#f87171") }
pub fn err_pulse_hi() -> Color { hex("#ef4444") }
pub fn err_pulse_lo() -> Color { hex("#7f1d1d") }
pub fn err_bg() -> Color { rgba(239, 68, 68, 0.16) }
pub fn err_border() -> Color { rgba(248, 113, 113, 0.42) }
pub fn err_card_bg() -> Color { hex("#251414") }

/// Environment badge colors (`STG` neutral glass, `PROD` warm amber-orange tint).
pub fn stg_bg() -> Color { Color::WHITE.alpha(0.08) }
pub fn stg_fg() -> Color { hex("#d6d3d1") }
pub fn stg_dot() -> Color { hex("#d6d3d1") }

pub fn prod_bg() -> Color { rgba(251, 146, 60, 0.16) }
pub fn prod_fg() -> Color { hex("#fdba74") }
pub fn prod_dot() -> Color { hex("#fb923c") }

/// Warm sand/gold accent for highlights and ingress/domain links.
pub fn warm_accent() -> Color { hex("#e5c59c") }

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
    static KITS: [OnceLock<Kit>; 2] = [const { OnceLock::new() }; 2];
    let srgb = rune::theme::blends_srgb();
    KITS[srgb as usize].get_or_init(|| {
        let mut k = rune_theme_opencode::kit(true).clone();
        k.dark = true;

        let c = &mut k.c;
        c.page = bg_canvas();
        c.bg_base = bg_card_solid();
        c.bg_deep = bg_deep();
        c.layer_01 = bg_chrome();
        c.layer_02 = hex("#191714");
        c.layer_03 = bg_elevated();
        c.layer_04 = hex("#2b2722");
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
        c.border_focus = Color::WHITE.alpha(0.45);
        c.border_hover = border_strong();
        c.border_active = accent_primary_bg();
        c.std_border = border_glass();

        c.surface_base = bg_card_solid();
        c.surface_weak = bg_chrome();
        c.surface_raised = bg_elevated();
        c.surface_raised_base = hex("#201d19");
        c.surface_raised_hover = hex("#2a2621");
        c.surface_float = hex("#1b1916");
        c.hover = glass_hover();
        c.pressed = Color::BLACK.alpha(0.22);

        c.success = StateColors {
            bg: ok_bg(),
            fg: ok_fg(),
            border: ok_border(),
            icon: ok_fg(),
        };
        c.warning = StateColors {
            bg: warn_bg(),
            fg: warn_fg(),
            border: rgba(252, 211, 77, 0.38),
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
