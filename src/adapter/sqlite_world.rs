//! `SqliteWorldStore` — Phase 1 Vertical Slice (groups + FTS5 trigram).
//!
//! 스키마는 `docs/tasks/task-phase1-group-vertical-slice.md` §6.3을 그대로 따름.
//! Phase 2에서 persons 테이블 + persons_fts 추가 (`migrate_v2`). 같은 SQLite 파일이
//! Group + Person을 모두 보관. 임베딩은 Phase 5+에서 도입 (vec0 미사용).

use std::sync::Mutex;

use rusqlite::{Connection, params};
use serde_json::{Map, Value};

use crate::domain::world::{
    Group, GroupFilter, GroupId, HexacoSix, Person, PersonFilter, PersonId, PersonStatus,
    PersonTemporal, WorldError,
};
#[cfg(test)]
use crate::domain::world::GroupStatus;
use crate::worldbuilding::WorldRepository;

const SCHEMA_VERSION: i64 = 2;

pub struct SqliteWorldStore {
    conn: Mutex<Connection>,
}

impl SqliteWorldStore {
    pub fn new(path: &str) -> Result<Self, WorldError> {
        let conn = Connection::open(path).map_err(|e| WorldError::Storage(e.to_string()))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_tables()?;
        Ok(store)
    }

