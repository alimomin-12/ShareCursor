//! Server (`serve`) and client (`connect`) run loops wiring capture + transport
//! + injection + encryption together. Native-only (needs input capture).
//!
//! Session bring-up:
//!  1. A TCP handshake (X25519 + PSK) authenticates the peers and derives two
//!     encrypted sessions — one for the UDP input channel, one for the TCP bulk
//!     channel (clipboard + files).
//!  2. The input session keys the UDP channel; the bulk session keys the TCP
//!     connection. From then on every byte on the wire is authenticated
//!     ChaCha20-Poly1305.

use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use sharecursor_protocol::crypto::Role;
use sharecursor_protocol::{BulkMsg, Edge, InputEvent, InputMsg};

/// A batch that releases every modifier key on the client. Sent on every
/// control hand-off so a modifier held during the switch can't stay stuck down
/// on the other machine (the classic "Alt+Tab / Ctrl stuck" bug).
fn release_all_modifiers() -> InputMsg {
    use sharecursor_protocol::Key::{LAlt, LCtrl, LMeta, LShift, RAlt, RCtrl, RMeta, RShift};
    InputMsg::Events(vec![
        InputEvent::Key {
            key: LCtrl,
            pressed: false,
        },
        InputEvent::Key {
            key: RCtrl,
            pressed: false,
        },
        InputEvent::Key {
            key: LAlt,
            pressed: false,
        },
        InputEvent::Key {
            key: RAlt,
            pressed: false,
        },
        InputEvent::Key {
            key: LShift,
            pressed: false,
        },
        InputEvent::Key {
            key: RShift,
            pressed: false,
        },
        InputEvent::Key {
            key: LMeta,
            pressed: false,
        },
        InputEvent::Key {
            key: RMeta,
            pressed: false,
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossing_maps_to_opposite_edge() {
        assert_eq!(opposite(Edge::Right), Edge::Left);
        assert_eq!(opposite(Edge::Left), Edge::Right);
        assert_eq!(opposite(Edge::Top), Edge::Bottom);
        assert_eq!(opposite(Edge::Bottom), Edge::Top);
        // Leave the server's RIGHT edge at y=432 → enter the client's LEFT edge
        // at y=432 on a 2560×1440 screen.
        assert_eq!(
            entry_point(opposite(Edge::Right), 432, 2560, 1440),
            (2, 432)
        );
        // Leave BOTTOM at x=1440 → enter TOP at x=1440 on a 1920×1080 screen.
        assert_eq!(
            entry_point(opposite(Edge::Bottom), 1440, 1920, 1080),
            (1440, 2)
        );
    }
}

/// The client enters from the edge opposite the one the server's cursor left by
/// (leave the Mac's right edge → arrive at the PC's left edge).
fn opposite(e: Edge) -> Edge {
    match e {
        Edge::Left => Edge::Right,
        Edge::Right => Edge::Left,
        Edge::Top => Edge::Bottom,
        Edge::Bottom => Edge::Top,
    }
}

use crate::bulk::BulkConn;
use crate::capture;
use crate::clipboard;
use crate::config::Config;
use crate::control::Control;
use crate::discovery;
use crate::edge::{client_return_span, entry_point, map_to_client, perp_dim, EdgeConfig};
use crate::filexfer::FileReceiver;
use crate::transport::InputChannel;

/// Everything one symmetric peer session shares between the capture thread,
/// the bulk (Hello/clipboard) thread and the input pump. Both machines build
/// exactly the same thing — there is no server/client asymmetry at runtime.
#[derive(Clone)]
struct Shared {
    control: Arc<Control>,
    /// (edge config, arrangement offset) — LIVE: the peer's Hello can install
    /// or update it (configure the layout once, on either machine).
    arrangement: Arc<Mutex<(EdgeConfig, i32)>>,
    /// My bordered edge (where the peer's screen sits). LIVE, like above.
    border: Arc<Mutex<Option<Edge>>>,
    /// The peer's screen size (LIVE, learned from its Hello).
    peer_screen: Arc<Mutex<(u32, u32)>>,
    /// My own screen size.
    screen: (u32, u32),
    /// Outgoing bulk-channel sender (set once the bulk thread is up) — lets the
    /// pump push a refreshed Hello when the user re-arranges mid-session.
    hello_tx: Arc<Mutex<Option<mpsc::SyncSender<BulkMsg>>>>,
}

struct Runtime {
    shared: Shared,
    input: Mutex<Receiver<InputEvent>>,
}

/// Auto-pair starts a listener and may also dial. Both must reuse the same
/// global hooks, control state and queue, including after reconnects.
fn runtime(cfg: &Config) -> Arc<Runtime> {
    static RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            let sh = build_shared(cfg);
            let (tx, rx) = mpsc::channel();
            let capture_sh = sh.clone();
            std::thread::spawn(move || {
                if let Err(e) = capture::run(
                    tx,
                    capture_sh.control,
                    capture_sh.arrangement,
                    capture_sh.screen,
                    capture_sh.peer_screen,
                ) {
                    tracing::error!(error = %e, "capture thread stopped");
                }
            });
            Arc::new(Runtime {
                shared: sh,
                input: Mutex::new(rx),
            })
        })
        .clone()
}

struct StopOnDrop(Arc<AtomicBool>);
impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

fn validate_hello(msg: &BulkMsg) -> anyhow::Result<()> {
    let BulkMsg::Hello {
        version, screen, ..
    } = msg
    else {
        anyhow::bail!("expected peer Hello");
    };
    anyhow::ensure!(
        *version == sharecursor_protocol::PROTOCOL_VERSION,
        "incompatible peer protocol {version}; update ShareCursor on both computers"
    );
    anyhow::ensure!(
        screen.0 > 0 && screen.1 > 0 && screen.0 <= i32::MAX as u32 && screen.1 <= i32::MAX as u32,
        "invalid peer screen size"
    );
    Ok(())
}

/// Build the shared state from the local config (arrangement may be absent —
/// the peer's Hello can supply it later).
fn build_shared(cfg: &Config) -> Shared {
    let (sw, sh) = screen_size(cfg);
    tracing::info!(width = sw, height = sh, "screen size (auto-detected)");
    let border = cfg.machine(&cfg.name).and_then(|m| {
        if m.right.is_some() {
            Some(Edge::Right)
        } else if m.left.is_some() {
            Some(Edge::Left)
        } else if m.top.is_some() {
            Some(Edge::Top)
        } else if m.bottom.is_some() {
            Some(Edge::Bottom)
        } else {
            None
        }
    });
    let edges = match (cfg.machine(&cfg.name), cfg.auto_edge_switch) {
        (Some(m), true) => EdgeConfig::new(
            sw,
            sh,
            m.left.is_some(),
            m.right.is_some(),
            m.top.is_some(),
            m.bottom.is_some(),
        ),
        _ => EdgeConfig::none(),
    };
    let peer_screen = cfg
        .machines
        .iter()
        .find(|m| m.name != cfg.name)
        .and_then(|m| m.screen)
        .unwrap_or((1920, 1080));
    Shared {
        control: Arc::new(Control::new()),
        arrangement: Arc::new(Mutex::new((edges, cfg.offset))),
        border: Arc::new(Mutex::new(border)),
        peer_screen: Arc::new(Mutex::new(peer_screen)),
        screen: (sw, sh),
        hello_tx: Arc::new(Mutex::new(None)),
    }
}

/// My own Hello: name + screen + my arrangement (so a peer with none adopts it).
/// `refresh` = the user just re-arranged; the peer must adopt unconditionally.
fn my_hello(cfg_name: &str, sh: &Shared, refresh: bool) -> BulkMsg {
    BulkMsg::Hello {
        version: sharecursor_protocol::PROTOCOL_VERSION,
        name: cfg_name.to_string(),
        screen: sh.screen,
        edge: *sh.border.lock().unwrap(),
        offset: sh.arrangement.lock().unwrap().1,
        refresh,
    }
}

/// Reload the arrangement from the config (the settings window saved while we
/// were running) into the live shared state, and tell the peer so it adopts.
/// This is what makes "connect first, arrange after" work without restarts.
fn reload_arrangement(cfg_name: &str, sh: &Shared) {
    let Ok(cfg) = Config::load(&Config::default_path()) else {
        return;
    };
    let fresh = build_shared(&cfg);
    *sh.border.lock().unwrap() = *fresh.border.lock().unwrap();
    *sh.arrangement.lock().unwrap() = *fresh.arrangement.lock().unwrap();
    if let Some(tx) = sh.hello_tx.lock().unwrap().as_ref() {
        let _ = tx.try_send(my_hello(cfg_name, sh, true));
    }
    tracing::info!("arrangement reloaded from settings and sent to the peer");
}

/// Load settings, initializing them on a fresh installation.
fn load_config() -> anyhow::Result<Config> {
    Config::load_or_create(&Config::default_path())
}

/// This machine's screen size. Always prefer the LIVE OS-detected size so a
/// stale value in the config can never break edge detection or the offset math;
/// a config `screen` is only a fallback when detection isn't available.
fn screen_size(cfg: &Config) -> (u32, u32) {
    crate::emit::main_display_size()
        .ok()
        .or_else(|| cfg.machine(&cfg.name).and_then(|m| m.screen))
        .unwrap_or((1920, 1080))
}

fn resolve(addr: &str) -> anyhow::Result<SocketAddr> {
    addr.to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("could not resolve address {addr}"))
}

