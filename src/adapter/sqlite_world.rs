//! `SqliteWorldStore` — Phase 1 Vertical Slice (groups + FTS5 trigram).
//!
//! 스키마는 `docs/tasks/task-phase1-group-vertical-slice.md` §6.3을 그대로 따름.
//! 임베딩은 Phase 2+에서 도입 (vec0 미사용).

use std::sync::Mutex;

use rusqlite::{Connection, params};
use serde_json::{Map, Value};

use crate::domain::world::{Group, GroupFilter, GroupId, GroupStatus, WorldError};
use crate::worldbuilding::WorldRepository;

const SCHEMA_VERSION: i64 = 1;

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
        conn.execute(
            "INSERT OR REPLACE INTO world_schema_meta(version) VALUES (?)",
            [SCHEMA_VERSION],
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
        Ok(rows.filter_map(|r| r.ok()).collect())
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

        let escaped = q.replace('"', "\"\"");
        let phrase = format!("\"{}\"", escaped);
        let mut stmt = conn
            .prepare(
                "SELECT g.id, g.project_id, g.kind, g.name, g.aliases_json, g.parent_group,
                        g.allied_groups_json, g.rival_groups_json, g.headquarters, g.status, g.alignment,
                        g.summary, g.tags_json, g.extras_json, g.body_sections_json, g.temporal_json,
                        g.members_json, g.source_path
                 FROM groups_fts f
                 JOIN groups g ON g.id = f.id
                 WHERE groups_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![phrase, top_k as i64], row_to_group)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let hits: Vec<Group> = rows.filter_map(|r| r.ok()).collect();
        if !hits.is_empty() {
            return Ok(hits);
        }
        // FTS5가 결과 없으면 LIKE fallback (id-스타일·짧은 한자 등 trigram 외).
        self.search_like(&conn, q, top_k)
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
}

impl SqliteWorldStore {
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
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}

fn row_to_group(row: &rusqlite::Row) -> rusqlite::Result<Group> {
    let id: String = row.get(0)?;
    let _project_id: String = row.get(1)?;
    let kind: String = row.get(2)?;
    let name: String = row.get(3)?;
    let aliases_json: String = row.get(4)?;
    let parent_group: Option<String> = row.get(5)?;
    let allied_json: String = row.get(6)?;
    let rival_json: String = row.get(7)?;
    let headquarters: Option<String> = row.get(8)?;
    let status: String = row.get(9)?;
    let _alignment: Option<String> = row.get(10)?;
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
    // status는 temporal 내부에 이미 있으므로 굳이 덮어쓰지 않는다 — temporal_json이 truth.
    let _ = (GroupStatus::from_str_loose(&status), &status);

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
}
