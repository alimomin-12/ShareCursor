# Using ShareCursor

How to actually share your keyboard, mouse, clipboard and files once ShareCursor
is installed on both machines. Two ways: the **app UI** (easiest) or the
**terminal** (best for a first test, because you can see the logs).

First, if the app won't open at all (macOS "damaged", Windows SmartScreen) or you
haven't installed yet, see [INSTALL.md](./INSTALL.md).

---

## Key idea (read this first)

- ShareCursor is **symmetric** (like ShareMouse): once connected, **both
  machines' mice and keyboards work** — grab whichever you like. Push the
  cursor through the shared screen edge and it crosses to the other machine;
  push it back and it returns.
- "Server" vs "client" only decides **who listens** for the connection — it
  doesn't matter for control. Easiest: don't pick at all and use
  **auto-pairing** (`sharecursor pair`, or just leave the role unset): the two
  machines find each other on the LAN and connect — **no IP addresses**.
- Both machines need the **same passphrase** (it authenticates + encrypts).
- Both machines must be on the **same Wi‑Fi / network**.
- Install ShareCursor **0.6.1 on both machines**; this update uses protocol 6.
- The **monitor arrangement** only needs to be set on ONE machine — the other
  adopts the mirrored layout automatically when they connect.

---

## Option A — the app UI (recommended for normal use)

### On BOTH machines: open Settings and set the passphrase

1. Launch ShareCursor. On a fresh installation it creates `config.toml` and
   opens Settings automatically. It also lives in the **menu bar** (macOS,
   top-right) or the **system tray** (Windows, bottom-right).
2. On later launches, click the icon → **Settings & Monitor Manager** to open
   the settings window again.
3. Set the **shared passphrase** (same on both machines) and the machine names.
   Leave **Server host** blank (auto‑discovery finds the peer).
4. On ONE machine, in the **arrangement** panel, drag the second monitor to
   where it physically sits (left / right / above / below — with any offset).
   The other machine adopts this layout automatically on connect. **Save**.
5. Close Settings to start pairing. Each fresh installation generates its own
   pairing code; copy the code from one machine into the other and save it on
   both. Existing settings are preserved on later launches.

The config is stored at `%APPDATA%\sharecursor\config.toml` on Windows and
`~/Library/Application Support/sharecursor/config.toml` on macOS. No manual
`init-config` command is needed for the app UI.

> Screen sizes are **auto-detected**: this machine's from the OS, and the other
> machine's is reported automatically the first time it connects. You can also
> type the other screen's size in the window. Once arranged, just push the
> cursor across the shared edge — no hotkey needed (both Shift keys still works
> as a fallback).

<details><summary>Prefer to edit the config file by hand? (advanced)</summary>

The window writes a plain `config.toml`; you can edit it directly instead.
Add `screen = [w, h]` under a machine only to override a wrong auto-detection.</details>

**On the Mac (server)** set:
```toml
name = "mac"                    # this machine's name
psk  = "pick-a-long-secret"     # SAME on both machines
port = 24800
auto_edge_switch = true

[[machines]]
name = "mac"
right  = "windows"              # the PC is to the right of the Mac

[[machines]]
name = "windows"
left   = "mac"
```