/// Zero-config auto-pairing: advertise ourselves and search for a peer on the
/// LAN, then connect automatically — no IP, no manual matching. If both sides
/// have an explicit `role`, that decides who serves; otherwise a deterministic
/// name tiebreaker makes exactly one side the server. The client retries until
/// the server is up, so start order doesn't matter.
#[cfg(feature = "native")]
pub fn pair() -> anyhow::Result<()> {
    let cfg = load_config()?;
    let me = cfg.name.clone();
    let port = cfg.port;

    // Listen + advertise IMMEDIATELY (serve does both) — so even if WE can't
    // see the peer (its mDNS blocked by a firewall), the peer can still find
    // and dial US. Browsing below only decides whether WE should dial.
    {
        let bind = format!("0.0.0.0:{port}");
        std::thread::spawn(move || {
            if let Err(e) = serve(&bind) {
                tracing::warn!(error = %e, "listener stopped");
            }
        });
    }
    tracing::info!(name = %me, "auto-pairing: listening, advertising and searching…");

    let my_id = Config::ensure_device_id(&Config::default_path());
    loop {
        let peers = discovery::list(Duration::from_secs(2)).unwrap_or_default();
        // Identity = stable device id (never the name; names can collide or be
        // renamed to "mac (2)" by mDNS). Skip ourselves by id.
        if let Some((fullname, addr, pid)) = peers
            .into_iter()
            .find(|(_, _, pid)| !pid.is_empty() && *pid != my_id)
        {
            let peer = fullname.split('.').next().unwrap_or("peer").to_string();
            // Deterministic tiebreaker on the ids: exactly one side dials.
            if my_id > pid {
                tracing::info!(%peer, %addr, "paired — dialing");
                loop {
                    if let Err(e) = connect(Some(&addr.to_string())) {
                        tracing::warn!(error = %e, "connect failed; retrying in 2s");
                        std::thread::sleep(Duration::from_secs(2));
                    }
                }
            }
            tracing::info!(%peer, "paired — the peer will connect to us");
            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        }
        tracing::info!("no peer found yet; still searching… (the peer may still find US)");
    }
}