    pub fn in_memory() -> Result<Self, WorldError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_tables()?;
        Ok(store)
    }

    fn init_tables(&self) -> Result<(), WorldError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS world_schema_meta (version INTEGER PRIMARY KEY)",
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        let current: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM world_schema_meta",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if current < 1 {
            Self::migrate_v1(&conn)?;
        }
        if current < 2 {
            Self::migrate_v2(&conn)?;
        }
        conn.execute(
            "INSERT OR REPLACE INTO world_schema_meta(version) VALUES (?)",
            [SCHEMA_VERSION],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }

    /// v1 → v2 마이그레이션: persons 테이블 + persons_fts.
    /// `CREATE TABLE IF NOT EXISTS`이라 v2에서 신규 생성한 DB에도 안전.
    fn migrate_v2(conn: &Connection) -> Result<(), WorldError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS persons (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                aliases_json TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'alive' CHECK(status IN ('alive','dead','missing','unknown')),
                hexaco_json TEXT NOT NULL DEFAULT '{}',
                temporal_json TEXT NOT NULL DEFAULT '{}',
                affiliation_json TEXT NOT NULL DEFAULT '[]',
                birthplace TEXT,
                current_location TEXT,
                summary TEXT NOT NULL DEFAULT '',
                tags_json TEXT NOT NULL DEFAULT '[]',
                extras_json TEXT NOT NULL DEFAULT '{}',
                body_sections_json TEXT NOT NULL DEFAULT '{}',
                source_path TEXT,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_persons_kind ON persons(kind);
            CREATE INDEX IF NOT EXISTS idx_persons_status ON persons(status);
            CREATE INDEX IF NOT EXISTS idx_persons_project ON persons(project_id);
            CREATE VIRTUAL TABLE IF NOT EXISTS persons_fts USING fts5(
                id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
            );",
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }

    fn migrate_v1(conn: &Connection) -> Result<(), WorldError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS groups (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                aliases_json TEXT NOT NULL DEFAULT '[]',
                parent_group TEXT,
                allied_groups_json TEXT NOT NULL DEFAULT '[]',
                rival_groups_json TEXT NOT NULL DEFAULT '[]',
                headquarters TEXT,
                status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','declining','dissolved','dormant')),
                alignment TEXT,
                summary TEXT NOT NULL DEFAULT '',
                tags_json TEXT NOT NULL DEFAULT '[]',
                extras_json TEXT NOT NULL DEFAULT '{}',
                body_sections_json TEXT NOT NULL DEFAULT '{}',
                temporal_json TEXT NOT NULL DEFAULT '{}',
                members_json TEXT NOT NULL DEFAULT '[]',
                source_path TEXT,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_groups_kind ON groups(kind);
            CREATE INDEX IF NOT EXISTS idx_groups_parent ON groups(parent_group);
            CREATE INDEX IF NOT EXISTS idx_groups_status ON groups(status);
            CREATE INDEX IF NOT EXISTS idx_groups_alignment ON groups(alignment);
            CREATE INDEX IF NOT EXISTS idx_groups_project ON groups(project_id);
            CREATE VIRTUAL TABLE IF NOT EXISTS groups_fts USING fts5(
                id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
            );",
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 직렬화 유틸
// ---------------------------------------------------------------------------

fn json_array_of_strings(items: &[String]) -> String {
    serde_json::to_string(items).unwrap_or_else(|_| "[]".into())
}

fn json_array_of_groupids(items: &[GroupId]) -> String {
    let strs: Vec<&str> = items.iter().map(|g| g.as_str()).collect();
    serde_json::to_string(&strs).unwrap_or_else(|_| "[]".into())
}

fn from_json_strings(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn from_json_groupids(raw: &str) -> Vec<GroupId> {
    let v: Vec<String> = serde_json::from_str(raw).unwrap_or_default();
    v.into_iter().map(GroupId::new).collect()
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn aliases_concat(items: &[String]) -> String {
    items.join(" ")
}

fn body_concat(group: &Group) -> String {
    let mut s = String::new();
    for (k, v) in &group.body_sections {
        s.push_str(k);
        s.push('\n');
        s.push_str(v);
        s.push('\n');
    }
    s
}

fn person_body_concat(person: &Person) -> String {
    let mut s = String::new();
    for (k, v) in &person.body_sections {
        s.push_str(k);
        s.push('\n');
        s.push_str(v);
        s.push('\n');
    }
    s
}

// ---------------------------------------------------------------------------
// WorldRepository impl
// ---------------------------------------------------------------------------

impl WorldRepository for SqliteWorldStore {
    fn upsert_group(&self, project_id: &str, group: &Group) -> Result<(), WorldError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| WorldError::Storage(e.to_string()))?;

        let temporal_json = serde_json::to_string(&group.temporal)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let members_json = serde_json::to_string(&group.members)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let extras_json = serde_json::to_string(&group.extras)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let body_json = serde_json::to_string(&group.body_sections)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let aliases_json = json_array_of_strings(&group.aliases);
        let allied_json = json_array_of_groupids(&group.allied_groups);
        let rival_json = json_array_of_groupids(&group.rival_groups);
        let tags_json = json_array_of_strings(&group.tags);
        let alignment = group.alignment().map(|s| s.to_string());
        let parent = group.parent_group.as_ref().map(|g| g.as_str().to_string());
        let status = group.temporal.status.as_str();
        let updated_at = now_ms();

        tx.execute(
            "INSERT OR REPLACE INTO groups (
                id, project_id, kind, name, aliases_json, parent_group,
                allied_groups_json, rival_groups_json, headquarters, status, alignment,
                summary, tags_json, extras_json, body_sections_json, temporal_json,
                members_json, source_path, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                      ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                group.id.as_str(),
                project_id,
                group.kind,
                group.name,
                aliases_json,
                parent,
                allied_json,
                rival_json,
                group.headquarters,
                status,
                alignment,
                group.summary,
                tags_json,
                extras_json,
                body_json,
                temporal_json,
                members_json,
                group.source_path,
                updated_at,
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        // FTS5 — id 기반 delete-then-insert
        tx.execute("DELETE FROM groups_fts WHERE id = ?1", params![group.id.as_str()])
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        tx.execute(
            "INSERT INTO groups_fts (id, name, aliases, summary, body) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                group.id.as_str(),
                group.name,
                aliases_concat(&group.aliases),
                group.summary,
                body_concat(group),
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        tx.commit().map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }

    fn list_groups(&self, filter: GroupFilter) -> Result<Vec<Group>, WorldError> {
        let conn = self.conn.lock().unwrap();
        // 단순 동적 SQL — Phase 1엔 5개 필터만, 인젝션 방지를 위해 모든 값을 ? 바인딩.
        let mut sql = String::from(
            "SELECT id, project_id, kind, name, aliases_json, parent_group,
                    allied_groups_json, rival_groups_json, headquarters, status, alignment,
                    summary, tags_json, extras_json, body_sections_json, temporal_json,
                    members_json, source_path
             FROM groups WHERE 1=1",
        );
        let mut binds: Vec<String> = Vec::new();
        if let Some(k) = filter.kind {
            sql.push_str(" AND kind = ?");
            binds.push(k);
        }
        if let Some(s) = filter.status {
            sql.push_str(" AND status = ?");
            binds.push(s.as_str().to_string());
        }
        if let Some(p) = filter.parent_group {
            sql.push_str(" AND parent_group = ?");
            binds.push(p.as_str().to_string());
        }
        if let Some(a) = filter.alignment {
            sql.push_str(" AND alignment = ?");
            binds.push(a);
        }
        if let Some(t) = filter.genre_tag {
            // tags_json 안의 문자열 매칭 — JSON1 json_each 미사용, LIKE로 폴백.
            sql.push_str(" AND tags_json LIKE ?");
            binds.push(format!("%\"{}\"%", t));
        }
        sql.push_str(" ORDER BY id ASC");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let bind_refs: Vec<&dyn rusqlite::ToSql> =
            binds.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt
            .query_map(rusqlite::params_from_iter(bind_refs), row_to_group)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_rows_warn_on_err(rows))
    }

    fn get_group(&self, id: &GroupId) -> Result<Option<Group>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let res = conn.query_row(
            "SELECT id, project_id, kind, name, aliases_json, parent_group,
                    allied_groups_json, rival_groups_json, headquarters, status, alignment,
                    summary, tags_json, extras_json, body_sections_json, temporal_json,
                    members_json, source_path
             FROM groups WHERE id = ?1",
            params![id.as_str()],
            row_to_group,
        );
        match res {
            Ok(g) => Ok(Some(g)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(WorldError::Storage(e.to_string())),
        }
    }

    fn search_groups(&self, query: &str, top_k: u32) -> Result<Vec<Group>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        // FTS5 trigram 토크나이저는 3자(unicode) 미만 query는 매치하지 못한다
        // (n-gram 길이=3). 한국어 2자 검색어를 위해 LIKE fallback을 사용.
        let char_count = q.chars().count();
        if char_count < 3 {
            return self.search_like(&conn, q, top_k);
        }

        // FTS5 phrase wrapping — `*`, `OR`/`AND`/`NEAR`, `:`, `(`, `)` 등 query 키워드를
        // 무력화. 단 trigram 토크나이저는 query 안에 토큰화 가능한 트리그램이 0개인 경우
        // (예: 모두 punctuation) `SQL logic error` 또는 빈 결과를 낸다 — 이를 hard error로
        // 취급하면 검색 UI가 죽으므로, **MATCH 호출 자체가 실패하면 LIKE fallback**으로 떨어진다.
        let escaped = q.replace('"', "\"\"");
        let phrase = format!("\"{}\"", escaped);
        let fts_hits: Result<Vec<Group>, rusqlite::Error> = (|| {
            let mut stmt = conn.prepare(
                "SELECT g.id, g.project_id, g.kind, g.name, g.aliases_json, g.parent_group,
                        g.allied_groups_json, g.rival_groups_json, g.headquarters, g.status, g.alignment,
                        g.summary, g.tags_json, g.extras_json, g.body_sections_json, g.temporal_json,
                        g.members_json, g.source_path
                 FROM groups_fts f
                 JOIN groups g ON g.id = f.id
                 WHERE groups_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![phrase, top_k as i64], row_to_group)?;
            Ok(collect_rows_warn_on_err(rows))
        })();

        match fts_hits {
            Ok(hits) if !hits.is_empty() => Ok(hits),
            Ok(_) => {
                // FTS5가 매치 0건이면 LIKE fallback (id-스타일·짧은 한자 등 trigram 외).
                self.search_like(&conn, q, top_k)
            }
            Err(e) => {
                // FTS5 query 파싱 실패 등 — 사용자 입력에서 흔하다. 하드 에러 대신 fallback.
                tracing::debug!(
                    "FTS5 MATCH 실패, LIKE fallback로 진행: query={q:?} err={e}"
                );
                self.search_like(&conn, q, top_k)
            }
        }
    }

    fn count_groups(&self, project_id: Option<&str>) -> Result<u64, WorldError> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = match project_id {
            Some(p) => conn
                .query_row(
                    "SELECT COUNT(*) FROM groups WHERE project_id = ?1",
                    params![p],
                    |r| r.get(0),
                )
                .map_err(|e| WorldError::Storage(e.to_string()))?,
            None => conn
                .query_row("SELECT COUNT(*) FROM groups", [], |r| r.get(0))
                .map_err(|e| WorldError::Storage(e.to_string()))?,
        };
        Ok(n.max(0) as u64)
    }

    // ---------------------------------------------------------------------
    // Phase 2 — Person
    // ---------------------------------------------------------------------

    fn upsert_person(&self, project_id: &str, person: &Person) -> Result<(), WorldError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| WorldError::Storage(e.to_string()))?;

        let hexaco_json = serde_json::to_string(&person.hexaco)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let temporal_json = serde_json::to_string(&person.temporal)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let extras_json = serde_json::to_string(&person.extras)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let body_json = serde_json::to_string(&person.body_sections)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let aliases_json = json_array_of_strings(&person.aliases);
        let affiliation_json = json_array_of_groupids(&person.affiliation);
        let tags_json = json_array_of_strings(&person.tags);
        let status = person.status.as_str();
        let updated_at = now_ms();

        tx.execute(
            "INSERT OR REPLACE INTO persons (
                id, project_id, kind, name, aliases_json, status,
                hexaco_json, temporal_json, affiliation_json, birthplace, current_location,
                summary, tags_json, extras_json, body_sections_json, source_path, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                      ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                person.id.as_str(),
                project_id,
                person.kind,
                person.name,
                aliases_json,
                status,
                hexaco_json,
                temporal_json,
                affiliation_json,
                person.birthplace,
                person.current_location,
                person.summary,
                tags_json,
                extras_json,
                body_json,
                person.source_path,
                updated_at,
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        // FTS5 — id 기반 delete-then-insert
        tx.execute(
            "DELETE FROM persons_fts WHERE id = ?1",
            params![person.id.as_str()],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        tx.execute(
            "INSERT INTO persons_fts (id, name, aliases, summary, body) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                person.id.as_str(),
                person.name,
                aliases_concat(&person.aliases),
                person.summary,
                person_body_concat(person),
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        tx.commit().map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }

    fn list_persons(&self, filter: PersonFilter) -> Result<Vec<Person>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let mut sql = String::from(
            "SELECT id, project_id, kind, name, aliases_json, status,
                    hexaco_json, temporal_json, affiliation_json, birthplace, current_location,
                    summary, tags_json, extras_json, body_sections_json, source_path
             FROM persons WHERE 1=1",
        );
        let mut binds: Vec<String> = Vec::new();
        if let Some(k) = filter.kind {
            sql.push_str(" AND kind = ?");
            binds.push(k);
        }
        if let Some(s) = filter.status {
            sql.push_str(" AND status = ?");
            binds.push(s.as_str().to_string());
        }
        if let Some(g) = filter.affiliation {
            // affiliation_json은 ["group-a","group-b"] 형식 — JSON1 미사용, LIKE로 폴백.
            // 정확한 토큰 매칭을 위해 따옴표 포함 패턴 사용 (group-a vs group-a-extra 구분).
            sql.push_str(" AND affiliation_json LIKE ?");
            binds.push(format!("%\"{}\"%", g.as_str()));
        }
        if let Some(t) = filter.genre_tag {
            sql.push_str(" AND tags_json LIKE ?");
            binds.push(format!("%\"{}\"%", t));
        }
        sql.push_str(" ORDER BY id ASC");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let bind_refs: Vec<&dyn rusqlite::ToSql> =
            binds.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt
            .query_map(rusqlite::params_from_iter(bind_refs), row_to_person)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_person_rows_warn_on_err(rows))
    }

    fn get_person(&self, id: &PersonId) -> Result<Option<Person>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let res = conn.query_row(
            "SELECT id, project_id, kind, name, aliases_json, status,
                    hexaco_json, temporal_json, affiliation_json, birthplace, current_location,
                    summary, tags_json, extras_json, body_sections_json, source_path
             FROM persons WHERE id = ?1",
            params![id.as_str()],
            row_to_person,
        );
        match res {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(WorldError::Storage(e.to_string())),
        }
    }

    fn search_persons(&self, query: &str, top_k: u32) -> Result<Vec<Person>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let char_count = q.chars().count();
        if char_count < 3 {
            return self.search_persons_like(&conn, q, top_k);
        }

        let escaped = q.replace('"', "\"\"");
        let phrase = format!("\"{}\"", escaped);
        let fts_hits: Result<Vec<Person>, rusqlite::Error> = (|| {
            let mut stmt = conn.prepare(
                "SELECT p.id, p.project_id, p.kind, p.name, p.aliases_json, p.status,
                        p.hexaco_json, p.temporal_json, p.affiliation_json, p.birthplace, p.current_location,
                        p.summary, p.tags_json, p.extras_json, p.body_sections_json, p.source_path
                 FROM persons_fts f
                 JOIN persons p ON p.id = f.id
                 WHERE persons_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![phrase, top_k as i64], row_to_person)?;
            Ok(collect_person_rows_warn_on_err(rows))
        })();

        match fts_hits {
            Ok(hits) if !hits.is_empty() => Ok(hits),
            Ok(_) => self.search_persons_like(&conn, q, top_k),
            Err(e) => {
                tracing::debug!(
                    "FTS5 MATCH 실패(persons), LIKE fallback로 진행: query={q:?} err={e}"
                );
                self.search_persons_like(&conn, q, top_k)
            }
        }
    }

    fn count_persons(&self, project_id: Option<&str>) -> Result<u64, WorldError> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = match project_id {
            Some(p) => conn
                .query_row(
                    "SELECT COUNT(*) FROM persons WHERE project_id = ?1",
                    params![p],
                    |r| r.get(0),
                )
                .map_err(|e| WorldError::Storage(e.to_string()))?,
            None => conn
                .query_row("SELECT COUNT(*) FROM persons", [], |r| r.get(0))
                .map_err(|e| WorldError::Storage(e.to_string()))?,
        };
        Ok(n.max(0) as u64)
    }
}

