//! Ordered, encrypted file transfers. Incomplete files never become visible
//! clipboard entries and existing downloads are never overwritten.

use crate::bulk::BulkConn;
use sharecursor_protocol::crypto::Role;
use sharecursor_protocol::BulkMsg;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};

const CHUNK: usize = 64 * 1024;
pub const MAX_FILES: usize = 256;

pub fn receive_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("ShareCursor")
}

pub fn send_file(addr: SocketAddr, path: &Path) -> anyhow::Result<()> {
    let cfg = crate::config::Config::load_or_create(&crate::config::Config::default_path())?;
    send_file_with_psk(addr, path, cfg.psk.as_bytes())
}

fn send_file_with_psk(addr: SocketAddr, path: &Path, psk: &[u8]) -> anyhow::Result<()> {
    let stream = TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(10))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
    let (mut conn, _) = BulkConn::handshake(stream, psk, Role::Initiator)?;
    let id = rand_id();
    stream_file(path, id, |msg| conn.send(&msg))?;
    anyhow::ensure!(
        conn.recv()? == BulkMsg::FileReceived { id },
        "peer did not confirm file receipt"
    );
    tracing::info!(path = %path.display(), "file saved by peer");
    Ok(())
}

/// Stream through either a TCP writer or a bounded clipboard queue.
pub fn stream_file(
    path: &Path,
    id: u64,
    mut send: impl FnMut(BulkMsg) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut file = File::open(path)?;
    let meta = file.metadata()?;
    anyhow::ensure!(
        meta.is_file(),
        "only regular files can be copied: {}",
        path.display()
    );
    let size = meta.len();
    let name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("missing file name"))?
        .to_string_lossy()
        .into_owned();
    send(BulkMsg::FileBegin { id, name, size })?;
    let mut buf = vec![0u8; CHUNK];
    let mut offset = 0;
    while offset < size {
        let wanted = ((size - offset).min(CHUNK as u64)) as usize;
        file.read_exact(&mut buf[..wanted])?;
        send(BulkMsg::FileChunk {
            id,
            offset,
            data: buf[..wanted].to_vec(),
        })?;
        offset += wanted as u64;
    }
    anyhow::ensure!(file.read(&mut buf[..1])? == 0, "file changed while copying");
    send(BulkMsg::FileEnd { id })
}

struct Pending {
    file: Option<File>,
    partial: PathBuf,
    name: String,
    size: u64,
    written: u64,
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.file.take(); // Windows must close the handle before removing it.
        let _ = fs::remove_file(&self.partial);
    }
}

pub struct FileReceiver {
    dir: PathBuf,
    open: HashMap<u64, Pending>,
}

impl FileReceiver {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            open: HashMap::new(),
        }
    }
    pub fn abort(&mut self) {
        self.open.clear();
    }

    /// Return a completed local path only after size validation and disk sync.
    pub fn handle(&mut self, msg: &BulkMsg) -> anyhow::Result<Option<(u64, PathBuf)>> {
        match msg {
            BulkMsg::FileBegin { id, name, size } => {
                anyhow::ensure!(!self.open.contains_key(id), "duplicate file id");
                anyhow::ensure!(self.open.len() < MAX_FILES, "too many open transfers");
                fs::create_dir_all(&self.dir)?;
                let partial = self
                    .dir
                    .join(format!(".sharecursor-{:016x}.part", rand_id()));
                let file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&partial)?;
                self.open.insert(
                    *id,
                    Pending {
                        file: Some(file),
                        partial,
                        name: sanitize(name),
                        size: *size,
                        written: 0,
                    },
                );
            }
            BulkMsg::FileChunk { id, offset, data } => {
                let pending = self
                    .open
                    .get_mut(id)
                    .ok_or_else(|| anyhow::anyhow!("unknown file id"))?;
                anyhow::ensure!(
                    *offset == pending.written && data.len() <= CHUNK,
                    "invalid file chunk"
                );
                let end = offset
                    .checked_add(data.len() as u64)
                    .ok_or_else(|| anyhow::anyhow!("file offset overflow"))?;
                anyhow::ensure!(end <= pending.size, "file chunk exceeds declared size");
                pending.file.as_mut().unwrap().write_all(data)?;
                pending.written = end;
            }
            BulkMsg::FileEnd { id } => {
                let mut pending = self
                    .open
                    .remove(id)
                    .ok_or_else(|| anyhow::anyhow!("unknown file id"))?;
                anyhow::ensure!(pending.written == pending.size, "incomplete file");
                pending.file.take().unwrap().sync_all()?;
                for suffix in 0..10_000 {
                    let name = if suffix == 0 {
                        pending.name.clone()
                    } else {
                        let path = Path::new(&pending.name);
                        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                        match path.extension() {
                            Some(ext) => format!("{stem} ({suffix}).{}", ext.to_string_lossy()),
                            None => format!("{stem} ({suffix})"),
                        }
                    };
                    let dest = self.dir.join(name);
                    match fs::hard_link(&pending.partial, &dest) {
                        Ok(()) => {
                            tracing::info!(path = %dest.display(), "file complete");
                            return Ok(Some((*id, dest)));
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                        Err(e) => return Err(e.into()),
                    }
                }
                anyhow::bail!("too many files with the same name");
            }
            _ => {}
        }
        Ok(None)
    }
}