**On the Windows PC (client)** set the *same* file, but change `name` and add
`server_host` (the Mac's IP address):
```toml
name = "windows"
psk  = "pick-a-long-secret"     # EXACTLY the same as the Mac
port = 24800
auto_edge_switch = true
server_host = "192.168.1.20"    # the Mac's IP (see "Find the server's IP")

[[machines]]
name = "mac"
right  = "windows"

[[machines]]
name = "windows"
left   = "mac"
```

### Then start it

1. **On the Mac:** grant permissions the first time — System Settings → Privacy &
   Security → enable **Accessibility** and **Input Monitoring** for ShareCursor.
   Then tray icon → **Start Server**.
2. **On the Windows PC:** tray icon → **Start Client**. Allow it through the
   **firewall** when Windows asks (Private networks → Allow).

### Use it

- Push your mouse into the **shared screen edge** (in the example: the Mac's
  right edge) → your keyboard & mouse now control the PC.
- Push back to the opposite edge to return. Or press **F12** on the server to
  toggle manually.
- **Copy** on one machine, **paste** on the other — the clipboard syncs.

---

## Option B — the terminal (best for the first test / debugging)

Running from a terminal shows live logs, which makes the first setup much easier
to diagnose.

### macOS (server)
```bash
# from the app, or the built binary:
sharecursor init-config                 # writes the config once
open -e ~/Library/Application\ Support/sharecursor/config.toml   # edit + save
sharecursor serve                       # start the server
```
Grant **Accessibility** + **Input Monitoring** to the Terminal (or the app) in
System Settings → Privacy & Security, then run `serve` again.

### Windows (client)
Open **PowerShell** and find the installed exe:
```powershell
$exe = Get-ChildItem "$env:LOCALAPPDATA\Programs\ShareCursor","$env:ProgramFiles\ShareCursor" `
       -Filter sharecursor.exe -Recurse -ErrorAction SilentlyContinue |
       Select-Object -First 1 -ExpandProperty FullName
& $exe init-config
notepad "$env:APPDATA\sharecursor\config.toml"    # edit (name=windows, same psk, server_host=Mac IP) + save
& $exe connect                                    # allow through the firewall when asked
```

You should see **"client authenticated (encrypted session established)"** on the
Mac. Then test the edge switch, clipboard, and:
```bash
# send a file to the other machine (use the OTHER machine's IP:port)
sharecursor send-file 192.168.1.30:24800 ./report.pdf   # lands in Downloads/ShareCursor there
```

---

## Copy and paste files

Select regular files in Explorer or Finder, then copy them (`Ctrl+C` on Windows,
`Cmd+C` on Mac). Leave ShareCursor connected while the files transfer. Once the
selection finishes downloading, paste into the destination folder (`Ctrl+V` or
`Cmd+V`). A Windows keyboard controlling the Mac maps Ctrl to Cmd automatically.

The received files are also available in `Downloads/ShareCursor`. Larger files
need time to transfer before they can be pasted. Zip folders before copying;
dragging files across screens and folder clipboard copying are not supported.

## Find the server's IP

- **macOS:** System Settings → Wi‑Fi → **Details** → IP address, or in Terminal:
  `ipconfig getifaddr en0`
- **Windows:** in PowerShell: `ipconfig` → look for **IPv4 Address** under your
  Wi‑Fi/Ethernet adapter.

Tip: with `auto_edge_switch` on and both on the same LAN, the client can also
find the server automatically over mDNS — running `connect` with no
`server_host` will search for it.

---

## Config fields

| Field | Meaning |
|---|---|
| `name` | This machine's name. Must match one entry under `[[machines]]`. |
| `psk` | Shared passphrase (≥ 8 chars). Identical on both machines. Authenticates + encrypts. |
| `port` | Network port (default 24800). Same on both. |
| `auto_edge_switch` | Hand control over when the cursor hits a bordered edge. |
| `server_host` | (Client only) the server's IP, e.g. `192.168.1.20`. Omit to auto‑discover. |
| `[[machines]]` | The layout: which peer is on each edge (`left`/`right`/`top`/`bottom`). Screen sizes are auto-detected; add `screen = [w, h]` only to override. |

---

## Troubleshooting

| Symptom | Fix |
|---|---|
| Mouse doesn't move (Mac server) | Enable **Accessibility** *and* **Input Monitoring**; quit and reopen after granting. |
| `handshake/auth failed` | The `psk` isn't identical on both machines. |
| Client can't connect | Same Wi‑Fi? Firewall allowed on Windows? Correct `server_host` IP? Try `sharecursor discover`. |
| `name ... is not present` | `name` must match a `[[machines]]` entry. |
| Nothing happens at the edge | Check the layout edges (`right`/`left`) and that `auto_edge_switch = true`. **Hold both Shift keys** to toggle control from the keyboard (reliable escape). |
| Windows: no tray icon after launching | New icons hide under the **"^" (show hidden icons)** arrow by the clock — drag ShareCursor onto the taskbar. For a first test you can skip the tray entirely and run `sharecursor.exe connect` from PowerShell. |

Still stuck? Open an issue with the terminal output from both machines:
<https://github.com/phun333/ShareCursor/issues>.