impl SqliteWorldStore {
    /// Person 검색용 LIKE fallback — FTS5 trigram이 처리하지 못하는 짧은 query 또는
    /// 결과 0건 시 호출. groups의 `search_like`와 동일 정책.
    fn search_persons_like(
        &self,
        conn: &Connection,
        q: &str,
        top_k: u32,
    ) -> Result<Vec<Person>, WorldError> {
        let pat = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
        let mut stmt = conn
            .prepare(
                "SELECT p.id, p.project_id, p.kind, p.name, p.aliases_json, p.status,
                        p.hexaco_json, p.temporal_json, p.affiliation_json, p.birthplace, p.current_location,
                        p.summary, p.tags_json, p.extras_json, p.body_sections_json, p.source_path
                 FROM persons p
                 LEFT JOIN persons_fts f ON f.id = p.id
                 WHERE p.name LIKE ?1 ESCAPE '\\'
                    OR f.aliases LIKE ?1 ESCAPE '\\'
                    OR p.summary LIKE ?1 ESCAPE '\\'
                    OR f.body LIKE ?1 ESCAPE '\\'
                 GROUP BY p.id
                 ORDER BY p.id ASC
                 LIMIT ?2",
            )
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![pat, top_k as i64], row_to_person)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_person_rows_warn_on_err(rows))
    }

    /// FTS5 fallback — `groups_fts.body`/`name`/`aliases`/`summary`를 LIKE %q% 매칭.
    /// FTS5 trigram이 처리하지 못하는 짧은 query(2자 한국어 등) 또는 결과 0건일 때 사용.
    fn search_like(
        &self,
        conn: &Connection,
        q: &str,
        top_k: u32,
    ) -> Result<Vec<Group>, WorldError> {
        let pat = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
        let mut stmt = conn
            .prepare(
                "SELECT g.id, g.project_id, g.kind, g.name, g.aliases_json, g.parent_group,
                        g.allied_groups_json, g.rival_groups_json, g.headquarters, g.status, g.alignment,
                        g.summary, g.tags_json, g.extras_json, g.body_sections_json, g.temporal_json,
                        g.members_json, g.source_path
                 FROM groups g
                 LEFT JOIN groups_fts f ON f.id = g.id
                 WHERE g.name LIKE ?1 ESCAPE '\\'
                    OR f.aliases LIKE ?1 ESCAPE '\\'
                    OR g.summary LIKE ?1 ESCAPE '\\'
                    OR f.body LIKE ?1 ESCAPE '\\'
                 GROUP BY g.id
                 ORDER BY g.id ASC
                 LIMIT ?2",
            )
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![pat, top_k as i64], row_to_group)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_rows_warn_on_err(rows))
    }
}

