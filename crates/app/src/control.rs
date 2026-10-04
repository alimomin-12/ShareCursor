//! Shared control state between the capture thread and the network pump.
//!
//! SYMMETRIC (ShareMouse-style) model: both machines always capture their own
//! input AND inject the peer's. Exactly one pointer is "away" at a time:
//!
//!  * `my_away`   — MY pointer crossed onto the peer's screen: my physical
//!    input is suppressed locally and forwarded; my cursor is hidden/parked.
//!  * `peer_away` — the PEER's pointer is on MY screen: their forwarded input
//!    is injected here and drives my real cursor. My own physical input still
//!    works locally (local-first, inputs merge like ShareMouse).
//!
//! Capture flips these on edge hits / hotkeys; the network pump diff-detects
//! and sends the matching `PointerEnter` / `PointerEnd` messages.

use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use sharecursor_protocol::Edge;

/// Shared, thread-safe control state.
pub struct Control {
    /// My pointer is on the peer's screen (forward my input, hide my cursor).
    pub my_away: AtomicBool,
    /// The peer's pointer is on my screen (their input is injected here).
    pub peer_away: AtomicBool,
    /// How my pointer last went away:
    ///  * `Some((edge, perp))` — crossed a screen edge at that perpendicular px.
    ///  * `None` — manual hotkey toggle (enter at the peer's centre).
    pub entry: Mutex<Option<(Edge, i32)>>,
    /// Where my cursor re-appears when my pointer comes home: border edge +
    /// perpendicular pixel. `None` = stay where it is / centre.
    pub return_to: Mutex<Option<(Edge, i32)>>,
    /// Set by capture when the VISITING pointer (peer's) crossed back home at
    /// this my-local perpendicular pixel; the pump maps + sends `PointerEnd`.
    pub send_peer_home: Mutex<Option<i32>>,
    /// While `peer_away`: the edge the visitor entered through + the span along
    /// it where crossing back is allowed (from its `PointerEnter`).
    pub host_span: Mutex<Option<(Edge, (i32, i32))>>,
    /// While `peer_away`: the visitor must move some distance IN from the entry
    /// edge before a return-crossing is allowed — otherwise it bounces straight
    /// back out (jitter/ping-pong at the border).
    pub host_armed: AtomicBool,
}

impl Control {
    /// Track the visiting pointer from both physical capture and injected
    /// movement. Windows intentionally excludes injected events from its hooks.
    pub fn visitor_position(&self, x: i32, y: i32, screen: (u32, u32)) {
        use std::sync::atomic::Ordering;
        if !self.peer_away.load(Ordering::Relaxed) {
            return;
        }
        let Some((edge, span)) = *self.host_span.lock().unwrap() else {
            return;
        };
        let (perp, distance) = match edge {
            Edge::Left => (y, x),
            Edge::Right => (y, screen.0 as i32 - 1 - x),
            Edge::Top => (x, y),
            Edge::Bottom => (x, screen.1 as i32 - 1 - y),
        };
        if distance > 60 {
            self.host_armed.store(true, Ordering::Relaxed);
        }
        if distance <= 0
            && self.host_armed.load(Ordering::Relaxed)
            && crate::edge::in_span(perp, span)
        {
            self.peer_away.store(false, Ordering::Relaxed);
            *self.send_peer_home.lock().unwrap() = Some(perp);
        }
    }
    pub fn new() -> Self {
        Self {
            my_away: AtomicBool::new(false),
            peer_away: AtomicBool::new(false),
            entry: Mutex::new(None),
            return_to: Mutex::new(None),
            send_peer_home: Mutex::new(None),
            host_span: Mutex::new(None),
            host_armed: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn injected_pointer_can_return_without_being_recaptured() {
        let c = Control::new();
        c.peer_away.store(true, Ordering::Relaxed);
        *c.host_span.lock().unwrap() = Some((Edge::Left, (100, 800)));
        c.visitor_position(2, 400, (1920, 1080));
        assert!(c.peer_away.load(Ordering::Relaxed));
        c.visitor_position(100, 400, (1920, 1080));
        c.visitor_position(0, 50, (1920, 1080));
        assert!(c.peer_away.load(Ordering::Relaxed));
        c.visitor_position(0, 400, (1920, 1080));
        assert!(!c.peer_away.load(Ordering::Relaxed));
        assert_eq!(*c.send_peer_home.lock().unwrap(), Some(400));
    }
}

impl Default for Control {
    fn default() -> Self {
        Self::new()
    }
}