/// Persist a peer's reported screen size into our config, so the settings window
/// can show the real remote resolution. The client reports it on connect (like
/// Deskflow's DINF message).
fn record_peer_screen(name: &str, screen: (u32, u32)) {
    let path = Config::default_path();
    if let Ok(mut cfg) = Config::load(&path) {
        if let Some(m) = cfg.machines.iter_mut().find(|m| m.name == name) {
            if m.screen != Some(screen) {
                m.screen = Some(screen);
                let _ = cfg.save(&path);
                tracing::info!(%name, width = screen.0, height = screen.1, "recorded peer screen size");
            }
        }
    }
}

/// Persist an adopted arrangement: my machine gets the peer on `my_edge`, the
/// peer machine gets the reciprocal, and the offset is stored — so the layout
/// survives restarts even though it was only ever configured on the peer.
fn record_peer_layout(peer: &str, my_edge: Edge, my_offset: i32) {
    let path = Config::default_path();
    let Ok(mut cfg) = Config::load(&path) else {
        return;
    };
    let me = cfg.name.clone();
    let before = toml::to_string(&cfg).ok();
    let set = |m: &mut crate::config::Machine, e: Edge, n: &str| {
        m.left = None;
        m.right = None;
        m.top = None;
        m.bottom = None;
        match e {
            Edge::Left => m.left = Some(n.into()),
            Edge::Right => m.right = Some(n.into()),
            Edge::Top => m.top = Some(n.into()),
            Edge::Bottom => m.bottom = Some(n.into()),
        }
    };
    let peer_owned = peer.to_string();
    for m in cfg.machines.iter_mut() {
        if m.name == me {
            set(m, my_edge, &peer_owned);
        } else if m.name == peer_owned {
            set(m, opposite(my_edge), &me);
        }
    }
    cfg.offset = my_offset;
    if toml::to_string(&cfg).ok() != before {
        let _ = cfg.save(&path);
    }
}

