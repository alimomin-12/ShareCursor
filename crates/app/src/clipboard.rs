//! Native text, image and file clipboard synchronization, with echo suppression.
use crate::filexfer::{rand_id, stream_file, MAX_FILES};
use arboard::{Clipboard, ImageData};
use sharecursor_protocol::{BulkMsg, ClipboardData};
use std::borrow::Cow;
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

#[derive(Clone, PartialEq)]
pub(crate) enum Fingerprint {
    Text(String),
    Image(u64),
    Files(Vec<PathBuf>, u64),
}
pub(crate) type LastSeen = Arc<Mutex<Option<Fingerprint>>>;

pub(crate) enum ApplyClipboard {
    Data(ClipboardData),
    Files(Vec<PathBuf>),
}

fn hash_image(width: u32, height: u32, rgba: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    width.hash(&mut h);
    height.hash(&mut h);
    rgba.hash(&mut h);
    h.finish()
}

fn files_fingerprint(paths: &[PathBuf]) -> Fingerprint {
    let mut h = DefaultHasher::new();
    for path in paths {
        if let Ok(meta) = path.metadata() {
            meta.len().hash(&mut h);
            meta.modified().ok().hash(&mut h);
        }
    }
    Fingerprint::Files(paths.to_vec(), h.finish())
}

