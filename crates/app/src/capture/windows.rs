//! Low-level Windows hooks with injection filtering and a real message loop.
//! A suppressed move is relative to the parked OS cursor, not the last hook
//! position (the latter makes repeated equal physical moves cancel to zero).

use rdev::{Button, Event, EventType, Key};
use std::cell::RefCell;
use std::time::SystemTime;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

type Callback = Box<dyn FnMut(Event) -> Option<Event>>;
thread_local! { static CALLBACK: RefCell<Option<Callback>> = RefCell::new(None); }

pub fn position() -> Option<(i32, i32)> {
    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: point is a live, writable POINT.
    (unsafe { GetCursorPos(&mut point) } != 0).then_some((point.x, point.y))
}

pub fn park(x: i32, y: i32) {
    unsafe {
        SetCursorPos(x, y);
    }
}

pub fn motion_delta(point: (i32, i32), cursor: (i32, i32)) -> (i32, i32) {
    (point.0 - cursor.0, point.1 - cursor.1)
}

unsafe fn dispatch(code: i32, param: WPARAM, data: LPARAM, mouse: bool) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(std::ptr::null_mut(), code, param, data);
    }
    let event_type = if mouse {
        let m = &*(data as *const MSLLHOOKSTRUCT);
        // Peer input and our own parking warps must reach Windows, but must
        // never trigger an outgoing hand-off or echo back to the other machine.
        if m.flags & LLMHF_INJECTED != 0 {
            return CallNextHookEx(std::ptr::null_mut(), code, param, data);
        }
        let pressed = matches!(
            param as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        );
        let button = match param as u32 {
            WM_LBUTTONDOWN | WM_LBUTTONUP => Some(Button::Left),
            WM_RBUTTONDOWN | WM_RBUTTONUP => Some(Button::Right),
            WM_MBUTTONDOWN | WM_MBUTTONUP => Some(Button::Middle),
            WM_XBUTTONDOWN | WM_XBUTTONUP => Some(Button::Unknown(
                ((m.mouseData >> 16) as u8).saturating_sub(1),
            )),
            _ => None,
        };
        if let Some(button) = button {
            Some(if pressed {
                EventType::ButtonPress(button)
            } else {
                EventType::ButtonRelease(button)
            })
        } else {
            match param as u32 {
                WM_MOUSEMOVE => Some(EventType::MouseMove {
                    x: m.pt.x as f64,
                    y: m.pt.y as f64,
                }),
                WM_MOUSEWHEEL => Some(EventType::Wheel {
                    delta_x: 0,
                    delta_y: ((m.mouseData >> 16) as i16 / 120) as i64,
                }),
                WM_MOUSEHWHEEL => Some(EventType::Wheel {
                    delta_x: ((m.mouseData >> 16) as i16 / 120) as i64,
                    delta_y: 0,
                }),
                _ => None,
            }
        }
    } else {
        let k = &*(data as *const KBDLLHOOKSTRUCT);
        if k.flags & LLKHF_INJECTED != 0 {
            return CallNextHookEx(std::ptr::null_mut(), code, param, data);
        }
        let key = key_from_vk(k.vkCode, k.scanCode, k.flags & LLKHF_EXTENDED != 0);
        match param as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => Some(EventType::KeyPress(key)),
            WM_KEYUP | WM_SYSKEYUP => Some(EventType::KeyRelease(key)),
            _ => None,
        }
    };
    if let Some(event_type) = event_type {
        let consumed = CALLBACK.with(|slot| {
            // A parking warp can re-enter the hook; leave it to the OS.
            let Ok(mut slot) = slot.try_borrow_mut() else {
                return false;
            };
            slot.as_mut().is_some_and(|callback| {
                callback(Event {
                    time: SystemTime::now(),
                    name: None,
                    event_type,
                })
                .is_none()
            })
        });
        if consumed {
            return 1;
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, param, data)
}

