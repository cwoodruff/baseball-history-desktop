//! Response cache for API GETs, in memory and on disk.
//!
//! The data is historical and changes about once a year, so a response stays
//! fresh for [`FRESH_FOR`]. A stale response is still served when the service
//! cannot be reached. Reload marks everything cached so far as stale, so the
//! page it reloads (and any page visited after) asks the service again.
//!
//! Each response is one file in the platform cache directory, named by a hash
//! of its URL. The file starts with the URL and the fetch time, one per line,
//! followed by the body.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long a response is used without asking the service again.
pub const FRESH_FOR: Duration = Duration::from_secs(24 * 60 * 60);
/// Files older than this are deleted when the cache opens.
const KEEP_FOR: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// Responses kept in memory; the oldest are dropped past this.
const MEMORY_ENTRIES: usize = 512;

#[derive(Clone)]
pub struct Entry {
    pub body: Arc<str>,
    pub fetched: SystemTime,
}

pub struct Cache {
    dir: Option<PathBuf>,
    memory: Mutex<HashMap<String, Entry>>,
    /// Entries fetched before this are stale (set by Reload).
    stale_before: Mutex<SystemTime>,
}

/// A cached response and whether it can be used without asking the service.
pub struct Hit {
    pub body: Arc<str>,
    pub fresh: bool,
}

impl Cache {
    /// A cache backed by `dir`, or memory only when `dir` is `None`.
    pub fn new(dir: Option<PathBuf>) -> Self {
        let dir = dir.filter(|dir| fs::create_dir_all(dir).is_ok());
        if let Some(dir) = dir.clone() {
            std::thread::spawn(move || prune(&dir, KEEP_FOR));
        }
        Self {
            dir,
            memory: Mutex::new(HashMap::new()),
            stale_before: Mutex::new(UNIX_EPOCH),
        }
    }

    pub fn get(&self, url: &str) -> Option<Hit> {
        let entry = self.memory_get(url).or_else(|| {
            let entry = read_file(&self.file(url)?, url)?;
            self.memory_put(url, entry.clone());
            Some(entry)
        })?;
        let stale_before = *self.stale_before.lock().unwrap();
        let fresh = entry.fetched >= stale_before
            && entry.fetched.elapsed().is_ok_and(|age| age < FRESH_FOR);
        Some(Hit {
            body: entry.body,
            fresh,
        })
    }

    pub fn put(&self, url: &str, body: Arc<str>) {
        let entry = Entry {
            body,
            fetched: SystemTime::now(),
        };
        if let Some(path) = self.file(url) {
            // A failed write only costs a refetch next launch.
            let _ = write_file(&path, url, &entry);
        }
        self.memory_put(url, entry);
    }

    /// Marks every response cached so far as stale.
    pub fn invalidate(&self) {
        *self.stale_before.lock().unwrap() = SystemTime::now();
    }

    fn memory_get(&self, url: &str) -> Option<Entry> {
        self.memory.lock().unwrap().get(url).cloned()
    }

    fn memory_put(&self, url: &str, entry: Entry) {
        let mut memory = self.memory.lock().unwrap();
        if memory.len() >= MEMORY_ENTRIES && !memory.contains_key(url) {
            let oldest = memory
                .iter()
                .min_by_key(|(_, e)| e.fetched)
                .map(|(k, _)| k.clone());
            if let Some(oldest) = oldest {
                memory.remove(&oldest);
            }
        }
        memory.insert(url.to_string(), entry);
    }

    fn file(&self, url: &str) -> Option<PathBuf> {
        Some(self.dir.as_ref()?.join(format!("{:016x}.json", fnv1a(url))))
    }
}

/// The platform's per-user cache directory for this app.
pub fn default_dir() -> Option<PathBuf> {
    let env = |key| {
        std::env::var_os(key)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let base = if cfg!(target_os = "macos") {
        env("HOME").map(|home| home.join("Library/Caches"))
    } else if cfg!(windows) {
        env("LOCALAPPDATA")
    } else {
        env("XDG_CACHE_HOME").or_else(|| env("HOME").map(|home| home.join(".cache")))
    }?;
    Some(base.join("com.baseballhistoryproject.desktop").join("api"))
}

/// FNV-1a: a hash that is stable across builds, unlike `DefaultHasher`.
fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn read_file(path: &Path, url: &str) -> Option<Entry> {
    let text = fs::read_to_string(path).ok()?;
    let (stored_url, rest) = text.split_once('\n')?;
    let (secs, body) = rest.split_once('\n')?;
    // A different URL is a hash collision; treat it as a miss.
    if stored_url != url {
        return None;
    }
    Some(Entry {
        body: body.into(),
        fetched: UNIX_EPOCH + Duration::from_secs(secs.parse().ok()?),
    })
}

fn write_file(path: &Path, url: &str, entry: &Entry) -> std::io::Result<()> {
    let secs = entry
        .fetched
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Write then rename, so a reader never sees half a file.
    let partial = path.with_extension("partial");
    fs::write(&partial, format!("{url}\n{secs}\n{}", entry.body))?;
    fs::rename(&partial, path)
}

fn prune(dir: &Path, keep_for: Duration) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > keep_for);
        if old {
            let _ = fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bhd-cache-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn serves_fresh_entries_from_disk_across_instances() {
        let dir = temp_dir("disk");
        Cache::new(Some(dir.clone())).put("https://x/api/a?b=1", "{\"a\":1}".into());
        let hit = Cache::new(Some(dir.clone()))
            .get("https://x/api/a?b=1")
            .unwrap();
        assert_eq!(&*hit.body, "{\"a\":1}");
        assert!(hit.fresh);
        assert!(
            Cache::new(Some(dir.clone()))
                .get("https://x/api/a?b=2")
                .is_none()
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalidate_marks_entries_stale_but_keeps_them() {
        let cache = Cache::new(None);
        cache.put("u", "body".into());
        cache.invalidate();
        let hit = cache.get("u").unwrap();
        assert!(!hit.fresh);
        assert_eq!(&*hit.body, "body");
    }

    #[test]
    fn bodies_with_newlines_round_trip() {
        let dir = temp_dir("newlines");
        Cache::new(Some(dir.clone())).put("u", "[\n1,\n2\n]".into());
        let hit = Cache::new(Some(dir.clone())).get("u").unwrap();
        assert_eq!(&*hit.body, "[\n1,\n2\n]");
        let _ = fs::remove_dir_all(&dir);
    }
}
