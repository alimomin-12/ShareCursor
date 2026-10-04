//! Settings + monitor manager, persisted as TOML.
//!
//! The **monitor manager** is the layout of machines: which peer sits on each
//! screen edge. Auto edge-switching uses it to decide where the cursor goes
//! when it leaves a screen. The same file holds the pre-shared key used to
//! authenticate + encrypt the session.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sharecursor_protocol::Edge;

/// Top-level configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// This machine's name (must match one entry in `machines`).
    pub name: String,
    /// Pre-shared key / passphrase — authenticates peers and derives the
    /// session encryption keys. Keep it secret and identical on both machines.
    pub psk: String,
    /// Port used by both the input (UDP) and bulk (TCP) channels.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Enable moving control by pushing the cursor into a screen edge that has
    /// a neighbour (in addition to the F12 hotkey).
    #[serde(default = "default_true")]
    pub auto_edge_switch: bool,
    /// For a client: the server's host (or `host:port`) to connect to. Lets
    /// `connect` and the tray "Start Client" run without a CLI argument.
    #[serde(default)]
    pub server_host: Option<String>,
    /// Which side this machine runs as: `"server"` (shares its keyboard & mouse)
    /// or `"client"` (is controlled).
    #[serde(default)]
    pub role: Option<String>,
    /// Stable random device identity (generated on first run). Pairing and
    /// peer identity key off this — names are just display labels.
    #[serde(default)]
    pub device_id: Option<String>,
    /// Arrangement offset (pixels): the *other* screen's top (for left/right
    /// adjacency) or left (for top/bottom) relative to this screen's, so the
    /// cursor crosses at the exact placed position. 0 = tops/edges aligned.
    #[serde(default)]
    pub offset: i32,
    /// The machines participating and their geometry/neighbours.
    pub machines: Vec<Machine>,
}

/// One machine in the layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Machine {
    pub name: String,
    /// Screen size in pixels. Optional — auto-detected at runtime if omitted.
    /// Set it only to override a wrong auto-detection.
    #[serde(default)]
    pub screen: Option<(u32, u32)>,
    /// Neighbour machine names by edge (any may be absent).
    #[serde(default)]
    pub left: Option<String>,
    #[serde(default)]
    pub right: Option<String>,
    #[serde(default)]
    pub top: Option<String>,
    #[serde(default)]
    pub bottom: Option<String>,
}

fn default_port() -> u16 {
    24800
}
fn default_true() -> bool {
    true
}

impl Config {
    /// Load-or-create the stable device id, persisting it on first use.
    pub fn ensure_device_id(path: &std::path::Path) -> String {
        let mut cfg = match Self::load(path) {
            Ok(c) => c,
            Err(_) => return "anon".into(),
        };
        if let Some(id) = &cfg.device_id {
            return id.clone();
        }
        use std::time::{SystemTime, UNIX_EPOCH};
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let id = format!("{:016x}", (n as u64) ^ ((std::process::id() as u64) << 48));
        cfg.device_id = Some(id.clone());
        let _ = cfg.save(path);
        id
    }