unsafe extern "system" fn mouse_hook(code: i32, param: WPARAM, data: LPARAM) -> LRESULT {
    dispatch(code, param, data, true)
}
unsafe extern "system" fn key_hook(code: i32, param: WPARAM, data: LPARAM) -> LRESULT {
    dispatch(code, param, data, false)
}

struct Hook(HHOOK);
impl Drop for Hook {
    fn drop(&mut self) {
        unsafe {
            UnhookWindowsHookEx(self.0);
        }
    }
}

pub fn grab(callback: impl FnMut(Event) -> Option<Event> + 'static) -> anyhow::Result<()> {
    CALLBACK.with(|slot| *slot.borrow_mut() = Some(Box::new(callback)));
    let result = (|| {
        // SAFETY: both callbacks live for the message-loop lifetime. Hooks are
        // unregistered by their guards before this thread's callback is cleared.
        unsafe {
            let module = GetModuleHandleW(std::ptr::null());
            let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), module, 0);
            if mouse.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            let _mouse = Hook(mouse);
            let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_hook), module, 0);
            if keyboard.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            let _keyboard = Hook(keyboard);
            let mut message: MSG = std::mem::zeroed();
            loop {
                match GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) {
                    -1 => return Err(std::io::Error::last_os_error().into()),
                    0 => return Ok(()),
                    _ => {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
            }
        }
    })();
    CALLBACK.with(|slot| *slot.borrow_mut() = None);
    result
}

fn key_from_vk(vk: u32, scan: u32, extended: bool) -> Key {
    use Key::*;
    const LETTERS: [Key; 26] = [
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
        KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    ];
    const DIGITS: [Key; 10] = [Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9];
    const FUNCTIONS: [Key; 12] = [F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12];
    match vk {
        0x41..=0x5a => LETTERS[(vk - 0x41) as usize],
        0x30..=0x39 => DIGITS[(vk - 0x30) as usize],
        0x70..=0x7b => FUNCTIONS[(vk - 0x70) as usize],
        0x10 if scan == 0x36 => ShiftRight,
        0x10 | 0xa0 => ShiftLeft,
        0xa1 => ShiftRight,
        0x11 if extended => ControlRight,
        0x11 | 0xa2 => ControlLeft,
        0xa3 => ControlRight,
        0x12 if extended => AltGr,
        0x12 | 0xa4 => Alt,
        0xa5 => AltGr,
        0x08 => Backspace,
        0x09 => Tab,
        0x0d => Return,
        0x14 => CapsLock,
        0x1b => Escape,
        0x20 => Space,
        0x21 => PageUp,
        0x22 => PageDown,
        0x23 => End,
        0x24 => Home,
        0x25 => LeftArrow,
        0x26 => UpArrow,
        0x27 => RightArrow,
        0x28 => DownArrow,
        0x2d => Insert,
        0x2e => Delete,
        0x5b => MetaLeft,
        0x5c => MetaRight,
        0xba => SemiColon,
        0xbb => Equal,
        0xbc => Comma,
        0xbd => Minus,
        0xbe => Dot,
        0xbf => Slash,
        0xc0 => BackQuote,
        0xdb => LeftBracket,
        0xdc => BackSlash,
        0xdd => RightBracket,
        0xde => Quote,
        _ => Unknown(vk),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_moves_from_a_parked_cursor_do_not_cancel_out() {
        assert_eq!(motion_delta((965, 540), (960, 540)), (5, 0));
        assert_eq!(motion_delta((965, 540), (960, 540)), (5, 0));
        assert_eq!(motion_delta((952, 542), (960, 540)), (-8, 2));
    }
    #[test]
    fn modifiers_are_resolved_for_both_sides() {
        assert_eq!(key_from_vk(0x10, 0x36, false), Key::ShiftRight);
        assert_eq!(key_from_vk(0x11, 0, true), Key::ControlRight);
        assert_eq!(key_from_vk(0x43, 0, false), Key::KeyC);
    }
}