/// Wire clipboard + file sync onto one (already-encrypted) bulk connection.
/// Blocks on the reader loop; returns when the peer disconnects.
/// `adopt` = always take the peer's arrangement (the dialer does; the listener
/// only takes it when it has none of its own).
fn serve_bulk(
    conn: BulkConn,
    hello: Option<BulkMsg>,
    sh: Shared,
    adopt: bool,
    first: Option<BulkMsg>,
    stop: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let _stop = StopOnDrop(stop.clone());
    let last = clipboard::shared_last();
    // Backpressure bounds queued file bytes while UDP input remains independent.
    let (out_tx, out_rx) = mpsc::sync_channel::<BulkMsg>(8);
    let (in_tx, in_rx) = mpsc::channel::<clipboard::ApplyClipboard>();

    // Send our own screen size first (client → server), before any other frame,
    // so the encrypted send-counter stays in sync with the peer's recv-counter.
    if let Some(h) = hello {
        let _ = out_tx.send(h);
    }
    // Let the input pump push refreshed Hellos (live re-arrangement).
    *sh.hello_tx.lock().unwrap() = Some(out_tx.clone());

    let mut wconn = conn.try_clone()?;
    std::thread::spawn(move || {
        while let Ok(msg) = out_rx.recv() {
            if wconn.send(&msg).is_err() {
                break;
            }
        }
        wconn.shutdown();
    });
    let last_apply = last.clone();
    std::thread::spawn(move || clipboard::apply(in_rx, last_apply));
    std::thread::spawn(move || clipboard::watch(out_tx, last, stop));

    let mut receiver = FileReceiver::new(crate::filexfer::receive_dir());
    let mut files = clipboard::FileClipboard::default();
    let mut rconn = conn;
    let mut first = first;
    loop {
        match first.take().map(Ok).unwrap_or_else(|| rconn.recv()) {
            Ok(BulkMsg::Clipboard(data)) => {
                let _ = in_tx.send(clipboard::ApplyClipboard::Data(data));
            }
            Ok(BulkMsg::ClipboardFilesBegin { id, files: ids }) => {
                receiver.abort();
                files.begin(id, ids)?;
            }
            Ok(BulkMsg::ClipboardFilesEnd { id }) => {
                let paths = files.finish(id)?;
                let _ = in_tx.send(clipboard::ApplyClipboard::Files(paths));
            }
            Ok(BulkMsg::ClipboardFilesCancel { id }) => {
                receiver.abort();
                files.cancel(id);
            }
            Ok(
                msg @ (BulkMsg::FileBegin { .. }
                | BulkMsg::FileChunk { .. }
                | BulkMsg::FileEnd { .. }),
            ) => {
                if let Some((id, path)) = receiver.handle(&msg)? {
                    files.completed(id, &path);
                }
            }
            // Peer's Hello: its screen size (kept LIVE for the offset maths,
            // Deskflow's DINF pattern) and optionally its monitor arrangement —
            // adopt the mirrored version so the layout is only ever configured
            // on ONE machine.
            Ok(BulkMsg::Hello {
                version,
                name,
                screen,
                edge,
                offset,
                refresh,
                ..
            }) => {
                anyhow::ensure!(
                    version == sharecursor_protocol::PROTOCOL_VERSION,
                    "peer protocol changed"
                );
                tracing::info!(peer = %name, width = screen.0, height = screen.1, "peer reported its screen size (Hello)");
                *sh.peer_screen.lock().unwrap() = screen;
                record_peer_screen(&name, screen);
                if let Some(their_edge) = edge {
                    let have_own = sh.border.lock().unwrap().is_some();
                    if refresh || adopt || !have_own {
                        let my_edge = opposite(their_edge);
                        let my_offset = -offset;
                        *sh.border.lock().unwrap() = Some(my_edge);
                        *sh.arrangement.lock().unwrap() = (
                            EdgeConfig::new(
                                sh.screen.0,
                                sh.screen.1,
                                my_edge == Edge::Left,
                                my_edge == Edge::Right,
                                my_edge == Edge::Top,
                                my_edge == Edge::Bottom,
                            ),
                            my_offset,
                        );
                        record_peer_layout(&name, my_edge, my_offset);
                        tracing::info!(
                            ?my_edge,
                            my_offset,
                            "adopted the peer's monitor arrangement"
                        );
                    }
                }
            }
            Ok(_) => {}
            Err(_) => return Ok(()),
        }
    }
}

