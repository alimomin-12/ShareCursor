# Wire protocol

All types live in `crates/protocol/src/lib.rs`. Encoding is
[postcard](https://docs.rs/postcard) — a compact, `serde`-based binary format
chosen because it is small and fast (important on the input hot path).

**Versioning:** `PROTOCOL_VERSION` is `6`. Input peers exchange and validate
versions before starting a session; both computers must run the same version.
Version 6 adds native file clipboard batches and standalone file receipt acknowledgements.

## Two channels

| Channel | Transport | Carries | Type |
|---|---|---|---|
| Input | UDP | mouse/keys, control signals, latency probes | `InputPacket` → `InputMsg` |
| Bulk | TCP | clipboard, files, handshake | `BulkMsg` |

## Input channel

### `InputPacket`
```
struct InputPacket { seq: u32, msg: InputMsg }
```
`seq` is a per-sender monotonic counter. The receiver drops any `Events` packet
whose seq is ≤ the highest seen (duplicate/straggler rejection **without**
blocking — the reason we use UDP). Control/ping messages bypass this check.

### `InputMsg`
```
enum InputMsg {
    Events(Vec<InputEvent>),   // a coalesced tick of input
    PointerEnter { edge: Edge, pos: i32, span: (i32, i32) },
    PointerEnd { pos: Option<i32> },
    Ping { nonce: u64, echo_nanos: u64 },
    Pong { nonce: u64, echo_nanos: u64 },
}
```

### `InputEvent`
```
enum InputEvent {
    MouseMove { dx: i32, dy: i32 },      // RELATIVE motion (see DECISIONS #4)
    MouseButton { button: MouseButton, pressed: bool },
    Scroll { dx: f32, dy: f32 },
    Key { key: Key, pressed: bool },     // portable key, see below
}
```

Consecutive motion events are **coalesced per capture tick** into one `Events(Vec<..>)` so that a
high mouse polling rate does not flood the network or cause the classic
"jumpiness" when polling exceeds display refresh. Click/key ordering is preserved;
each packet contains at most 32 events, fitting the 2048-byte receive buffer.

### Portable `Key`
macOS and Windows use different raw keycodes, so we never send a raw scancode.
`keymap.rs` translates the native key to a portable `Key` enum on capture and
back to a native key on injection (the same approach Synergy/Deskflow use).
Unmappable keys become `Key::Unknown(u32)` and are dropped on injection.

## Bulk channel

### Framing
Each frame is `u32` big-endian length prefix + the (optionally encrypted)
postcard bytes. Max frame size is 64 MiB (guards against a hostile peer forcing
a huge allocation).

### `BulkMsg`
```
enum BulkMsg {
    Hello { version: u16, name: String, screen: (u32,u32), edge: Option<Edge>, offset: i32, refresh: bool },
    Welcome { version: u16, name: String },                   // reserved
    Clipboard(ClipboardData),
    FileBegin { id: u64, name: String, size: u64 },
    FileChunk { id: u64, offset: u64, data: Vec<u8> },
    FileEnd { id: u64 },
    Heartbeat,
    ClipboardFilesBegin { id: u64, files: Vec<u64> },
    ClipboardFilesEnd { id: u64 },
    ClipboardFilesCancel { id: u64 },
    FileReceived { id: u64 },
}

enum ClipboardData {
    Text(String),
    Image { width: u32, height: u32, rgba: Vec<u8> },  // raw RGBA, no codec
}
```

### File transfer
`FileBegin` → many `FileChunk` (64 KiB each, written at `offset`) → `FileEnd`.
Chunks must be contiguous and within the declared size. The receiver sanitizes
filenames for both platforms and writes temporary files into `Downloads/ShareCursor`.
It publishes a complete file after size validation and disk sync, chooses a unique
name instead of overwriting downloads, and cleans up interrupted partial files.

File clipboard flow is `ClipboardFilesBegin` → the listed file transfers →
`ClipboardFilesEnd`. Only then are local file URLs/paths installed on the native
clipboard. Cancelled/incomplete batches never replace the clipboard. Up to 256
regular files are supported per selection; folders should be zipped first.
The outgoing queue has eight slots to bound buffered file data.

Standalone `send-file` performs the same encrypted handshake, starts with
`FileBegin` rather than `Hello`, and waits for `FileReceived`. The listener accepts
these connections independently while an input session is active.

## Encryption framing

See [SECURITY.md](./SECURITY.md) for the crypto design. Framing specifics:

- **Bulk (TCP):** the record is `seal(counter, aad=[], plaintext)`. The counter
  is *implicit* — TCP is ordered, so both peers keep in-sync send/recv counters
  starting at 0 and never transmit them.
- **Input (UDP):** the wire is `seq(4 bytes, cleartext) || seal(seq, aad=seq,
  ciphertext)`. The sequence number doubles as the nonce counter and is bound in
  as associated data, so it cannot be tampered with. Packets that fail
  authentication are silently dropped.

## Adding a new message

1. Add the variant to `InputMsg`/`BulkMsg`/`InputEvent` as appropriate.
2. Handle it on both send and receive sides (`run.rs`, `transport.rs`,
   `bulk.rs`).
3. If it changes the meaning of existing bytes, bump `PROTOCOL_VERSION` and add
   a note here.
4. Add a round-trip unit test (see `protocol` tests for the pattern).
