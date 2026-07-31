//! # Storage Module — محرك التخزين الهجين
//!
//! ## طبقتان:
//! 1. **HuggingFace Bucket Mount** — `/data` (دائم — مُعلَّق مباشرة في الحاوية)
//!    الدلو `rayig/Dev-storage` مُربط في `/data` بصلاحية Read & Write.
//!    كل ملف يُكتب في `/data` يُحفظ تلقائياً في الدلو ويبقى بعد إعادة التشغيل.
//! 2. **محلي** — `/app/data` (بديل للتطوير المحلي فقط)
//!
//! ## العزل:
//! كل مستخدم يحصل على مجلد خاص: `users/{user_id}/sessions/{session_id}/`
//! لا يمكن لمستخدم الوصول لملفات مستخدم آخر بسبب path_safety.

pub mod workspace;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::fs;
use libsql::{Builder, Connection};
use tracing::{debug, warn};
use base64::Engine as _;

use crate::path_safety::{UserPathRoot, ensure_safe_path};

// ─── HF Bucket Client ─────────────────────────────────────────────────────────

/// رابط HF Datasets API للتخزين الدائم
const HF_DATASET: &str = "rayig/Dev-storage";
const HF_API_BASE: &str = "https://huggingface.co/api/datasets";

/// رفع ملف إلى HF bucket (dataset)
pub async fn hf_upload_file(
    hf_token: &str,
    user_id: &str,
    session_id: &str,
    file_name: &str,
    content: &str,
) -> Result<(), String> {
    if hf_token.is_empty() {
        return Ok(()); // لا رمز مميز — تخطى الرفع
    }
    let path = format!("users/{user_id}/sessions/{session_id}/{file_name}");
    // HF Datasets commit API
    let commit_url = format!("https://huggingface.co/api/datasets/{HF_DATASET}/commit/main");
    let client = reqwest::Client::new();
    let resp = client
        .post(&commit_url)
        .bearer_auth(hf_token)
        .json(&serde_json::json!({
            "summary": format!("Upload {file_name} for user {user_id}"),
            "files": [{
                "path": path,
                "content": base64::engine::general_purpose::STANDARD.encode(content.as_bytes()),
                "encoding": "base64"
            }]
        }))
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| format!("HF upload send: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        warn!("HF upload failed {status}: {}", &body[..body.len().min(200)]);
        return Err(format!("HF upload {status}"));
    }
    debug!("HF uploaded: {path}");
    Ok(())
}

/// قراءة ملف من HF bucket
pub async fn hf_read_file(
    hf_token: &str,
    user_id: &str,
    session_id: &str,
    file_name: &str,
) -> Result<String, String> {
    if hf_token.is_empty() {
        return Err("no HF token".to_string());
    }
    let path = format!("users/{user_id}/sessions/{session_id}/{file_name}");
    let url = format!("https://huggingface.co/datasets/{HF_DATASET}/resolve/main/{path}");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .bearer_auth(hf_token)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("HF read: {e}"))?;

    if resp.status().as_u16() == 404 {
        return Err("file not found in HF bucket".to_string());
    }
    if !resp.status().is_success() {
        return Err(format!("HF read status {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("HF read body: {e}"))
}

/// قائمة ملفات المستخدم في HF bucket
pub async fn hf_list_files(
    hf_token: &str,
    user_id: &str,
    session_id: &str,
) -> Result<Vec<String>, String> {
    if hf_token.is_empty() {
        return Ok(vec![]);
    }
    let prefix = format!("users/{user_id}/sessions/{session_id}/");
    let url = format!("{HF_API_BASE}/{HF_DATASET}/tree/main/{prefix}");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .bearer_auth(hf_token)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("HF list: {e}"))?;

    if !resp.status().is_success() {
        return Ok(vec![]);
    }
    let items: Vec<serde_json::Value> = resp.json().await.unwrap_or_default();
    Ok(items.iter()
        .filter(|i| i["type"].as_str() == Some("file"))
        .filter_map(|i| {
            i["path"].as_str().and_then(|p| p.strip_prefix(&prefix).map(|s| s.to_string()))
        })
        .collect())
}

/// حذف ملف من HF bucket
pub async fn hf_delete_file(
    hf_token: &str,
    user_id: &str,
    session_id: &str,
    file_name: &str,
) -> Result<(), String> {
    if hf_token.is_empty() {
        return Ok(());
    }
    let path = format!("users/{user_id}/sessions/{session_id}/{file_name}");
    let commit_url = format!("https://huggingface.co/api/datasets/{HF_DATASET}/commit/main");
    let client = reqwest::Client::new();
    let resp = client
        .post(&commit_url)
        .bearer_auth(hf_token)
        .json(&serde_json::json!({
            "summary": format!("Delete {file_name} for user {user_id}"),
            "deletedFiles": [path]
        }))
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("HF delete: {e}"))?;

    if !resp.status().is_success() {
        warn!("HF delete failed {}: {path}", resp.status());
    }
    Ok(())
}

