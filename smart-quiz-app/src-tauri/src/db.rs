// bank.db / user.db 打开与 schema 迁移（设计方案 V1.1 §3.1/§3.2）
use rusqlite::Connection;
use std::path::Path;

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn open_user(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate_user(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA_V1)?;
    // 未来版本按 user_version 递进迁移
    conn.pragma_update(None, "user_version", 1)?;
    Ok(())
}

fn migrate_user(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(USER_SCHEMA_V1)?;
    conn.pragma_update(None, "user_version", 1)?;
    Ok(())
}

const USER_SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
  session_id  INTEGER PRIMARY KEY AUTOINCREMENT,
  mode        TEXT NOT NULL CHECK(mode IN ('practice','random','recite','review','wrong','fav','exam')),
  title       TEXT,
  bank_id     TEXT,
  paper_id    INTEGER,
  multi_score_policy TEXT NOT NULL DEFAULT 'all_or_nothing',
  time_limit_sec INTEGER,
  total_qty   INTEGER NOT NULL,
  scored_qty  INTEGER DEFAULT 0,
  correct_qty INTEGER DEFAULT 0,
  score       REAL,
  duration_ms INTEGER,
  started_at  TEXT NOT NULL,
  finished_at TEXT,
  qid_list    TEXT NOT NULL,             -- [["bank","qid"],...] 有序
  bank_version INTEGER,
  draft       TEXT                       -- 断点续考草稿 JSON {picks:{}, marks:{}, remaining_sec}
);

CREATE TABLE IF NOT EXISTS answer_records (
  record_id   INTEGER PRIMARY KEY AUTOINCREMENT,
  bank_id     TEXT NOT NULL, qid TEXT NOT NULL, q_version INTEGER NOT NULL,
  session_id  INTEGER NOT NULL,
  picked      TEXT,
  is_correct  INTEGER,                   -- 0/1/null 不计分
  time_cost_ms INTEGER,
  answered_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_ar_question ON answer_records(bank_id, qid);
CREATE INDEX IF NOT EXISTS idx_ar_session ON answer_records(session_id);
CREATE INDEX IF NOT EXISTS idx_ar_time ON answer_records(answered_at);

CREATE TABLE IF NOT EXISTS wrong_book (
  bank_id TEXT, qid TEXT,
  wrong_count INTEGER DEFAULT 1, last_wrong_at TEXT,
  PRIMARY KEY (bank_id, qid)
);

CREATE TABLE IF NOT EXISTS review_queue (
  bank_id TEXT, qid TEXT,
  ease REAL DEFAULT 2.5, interval_days REAL DEFAULT 0,
  repetitions INTEGER DEFAULT 0, due_date TEXT,
  PRIMARY KEY (bank_id, qid)
);

CREATE TABLE IF NOT EXISTS favorites (
  bank_id TEXT, qid TEXT, created_at TEXT, PRIMARY KEY (bank_id, qid)
);

CREATE TABLE IF NOT EXISTS notes (
  bank_id TEXT, qid TEXT, content TEXT, updated_at TEXT,
  PRIMARY KEY (bank_id, qid)
);

CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT);
"#;

const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS banks (
  bank_id     TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  version     INTEGER NOT NULL,
  schema_ver  INTEGER NOT NULL,
  description TEXT,
  is_builtin  INTEGER DEFAULT 0,
  is_enabled  INTEGER DEFAULT 1,
  asset_root  TEXT,
  imported_at TEXT
);

CREATE TABLE IF NOT EXISTS topics (
  topic_id  INTEGER PRIMARY KEY AUTOINCREMENT,
  bank_id   TEXT NOT NULL,
  parent_id INTEGER,
  name      TEXT NOT NULL,
  sort_order INTEGER DEFAULT 0,
  FOREIGN KEY (bank_id) REFERENCES banks(bank_id)
);

CREATE TABLE IF NOT EXISTS questions (
  bank_id     TEXT NOT NULL,
  qid         TEXT NOT NULL,
  version     INTEGER NOT NULL DEFAULT 1,
  type        TEXT NOT NULL CHECK(type IN ('single','multi','judge','fill')),
  stem        TEXT NOT NULL,
  img_path    TEXT,
  options     TEXT NOT NULL,
  answer      TEXT NOT NULL,
  answer_conf TEXT DEFAULT 'high',
  explain     TEXT,
  source      TEXT,
  difficulty  INTEGER DEFAULT 3,
  status      TEXT DEFAULT 'active' CHECK(status IN ('active','retired','pending_review')),
  created_at  TEXT,
  updated_at  TEXT,
  PRIMARY KEY (bank_id, qid),
  FOREIGN KEY (bank_id) REFERENCES banks(bank_id)
);
CREATE INDEX IF NOT EXISTS idx_q_status ON questions(bank_id, status, type);

CREATE TABLE IF NOT EXISTS question_topics (
  bank_id TEXT NOT NULL, qid TEXT NOT NULL, topic_id INTEGER NOT NULL,
  PRIMARY KEY (bank_id, qid, topic_id),
  FOREIGN KEY (bank_id, qid) REFERENCES questions(bank_id, qid)
);
CREATE INDEX IF NOT EXISTS idx_qt_topic ON question_topics(topic_id);

CREATE TABLE IF NOT EXISTS papers (
  paper_id  INTEGER PRIMARY KEY AUTOINCREMENT,
  bank_id   TEXT NOT NULL,
  name      TEXT NOT NULL,
  source_url TEXT,
  description TEXT,
  is_builtin INTEGER DEFAULT 0,
  FOREIGN KEY (bank_id) REFERENCES banks(bank_id)
);
CREATE TABLE IF NOT EXISTS paper_questions (
  paper_id INTEGER NOT NULL, bank_id TEXT NOT NULL, qid TEXT NOT NULL,
  sort_no INTEGER, score REAL DEFAULT 1,
  PRIMARY KEY (paper_id, bank_id, qid),
  FOREIGN KEY (bank_id, qid) REFERENCES questions(bank_id, qid)
);

-- 题目映射（并题/迁移追溯，保持 answer_records 只追加——M1 起用）
CREATE TABLE IF NOT EXISTS qid_mapping (
  old_bank TEXT, old_qid TEXT, new_bank TEXT, new_qid TEXT,
  mapped_at TEXT, PRIMARY KEY (old_bank, old_qid)
);
"#;
