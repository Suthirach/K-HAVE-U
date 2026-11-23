mod auth;
mod k8s;
mod pty_k9s;
mod state;
mod ui;

use rune::{AppSpec, Cx, El, Key, KeyEvent, Tx, WindowOptions, WindowSpec};
use rune::runtime::Waker;
use state::{AppState, Page, ServiceTab, Tab};

fn setup(s: &mut AppState, waker: Waker) {
    s.waker = Some(waker);
}

fn poll(s: &mut AppState, t: f64, tx: &mut Tx) -> bool {
    let mut changed = s.drain_events();
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
    if s.session.is_none() {
        if matches!(ev.key, Key::Enter) {
            s.try_login();
            return true;
        }
        return false;
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

fn view(s: &AppState, cx: &mut Cx<AppState>) -> El {
    let k = rune_theme_opencode::kit(s.dark && cx.system_dark() || s.dark);
    let base = if s.session.is_some() {
        ui::app_shell(s, k)
    } else {
        ui::login::view(s, k)
    };
    rune_kit::root(base, Vec::new())
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

fn main() {
    let app = AppSpec {
        min_size: Some((1020.0, 660.0)),
        scenes: &[
            ("overview", scene_overview),
            ("topology", scene_topology),
            ("service", scene_service_detail),
        ],
        fonts: rune_theme_opencode::FONTS,
        families: rune_theme_opencode::FAMILIES,
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
        ..AppSpec::new("K9s Studio", (1340.0, 860.0), AppState::init, view)
    };

    rune::run_windows(|_| {
        let win = WindowSpec::new("main", app).options(WindowOptions {
            borderless: true,
            ..Default::default()
        });
        [win]
    });
}
