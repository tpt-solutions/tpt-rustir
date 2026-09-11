//! Proof-state cache: remembers which spec obligations were last discharged
//! successfully, keyed on a hash of the spec source string, so `cargo tpt
//! verify` doesn't redo expensive e-graph/SMT work for unchanged specs.
//!
//! The cache only ever remembers *successes*. A cache miss (or a disabled
//! cache) always falls through to a real verification; failures are never
//! cached, so a fixed spec is picked up on the very next run.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

const CACHE_VERSION: u32 = 1;

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct CacheFile {
    version: u32,
    ok: HashSet<String>,
}

pub struct Cache {
    path: PathBuf,
    ok: HashSet<String>,
    dirty: bool,
}

/// Hash a spec string into its cache key.
pub fn hash_spec(spec: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(spec.as_bytes());
    format!("{:x}", hasher.finalize())
}

impl Cache {
    /// Load the cache from `<root>/target/tpt/cache.json`, or start empty if
    /// it doesn't exist or fails to parse (a corrupt/stale cache is never a
    /// hard error — it just means everything re-verifies once).
    pub fn load(root: &Path) -> Self {
        let path = cache_path(root);
        let ok = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<CacheFile>(&s).ok())
            .filter(|c| c.version == CACHE_VERSION)
            .map(|c| c.ok)
            .unwrap_or_default();
        Cache {
            path,
            ok,
            dirty: false,
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.ok.contains(key)
    }

    pub fn record_ok(&mut self, key: &str) {
        if self.ok.insert(key.to_string()) {
            self.dirty = true;
        }
    }

    /// Persist the cache to disk, if it changed. Best-effort: a write
    /// failure (e.g. no `target/` directory) is silently ignored, since the
    /// cache is purely a performance optimization.
    pub fn save(&self) {
        if !self.dirty {
            return;
        }
        let file = CacheFile {
            version: CACHE_VERSION,
            ok: self.ok.clone(),
        };
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&file) {
            let _ = std::fs::write(&self.path, json);
        }
    }
}

fn cache_path(root: &Path) -> PathBuf {
    root.join("target").join("tpt").join("cache.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_content_sensitive() {
        assert_eq!(hash_spec("a"), hash_spec("a"));
        assert_ne!(hash_spec("a"), hash_spec("b"));
    }

    #[test]
    fn missing_cache_file_starts_empty() {
        let dir = std::env::temp_dir().join(format!("tpt-cache-test-{}", std::process::id()));
        let cache = Cache::load(&dir);
        assert!(!cache.contains("anything"));
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = std::env::temp_dir().join(format!(
            "tpt-cache-roundtrip-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let key = hash_spec("(+ a b) = (+ b a)");

        let mut cache = Cache::load(&dir);
        assert!(!cache.contains(&key));
        cache.record_ok(&key);
        cache.save();

        let reloaded = Cache::load(&dir);
        assert!(reloaded.contains(&key));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