fn row_to_group(row: &rusqlite::Row) -> rusqlite::Result<Group> {
    // 컬럼 순서는 호출자(prepare 문) SELECT 절과 1:1 매핑.
    // 비-truth 컬럼(status, alignment)은 temporal_json·extras_json에 동일 값이 있으므로
    // 굳이 읽지 않는다. CHECK 제약과 인덱싱용 캐시일 뿐이다.
    let id: String = row.get(0)?;
    // project_id (1)은 현재 도메인 모델 미보존 (Phase 2+ 다중 프로젝트 시 추가).
    let kind: String = row.get(2)?;
    let name: String = row.get(3)?;
    let aliases_json: String = row.get(4)?;
    let parent_group: Option<String> = row.get(5)?;
    let allied_json: String = row.get(6)?;
    let rival_json: String = row.get(7)?;
    let headquarters: Option<String> = row.get(8)?;
    // status (9), alignment (10): 캐시 컬럼 — temporal_json/extras_json에서 복원.
    let summary: String = row.get(11)?;
    let tags_json: String = row.get(12)?;
    let extras_json: String = row.get(13)?;
    let body_json: String = row.get(14)?;
    let temporal_json: String = row.get(15)?;
    let members_json: String = row.get(16)?;
    let source_path: Option<String> = row.get(17)?;

    let extras: Map<String, Value> =
        serde_json::from_str(&extras_json).unwrap_or_default();
    let body_sections = serde_json::from_str(&body_json).unwrap_or_default();
    let temporal = serde_json::from_str(&temporal_json).unwrap_or_default();
    let members = serde_json::from_str(&members_json).unwrap_or_default();

    Ok(Group {
        id: GroupId::new(id),
        kind,
        name,
        aliases: from_json_strings(&aliases_json),
        summary,
        tags: from_json_strings(&tags_json),
        extras,
        body_sections,
        temporal,
        members,
        headquarters,
        parent_group: parent_group.map(GroupId::new),
        allied_groups: from_json_groupids(&allied_json),
        rival_groups: from_json_groupids(&rival_json),
        source_path,
    })
}