/// The SYMMETRIC input pump — identical on both machines. Injects the peer's
/// forwarded input, forwards ours while our pointer is away, and translates
/// capture-thread state flips into `PointerEnter` / `PointerEnd` messages.
fn run_peer_input(
    udp: &InputChannel,
    rx: &Receiver<InputEvent>,
    sh: &Shared,
    cfg_name: &str,
    mut peer: Option<SocketAddr>,
    stop: &AtomicBool,
) -> anyhow::Result<()> {
    let control = &sh.control;
    let mut injector = crate::emit::Injector::new()?;
    let mut prev_my_away = false;
    let mut keepalive = Instant::now();
    let mut config_poll = Instant::now();
    let mut cfg_mtime = std::fs::metadata(Config::default_path())
        .and_then(|m| m.modified())
        .ok();
    let mut buf = [0u8; 2048];
    udp.set_nonblocking(true)?;
    #[cfg(windows)]
    let _timer = crate::input_batch::windows_timer();
    while !stop.load(Ordering::Relaxed) {
        // "Connect first, arrange after": watch the config; when the settings
        // window saves, reload the arrangement live + push it to the peer.
        if config_poll.elapsed() >= Duration::from_secs(1) {
            config_poll = Instant::now();
            let m = std::fs::metadata(Config::default_path())
                .and_then(|m| m.modified())
                .ok();
            if m != cfg_mtime {
                cfg_mtime = m;
                reload_arrangement(cfg_name, sh);
            }
        }
        // ---- receive from the peer ----
        let mut received = false;
        if let Ok(Some((pkt, from))) = udp.recv(&mut buf) {
            received = true;
            if peer != Some(from) {
                tracing::info!(%from, "peer input channel online");
                peer = Some(from);
            }
            match pkt.msg {
                InputMsg::Ping { nonce, echo_nanos } => {
                    let _ = udp.send_to(InputMsg::Pong { nonce, echo_nanos }, from);
                }
                // The peer's physical input drives MY real cursor.
                InputMsg::Events(events) => {
                    for ev in events {
                        if let Err(e) = injector.apply(ev) {
                            tracing::warn!(error = %e, "inject failed");
                        }
                        if matches!(ev, InputEvent::MouseMove { .. }) {
                            if let Ok((x, y)) = injector.location() {
                                control.visitor_position(x, y, sh.screen);
                            }
                        }
                    }
                }
                // The peer's pointer arrives on my screen.
                InputMsg::PointerEnter { edge, pos, span } => {
                    if control.my_away.swap(false, Ordering::Relaxed) {
                        prev_my_away = false; // crossed paths: mine implicitly came home
                    }
                    control.peer_away.store(true, Ordering::Relaxed);
                    control.host_armed.store(false, Ordering::Relaxed);
                    *control.host_span.lock().unwrap() = Some((edge, span));
                    let (ex, ey) = entry_point(edge, pos, sh.screen.0, sh.screen.1);
                    let _ = injector.move_to(ex, ey);
                    tracing::info!(?edge, ex, ey, "peer pointer entered my screen");
                }
                InputMsg::Pong { .. } => {}
                // An away-state ends.
                InputMsg::PointerEnd { pos } => {
                    if control.my_away.swap(false, Ordering::Relaxed) {
                        prev_my_away = false;
                        let border = sh.border.lock().unwrap().unwrap_or(Edge::Right);
                        *control.return_to.lock().unwrap() = pos.map(|p| (border, p));
                        // macOS re-shows + warps in capture; elsewhere warp here.
                        #[cfg(not(target_os = "macos"))]
                        if let Some(p) = pos {
                            let (ex, ey) = entry_point(border, p, sh.screen.0, sh.screen.1);
                            let _ = injector.move_to(ex, ey);
                        }
                        tracing::info!("my pointer came home");
                    } else if control.peer_away.swap(false, Ordering::Relaxed) {
                        *control.host_span.lock().unwrap() = None;
                        tracing::info!("peer reclaimed its pointer");
                    }
                }
            }
        } else {
            // Idle keep-alive so the path stays warm and the peer learns our
            // address (the dialer pings first; NAT/firewall state stays open).
            if keepalive.elapsed() >= Duration::from_secs(1) {
                keepalive = Instant::now();
                if let Some(p) = peer {
                    let _ = udp.send_to(
                        InputMsg::Ping {
                            nonce: 0,
                            echo_nanos: 0,
                        },
                        p,
                    );
                }
            }
        }

        // ---- capture: the visiting pointer crossed home ----
        if let Some(perp) = control.send_peer_home.lock().unwrap().take() {
            if let Some(p) = peer {
                let _ = udp.send_to(release_all_modifiers(), p);
                let msg = if perp == i32::MAX {
                    InputMsg::PointerEnd { pos: None } // hotkey: no position
                } else {
                    let (_, offset) = *sh.arrangement.lock().unwrap();
                    let ps = *sh.peer_screen.lock().unwrap();
                    let border = sh.border.lock().unwrap().unwrap_or(Edge::Right);
                    let cdim = perp_dim(border, ps.0, ps.1);
                    InputMsg::PointerEnd {
                        pos: Some(map_to_client(perp, offset, cdim)),
                    }
                };
                let _ = udp.send_to(msg, p);
            }
            *control.host_span.lock().unwrap() = None;
        }

        // ---- capture: my pointer went away / was reclaimed ----
        let my_away = control.my_away.load(Ordering::Relaxed);
        if my_away != prev_my_away {
            if let Some(p) = peer {
                let _ = udp.send_to(release_all_modifiers(), p);
                if my_away {
                    let (_, offset) = *sh.arrangement.lock().unwrap();
                    let ps = *sh.peer_screen.lock().unwrap();
                    let (edge_out, pos, span) = match *control.entry.lock().unwrap() {
                        Some((edge, perp)) => {
                            let cdim = perp_dim(edge, ps.0, ps.1);
                            let sdim = perp_dim(edge, sh.screen.0, sh.screen.1) as i32;
                            (
                                edge,
                                map_to_client(perp, offset, cdim),
                                client_return_span(offset, sdim, cdim as i32),
                            )
                        }
                        None => {
                            // Hotkey push: enter at the peer's centre.
                            let border = sh.border.lock().unwrap().unwrap_or(Edge::Right);
                            let cdim = perp_dim(border, ps.0, ps.1);
                            let sdim = perp_dim(border, sh.screen.0, sh.screen.1) as i32;
                            (
                                border,
                                cdim as i32 / 2,
                                client_return_span(offset, sdim, cdim as i32),
                            )
                        }
                    };
                    let _ = udp.send_to(
                        InputMsg::PointerEnter {
                            edge: opposite(edge_out),
                            pos,
                            span,
                        },
                        p,
                    );
                } else {
                    // Hotkey reclaim — tell the peer the visit ended.
                    let _ = udp.send_to(InputMsg::PointerEnd { pos: None }, p);
                }
            }
            prev_my_away = my_away;
        }

        // ---- forward my captured input while my pointer is away ----
        let batch = crate::input_batch::drain(rx);
        if !batch.is_empty() {
            if let Some(p) = peer {
                let _ = udp.send_to(InputMsg::Events(batch), p);
            }
        } else if !received {
            std::thread::sleep(Duration::from_micros(500));
        }
    }
    control.my_away.store(false, Ordering::Relaxed);
    control.peer_away.store(false, Ordering::Relaxed);
    *control.host_span.lock().unwrap() = None;
    *sh.hello_tx.lock().unwrap() = None;
    for ev in match release_all_modifiers() {
        InputMsg::Events(events) => events,
        _ => unreachable!(),
    } {
        let _ = injector.apply(ev);
    }
    Ok(())
}

