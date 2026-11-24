//! Thai / English UI text.
//!
//! Every user-facing string is written as `tr!("ไทย", "English")` (plain text) or
//! `tf!("ไทย {x}", "English {x}", args…)` (formatted). The language is a process-wide switch,
//! set from the app state each frame, so background threads report in the chosen language too.

use std::sync::atomic::{AtomicBool, Ordering};

static THAI: AtomicBool = AtomicBool::new(true);

pub fn set_thai(thai: bool) {
    THAI.store(thai, Ordering::Relaxed);
}

pub fn is_thai() -> bool {
    THAI.load(Ordering::Relaxed)
}

/// Picks the Thai or English text.
#[macro_export]
macro_rules! tr {
    ($th:expr, $en:expr) => {
        if $crate::i18n::is_thai() { $th } else { $en }
    };
}

/// `format!` with a Thai and an English template (same placeholders in both).
#[macro_export]
macro_rules! tf {
    ($th:literal, $en:literal $($rest:tt)*) => {
        if $crate::i18n::is_thai() { format!($th $($rest)*) } else { format!($en $($rest)*) }
    };
}
