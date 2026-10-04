//! Menu-bar / system-tray front-end.
//!
//! * **macOS:** a status item in the top-right menu bar (no dock icon — the app
//!   runs as an "accessory").
//! * **Windows:** a system-tray icon.
//!
//! The menu lets you start the server or client, jump to the settings +
//! monitor-manager file, and quit. The heavy GUI event loop lives here and on
//! macOS must own the main thread, so the actual server/client run in spawned
//! threads.

#![cfg(feature = "tray")]

use std::path::PathBuf;

use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

use crate::config::Config;

/// Messages pumped into the tao event loop.
enum UserEvent {
    Menu(MenuEvent),
    SettingsClosed(std::io::Result<std::process::ExitStatus>),
    PairFailed(String),
}

/// Launch the tray/menu-bar app. Blocks running the event loop.
pub fn run() -> anyhow::Result<()> {
    let config_path = Config::default_path();
    let first_run = !config_path.exists();
    Config::load_or_create(&config_path)?;

    let event_loop = build_event_loop();

    // Forward menu events into the loop so it wakes without busy-polling.
    let proxy = event_loop.create_proxy();
    let menu_proxy = proxy.clone();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = menu_proxy.send_event(UserEvent::Menu(event));
    }));

    // Menu items — ids captured so we can match clicks.
    let item_status = MenuItem::new("ShareCursor — idle", false, None);
    let item_start = MenuItem::new("Start (find & connect the other PC)", true, None);
    let item_settings = MenuItem::new("Settings & Monitor Manager…", true, None);
    let item_quit = MenuItem::new("Quit ShareCursor", true, None);

    let id_start = item_start.id().clone();
    let id_settings = item_settings.id().clone();
    let id_quit = item_quit.id().clone();

    let menu = Menu::new();
    menu.append(&item_status)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&item_start)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&item_settings)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&item_quit)?;

    // On a fresh installation, finish setup before pairing reads the passphrase.
    let mut pairing = false;
    let mut settings_open = false;

    // The tray icon must be created after the loop starts on macOS, so we build
    // it lazily on the first `Init` event and keep it alive here (RAII).
    let mut _tray = None;
    let menu_holder = menu;

    event_loop.run(move |event, _target, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(tao::event::StartCause::Init) => {
                let icon = brand_icon();
                match TrayIconBuilder::new()
                    .with_tooltip("ShareCursor — low-latency KVM")
                    .with_menu(Box::new(menu_holder.clone()))
                    .with_icon(icon)
                    .build()
                {
                    Ok(t) => {
                        _tray = Some(t);
                        #[cfg(target_os = "macos")]
                        wake_macos_run_loop();

                        if first_run {
                            item_status.set_text("ShareCursor — finish setup in Settings");
                            match open_settings(&config_path, proxy.clone()) {
                                Ok(()) => {
                                    settings_open = true;
                                    item_start.set_enabled(false);
                                }
                                Err(e) => item_status.set_text(format!("Setup failed: {e}")),
                            }
                        } else {
                            pairing = true;
                            item_start.set_enabled(false);
                            item_status.set_text("ShareCursor — pairing…");
                            spawn_pair(proxy.clone());
                        }
                    }
                    Err(e) => {
                        eprintln!("failed to create tray icon: {e}");
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
            Event::UserEvent(UserEvent::Menu(ev)) => {
                if ev.id == id_quit {
                    *control_flow = ControlFlow::Exit;
                } else if ev.id == id_start {
                    if !pairing && !settings_open {
                        pairing = true;
                        item_start.set_enabled(false);
                        item_status.set_text("ShareCursor — pairing…");
                        spawn_pair(proxy.clone());
                    }
                } else if ev.id == id_settings {
                    if !settings_open {
                        match open_settings(&config_path, proxy.clone()) {
                            Ok(()) => {
                                settings_open = true;
                                item_start.set_enabled(false);
                            }
                            Err(e) => item_status.set_text(format!("Settings failed: {e}")),
                        }
                    }
                }
            }
            Event::UserEvent(UserEvent::SettingsClosed(result)) => {
                settings_open = false;
                match result {
                    Ok(status) if status.success() && !pairing => {
                        pairing = true;
                        item_status.set_text("ShareCursor — pairing…");
                        spawn_pair(proxy.clone());
                    }
                    Ok(status) if !status.success() => {
                        item_status.set_text("ShareCursor — Settings failed to open");
                    }
                    Err(e) => item_status.set_text(format!("Settings failed: {e}")),
                    _ => {}
                }
                item_start.set_enabled(!pairing);
            }
            Event::UserEvent(UserEvent::PairFailed(error)) => {
                pairing = false;
                item_start.set_enabled(!settings_open);
                item_status.set_text(format!("Pairing failed: {error}"));
            }
            _ => {}
        }
    });
}