/// Listener accepts input sessions and independent file transfers concurrently.
pub fn serve(bind: &str) -> anyhow::Result<()> {
    let cfg = load_config()?;
    let bind_addr = resolve(bind)?;
    let my_id = Config::ensure_device_id(&Config::default_path());
    let listener = TcpListener::bind(bind_addr)?;
    let _advert = discovery::advertise(&cfg.name, bind_addr.port(), &my_id)
        .map_err(|e| tracing::warn!(error = %e, "mDNS advertise failed"))
        .ok();
    tracing::info!(%bind_addr, name = %cfg.name, "listening for encrypted input and file transfers");
    loop {
        let (stream, addr) = listener.accept()?;
        let cfg = cfg.clone();
        std::thread::spawn(move || {
            let session = || -> anyhow::Result<()> {
                stream.set_read_timeout(Some(Duration::from_secs(10)))?;
                let (mut conn, input_sess) =
                    BulkConn::handshake(stream, cfg.psk.as_bytes(), Role::Responder)?;
                let first = conn.recv()?;
                if matches!(first, BulkMsg::FileBegin { .. }) {
                    conn.set_read_timeout(Some(Duration::from_secs(30)))?;
                    let mut files = FileReceiver::new(crate::filexfer::receive_dir());
                    let mut msg = first;
                    loop {
                        anyhow::ensure!(
                            matches!(
                                msg,
                                BulkMsg::FileBegin { .. }
                                    | BulkMsg::FileChunk { .. }
                                    | BulkMsg::FileEnd { .. }
                            ),
                            "unexpected file message"
                        );
                        if let Some((id, _)) = files.handle(&msg)? {
                            conn.send(&BulkMsg::FileReceived { id })?;
                            return Ok(());
                        }
                        msg = conn.recv()?;
                    }
                }
                validate_hello(&first)?;
                conn.set_read_timeout(None)?;
                let rt = runtime(&cfg);
                let rx = rt
                    .input
                    .try_lock()
                    .map_err(|_| anyhow::anyhow!("input session already active"))?;
                while rx.try_recv().is_ok() {} // discard stale disconnected input
                let sh = rt.shared.clone();
                let udp = InputChannel::bind(bind_addr, None)?.with_cipher(Arc::new(input_sess));
                let stop = Arc::new(AtomicBool::new(false));
                let bulk_stop = stop.clone();
                let bulk_sh = sh.clone();
                let hello = my_hello(&cfg.name, &sh, false);
                std::thread::spawn(move || {
                    if let Err(e) =
                        serve_bulk(conn, Some(hello), bulk_sh, false, Some(first), bulk_stop)
                    {
                        tracing::warn!(error = %e, "bulk channel closed");
                    }
                });
                run_peer_input(&udp, &rx, &sh, &cfg.name, None, &stop)
            };
            if let Err(e) = session() {
                tracing::warn!(peer = %addr, error = %e, "connection ended");
            }
        });
    }
}

