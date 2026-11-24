//! The Klar owl loading animation for native (Rune) screens.
//!
//! Same choreography as the G6 web loader: the owl is drawn part by part (brow, left eye,
//! right eye, pupils, beak, body), then floats up and down while glancing and blinking.
//! Frames are pre-built SVG icons (one per animation step) so drawing a frame is just picking
//! the right cached icon.

use crate::ui::{icons, theme};
use rune::{El, Icon, col, text};
use rune_kit::Kit;
use std::sync::OnceLock;

const BROW: &str = "M196,236 C296,320 410,340 512,330 C614,340 728,320 828,236 C822,352 762,410 672,432 C600,450 550,480 532,526 Q512,574 492,526 C474,480 424,450 352,432 C262,410 202,352 196,236 Z";
const RING_L: &str = "M490.6,538.3 A122,122 0 1,1 282.5,501.6";
const RING_R: &str = "M533.4,538.3 A122,122 0 1,0 741.5,501.6";
const BEAK: &str = "M480,644 L544,644 L512,700 Z";
const BODY_L: &str = "M226,690 C246,790 320,850 420,880";
const BODY_R: &str = "M798,690 C778,790 704,850 604,880";
const LEN_BROW: f32 = 1637.6;
const LEN_RING: f32 = 511.0;
const LEN_BODY: f32 = 286.6;

/// Seconds the "draw it line by line" intro takes; after that the idle loop runs.
pub const INTRO_SECS: f32 = 1.9;
const INTRO_FPS: f32 = 30.0;
/// One idle cycle covers both the float (2.4 s) and the glance/blink (3.6 s) periods.
const LOOP_SECS: f32 = 7.2;
const LOOP_FPS: f32 = 24.0;

fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

/// Progress (0..1) of a part that starts at `start` and takes `dur` seconds.
fn part(t: f32, start: f32, dur: f32) -> f32 {
    let p = clamp01((t - start) / dur);
    1.0 - (1.0 - p) * (1.0 - p) // ease-out
}

/// A stroke that draws itself: dash offset shrinks as `p` goes 0 → 1.
fn drawn(d: &str, len: f32, p: f32, width: f32) -> String {
    format!(
        r#"<path d="{d}" fill="none" stroke="currentColor" stroke-width="{width}" stroke-linecap="round" stroke-linejoin="round" stroke-dasharray="{len}" stroke-dashoffset="{off:.1}" opacity="{op:.2}"/>"#,
        off = len * (1.0 - p),
        op = clamp01(p * 4.0),
    )
}

/// Horizontal pupil offset of the glance (matches the CSS keyframes of the web loader).
fn glance(cycle: f32) -> f32 {
    let k = cycle / 3.6;
    let lerp = |a: f32, b: f32, x: f32| a + (b - a) * x;
    let ease = |x: f32| 0.5 - 0.5 * (std::f32::consts::PI * x).cos();
    match k {
        k if k < 0.15 => 0.0,
        k if k < 0.25 => lerp(0.0, -22.0, ease((k - 0.15) / 0.10)),
        k if k < 0.40 => -22.0,
        k if k < 0.55 => lerp(-22.0, 22.0, ease((k - 0.40) / 0.15)),
        k if k < 0.70 => 22.0,
        k if k < 0.80 => lerp(22.0, 0.0, ease((k - 0.70) / 0.10)),
        _ => 0.0,
    }
}

/// Vertical pupil scale for the blink.
fn blink(cycle: f32) -> f32 {
    let k = cycle / 3.6;
    if (0.44..0.50).contains(&k) {
        let x = (k - 0.44) / 0.06; // 0 → 1
        1.0 - 0.92 * (1.0 - (2.0 * x - 1.0).abs())
    } else {
        1.0
    }
}