pub(crate) fn watch(out: SyncSender<BulkMsg>, last: LastSeen, stop: Arc<AtomicBool>) {
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "clipboard unavailable");
            return;
        }
    };
    #[cfg(windows)]
    let mut sequence = 0;
    while !stop.load(Ordering::Relaxed) {
        #[cfg(windows)]
        let current =
            unsafe { windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber() };
        #[cfg(windows)]
        {
            if current != 0 && current == sequence {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        }
        // Hold the echo guard across the native read/write, so apply cannot set
        // a clipboard between the watcher's read and fingerprint update.
        let mut guard = last.lock().unwrap();
        if let Ok(paths) = clipboard.get().file_list() {
            if !paths.is_empty() {
                #[cfg(windows)]
                {
                    sequence = current;
                }
                let fp = files_fingerprint(&paths);
                let changed = guard.as_ref() != Some(&fp);
                if changed {
                    *guard = Some(fp);
                }
                drop(guard);
                if changed {
                    if paths.len() > MAX_FILES || paths.iter().any(|p| !p.is_file()) {
                        tracing::warn!("clipboard file copy supports up to 256 regular files; zip folders before copying");
                    } else if send_files(&out, &paths).is_err() {
                        return;
                    }
                }
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        }
        let data = if let Ok(text) = clipboard.get_text() {
            Some((Fingerprint::Text(text.clone()), ClipboardData::Text(text)))
        } else if let Ok(img) = clipboard.get_image() {
            let (width, height) = (img.width as u32, img.height as u32);
            let rgba = img.bytes.into_owned();
            Some((
                Fingerprint::Image(hash_image(width, height, &rgba)),
                ClipboardData::Image {
                    width,
                    height,
                    rgba,
                },
            ))
        } else {
            None
        };
        #[cfg(windows)]
        if data.is_some() {
            sequence = current;
        }
        let changed = data.filter(|(fp, _)| guard.as_ref() != Some(fp));
        if let Some((fp, _)) = &changed {
            *guard = Some(fp.clone());
        }
        drop(guard);
        if let Some((_, data)) = changed {
            if out.send(BulkMsg::Clipboard(data)).is_err() {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn send_files(out: &SyncSender<BulkMsg>, paths: &[PathBuf]) -> anyhow::Result<()> {
    let id = rand_id();
    let files: Vec<u64> = paths.iter().map(|_| rand_id()).collect();
    out.send(BulkMsg::ClipboardFilesBegin {
        id,
        files: files.clone(),
    })?;
    for (path, file_id) in paths.iter().zip(files) {
        if let Err(e) = stream_file(path, file_id, |msg| {
            out.send(msg)?;
            Ok(())
        }) {
            tracing::warn!(path = %path.display(), error = %e, "clipboard file transfer failed");
            out.send(BulkMsg::ClipboardFilesCancel { id })?;
            return Ok(());
        }
    }
    out.send(BulkMsg::ClipboardFilesEnd { id })?;
    Ok(())
}

pub(crate) fn apply(inbox: Receiver<ApplyClipboard>, last: LastSeen) {
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "clipboard unavailable");
            return;
        }
    };
    while let Ok(data) = inbox.recv() {
        let fp = match &data {
            ApplyClipboard::Data(ClipboardData::Text(text)) => Fingerprint::Text(text.clone()),
            ApplyClipboard::Data(ClipboardData::Image {
                width,
                height,
                rgba,
            }) => Fingerprint::Image(hash_image(*width, *height, rgba)),
            ApplyClipboard::Files(paths) => files_fingerprint(paths),
        };
        for attempt in 0..3 {
            let mut guard = last.lock().unwrap();
            let result = match &data {
                ApplyClipboard::Data(ClipboardData::Text(text)) => clipboard.set_text(text.clone()),
                ApplyClipboard::Data(ClipboardData::Image {
                    width,
                    height,
                    rgba,
                }) => clipboard.set_image(ImageData {
                    width: *width as usize,
                    height: *height as usize,
                    bytes: Cow::Borrowed(rgba),
                }),
                ApplyClipboard::Files(paths) => clipboard.set().file_list(paths),
            };
            match result {
                Ok(()) => {
                    *guard = Some(fp.clone());
                    break;
                }
                Err(e) if attempt == 2 => tracing::warn!(error = %e, "failed to set clipboard"),
                Err(_) => {}
            }
            drop(guard);
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

pub(crate) fn shared_last() -> LastSeen {
    static LAST: OnceLock<LastSeen> = OnceLock::new();
    LAST.get_or_init(|| Arc::new(Mutex::new(None))).clone()
}

#[derive(Default)]
pub(crate) struct FileClipboard {
    batch: Option<(u64, Vec<u64>)>,
    completed: HashMap<u64, PathBuf>,
}
impl FileClipboard {
    pub fn begin(&mut self, id: u64, files: Vec<u64>) -> anyhow::Result<()> {
        anyhow::ensure!(
            !files.is_empty() && files.len() <= MAX_FILES,
            "invalid clipboard file count"
        );
        anyhow::ensure!(
            files.iter().collect::<HashSet<_>>().len() == files.len(),
            "duplicate clipboard file id"
        );
        self.batch = Some((id, files));
        self.completed.clear();
        Ok(())
    }
    pub fn completed(&mut self, id: u64, path: &Path) {
        if self
            .batch
            .as_ref()
            .is_some_and(|(_, files)| files.contains(&id))
        {
            self.completed.insert(id, path.to_path_buf());
        }
    }
    pub fn finish(&mut self, id: u64) -> anyhow::Result<Vec<PathBuf>> {
        let (batch_id, files) = self
            .batch
            .take()
            .ok_or_else(|| anyhow::anyhow!("no clipboard file batch"))?;
        anyhow::ensure!(
            batch_id == id && files.iter().all(|f| self.completed.contains_key(f)),
            "incomplete clipboard file batch"
        );
        Ok(files
            .iter()
            .map(|f| self.completed.remove(f).unwrap())
            .collect())
    }
    pub fn cancel(&mut self, id: u64) {
        if self
            .batch
            .as_ref()
            .is_some_and(|(batch_id, _)| *batch_id == id)
        {
            self.batch = None;
            self.completed.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_clipboard_requires_all_files_and_preserves_selection_order() {
        let mut batch = FileClipboard::default();
        batch.begin(1, vec![10, 20]).unwrap();
        batch.completed(10, Path::new("a.txt"));
        assert!(batch.finish(1).is_err());
        batch.begin(2, vec![10, 20]).unwrap();
        batch.completed(20, Path::new("b.txt"));
        batch.completed(10, Path::new("a.txt"));
        assert_eq!(
            batch.finish(2).unwrap(),
            vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")]
        );
        batch.begin(3, vec![10]).unwrap();
        batch.cancel(3);
        assert!(batch.finish(3).is_err());
        assert!(batch.begin(4, vec![10, 10]).is_err());
    }

    #[test]
    fn copied_selection_streams_complete_files_before_clipboard_publication() {
        let tmp = std::env::temp_dir().join(format!("sc_clip_{}", rand_id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let paths = vec![tmp.join("file with spaces.txt"), tmp.join("empty.bin")];
        std::fs::write(&paths[0], vec![42; 150_000]).unwrap();
        std::fs::write(&paths[1], []).unwrap();
        let (tx, rx) = std::sync::mpsc::sync_channel(2);
        let send_paths = paths.clone();
        let sender = std::thread::spawn(move || send_files(&tx, &send_paths).unwrap());
        let mut receiver = crate::filexfer::FileReceiver::new(tmp.join("received"));
        let mut batch = FileClipboard::default();
        let mut published = None;
        while let Ok(msg) = rx.recv() {
            match msg {
                BulkMsg::ClipboardFilesBegin { id, files } => batch.begin(id, files).unwrap(),
                BulkMsg::ClipboardFilesEnd { id } => {
                    published = Some(batch.finish(id).unwrap());
                }
                _ => {
                    assert!(published.is_none());
                    if let Some((id, path)) = receiver.handle(&msg).unwrap() {
                        batch.completed(id, &path);
                    }
                }
            }
        }
        sender.join().unwrap();
        let published = published.unwrap();
        assert_eq!(published.len(), 2);
        for (source, destination) in paths.iter().zip(published) {
            assert_eq!(
                std::fs::read(source).unwrap(),
                std::fs::read(destination).unwrap()
            );
        }
        std::fs::remove_dir_all(tmp).unwrap();
    }

    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn native_file_clipboard_roundtrips_local_paths() {
        let tmp = std::env::temp_dir().join(format!("sc_native_clip_{}", rand_id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let paths = vec![tmp.join("file with spaces.txt"), tmp.join("second.bin")];
        for path in &paths {
            std::fs::write(path, b"clipboard test").unwrap();
        }
        let mut clipboard = Clipboard::new().unwrap();
        clipboard.set().file_list(&paths).unwrap();
        assert_eq!(clipboard.get().file_list().unwrap(), paths);
        clipboard.clear().unwrap();
        std::fs::remove_dir_all(tmp).unwrap();
    }
}