pub fn connect(server: Option<&str>) -> anyhow::Result<()> {
    let cfg = load_config()?;
    let with_port = |host: &str| {
        if host.contains(':') {
            host.to_string()
        } else {
            format!("{host}:{}", cfg.port)
        }
    };
    let server_addr = match server
        .map(str::to_string)
        .or_else(|| cfg.server_host.clone())
    {
        Some(host) => resolve(&with_port(&host))?,
        None => discovery::discover(Duration::from_secs(3))?.ok_or_else(|| {
            anyhow::anyhow!("no server found via mDNS; pass a host or set server_host")
        })?,
    };
    let stream = TcpStream::connect_timeout(&server_addr, Duration::from_secs(10))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let (mut conn, input_sess) = BulkConn::handshake(stream, cfg.psk.as_bytes(), Role::Initiator)?;
    let rt = runtime(&cfg);
    let rx = rt
        .input
        .try_lock()
        .map_err(|_| anyhow::anyhow!("input session already active"))?;
    while rx.try_recv().is_ok() {}
    let sh = rt.shared.clone();
    conn.send(&my_hello(&cfg.name, &sh, false))?;
    let first = conn.recv()?;
    validate_hello(&first)?;
    conn.set_read_timeout(None)?;
    let channel = InputChannel::bind("0.0.0.0:0".parse().unwrap(), Some(server_addr))?
        .with_cipher(Arc::new(input_sess));
    let stop = Arc::new(AtomicBool::new(false));
    let bulk_stop = stop.clone();
    let bulk_sh = sh.clone();
    std::thread::spawn(move || {
        if let Err(e) = serve_bulk(conn, None, bulk_sh, true, Some(first), bulk_stop) {
            tracing::warn!(error = %e, "bulk channel closed");
        }
    });
    channel.send(InputMsg::Ping {
        nonce: 0,
        echo_nanos: 0,
    })?;
    run_peer_input(&channel, &rx, &sh, &cfg.name, Some(server_addr), &stop)
}