    /// Default config path, e.g. `~/.config/sharecursor/config.toml`.
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sharecursor")
            .join("config.toml")
    }

    /// Load and parse a config file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("cannot read config {}: {e}", path.display()))?;
        Self::parse(&text, path)
    }

    /// Initialize a fresh installation before starting any background workers.
    /// Existing files, including invalid ones, are never replaced.
    pub fn load_or_create(path: &Path) -> anyhow::Result<Self> {
        use std::io::{ErrorKind, Write};

        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text, path),
            Err(e) if e.kind() == ErrorKind::NotFound => {
                let cfg = Self::starter()?;
                if let Some(dir) = path.parent() {
                    std::fs::create_dir_all(dir)?;
                }
                let text = cfg.to_toml()?;
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                {
                    Ok(mut file) => {
                        file.write_all(text.as_bytes())?;
                        Ok(cfg)
                    }
                    // Another launch may have initialized the file meanwhile.
                    Err(e) if e.kind() == ErrorKind::AlreadyExists => Self::load(path),
                    Err(e) => Err(anyhow::anyhow!(
                        "cannot create config {}: {e}",
                        path.display()
                    )),
                }
            }
            Err(e) => Err(anyhow::anyhow!(
                "cannot read config {}: {e}",
                path.display()
            )),
        }
    }

    fn parse(text: &str, path: &Path) -> anyhow::Result<Self> {
        let cfg: Config = toml::from_str(text)
            .map_err(|e| anyhow::anyhow!("invalid config {}: {e}", path.display()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// A platform-appropriate starter without a hard-coded host or role.
    fn starter() -> anyhow::Result<Self> {
        let mut cfg = Self::example();
        cfg.name = if cfg!(target_os = "windows") {
            "windows"
        } else {
            "mac"
        }
        .into();
        cfg.psk = Self::generate_pairing_code()?;
        cfg.server_host = None;
        cfg.role = None;
        cfg.device_id = Some(Self::random_hex()?);
        cfg.validate()?;
        Ok(cfg)
    }

    /// Generate a pairing passphrase with 128 bits of OS-provided randomness.
    pub fn generate_pairing_code() -> anyhow::Result<String> {
        let hex = Self::random_hex()?;
        Ok(format!(
            "{}-{}-{}-{}",
            &hex[..8],
            &hex[8..16],
            &hex[16..24],
            &hex[24..]
        ))
    }

    fn random_hex() -> anyhow::Result<String> {
        let mut bytes = [0u8; 16];
        getrandom::getrandom(&mut bytes)
            .map_err(|e| anyhow::anyhow!("cannot generate pairing identity: {e}"))?;
        Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
    }

    /// Serialize to a TOML string.
    pub fn to_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Write to `path`, creating parent directories.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.to_toml()?)?;
        Ok(())
    }

    /// A ready-to-edit two-machine example (mac ↔ windows, side by side).
    pub fn example() -> Self {
        Config {
            name: "mac".into(),
            psk: "change-me-to-a-long-random-passphrase".into(),
            port: default_port(),
            auto_edge_switch: true,
            server_host: Some("192.168.1.20".into()),
            role: Some("server".into()),
            device_id: None,
            offset: 0,
            machines: vec![
                Machine {
                    name: "mac".into(),
                    screen: None,
                    left: None,
                    right: Some("windows".into()),
                    top: None,
                    bottom: None,
                },
                Machine {
                    name: "windows".into(),
                    screen: None,
                    left: Some("mac".into()),
                    right: None,
                    top: None,
                    bottom: None,
                },
            ],
        }
    }

    /// Basic sanity checks.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.psk.len() < 8 {
            anyhow::bail!("psk must be at least 8 characters");
        }
        if self.machine(&self.name).is_none() {
            anyhow::bail!("name '{}' is not present in [machines]", self.name);
        }
        // Every referenced neighbour must exist.
        for m in &self.machines {
            for n in [&m.left, &m.right, &m.top, &m.bottom].into_iter().flatten() {
                if self.machine(n).is_none() {
                    anyhow::bail!("machine '{}' references unknown neighbour '{}'", m.name, n);
                }
            }
        }
        Ok(())
    }

    /// Look up a machine by name.
    pub fn machine(&self, name: &str) -> Option<&Machine> {
        self.machines.iter().find(|m| m.name == name)
    }

    /// The neighbour of `machine` across `edge`, if any.
    #[allow(dead_code)]
    pub fn neighbor(&self, machine: &str, edge: Edge) -> Option<&str> {
        let m = self.machine(machine)?;
        match edge {
            Edge::Left => m.left.as_deref(),
            Edge::Right => m.right.as_deref(),
            Edge::Top => m.top.as_deref(),
            Edge::Bottom => m.bottom.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "sharecursor-config-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn config_path(&self) -> PathBuf {
            self.0.join("sharecursor").join("config.toml")
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn first_launch_creates_config_and_parent_directory() {
        let dir = TestDir::new();
        let path = dir.config_path();
        let cfg = Config::load_or_create(&path).unwrap();

        assert_eq!(Config::load(&path).unwrap(), cfg);
        assert_eq!(cfg.name, if cfg!(windows) { "windows" } else { "mac" });
        assert_eq!(cfg.role, None);
        assert_eq!(cfg.server_host, None);
        assert_eq!(cfg.psk.len(), 35);
        assert_eq!(cfg.device_id.as_ref().unwrap().len(), 32);
    }

    #[test]
    fn relaunch_preserves_settings_and_identity() {
        let dir = TestDir::new();
        let path = dir.config_path();
        let mut cfg = Config::load_or_create(&path).unwrap();
        cfg.port = 25000;
        cfg.psk = "my-existing-shared-passphrase".into();
        cfg.role = Some("client".into());
        cfg.save(&path).unwrap();
        let before = std::fs::read(&path).unwrap();

        assert_eq!(Config::load_or_create(&path).unwrap(), cfg);
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn invalid_existing_config_is_reported_and_preserved() {
        let dir = TestDir::new();
        let path = dir.config_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let text = "this is not valid TOML";
        std::fs::write(&path, text).unwrap();

        assert!(Config::load_or_create(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);

        let mut invalid = Config::example();
        invalid.psk = "short".into();
        invalid.save(&path).unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(Config::load_or_create(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn config_read_errors_do_not_trigger_replacement() {
        let dir = TestDir::new();
        let path = dir.config_path();
        std::fs::create_dir_all(&path).unwrap();

        assert!(Config::load_or_create(&path).is_err());
        assert!(path.is_dir());
    }

    #[test]
    fn separate_installs_have_distinct_passphrases_and_device_ids() {
        let first = TestDir::new();
        let second = TestDir::new();
        let a = Config::load_or_create(&first.config_path()).unwrap();
        let b = Config::load_or_create(&second.config_path()).unwrap();

        assert_ne!(a.psk, b.psk);
        assert_ne!(a.device_id, b.device_id);
    }

    #[test]
    fn example_roundtrips_through_toml() {
        let cfg = Config::example();
        let text = cfg.to_toml().unwrap();
        let parsed: Config = toml::from_str(&text).unwrap();
        assert_eq!(parsed, cfg);
        parsed.validate().unwrap();
    }

    #[test]
    fn neighbor_lookup_works() {
        let cfg = Config::example();
        assert_eq!(cfg.neighbor("mac", Edge::Right), Some("windows"));
        assert_eq!(cfg.neighbor("mac", Edge::Left), None);
        assert_eq!(cfg.neighbor("windows", Edge::Left), Some("mac"));
    }

    #[test]
    fn validate_rejects_unknown_neighbour() {
        let mut cfg = Config::example();
        cfg.machines[0].right = Some("ghost".into());
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_rejects_short_psk() {
        let mut cfg = Config::example();
        cfg.psk = "short".into();
        assert!(cfg.validate().is_err());
    }
}
