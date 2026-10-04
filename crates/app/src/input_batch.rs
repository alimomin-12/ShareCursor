//! Bound input packets and coalesce only consecutive motion events, preserving
//! the ordering of button and keyboard transitions.

use sharecursor_protocol::InputEvent;
use std::sync::mpsc::Receiver;

const MAX_EVENTS: usize = 32;
const MAX_CAPTURE_EVENTS: usize = 512;

#[cfg(windows)]
pub struct WindowsTimer;
#[cfg(windows)]
pub fn windows_timer() -> WindowsTimer {
    unsafe {
        windows_sys::Win32::Media::timeBeginPeriod(1);
    }
    WindowsTimer
}
#[cfg(windows)]
impl Drop for WindowsTimer {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Media::timeEndPeriod(1);
        }
    }
}

pub fn drain(rx: &Receiver<InputEvent>) -> Vec<InputEvent> {
    let mut batch = Vec::with_capacity(MAX_EVENTS);
    for _ in 0..MAX_CAPTURE_EVENTS {
        if batch.len() == MAX_EVENTS {
            break;
        }
        let Ok(event) = rx.try_recv() else { break };
        if let (Some(InputEvent::MouseMove { dx, dy }), InputEvent::MouseMove { dx: x, dy: y }) =
            (batch.last_mut(), event)
        {
            if let (Some(x), Some(y)) = (dx.checked_add(x), dy.checked_add(y)) {
                *dx = x;
                *dy = y;
                continue;
            }
        }
        batch.push(event);
    }
    batch
}

#[cfg(test)]
mod tests {
    use super::*;
    use sharecursor_protocol::{InputMsg, InputPacket, Key, MouseButton};
    use std::sync::mpsc;

    #[test]
    fn high_rate_motion_is_coalesced_without_losing_distance() {
        let (tx, rx) = mpsc::channel();
        for _ in 0..8000 {
            tx.send(InputEvent::MouseMove { dx: 2, dy: -1 }).unwrap();
        }
        drop(tx);
        let (mut x, mut y, mut packets) = (0, 0, 0);
        loop {
            let batch = drain(&rx);
            if batch.is_empty() {
                break;
            }
            assert_eq!(batch.len(), 1);
            if let InputEvent::MouseMove { dx, dy } = batch[0] {
                x += dx;
                y += dy;
            }
            packets += 1;
        }
        assert_eq!((x, y), (16000, -8000));
        assert_eq!(packets, 16);
    }

    #[test]
    fn clicks_and_keys_keep_their_position_between_motions() {
        let (tx, rx) = mpsc::channel();
        let click = InputEvent::MouseButton {
            button: MouseButton::Left,
            pressed: true,
        };
        let key = InputEvent::Key {
            key: Key::A,
            pressed: false,
        };
        for event in [
            InputEvent::MouseMove { dx: 1, dy: 2 },
            click,
            InputEvent::MouseMove { dx: 3, dy: 4 },
            key,
        ] {
            tx.send(event).unwrap();
        }
        assert_eq!(
            drain(&rx),
            vec![
                InputEvent::MouseMove { dx: 1, dy: 2 },
                click,
                InputEvent::MouseMove { dx: 3, dy: 4 },
                key
            ]
        );
    }

    #[test]
    fn bursts_fit_the_receive_buffer_and_leave_work_for_the_next_tick() {
        let (tx, rx) = mpsc::channel();
        for _ in 0..1000 {
            tx.send(InputEvent::Key {
                key: Key::Unknown(u32::MAX),
                pressed: true,
            })
            .unwrap();
        }
        let batch = drain(&rx);
        assert_eq!(batch.len(), MAX_EVENTS);
        let bytes = InputPacket {
            seq: 1,
            msg: InputMsg::Events(batch),
        }
        .encode()
        .unwrap();
        assert!(bytes.len() + 20 < 2048);
        assert_eq!(drain(&rx).len(), MAX_EVENTS);
    }
}
