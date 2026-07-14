use chrono::Utc;
use foundation::SqliteDb;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;

/// SQLite-backed SearXNG search result cache.
pub struct SearxngCache {
    db: Arc<SqliteDb>,
    ttl_secs: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CachedResult {
    pub query: String,
    pub number_of_results: u64,
    pub results_json: String,
    pub suggestions_json: String,
    pub unresponsive_engines_json: String,
}

#[derive(Debug, Clone)]
struct CacheRow {
    pub query: String,
    pub number_of_results: u64,
    pub results_json: String,
    pub suggestions_json: String,
    pub unresponsive_engines_json: String,
    pub created_at: i64,
}

impl SearxngCache {
    pub fn new(db: Arc<SqliteDb>, ttl_secs: u64) -> Self {
        let cache = SearxngCache { db, ttl_secs };
        cache.ensure_table();
        cache
    }

    fn ensure_table(&self) {
        let _ = self.db.with_conn(|conn| {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS searxng_cache (
                    cache_key TEXT PRIMARY KEY,
                    query TEXT NOT NULL,
                    number_of_results INTEGER NOT NULL DEFAULT 0,
                    results_json TEXT NOT NULL DEFAULT '[]',
                    suggestions_json TEXT NOT NULL DEFAULT '[]',
                    unresponsive_engines_json TEXT NOT NULL DEFAULT '[]',
                    created_at INTEGER NOT NULL
                )",
            )?;
            Ok(())
        });
    }

    pub fn cache_key(query: &str, language: &str, pageno: u32, categories: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        hasher.update(b"|");
        hasher.update(language.as_bytes());
        hasher.update(b"|");
        hasher.update(pageno.to_string().as_bytes());
        hasher.update(b"|");
        hasher.update(categories.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn get(
        &self,
        query: &str,
        language: &str,
        pageno: u32,
        categories: &str,
    ) -> Option<CachedResult> {
        let key = Self::cache_key(query, language, pageno, categories);
        let now = Utc::now().timestamp();
        let ttl = self.ttl_secs as i64;

        self.db
            .with_conn(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT query, number_of_results, results_json, suggestions_json,
                                unresponsive_engines_json, created_at
                         FROM searxng_cache WHERE cache_key = ?1",
                    )?;
                let row = stmt.query_row(rusqlite::params![&key], |row| {
                    Ok(CacheRow {
                        query: row.get(0)?,
                        number_of_results: row.get::<_, i64>(1)? as u64,
                        results_json: row.get(2)?,
                        suggestions_json: row.get(3)?,
                        unresponsive_engines_json: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                });

                match row {
                    Ok(r) => {
                        if now - r.created_at > ttl {
                            let _ = conn.execute(
                                "DELETE FROM searxng_cache WHERE cache_key = ?1",
                                rusqlite::params![&key],
                            );
                            Ok(None)
                        } else {
                            Ok(Some(CachedResult {
                                query: r.query,
                                number_of_results: r.number_of_results,
                                results_json: r.results_json,
                                suggestions_json: r.suggestions_json,
                                unresponsive_engines_json: r.unresponsive_engines_json,
                            }))
                        }
                    }
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(foundation::FoundationError::Sqlite(e)),
                }
            })
            .ok()
            .flatten()
    }

    pub fn set(
        &self,
        query: &str,
        language: &str,
        pageno: u32,
        categories: &str,
        results_json: &str,
        suggestions_json: &str,
        unresponsive_engines_json: &str,
        number_of_results: u64,
    ) {
        let key = Self::cache_key(query, language, pageno, categories);
        let now = Utc::now().timestamp();
        let q = query.to_string();
        let rj = results_json.to_string();
        let sj = suggestions_json.to_string();
        let uj = unresponsive_engines_json.to_string();

        let _ = self.db.with_conn(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO searxng_cache
                 (cache_key, query, number_of_results, results_json,
                  suggestions_json, unresponsive_engines_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    &key,
                    &q,
                    number_of_results as i64,
                    &rj,
                    &sj,
                    &uj,
                    now,
                ],
            )?;
            Ok(())
        });
    }

    pub fn cleanup(&self) {
        let ttl = self.ttl_secs as i64;
        let cutoff = Utc::now().timestamp() - ttl;
        let _ = self.db.with_conn(move |conn| {
            conn.execute(
                "DELETE FROM searxng_cache WHERE created_at < ?1",
                rusqlite::params![cutoff],
            )?;
            Ok(())
        });
    }

    pub fn ttl(&self) -> Duration {
        Duration::from_secs(self.ttl_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn setup_cache() -> (SearxngCache, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(foundation::SqliteDb::open(&db_path).unwrap());
        let cache = SearxngCache::new(db, 5); // 5-second TTL for testing
        (cache, dir)
    }

    #[test]
    fn test_cache_key_deterministic() {
        let k1 = SearxngCache::cache_key("劳动法", "zh", 1, "");
        let k2 = SearxngCache::cache_key("劳动法", "zh", 1, "");
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_cache_key_different() {
        let k1 = SearxngCache::cache_key("劳动法", "zh", 1, "");
        let k2 = SearxngCache::cache_key("劳动合同", "zh", 1, "");
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_cache_miss_on_empty() {
        let (cache, _dir) = setup_cache();
        let result = cache.get("劳动法", "zh", 1, "");
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_hit_after_set() {
        let (cache, _dir) = setup_cache();
        cache.set("劳动争议", "zh", 1, "", "[{\"title\":\"test\"}]", "[]", "[]", 1);

        let result = cache.get("劳动争议", "zh", 1, "");
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.number_of_results, 1);
        assert!(r.results_json.contains("test"));
    }

    #[test]
    fn test_cache_expiry() {
        let (cache, _dir) = setup_cache();
        cache.set("劳动争议", "zh", 1, "", "[{\"title\":\"test\"}]", "[]", "[]", 1);

        // Should hit immediately
        assert!(cache.get("劳动争议", "zh", 1, "").is_some());

        // Wait for TTL to expire (5 sec)
        std::thread::sleep(Duration::from_secs(6));

        // Should miss after expiry
        assert!(cache.get("劳动争议", "zh", 1, "").is_none());
    }

    #[test]
    fn test_cleanup_removes_expired() {
        let (cache, _dir) = setup_cache();
        cache.set("expired_query", "zh", 1, "", "[{\"title\":\"x\"}]", "[]", "[]", 1);

        std::thread::sleep(Duration::from_secs(6));
        cache.cleanup();

        assert!(cache.get("expired_query", "zh", 1, "").is_none());
    }

    #[test]
    fn test_ttl_value() {
        let (cache, _dir) = setup_cache();
        assert_eq!(cache.ttl(), Duration::from_secs(5));
    }
}
