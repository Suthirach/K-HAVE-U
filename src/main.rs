mod auth;
mod gcp;
mod i18n;
mod k8s;
mod pty_k9s;
mod state;
mod tools;
mod ui;
mod update;

use rune::{AppSpec, Cx, El, Key, KeyEvent, Tx, WindowOptions, WindowSpec};
use rune::runtime::Waker;
use state::{AppState, Page, ServiceTab, Tab};

fn setup(s: &mut AppState, waker: Waker) {
    s.waker = Some(waker);
    s.check_tools();
    s.check_for_update();
}

fn poll(s: &mut AppState, t: f64, tx: &mut Tx) -> bool {
    let selected_before = s.selected_service.clone();
    let mut changed = s.drain_events();
    changed |= s.track_navigation();
    if s.tick_hold() {
        tx.wake_after(std::time::Duration::from_millis(16));
        changed = true;
    }
    if s.tick_loader(t) {
        tx.wake_after(std::time::Duration::from_millis(33));
        changed = true;
    }
    // A G6 click arrives over IPC, not as window input, so ask for the drawer's slide explicitly.
    if s.selected_service != selected_before {
        tx.animated();
    }
    if let Some(left) = ui::topology_g6::deferred_resize_remaining() {
        if left.is_zero() {
            ui::topology_g6::sync_webview(s);
        } else {
            tx.wake_after(left);
        }
    }
    if s.session.is_some() && s.has_any_issue() {
        tx.wake_after(std::time::Duration::from_millis(450));
        let next = ((t * 2.2) as u64) % 2 == 0;
        if next != s.pulse_on {
            s.pulse_on = next;
            changed = true;
        }
    }
    changed
}

fn on_key(s: &mut AppState, ev: &KeyEvent, _tx: &mut Tx) -> bool {
    if s.show_setup {
        return false;
    }
    if s.session.is_none() {
        if matches!(ev.key, Key::Enter) {
            s.try_login();
            return true;
        }
        return false;
    }

    if s.svc_picker_open && matches!(ev.key, Key::Escape) {
        s.close_svc_picker();
        return true;
    }
    if s.diagram_fullscreen && matches!(ev.key, Key::Escape) {
        s.diagram_fullscreen = false;
        return true;
    }

    if s.page == Page::Namespace && s.tab == Tab::K9sCli && s.k9s_pty.is_running() {
        match &ev.key {
            Key::Text(t) => s.k9s_pty.send_bytes(t.as_bytes()),
            Key::Enter => s.k9s_pty.send_bytes(b"\r"),
            Key::Backspace => s.k9s_pty.send_bytes(b"\x7f"),
            Key::Escape => s.k9s_pty.send_bytes(b"\x1b"),
            Key::Tab => s.k9s_pty.send_bytes(b"\t"),
            Key::Up => s.k9s_pty.send_bytes(b"\x1b[A"),
            Key::Down => s.k9s_pty.send_bytes(b"\x1b[B"),
            Key::Right => s.k9s_pty.send_bytes(b"\x1b[C"),
            Key::Left => s.k9s_pty.send_bytes(b"\x1b[D"),
            Key::Home => s.k9s_pty.send_bytes(b"\x1b[H"),
            Key::End => s.k9s_pty.send_bytes(b"\x1b[F"),
            Key::PageUp => s.k9s_pty.send_bytes(b"\x1b[5~"),
            Key::PageDown => s.k9s_pty.send_bytes(b"\x1b[6~"),
            _ => return false,
        }
        return true;
    }

    false
}

fn view(s: &AppState, _cx: &mut Cx<AppState>) -> El {
    let screen = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        format!(
            "{}{}{}{}{:?}{:?}{:?}{}{:?}",
            s.show_setup,
            s.session.is_some(),
            s.picking_project,
            s.selected_project.is_some(),
            s.page,
            s.tab,
            s.service_tab,
            s.active_tab_id,
            s.selected_service
        )
        .hash(&mut h);
        h.finish()
    };
    ui::pill::begin_view(screen);
    ui::theme::set_dark(s.dark);
    i18n::set_thai(s.lang_th);
    let k = ui::theme::warm_dark_kit();
    let base = if s.show_setup {
        ui::setup::view(s, k)
    } else if s.session.is_some() && s.picking_project {
        ui::project_picker::view(s, k)
    } else if s.session.is_some() {
        ui::app_shell(s, k)
    } else {
        ui::login::view(s, k)
    };
    let mut layer = Vec::new();
    if s.svc_picker_open && s.session.is_some() && s.page == Page::ServiceDetail {
        layer.push(rune_kit::scrim("svc-picker-scrim", |s: &mut AppState| s.close_svc_picker()));
        layer.push(ui::service_detail::service_picker(s, k));
    }
    rune_kit::root(base, layer)
}

