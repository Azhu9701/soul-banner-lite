//! SoulStore — 内置 Storage 实现，开箱即用。
//!
//! 基于 FileStore + SqliteDb，实现 foundation::Storage trait。
//! Soul Agent SDK 的默认存储后端。

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use foundation::{
    FileStore, SqliteDb, Storage, Result, HealthStatus,
    Session, SessionFilter, SessionSummary, SessionObservation,
    Message, CallRecord, CallFilter,
    SoulProfile, SoulRevision, SoulRevisionFilter,
    Registry, Annotation,
    BlindSpot, BlindSpotFilter,
    KnowledgeCard, KnowledgeCardFilter, KnowledgeTopic,
    RevisionProposal, ProposalStatus,
};

pub struct SoulStore {
    pub fs: Arc<FileStore>,
    pub db: Arc<SqliteDb>,
}

impl SoulStore {
    pub fn new(data_dir: &str) -> Result<Self> {
        let base = PathBuf::from(data_dir);
        std::fs::create_dir_all(&base)?;

        let mut fs = FileStore::new(
            base.join("souls"),
            base.join("archive"),
            base.join("registry.yaml"),
            base.join("call-records.yaml"),
        )?;

        // Load internal souls
        if let Ok(internal) = std::env::var("WANMINFAN_SOULS_INTERNAL_DIR") {
            fs.set_souls_internal_dir(PathBuf::from(internal));
        } else {
            let default = base.join("souls-internal");
            if default.exists() {
                fs.set_souls_internal_dir(default);
            }
        }

        let fs = Arc::new(fs);
        std::fs::create_dir_all(base.join("db"))?;
        let db = Arc::new(SqliteDb::open(&base.join("soul-agent.db"))?);
        Ok(SoulStore { fs, db })
    }

    pub fn db(&self) -> Arc<SqliteDb> { self.db.clone() }
}

// ── 全部委托给 FileStore / SqliteDb ──

#[async_trait]
impl Storage for SoulStore {
    async fn read_soul(&self, n: &str) -> Result<SoulProfile> { self.fs.read_soul(n) }
    async fn write_soul(&self, p: &SoulProfile) -> Result<()> { self.fs.write_soul(p) }
    async fn delete_soul(&self, n: &str) -> Result<()> { self.fs.delete_soul(n) }
    async fn list_soul_names(&self) -> Result<Vec<String>> { self.fs.list_soul_names() }
    async fn read_registry(&self) -> Result<Registry> { self.fs.read_registry_raw() }
    async fn write_registry(&self, r: &Registry) -> Result<()> { self.fs.write_registry_raw(r) }

    async fn create_session(&self, s: &Session) -> Result<()> { self.db.insert_session(s) }
    async fn update_session(&self, s: &Session) -> Result<()> { self.db.update_session(s) }
    async fn delete_session(&self, id: &str) -> Result<()> { self.db.delete_session(id) }
    async fn get_session(&self, id: &str) -> Result<Session> { self.db.get_session(id) }
    async fn list_sessions(&self, f: &SessionFilter) -> Result<Vec<SessionSummary>> { self.db.list_sessions(f) }

    async fn append_message(&self, m: &Message) -> Result<()> { self.db.append_message(m) }
    async fn get_messages(&self, sid: &str) -> Result<Vec<Message>> { self.db.get_messages(sid) }
    async fn delete_messages_from_seq(&self, sid: &str, seq: i64) -> Result<u32> { self.db.delete_messages_from_seq(sid, seq) }

    async fn record_call(&self, r: &CallRecord) -> Result<()> { self.db.insert_call_record(r) }
    async fn query_call_records(&self, f: &CallFilter) -> Result<Vec<CallRecord>> { self.db.query_call_records(f) }

    async fn archive_soul_output(&self, sid: &str, soul: &str, content: &str) -> Result<String> {
        let filename = format!("{}-{}.md", soul, chrono::Utc::now().format("%Y%m%d-%H%M%S"));
        self.fs.archive_output(sid, &filename, content)
    }
    async fn archive_synthesis(&self, sid: &str, content: &str) -> Result<String> {
        self.fs.archive_output(sid, "synthesis.md", content)
    }
    async fn read_archive(&self, path: &str) -> Result<String> { self.fs.read_archive_path(path) }

    async fn search_knowledge(&self, q: &str, limit: usize) -> Result<Vec<foundation::sqlite::KnowledgeResult>> { self.db.search_knowledge(q, limit) }
    async fn rebuild_fts(&self) -> Result<usize> { self.db.rebuild_fts() }

    async fn insert_soul_revision(&self, r: &SoulRevision) -> Result<()> { self.db.insert_soul_revision(r) }
    async fn get_soul_revisions(&self, f: &SoulRevisionFilter) -> Result<Vec<SoulRevision>> { self.db.get_soul_revisions(f) }

    async fn insert_blind_spot(&self, b: &BlindSpot) -> Result<()> { self.db.insert_blind_spot(b) }
    async fn update_blind_spot(&self, b: &BlindSpot) -> Result<()> { self.db.update_blind_spot(b) }
    async fn get_blind_spots(&self, f: &BlindSpotFilter) -> Result<Vec<BlindSpot>> { self.db.get_blind_spots(f) }

    async fn insert_knowledge_card(&self, c: &KnowledgeCard) -> Result<()> { self.db.insert_knowledge_card(c) }
    async fn update_knowledge_card(&self, c: &KnowledgeCard) -> Result<()> { self.db.update_knowledge_card(c) }
    async fn get_knowledge_cards(&self, f: &KnowledgeCardFilter) -> Result<Vec<KnowledgeCard>> { self.db.get_knowledge_cards(f) }

    async fn list_knowledge_topics(&self, mode: Option<&str>, limit: usize, offset: usize) -> Result<Vec<foundation::KnowledgeTopic>> { self.db.list_knowledge_topics(mode, limit, offset) }

    async fn insert_revision_proposal(&self, p: &RevisionProposal) -> Result<()> { self.db.insert_revision_proposal(p) }
    async fn update_revision_proposal(&self, p: &RevisionProposal) -> Result<()> { self.db.update_revision_proposal(p) }
    async fn get_revision_proposals(&self, sn: Option<&str>, s: Option<ProposalStatus>) -> Result<Vec<RevisionProposal>> { self.db.get_revision_proposals(sn, s) }

    async fn insert_session_observations(&self, o: &[SessionObservation]) -> Result<()> { self.db.insert_session_observations(o) }
    async fn get_session_observations(&self, sid: &str) -> Result<Vec<SessionObservation>> { self.db.get_session_observations(sid) }
    async fn get_observations_by_soul(&self, sn: &str, limit: u32) -> Result<Vec<SessionObservation>> { self.db.get_observations_by_soul(sn, limit) }
    async fn update_session_digest(&self, sid: &str, summary: &str) -> Result<()> { self.db.update_session_digest(sid, summary) }

    async fn insert_annotations(&self, a: &[Annotation]) -> Result<()> { self.db.insert_annotations(a) }
    async fn get_annotations(&self, sid: &str) -> Result<Vec<Annotation>> { self.db.get_annotations(sid) }

    async fn health_check(&self) -> Result<HealthStatus> {
        Ok(HealthStatus { ok: true, sqlite_ok: true, fs_ok: true, yaml_count: 0, sqlite_record_count: 0, soul_files_count: 0, registry_entries_count: 0 })
    }
}