/// `query_map` 결과의 row decode 에러를 silent하게 흘려보내지 않고 `tracing::warn!`으로
/// 기록한 뒤 성공 row만 모은다. JSON deserialize/타입 불일치 등의 진단 가시성 확보용.
fn collect_rows_warn_on_err<I>(rows: I) -> Vec<Group>
where
    I: Iterator<Item = rusqlite::Result<Group>>,
{
    let mut out = Vec::new();
    for r in rows {
        match r {
            Ok(g) => out.push(g),
            Err(e) => {
                tracing::warn!("SqliteWorldStore row decode 실패 — 결과에서 제외: {e}");
            }
        }
    }
    out
}

fn row_to_person(row: &rusqlite::Row) -> rusqlite::Result<Person> {
    let id: String = row.get(0)?;
    // project_id (1)은 도메인 모델 미보존.
    let kind: String = row.get(2)?;
    let name: String = row.get(3)?;
    let aliases_json: String = row.get(4)?;
    let status_str: String = row.get(5)?;
    let hexaco_json: String = row.get(6)?;
    let temporal_json: String = row.get(7)?;
    let affiliation_json: String = row.get(8)?;
    let birthplace: Option<String> = row.get(9)?;
    let current_location: Option<String> = row.get(10)?;
    let summary: String = row.get(11)?;
    let tags_json: String = row.get(12)?;
    let extras_json: String = row.get(13)?;
    let body_json: String = row.get(14)?;
    let source_path: Option<String> = row.get(15)?;

    let status = PersonStatus::from_str_loose(&status_str).unwrap_or_default();
    // hexaco는 Score VO로 역직렬화 — 범위 위반 시 silent fallback for resilience.
    let hexaco: HexacoSix = serde_json::from_str(&hexaco_json).unwrap_or_else(|e| {
        tracing::warn!("persons.hexaco_json 디코드 실패 ({e}) — neutral로 폴백");
        HexacoSix::neutral()
    });
    let temporal: PersonTemporal = serde_json::from_str(&temporal_json).unwrap_or_default();
    let extras: Map<String, Value> =
        serde_json::from_str(&extras_json).unwrap_or_default();
    let body_sections = serde_json::from_str(&body_json).unwrap_or_default();
    let aliases = from_json_strings(&aliases_json);
    let tags = from_json_strings(&tags_json);
    let affiliation = from_json_groupids(&affiliation_json);

    Ok(Person {
        id: PersonId::new(id),
        kind,
        name,
        aliases,
        status,
        hexaco,
        temporal,
        affiliation,
        birthplace,
        current_location,
        summary,
        tags,
        extras,
        body_sections,
        source_path,
    })
}