/// A windowless Tao loop needs a wake-up after creating the status item or
/// macOS can leave it invisible until another UI event occurs.
#[cfg(target_os = "macos")]
fn wake_macos_run_loop() {
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRunLoopGetMain() -> *mut std::ffi::c_void;
        fn CFRunLoopWakeUp(run_loop: *mut std::ffi::c_void);
    }
    // SAFETY: CoreFoundation returns the process's live main run loop; this
    // function is called on that thread during the initial Tao event.
    unsafe {
        CFRunLoopWakeUp(CFRunLoopGetMain());
    }
}

#[cfg(target_os = "macos")]
fn build_event_loop() -> tao::event_loop::EventLoop<UserEvent> {
    use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    // Accessory = menu-bar app with no dock icon.
    event_loop.set_activation_policy(ActivationPolicy::Accessory);
    event_loop
}

#[cfg(not(target_os = "macos"))]
fn build_event_loop() -> tao::event_loop::EventLoop<UserEvent> {
    EventLoopBuilder::<UserEvent>::with_user_event().build()
}

/// Zero-config auto-pairing in the background (find the peer + connect).
fn spawn_pair(proxy: EventLoopProxy<UserEvent>) {
    std::thread::spawn(move || {
        if let Err(e) = crate::run::pair() {
            tracing::error!(error = %e, "pairing stopped");
            let _ = proxy.send_event(UserEvent::PairFailed(e.to_string()));
        }
    });
}

/// Open the visual settings window (a separate `sharecursor settings` process,
/// so it has its own event loop). Falls back to opening the config file.
fn open_settings(_path: &PathBuf, proxy: EventLoopProxy<UserEvent>) -> anyhow::Result<()> {
    #[cfg(feature = "gui")]
    let mut child = {
        let mut command = std::process::Command::new(std::env::current_exe()?);
        command.arg("settings");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        command.spawn()?
    };
    #[cfg(not(feature = "gui"))]
    let mut child = open_path(_path)?;
    std::thread::spawn(move || {
        let _ = proxy.send_event(UserEvent::SettingsClosed(child.wait()));
    });
    Ok(())
}

#[cfg(all(not(feature = "gui"), target_os = "macos"))]
fn open_path(path: &PathBuf) -> std::io::Result<std::process::Child> {
    std::process::Command::new("open").arg(path).spawn()
}

#[cfg(all(not(feature = "gui"), target_os = "windows"))]
fn open_path(path: &PathBuf) -> std::io::Result<std::process::Child> {
    std::process::Command::new("explorer").arg(path).spawn()
}

#[cfg(all(
    not(feature = "gui"),
    not(target_os = "macos"),
    not(target_os = "windows")
))]
fn open_path(path: &PathBuf) -> std::io::Result<std::process::Child> {
    std::process::Command::new("xdg-open").arg(path).spawn()
}

/// The ShareCursor brand icon (a blue cursor-click glyph) — pre-rendered to raw
/// 64×64 RGBA and embedded so we ship no image files or SVG renderer.
fn brand_icon() -> Icon {
    const S: u32 = 64;
    let rgba = include_bytes!("tray_icon_64.rgba").to_vec();
    Icon::from_rgba(rgba, S, S).expect("valid rgba icon")
}