// ─── Session Database ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SessionDb {
    db_path: PathBuf,
}

impl SessionDb {
    pub async fn new(path: &Path) -> Result<Self, String> {
        let db = Self { db_path: path.to_path_buf() };
        db.init().await?;
        Ok(db)
    }

    async fn init(&self) -> Result<(), String> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| format!("Session DB dir: {e}"))?;
        }
        Ok(())
    }

    pub async fn connect(&self) -> Result<Connection, String> {
        let path_str = self.db_path.to_string_lossy().to_string();
        let db = Builder::new_local(path_str)
            .build().await
            .map_err(|e| format!("Session DB open: {e}"))?;
        let conn = db.connect()
            .map_err(|e| format!("Session DB connect: {e}"))?;

        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS files (
                name TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                mime_type TEXT,
                size INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS context (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                description TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL,
                completed_at TEXT
            );
            CREATE TABLE IF NOT EXISTS code_blocks (
                id TEXT PRIMARY KEY,
                language TEXT NOT NULL,
                code TEXT NOT NULL,
                file_name TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS memory (
                key TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                importance INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
        ").await.map_err(|e| format!("Session DB schema: {e}"))?;

        Ok(conn)
    }

    pub fn path(&self) -> &Path { &self.db_path }
}

// ─── Storage Engine ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct StorageEngine {
    base_path: PathBuf,
    hf_token: String,
}

impl StorageEngine {
    pub fn new(base_path: Option<&Path>) -> Self {
        let base = base_path.map(|p| p.to_path_buf()).unwrap_or_else(|| {
            // Priority order:
            // 1. /data  — HuggingFace Storage Bucket mount (persistent, survives restarts)
            //    The bucket rayig/Dev-storage is mounted at /data with Read & Write access
            // 2. /app/data — fallback for local dev
            // 3. REQUIEM_STORAGE env var
            let candidates = ["/data", "/app/data"];
            for p in &candidates {
                let path = PathBuf::from(p);
                if path.exists() {
                    let test = path.join(".perm_test");
                    if std::fs::write(&test, "").is_ok() {
                        std::fs::remove_file(&test).ok();
                        tracing::info!("Storage base: {p} (writable)");
                        return path;
                    }
                }
            }
            let fallback = std::env::var("REQUIEM_STORAGE")
                .unwrap_or_else(|_| "/data".to_string());
            tracing::warn!("Storage base fallback: {fallback}");
            PathBuf::from(fallback)
        });
        let hf_token = std::env::var("HF_TOKEN").unwrap_or_default();
        Self { base_path: base, hf_token }
    }

    pub fn user_root(&self, user_id: &str) -> UserPathRoot {
        UserPathRoot::new(user_id, Some(&self.base_path))
    }

    pub async fn init_user_storage(&self, user_id: &str) -> Result<UserPathRoot, String> {
        let root = self.user_root(user_id);
        root.ensure_dirs().await.map_err(|e| format!("Storage init: {e}"))?;
        Ok(root)
    }

    pub async fn init_session_storage(&self, user_id: &str, session_id: &str) -> Result<SessionDb, String> {
        let root = self.user_root(user_id);
        let sess_dir = root.sessions_dir().join(session_id);
        let files_dir = root.session_files_dir(session_id);
        fs::create_dir_all(&sess_dir).await
            .map_err(|e| format!("Session dir: {e}"))?;
        fs::create_dir_all(&files_dir).await
            .map_err(|e| format!("Files dir: {e}"))?;

        let ctx_path = sess_dir.join("context.json");
        if !ctx_path.exists() {
            fs::write(&ctx_path, "{}").await
                .map_err(|e| format!("Context init: {e}"))?;
        }

        let db_path = root.session_db_path(session_id);
        SessionDb::new(&db_path).await
    }

    /// حفظ ملف — محلياً + HF bucket في الخلفية
    pub async fn save_file(&self, user_id: &str, session_id: &str, file_name: &str, content: &str) -> Result<(), String> {
        let root = self.user_root(user_id);
        let fpath = root.session_files_dir(session_id).join(file_name);
        let _ = ensure_safe_path(&fpath, root.root())
            .map_err(|e| format!("Path safety: {e}"))?;
        if let Some(parent) = fpath.parent() {
            fs::create_dir_all(parent).await.map_err(|e| format!("Dir: {e}"))?;
        }
        // 1. حفظ محلي (أولوية)
        fs::write(&fpath, content).await.map_err(|e| format!("Save file: {e}"))?;
        // 2. رفع إلى HF bucket (خلفية، لا يوقف العملية)
        let token = self.hf_token.clone();
        let uid = user_id.to_string();
        let sid = session_id.to_string();
        let fname = file_name.to_string();
        let data = content.to_string();
        tokio::spawn(async move {
            if let Err(e) = hf_upload_file(&token, &uid, &sid, &fname, &data).await {
                warn!("HF bucket upload failed (non-fatal): {e}");
            }
        });
        Ok(())
    }

    /// قراءة ملف — محلياً أولاً، ثم HF bucket
    pub async fn read_file(&self, user_id: &str, session_id: &str, file_name: &str) -> Result<String, String> {
        let root = self.user_root(user_id);
        let fpath = root.session_files_dir(session_id).join(file_name);

        // حاول القراءة المحلية أولاً
        if let Ok(safe) = ensure_safe_path(&fpath, root.root()) {
            if let Ok(content) = fs::read_to_string(&safe).await {
                return Ok(content);
            }
        }

        // الملف غير موجود محلياً — جرب HF bucket
        debug!("File not local, fetching from HF bucket: {user_id}/{session_id}/{file_name}");
        let content = hf_read_file(&self.hf_token, user_id, session_id, file_name).await
            .map_err(|e| format!("Read file (local+HF): {e}"))?;

        // خزّنه محلياً للمرة القادمة
        if let Ok(safe) = ensure_safe_path(&fpath, root.root()) {
            if let Some(parent) = safe.parent() {
                let _ = fs::create_dir_all(parent).await;
            }
            let _ = fs::write(&safe, &content).await;
        }
        Ok(content)
    }

    /// قائمة الملفات — دمج المحلية + HF bucket
    pub async fn list_files(&self, user_id: &str, session_id: &str) -> Result<Vec<String>, String> {
        let root = self.user_root(user_id);
        let fdir = root.session_files_dir(session_id);

        let mut files = std::collections::HashSet::new();

        // الملفات المحلية
        if let Ok(safe) = ensure_safe_path(&fdir, root.root()) {
            if let Ok(mut entries) = fs::read_dir(&safe).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if entry.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
                        if let Some(name) = entry.file_name().to_str() {
                            files.insert(name.to_string());
                        }
                    }
                }
            }
        }

        // الملفات في HF bucket
        if let Ok(hf_files) = hf_list_files(&self.hf_token, user_id, session_id).await {
            for f in hf_files { files.insert(f); }
        }

        let mut result: Vec<String> = files.into_iter().collect();
        result.sort();
        Ok(result)
    }

    pub async fn delete_file(&self, user_id: &str, session_id: &str, file_name: &str) -> Result<(), String> {
        let root = self.user_root(user_id);
        let fpath = root.session_files_dir(session_id).join(file_name);
        // حذف محلي
        if let Ok(safe) = ensure_safe_path(&fpath, root.root()) {
            let _ = fs::remove_file(&safe).await;
        }
        // حذف من HF bucket
        let token = self.hf_token.clone();
        let uid = user_id.to_string();
        let sid = session_id.to_string();
        let fname = file_name.to_string();
        tokio::spawn(async move {
            let _ = hf_delete_file(&token, &uid, &sid, &fname).await;
        });
        Ok(())
    }

    pub async fn save_session_context(&self, user_id: &str, session_id: &str, context: &str) -> Result<(), String> {
        let root = self.user_root(user_id);
        let ctx_path = root.sessions_dir().join(session_id).join("context.json");
        let safe = ensure_safe_path(&ctx_path, root.root())
            .map_err(|e| format!("Path safety: {e}"))?;
        fs::write(&safe, context).await.map_err(|e| format!("Save context: {e}"))
    }

    pub async fn load_session_context(&self, user_id: &str, session_id: &str) -> Result<String, String> {
        let root = self.user_root(user_id);
        let ctx_path = root.sessions_dir().join(session_id).join("context.json");
        let safe = ensure_safe_path(&ctx_path, root.root())
            .map_err(|e| format!("Path safety: {e}"))?;
        fs::read_to_string(&safe).await.map_err(|e| format!("Load context: {e}"))
    }

    pub async fn list_user_sessions(&self, user_id: &str) -> Result<Vec<String>, String> {
        let root = self.user_root(user_id);
        let sess_dir = root.sessions_dir();
        let safe = ensure_safe_path(&sess_dir, root.root())
            .map_err(|e| format!("Path safety: {e}"))?;
        let mut entries = fs::read_dir(&safe).await.map_err(|e| format!("List sessions: {e}"))?;
        let mut sessions = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.len() > 5 { sessions.push(name.to_string()); }
                }
            }
        }
        Ok(sessions)
    }

    pub async fn open_user_db(&self, user_id: &str) -> Result<Connection, String> {
        let root = self.user_root(user_id);
        let db_path = root.user_db_path();
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| format!("User DB dir: {e}"))?;
        }
        let path_str = db_path.to_string_lossy().to_string();
        let db = Builder::new_local(path_str)
            .build().await.map_err(|e| format!("User DB open: {e}"))?;
        let conn = db.connect().map_err(|e| format!("User DB connect: {e}"))?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY, name TEXT NOT NULL,
                key_hash TEXT NOT NULL, created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY, value TEXT NOT NULL
            );
        ").await.map_err(|e| format!("User DB schema: {e}"))?;
        Ok(conn)
    }

    pub async fn open_session_db(&self, user_id: &str, session_id: &str) -> Result<SessionDb, String> {
        let root = self.user_root(user_id);
        let db_path = root.session_db_path(session_id);
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| format!("Session DB dir: {e}"))?;
        }
        SessionDb::new(&db_path).await
    }

    pub async fn user_storage_usage(&self, user_id: &str) -> Result<u64, String> {
        let root = self.user_root(user_id);
        let mut total = 0u64;
        fn walk(dir: PathBuf, total: &mut u64) {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() { walk(path, total); }
                    else if path.is_file() { *total += std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0); }
                }
            }
        }
        walk(root.root().to_path_buf(), &mut total);
        Ok(total)
    }
}