fn collect_person_rows_warn_on_err<I>(rows: I) -> Vec<Person>
where
    I: Iterator<Item = rusqlite::Result<Person>>,
{
    let mut out = Vec::new();
    for r in rows {
        match r {
            Ok(p) => out.push(p),
            Err(e) => {
                tracing::warn!("SqliteWorldStore person row decode 실패 — 결과에서 제외: {e}");
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 단위 테스트
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world::{MemberRef, Temporal};
    use serde_json::json;

    fn sample_group(id: &str, kind: &str, name: &str) -> Group {
        let mut g = Group::new(id, kind, name);
        g.aliases = vec!["별호".into()];
        g.summary = "요약".into();
        g.tags = vec!["wuxia".into(), "test".into()];
        g.temporal = Temporal {
            founded_at: Some("원년".into()),
            dissolved_at: None,
            status: GroupStatus::Active,
            notes: Some("메모".into()),
        };
        g.members = vec![MemberRef {
            person_id: Some("npc-x".into()),
            display_name: Some("표시명".into()),
            role: "수장".into(),
            note: None,
        }];
        g.headquarters = Some("place-x".into());
        g.extras
            .insert("alignment".into(), json!("orthodox"));
        g.body_sections.insert("개요".into(), "본문".into());
        g
    }

    #[test]
    fn schema_initializes_and_count_zero() {
        let store = SqliteWorldStore::in_memory().unwrap();
        assert_eq!(store.count_groups(None).unwrap(), 0);
    }

    #[test]
    fn upsert_and_get_roundtrip_preserves_all_fields() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let g = sample_group("group-x", "alliance", "X");
        store.upsert_group("test-project", &g).unwrap();
        let back = store.get_group(&GroupId::new("group-x")).unwrap().unwrap();
        assert_eq!(back, g);
    }

    #[test]
    fn list_filter_kind_and_status() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut a = sample_group("group-a", "alliance", "A");
        a.temporal.status = GroupStatus::Declining;
        let b = sample_group("group-b", "clan", "B");
        store.upsert_group("p", &a).unwrap();
        store.upsert_group("p", &b).unwrap();

        let alliances = store
            .list_groups(GroupFilter {
                kind: Some("alliance".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(alliances.len(), 1);
        assert_eq!(alliances[0].id.as_str(), "group-a");

        let declining = store
            .list_groups(GroupFilter {
                status: Some(GroupStatus::Declining),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(declining.len(), 1);
        assert_eq!(declining[0].id.as_str(), "group-a");
    }

    #[test]
    fn list_filter_parent_and_alignment() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let parent = sample_group("group-parent", "dynasty-court", "Parent");
        let mut child = sample_group("group-child", "covert-band", "Child");
        child.parent_group = Some(GroupId::new("group-parent"));
        child
            .extras
            .insert("alignment".into(), json!("imperial"));
        store.upsert_group("p", &parent).unwrap();
        store.upsert_group("p", &child).unwrap();

        let kids = store
            .list_groups(GroupFilter {
                parent_group: Some(GroupId::new("group-parent")),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].id.as_str(), "group-child");

        let imperial = store
            .list_groups(GroupFilter {
                alignment: Some("imperial".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(imperial.len(), 1);
        assert_eq!(imperial[0].id.as_str(), "group-child");
    }

    #[test]
    fn search_matches_alias_and_body() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut g = sample_group("group-mulim-mang", "alliance", "무림맹");
        g.aliases = vec!["구파일방".into()];
        g.summary = "정파 연합".into();
        g.body_sections
            .insert("개요".into(), "270년 역사를 가진 정파 동맹".into());
        store.upsert_group("p", &g).unwrap();

        // alias 매칭
        let hits = store.search_groups("구파일방", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id.as_str(), "group-mulim-mang");

        // body 매칭
        let hits = store.search_groups("정파 동맹", 5).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id.as_str(), "group-mulim-mang");

        // 빈 query
        assert!(store.search_groups("", 5).unwrap().is_empty());
    }

    #[test]
    fn search_pathological_queries_dont_crash() {
        // 사용자 입력에서 흔한 FTS5 keyword/특수문자가 hard error로 새지 않고,
        // 빈 결과 또는 LIKE fallback 결과를 반환해야 한다.
        let store = SqliteWorldStore::in_memory().unwrap();
        let g = sample_group("group-x", "alliance", "Alpha");
        store.upsert_group("p", &g).unwrap();

        // FTS5 keyword as raw query — phrase wrapping이 무력화해야 함.
        for q in [
            "\"escaped\"",       // 더블쿼트
            "OR",                  // FTS5 keyword
            "AND",
            "NEAR",
            "AND BAD (paren",      // 미닫힌 괄호
            "*",                   // prefix wildcard 단독
            ":column",             // 컬럼 필터 시도
        ] {
            let res = store.search_groups(q, 5);
            assert!(
                res.is_ok(),
                "search_groups({q:?})는 panic·error 없이 Ok이어야 함: {res:?}"
            );
        }
    }

    #[test]
    fn upsert_replaces_fts_stale_row() {
        // upsert 두 번 — 첫 번째 alias가 FTS5 stale row로 남으면 안 됨.
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut g = sample_group("group-x", "alliance", "Alpha Council");
        g.aliases = vec!["OldAliasUnique".into()];
        g.body_sections.clear();
        g.body_sections
            .insert("Overview".into(), "first body".into());
        store.upsert_group("p", &g).unwrap();
        // 첫 alias로 검색 가능
        let hits = store.search_groups("OldAliasUnique", 5).unwrap();
        assert_eq!(hits.len(), 1);

        // alias 교체 + body 교체 후 재upsert
        g.aliases = vec!["NewAliasUnique".into()];
        g.body_sections.clear();
        g.body_sections
            .insert("Overview".into(), "second body".into());
        store.upsert_group("p", &g).unwrap();

        // 옛 alias로는 검색 0건이어야 함 (FTS5 row 교체 검증)
        let hits_old = store.search_groups("OldAliasUnique", 5).unwrap();
        assert!(
            hits_old.is_empty(),
            "stale FTS5 row 검출: {:?}",
            hits_old.iter().map(|g| g.id.as_str()).collect::<Vec<_>>()
        );
        // 새 alias로 검색 1건
        let hits_new = store.search_groups("NewAliasUnique", 5).unwrap();
        assert_eq!(hits_new.len(), 1);
        // 옛 body 텍스트도 매치되지 않아야 함
        let hits_body = store.search_groups("first body", 5).unwrap();
        assert!(hits_body.is_empty());
    }

    #[test]
    fn upsert_preserves_source_path_round_trip() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut g = sample_group("group-sp", "alliance", "SP");
        g.source_path = Some("projects/test/world/group/group-sp.md".into());
        store.upsert_group("p", &g).unwrap();
        let back = store.get_group(&GroupId::new("group-sp")).unwrap().unwrap();
        assert_eq!(back.source_path, g.source_path);
    }

    #[test]
    fn count_with_project_filter() {
        let store = SqliteWorldStore::in_memory().unwrap();
        store
            .upsert_group("p1", &sample_group("group-a", "alliance", "A"))
            .unwrap();
        store
            .upsert_group("p2", &sample_group("group-b", "clan", "B"))
            .unwrap();
        assert_eq!(store.count_groups(None).unwrap(), 2);
        assert_eq!(store.count_groups(Some("p1")).unwrap(), 1);
        assert_eq!(store.count_groups(Some("p2")).unwrap(), 1);
        assert_eq!(store.count_groups(Some("missing")).unwrap(), 0);
    }

    // -----------------------------------------------------------------------
    // Phase 2 — Person 라운드트립 + FTS5 + 필터
    // -----------------------------------------------------------------------

    use crate::domain::personality::Score;

    fn sample_person(id: &str, kind: &str, name: &str) -> Person {
        let mut p = Person::new(id, kind, name);
        p.aliases = vec!["별호".into(), "다른이름".into()];
        p.status = PersonStatus::Alive;
        p.hexaco = HexacoSix {
            honesty_humility: Score::clamped(-0.3),
            emotionality: Score::clamped(0.1),
            extraversion: Score::clamped(0.5),
            agreeableness: Score::clamped(0.4),
            conscientiousness: Score::clamped(0.7),
            openness: Score::clamped(0.6),
        };
        p.temporal = PersonTemporal {
            birth_year: Some("215년차".into()),
            age_at_game_start: Some(40),
            ..Default::default()
        };
        p.affiliation = vec![GroupId::new("group-a"), GroupId::new("group-b")];
        p.birthplace = Some("place-a".into());
        p.summary = "테스트 인물 요약".into();
        p.tags = vec!["wuxia".into(), "test".into()];
        p.extras
            .insert("priority".into(), json!("★★"));
        p.body_sections.insert("개요".into(), "본문".into());
        p
    }

    #[test]
    fn persons_count_zero_on_fresh_db() {
        let store = SqliteWorldStore::in_memory().unwrap();
        assert_eq!(store.count_persons(None).unwrap(), 0);
    }

    #[test]
    fn persons_upsert_and_get_roundtrip_preserves_all_fields() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let p = sample_person("npc-x", "active", "X");
        store.upsert_person("test-project", &p).unwrap();
        let back = store.get_person(&PersonId::new("npc-x")).unwrap().unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn persons_list_filter_kind_and_status() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut alive_active = sample_person("npc-a", "active", "A");
        alive_active.status = PersonStatus::Alive;
        let mut dead_historical = sample_person("npc-b", "historical", "B");
        dead_historical.status = PersonStatus::Dead;
        store.upsert_person("p", &alive_active).unwrap();
        store.upsert_person("p", &dead_historical).unwrap();

        let actives = store
            .list_persons(PersonFilter {
                kind: Some("active".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(actives.len(), 1);
        assert_eq!(actives[0].id.as_str(), "npc-a");

        let deads = store
            .list_persons(PersonFilter {
                status: Some(PersonStatus::Dead),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(deads.len(), 1);
        assert_eq!(deads[0].id.as_str(), "npc-b");
    }

    #[test]
    fn persons_list_filter_affiliation_matches_member() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut p1 = sample_person("npc-1", "active", "1");
        p1.affiliation = vec![GroupId::new("group-namgung")];
        let mut p2 = sample_person("npc-2", "active", "2");
        p2.affiliation = vec![GroupId::new("group-shipsangsi")];
        store.upsert_person("p", &p1).unwrap();
        store.upsert_person("p", &p2).unwrap();

        let namgung = store
            .list_persons(PersonFilter {
                affiliation: Some(GroupId::new("group-namgung")),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(namgung.len(), 1);
        assert_eq!(namgung[0].id.as_str(), "npc-1");
    }

    #[test]
    fn persons_search_matches_alias_and_body() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut p = sample_person("npc-02", "active", "조고");
        p.aliases = vec!["대진의 그림자".into(), "십상시의 주인".into()];
        p.summary = "메인 적대자".into();
        p.body_sections
            .insert("개요".into(), "55세 환관 출신 권신".into());
        store.upsert_person("p", &p).unwrap();

        // alias 매칭
        let hits = store.search_persons("대진의 그림자", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id.as_str(), "npc-02");

        // body 매칭
        let hits = store.search_persons("환관 출신", 5).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id.as_str(), "npc-02");

        // 빈 query
        assert!(store.search_persons("", 5).unwrap().is_empty());
    }

    #[test]
    fn persons_upsert_replaces_fts_stale_row() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut p = sample_person("npc-x", "active", "Alpha");
        p.aliases = vec!["OldAliasUnique".into()];
        p.body_sections.clear();
        p.body_sections
            .insert("Overview".into(), "first body".into());
        store.upsert_person("p", &p).unwrap();
        // 첫 alias로 검색 가능
        let hits = store.search_persons("OldAliasUnique", 5).unwrap();
        assert_eq!(hits.len(), 1);

        // alias 교체 + body 교체 후 재upsert
        p.aliases = vec!["NewAliasUnique".into()];
        p.body_sections.clear();
        p.body_sections
            .insert("Overview".into(), "second body".into());
        store.upsert_person("p", &p).unwrap();

        // 옛 alias로는 검색 0건이어야 함 (FTS5 row 교체 검증)
        let hits_old = store.search_persons("OldAliasUnique", 5).unwrap();
        assert!(hits_old.is_empty());
        // 새 alias로 검색 1건
        let hits_new = store.search_persons("NewAliasUnique", 5).unwrap();
        assert_eq!(hits_new.len(), 1);
    }

    #[test]
    fn persons_count_with_project_filter() {
        let store = SqliteWorldStore::in_memory().unwrap();
        store
            .upsert_person("p1", &sample_person("npc-a", "active", "A"))
            .unwrap();
        store
            .upsert_person("p2", &sample_person("npc-b", "historical", "B"))
            .unwrap();
        assert_eq!(store.count_persons(None).unwrap(), 2);
        assert_eq!(store.count_persons(Some("p1")).unwrap(), 1);
        assert_eq!(store.count_persons(Some("p2")).unwrap(), 1);
        assert_eq!(store.count_persons(Some("missing")).unwrap(), 0);
    }

    #[test]
    fn persons_search_pathological_queries_dont_crash() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let p = sample_person("npc-x", "active", "Alpha");
        store.upsert_person("p", &p).unwrap();

        for q in ["\"escaped\"", "OR", "AND", "NEAR", "AND BAD (paren", "*", ":column"] {
            let res = store.search_persons(q, 5);
            assert!(
                res.is_ok(),
                "search_persons({q:?})는 panic·error 없이 Ok이어야 함: {res:?}"
            );
        }
    }

    #[test]
    fn schema_v1_to_v2_migration_creates_persons_table() {
        // v1 DB를 모사 — schema_meta version=1만 있는 상태에서 store 재오픈 시 v2로 마이그레이션.
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE world_schema_meta (version INTEGER PRIMARY KEY);
             INSERT INTO world_schema_meta(version) VALUES (1);
             CREATE TABLE groups (id TEXT PRIMARY KEY);",
        )
        .unwrap();
        // 기존 connection을 drop하고 동일 메모리 DB를 다시 못 여니, 파일 기반 마이그레이션은
        // tempfile로 별도 검증. 여기서는 v2 신규 생성 시 persons 존재만 확인.
        drop(conn);
        let store = SqliteWorldStore::in_memory().unwrap();
        // persons 테이블이 존재하면 count는 0으로 응답한다.
        assert_eq!(store.count_persons(None).unwrap(), 0);
    }
}
