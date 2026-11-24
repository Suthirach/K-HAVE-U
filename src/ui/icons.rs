use rune::{Color, El, Icon, icon};

/// Renders a single-color minimalist vector SVG icon tinted to `color`.
pub fn mono_icon(ic: &'static Icon, size: f32, color: Color) -> El {
    icon(ic, size).color(color).icon_one_color().a11y_hidden()
}

// Re-export clean single-color stroke icons from rune_theme_opencode where available.
pub use rune_theme_opencode::icons::{
    ARROW_LEFT,
    ARROW_RIGHT,
    ARROW_UP,
    CHECK,
    CLOSE,
    COPY,
    CUBE,
    DOWNLOAD,
    EDIT,
    EYE,
    GLOBE,
    LINK,
    OUTLINE_ROTATE_CLOCKWISE as REFRESH,
    PLUS,
    SERVER,
    SLIDERS,
    TERMINAL,
    WARNING,
};

// Custom minimalist 16x16 stroke-only monochrome icons matching the visual refs.

pub static OVERVIEW_GRID: Icon = Icon {
    name: "k9s-overview-grid",
    view: 16.0,
    body: r#"<rect x="2" y="2" width="5" height="5" rx="1.2" stroke="currentColor" stroke-width="1.35"/><rect x="9" y="2" width="5" height="5" rx="1.2" stroke="currentColor" stroke-width="1.35"/><rect x="2" y="9" width="5" height="5" rx="1.2" stroke="currentColor" stroke-width="1.35"/><rect x="9" y="9" width="5" height="5" rx="1.2" stroke="currentColor" stroke-width="1.35"/>"#,
};

pub static TOPOLOGY: Icon = Icon {
    name: "k9s-topology",
    view: 16.0,
    body: r#"<circle cx="3.2" cy="8" r="1.8" stroke="currentColor" stroke-width="1.35"/><circle cx="12.8" cy="3.6" r="1.8" stroke="currentColor" stroke-width="1.35"/><circle cx="12.8" cy="12.4" r="1.8" stroke="currentColor" stroke-width="1.35"/><path d="M4.9 7.2L11.1 4.4M4.9 8.8L11.1 11.6" stroke="currentColor" stroke-width="1.35" stroke-linecap="round"/>"#,
};

pub static SERVICES: Icon = Icon {
    name: "k9s-services",
    view: 16.0,
    body: r#"<path d="M2 4.8L8 2L14 4.8L8 7.6L2 4.8Z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/><path d="M2 8.2L8 11L14 8.2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/><path d="M2 11.4L8 14.2L14 11.4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static LOGS: Icon = Icon {
    name: "k9s-logs",
    view: 16.0,
    body: r#"<path d="M2.5 4H13.5M2.5 8H11M2.5 12H8.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>"#,
};

pub static DATABASE: Icon = Icon {
    name: "k9s-database",
    view: 16.0,
    body: r#"<ellipse cx="8" cy="3.8" rx="5.2" ry="1.8" stroke="currentColor" stroke-width="1.3"/><path d="M2.8 3.8V12.2C2.8 13.2 5.1 14 8 14C10.9 14 13.2 13.2 13.2 12.2V3.8" stroke="currentColor" stroke-width="1.3"/><path d="M2.8 8C2.8 9 5.1 9.8 8 9.8C10.9 9.8 13.2 9 13.2 8" stroke="currentColor" stroke-width="1.3"/>"#,
};

pub static CLOCK: Icon = Icon {
    name: "k9s-clock",
    view: 16.0,
    body: r#"<circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.35"/><path d="M8 4.6V8L10.4 9.6" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static EYE_OFF: Icon = Icon {
    name: "k9s-eye-off",
    view: 16.0,
    body: r#"<path d="M2 2L14 14M6.5 6.6C6.2 7 6 7.5 6 8C6 9.1 6.9 10 8 10C8.5 10 9 9.8 9.4 9.5M3.6 5.2C2.6 6 1.8 7 1.4 8C2.4 10.2 4.9 12 8 12C9.2 12 10.3 11.7 11.2 11.2M6.2 4.2C6.8 4.1 7.4 4 8 4C11.1 4 13.6 5.8 14.6 8C14.2 8.8 13.7 9.6 13 10.2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static ARROW_DOWN: Icon = Icon {
    name: "k9s-arrow-down",
    view: 16.0,
    body: r#"<path d="M8 3.5V12.5M4.5 9L8 12.5L11.5 9" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static MINUS: Icon = Icon {
    name: "k9s-minus",
    view: 16.0,
    body: r#"<path d="M3.5 8H12.5" stroke="currentColor" stroke-width="1.45" stroke-linecap="round"/>"#,
};

pub static MAXIMIZE: Icon = Icon {
    name: "k9s-maximize",
    view: 16.0,
    body: r#"<rect x="3" y="3" width="10" height="10" rx="1.6" stroke="currentColor" stroke-width="1.35"/>"#,
};

pub static FULLSCREEN: Icon = Icon {
    name: "k9s-fullscreen",
    view: 16.0,
    body: r#"<path d="M2.5 6V2.5H6M10 2.5H13.5V6M13.5 10V13.5H10M6 13.5H2.5V10" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static SWITCH_ARROWS: Icon = Icon {
    name: "k9s-switch-arrows",
    view: 16.0,
    body: r#"<path d="M2.5 5.5H13.5M10.8 2.8L13.5 5.5L10.8 8.2M13.5 10.5H2.5M5.2 7.8L2.5 10.5L5.2 13.2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static LOGOUT: Icon = Icon {
    name: "k9s-logout",
    view: 16.0,
    body: r#"<path d="M6.2 2.5H3.5C2.9 2.5 2.5 2.9 2.5 3.5V12.5C2.5 13.1 2.9 13.5 3.5 13.5H6.2M10.5 5L13.5 8L10.5 11M13.2 8H6" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static PLAY: Icon = Icon {
    name: "k9s-play",
    view: 16.0,
    body: r#"<path d="M4.5 3L12.5 8L4.5 13V3Z" stroke="currentColor" stroke-width="1.35" stroke-linejoin="round"/>"#,
};

pub static INFO_CIRCLE: Icon = Icon {
    name: "k9s-info-circle",
    view: 16.0,
    body: r#"<circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.35"/><path d="M8 7.2V11.2M8 4.8V5.1" stroke="currentColor" stroke-width="1.45" stroke-linecap="round"/>"#,
};