// ─── Backward Compatibility ──────────────────────────────────────────────────

static ENGINE: OnceLock<StorageEngine> = OnceLock::new();

fn engine() -> &'static StorageEngine {
    ENGINE.get_or_init(|| StorageEngine::new(None))
}

pub fn init_engine() { ENGINE.get_or_init(|| StorageEngine::new(None)); }

pub async fn init_user_storage(user_id: &str) -> Result<(), String> {
    engine().init_user_storage(user_id).await?; Ok(())
}
pub async fn init_session_storage(user_id: &str, session_id: &str) -> Result<(), String> {
    engine().init_session_storage(user_id, session_id).await?; Ok(())
}
pub async fn save_session_context(user_id: &str, session_id: &str, context: &str) -> Result<(), String> {
    engine().save_session_context(user_id, session_id, context).await
}
pub async fn load_session_context(user_id: &str, session_id: &str) -> Result<String, String> {
    engine().load_session_context(user_id, session_id).await
}
pub async fn list_user_sessions(user_id: &str) -> Result<Vec<String>, String> {
    engine().list_user_sessions(user_id).await
}
pub async fn save_file(user_id: &str, session_id: &str, file_name: &str, content: &str) -> Result<(), String> {
    engine().save_file(user_id, session_id, file_name, content).await
}
pub async fn read_file(user_id: &str, session_id: &str, file_name: &str) -> Result<String, String> {
    engine().read_file(user_id, session_id, file_name).await
}
pub async fn list_files(user_id: &str, session_id: &str) -> Result<Vec<String>, String> {
    engine().list_files(user_id, session_id).await
}
pub async fn delete_file(user_id: &str, session_id: &str, file_name: &str) -> Result<(), String> {
    engine().delete_file(user_id, session_id, file_name).await
}
pub async fn user_storage_usage(user_id: &str) -> Result<u64, String> {
    engine().user_storage_usage(user_id).await
}