fn frame_svg(t: f32) -> String {
    let intro = t < INTRO_SECS;
    // Idle-loop state (only after the intro).
    let lt = (t - INTRO_SECS).max(0.0);
    let bob = if intro { 0.0 } else { -6.0 * 0.5 * (1.0 - (2.0 * std::f32::consts::PI * lt / 2.4).cos()) };
    let cycle = lt % 3.6;
    let (gx, sy) = if intro { (0.0, 1.0) } else { (glance(cycle), blink(cycle)) };

    let brow = part(t, 0.0, 0.5);
    let brow_fill = part(t, 0.45, 0.3);
    let ring_l = part(t, 0.45, 0.45);
    let ring_r = part(t, 0.70, 0.45);
    let pupil_l = part(t, 1.00, 0.3);
    let pupil_r = part(t, 1.10, 0.3);
    let beak = part(t, 1.25, 0.3);
    let body = part(t, 1.35, 0.45);

    // Pupils: scale around their own centre for the pop-in, around the eye line for the blink.
    let pupil = |cx: f32, p: f32| {
        format!(
            r#"<g transform="translate({cx},580) scale({p:.3},{sp:.3}) translate({ncx},-580)"><circle cx="{cx}" cy="580" r="60" fill="currentColor" opacity="{op:.2}"/></g>"#,
            sp = p * sy,
            ncx = -cx,
            op = clamp01(p * 3.0),
        )
    };

    format!(
        r#"<g transform="translate(-132,{ty:.1})">
<path d="{BROW}" fill="currentColor" fill-opacity="{bf:.2}" stroke="currentColor" stroke-width="34" stroke-linejoin="round" stroke-dasharray="{LEN_BROW}" stroke-dashoffset="{bo:.1}" opacity="{bop:.2}"/>
{rl}{rr}
<g transform="translate({gx:.1},0)">{pl}{pr}</g>
<path d="{BEAK}" fill="currentColor" stroke="currentColor" stroke-width="22" stroke-linejoin="round" opacity="{beak:.2}"/>
{bl}{br}
</g>"#,
        ty = -182.0 + bob,
        bf = brow_fill,
        bo = LEN_BROW * (1.0 - brow),
        bop = clamp01(brow * 4.0),
        rl = drawn(RING_L, LEN_RING, ring_l, 46.0),
        rr = drawn(RING_R, LEN_RING, ring_r, 46.0),
        pl = pupil(376.0, pupil_l),
        pr = pupil(648.0, pupil_r),
        bl = drawn(BODY_L, LEN_BODY, body, 46.0),
        br = drawn(BODY_R, LEN_BODY, body, 46.0),
    )
}

fn leak_icon(name: String, body: String) -> &'static Icon {
    Box::leak(Box::new(Icon {
        name: Box::leak(name.into_boxed_str()),
        view: 760.0,
        body: Box::leak(body.into_boxed_str()),
    }))
}

fn frames() -> &'static (Vec<&'static Icon>, Vec<&'static Icon>) {
    static FRAMES: OnceLock<(Vec<&'static Icon>, Vec<&'static Icon>)> = OnceLock::new();
    FRAMES.get_or_init(|| {
        let intro_n = (INTRO_SECS * INTRO_FPS).ceil() as usize;
        let loop_n = (LOOP_SECS * LOOP_FPS).round() as usize;
        let intro = (0..intro_n)
            .map(|i| leak_icon(format!("klar-owl-intro-{i}"), frame_svg(i as f32 / INTRO_FPS)))
            .collect();
        let idle = (0..loop_n)
            .map(|i| leak_icon(format!("klar-owl-loop-{i}"), frame_svg(INTRO_SECS + i as f32 / LOOP_FPS)))
            .collect();
        (intro, idle)
    })
}

/// The icon for `t` seconds after the loader appeared.
pub fn frame_at(t: f32) -> &'static Icon {
    let (intro, idle) = frames();
    if t < INTRO_SECS {
        intro[((t * INTRO_FPS) as usize).min(intro.len() - 1)]
    } else {
        idle[(((t - INTRO_SECS) * LOOP_FPS) as usize) % idle.len()]
    }
}

/// Full-page loader: animated owl and a "loading…" line. `t` = seconds since it appeared.
pub fn view(t: f32, label: impl Into<String>, k: &Kit) -> El {
    sized(t, label, 96.0, 260.0, k)
}

/// Compact loader for use inside a card or list.
pub fn small(t: f32, label: impl Into<String>, k: &Kit) -> El {
    sized(t, label, 44.0, 110.0, k)
}

fn sized(t: f32, label: impl Into<String>, owl: f32, min_h: f32, k: &Kit) -> El {
    let dots = ".".repeat(1 + ((t * 2.5) as usize % 3));
    col()
        .key("owl-loader")
        .w_full()
        .min_h(min_h)
        .grow(1.0)
        .gap(if owl < 60.0 { 8.0 } else { 14.0 })
        .center()
        .child(icons::mono_icon(frame_at(t), owl, theme::brand_mark()))
        .child(
            text(format!("{}{dots}", label.into()))
                .font(k.ty.text_small)
                .color(theme::text_muted()),
        )
}