fn scene_overview(s: &mut AppState) {
    s.session = Some(auth::Session {
        username: "nootrax.x04@gmail.com".into(),
        display_name: "nootrax.x04@gmail.com".into(),
        role: auth::Role::Admin,
    });
    s.page = Page::Overview;
}

fn scene_topology(s: &mut AppState) {
    scene_overview(s);
    s.drill_into_namespace(k8s::EnvKind::Stg, "staging-api");
    s.tab = Tab::Topology;
    s.selected_service = Some("staging-pg-srv".into());
}

fn scene_service_detail(s: &mut AppState) {
    scene_overview(s);
    s.drill_into_namespace(k8s::EnvKind::Stg, "staging-api");
    s.drill_into_service("staging-pg-srv".into(), ServiceTab::Env);
}

/// An app launched from Finder gets a bare PATH (no Homebrew, no gcloud) and `/` as its
/// working directory. Add the usual tool locations and keep settings under Application Support.
#[cfg(target_os = "macos")]
fn prepare_macos_environment() {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut dirs: Vec<String> = vec![
        "/opt/homebrew/bin".into(),
        "/opt/homebrew/sbin".into(),
        "/usr/local/bin".into(),
        "/usr/local/sbin".into(),
        "/usr/local/share/google-cloud-sdk/bin".into(),
    ];
    if !home.is_empty() {
        dirs.push(format!("{home}/google-cloud-sdk/bin"));
    }
    if let Ok(entries) = std::fs::read_dir("/opt/homebrew/Caskroom/gcloud-cli") {
        for e in entries.flatten() {
            dirs.push(format!("{}/google-cloud-sdk/bin", e.path().display()));
        }
    }
    for d in std::env::var("PATH").unwrap_or_default().split(':') {
        if !d.is_empty() {
            dirs.push(d.to_string());
        }
    }
    let mut unique: Vec<String> = Vec::new();
    for d in dirs {
        if !unique.contains(&d) {
            unique.push(d);
        }
    }
    // SAFETY: called first thing in main, before any other thread exists.
    unsafe { std::env::set_var("PATH", unique.join(":")) };

    let from_bundle = std::env::current_exe()
        .map(|p| p.to_string_lossy().contains(".app/Contents/MacOS"))
        .unwrap_or(false);
    if from_bundle && !home.is_empty() {
        let dir = std::path::PathBuf::from(home).join("Library/Application Support/Klar");
        if std::fs::create_dir_all(&dir).is_ok() {
            let _ = std::env::set_current_dir(&dir);
        }
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    prepare_macos_environment();
    tools::add_to_path();
    i18n::set_thai(state::load_lang_thai());

    let app = AppSpec {
        min_size: Some((1020.0, 660.0)),
        scenes: &[
            ("overview", scene_overview),
            ("topology", scene_topology),
            ("service", scene_service_detail),
        ],
        fonts: ui::theme::FONTS,
        families: ui::theme::FAMILIES,
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
        ..AppSpec::new("Klar", (1340.0, 860.0), AppState::init, view)
    };

    rune::run_windows(|_| {
        let win = WindowSpec::new("main", app)
            .options(WindowOptions {
                borderless: true,
                ..Default::default()
            })
            .on_window(|handle| {
                ui::topology_g6::attach_window(handle.clone());
            })
            .on_frame_drawn(|rt| {
                ui::topology_g6::sync_webview(&rt.state);
            })
            .on_visibility(|s, visible, _| {
                s.g6_window_visible = visible;
                if !visible {
                    ui::topology_g6::hide_webview();
                }
            })
            .on_close(|s, _| {
                s.g6_window_visible = false;
                ui::topology_g6::destroy_webview();
            })
            .on_quit(|s, _| {
                s.g6_window_visible = false;
                ui::topology_g6::destroy_webview();
            });
        [win]
    });
}