fn sanitize(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let clean: String = base
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let clean = clean.trim_end_matches(['.', ' ']);
    if clean.is_empty() {
        return "received.bin".into();
    }
    let stem = clean.split('.').next().unwrap_or("").to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
    {
        format!("_{clean}")
    } else {
        clean.to_string()
    }
}

pub fn rand_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    t ^ COUNTER.fetch_add(0x9E3779B97F4A7C15, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn encrypted_file_transfer_matches_bytes_and_preserves_existing_download() {
        let tmp = std::env::temp_dir().join(format!("sc_send_{}", rand_id()));
        let recv_dir = tmp.join("in");
        fs::create_dir_all(&recv_dir).unwrap();
        let src = tmp.join("payload.bin");
        let data: Vec<u8> = (0..(CHUNK * 2 + 123)).map(|i| (i % 251) as u8).collect();
        fs::write(&src, &data).unwrap();
        fs::write(recv_dir.join("payload.bin"), b"keep me").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let dest = recv_dir.clone();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let (mut conn, _) = BulkConn::handshake(stream, b"test-psk", Role::Responder).unwrap();
            let mut rx = FileReceiver::new(dest);
            loop {
                if let Some((id, _)) = rx.handle(&conn.recv().unwrap()).unwrap() {
                    conn.send(&BulkMsg::FileReceived { id }).unwrap();
                    break;
                }
            }
        });
        send_file_with_psk(addr, &src, b"test-psk").unwrap();
        server.join().unwrap();
        assert_eq!(fs::read(recv_dir.join("payload (1).bin")).unwrap(), data);
        assert_eq!(fs::read(recv_dir.join("payload.bin")).unwrap(), b"keep me");
        fs::remove_dir_all(tmp).unwrap();
    }

    #[test]
    fn incomplete_or_out_of_order_files_are_not_published() {
        let tmp = std::env::temp_dir().join(format!("sc_bad_{}", rand_id()));
        let mut rx = FileReceiver::new(&tmp);
        rx.handle(&BulkMsg::FileBegin {
            id: 1,
            name: "x.txt".into(),
            size: 10,
        })
        .unwrap();
        assert!(rx
            .handle(&BulkMsg::FileChunk {
                id: 1,
                offset: 4,
                data: vec![0; 6]
            })
            .is_err());
        assert!(rx.handle(&BulkMsg::FileEnd { id: 1 }).is_err());
        assert!(!tmp.join("x.txt").exists());
        assert_eq!(fs::read_dir(&tmp).unwrap().count(), 0);
        rx.handle(&BulkMsg::FileBegin {
            id: 2,
            name: "y.txt".into(),
            size: 10,
        })
        .unwrap();
        drop(rx);
        assert_eq!(fs::read_dir(&tmp).unwrap().count(), 0);
        fs::remove_dir_all(tmp).unwrap();
    }

    #[test]
    fn names_are_safe_on_both_operating_systems() {
        assert_eq!(sanitize("C:\\temp\\a.txt"), "a.txt");
        assert_eq!(sanitize("../../b.txt"), "b.txt");
        assert_eq!(sanitize(".."), "received.bin");
        assert_eq!(sanitize("CON.txt"), "_CON.txt");
        assert_eq!(sanitize("a:b?.txt"), "a_b_.txt");
    }
}
