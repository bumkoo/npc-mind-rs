//! SqliteRumorStore — `Rumor` 애그리거트의 SQLite 구현 (Step C1 foundation)
//!
//! 사용 테이블(Step A `migrate_v2`에서 이미 생성됨):
//! - `rumors` : 애그리거트 루트 메타
//! - `rumor_hops` : hop 목록 (단조 증가 hop_index)
//! - `rumor_distortions` : 변형 DAG 노드
//!
//! 본 스토어는 `schema_meta` / `memories*` 테이블에는 일절 접근하지 않는다.
//! SqliteMemoryStore와 같은 DB 파일을 공유해도 무방하며, 다른 파일/메모리
//! 인스턴스도 지원한다 (테스트 분리 목적).

use crate::domain::rumor::{
    ReachPolicy, Rumor, RumorDistortion, RumorHop, RumorOrigin, RumorStatus,
};
use crate::ports::{MemoryError, RumorStore};
use rusqlite::{params, Connection};
use std::sync::Mutex;

/// SQLite 기반 소문 저장소.
pub struct SqliteRumorStore {
    conn: Mutex<Connection>,
}

impl SqliteRumorStore {
    /// 파일 기반 저장소 생성. (sqlite-vec 확장은 필요 없음)
    pub fn new(path: &str) -> Result<Self, MemoryError> {
        let conn =
            Connection::open(path).map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_tables()?;
        Ok(store)
    }

    /// 인메모리 저장소 (테스트용).
    pub fn in_memory() -> Result<Self, MemoryError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_tables()?;
        Ok(store)
    }

    /// `CREATE TABLE IF NOT EXISTS`로 rumor 3종 + 인덱스 보장.
    /// 스키마는 `SqliteMemoryStore::migrate_v2`와 동일하며 idempotent.
    fn init_tables(&self) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS rumors (
                id TEXT PRIMARY KEY,
                topic TEXT,
                seed_content TEXT,
                origin_kind TEXT NOT NULL,
                origin_ref TEXT,
                reach_regions TEXT,
                reach_factions TEXT,
                reach_npc_ids TEXT,
                reach_min_significance REAL,
                status TEXT NOT NULL DEFAULT 'active',
                created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS rumor_hops (
                rumor_id TEXT NOT NULL REFERENCES rumors(id),
                hop_index INTEGER NOT NULL,
                content_version TEXT,
                recipients TEXT NOT NULL,
                spread_at INTEGER NOT NULL,
                PRIMARY KEY (rumor_id, hop_index)
            );
            CREATE TABLE IF NOT EXISTS rumor_distortions (
                id TEXT PRIMARY KEY,
                rumor_id TEXT NOT NULL REFERENCES rumors(id),
                parent TEXT REFERENCES rumor_distortions(id),
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_rumors_topic ON rumors(topic) WHERE topic IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_rumors_status ON rumors(status);",
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Serialization helpers — RumorOrigin/Status/Reach ↔ SQL columns
// ---------------------------------------------------------------------------

fn origin_to_columns(origin: &RumorOrigin) -> (&'static str, Option<String>) {
    match origin {
        RumorOrigin::Seeded => ("seeded", None),
        RumorOrigin::FromWorldEvent { event_id } => ("from_world_event", Some(event_id.to_string())),
        RumorOrigin::Authored { by } => ("authored", by.clone()),
    }
}

fn origin_from_columns(kind: &str, origin_ref: Option<String>) -> Result<RumorOrigin, MemoryError> {
    match kind {
        "seeded" => Ok(RumorOrigin::Seeded),
        "from_world_event" => {
            let event_id: u64 = origin_ref
                .ok_or_else(|| MemoryError::StorageError("origin_ref missing".into()))?
                .parse()
                .map_err(|e: std::num::ParseIntError| MemoryError::StorageError(e.to_string()))?;
            Ok(RumorOrigin::FromWorldEvent { event_id })
        }
        "authored" => Ok(RumorOrigin::Authored { by: origin_ref }),
        other => Err(MemoryError::StorageError(format!(
            "unknown origin_kind '{other}'"
        ))),
    }
}

fn status_to_str(s: RumorStatus) -> &'static str {
    match s {
        RumorStatus::Active => "active",
        RumorStatus::Fading => "fading",
        RumorStatus::Faded => "faded",
    }
}

fn status_from_str(s: &str) -> Result<RumorStatus, MemoryError> {
    match s {
        "active" => Ok(RumorStatus::Active),
        "fading" => Ok(RumorStatus::Fading),
        "faded" => Ok(RumorStatus::Faded),
        other => Err(MemoryError::StorageError(format!(
            "unknown status '{other}'"
        ))),
    }
}

fn json_array(values: &[String]) -> String {
    serde_json::to_string(values).unwrap_or_else(|_| "[]".into())
}

fn json_array_parse(raw: Option<String>) -> Vec<String> {
    raw.and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// RumorStore impl
// ---------------------------------------------------------------------------

impl RumorStore for SqliteRumorStore {
    fn save(&self, rumor: &Rumor) -> Result<(), MemoryError> {
        // 저장 전에 불변식 재검증 — 호출자 버그로 깨진 상태가 들어오는 것을 방어.
        rumor
            .validate()
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        let (origin_kind, origin_ref) = origin_to_columns(&rumor.origin);
        let reach = &rumor.reach_policy;

        tx.execute(
            "INSERT OR REPLACE INTO rumors (
                id, topic, seed_content, origin_kind, origin_ref,
                reach_regions, reach_factions, reach_npc_ids, reach_min_significance,
                status, created_at
            ) VALUES (?,?,?,?,?,?,?,?,?,?,?)",
            params![
                rumor.id,
                rumor.topic,
                rumor.seed_content,
                origin_kind,
                origin_ref,
                json_array(&reach.regions),
                json_array(&reach.factions),
                json_array(&reach.npc_ids),
                reach.min_significance,
                status_to_str(rumor.status()),
                rumor.created_at as i64,
            ],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        // Hop·Distortion은 "있는 행 덮어쓰기" 시맨틱 — 기존 행을 전부 지우고 재삽입.
        // append-only 원칙은 애그리거트 레벨(RumorHop ::push via add_hop)에서 보장되므로
        // 저장 계층에서는 upsert로 안전.
        tx.execute("DELETE FROM rumor_hops WHERE rumor_id = ?", params![rumor.id])
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        tx.execute(
            "DELETE FROM rumor_distortions WHERE rumor_id = ?",
            params![rumor.id],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        for hop in rumor.hops() {
            tx.execute(
                "INSERT INTO rumor_hops (rumor_id, hop_index, content_version, recipients, spread_at)
                 VALUES (?,?,?,?,?)",
                params![
                    rumor.id,
                    hop.hop_index as i64,
                    hop.content_version,
                    json_array(&hop.recipients),
                    hop.spread_at as i64,
                ],
            )
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        }

        for dist in rumor.distortions() {
            tx.execute(
                "INSERT INTO rumor_distortions (id, rumor_id, parent, content, created_at)
                 VALUES (?,?,?,?,?)",
                params![
                    dist.id,
                    rumor.id,
                    dist.parent,
                    dist.content,
                    dist.created_at as i64,
                ],
            )
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        }

        tx.commit()
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(())
    }

    fn load(&self, id: &str) -> Result<Option<Rumor>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        load_internal(&conn, id)
    }

    fn find_by_topic(&self, topic: &str) -> Result<Vec<Rumor>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let ids: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT id FROM rumors WHERE topic = ?")
                .map_err(|e| MemoryError::StorageError(e.to_string()))?;
            let rows = stmt
                .query_map(params![topic], |r| r.get::<_, String>(0))
                .map_err(|e| MemoryError::StorageError(e.to_string()))?;
            rows.filter_map(|r| r.ok()).collect()
        };
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(r) = load_internal(&conn, &id)? {
                out.push(r);
            }
        }
        Ok(out)
    }

    fn find_active_in_reach(&self, reach: &ReachPolicy) -> Result<Vec<Rumor>, MemoryError> {
        // Step C1 단순 구현: 'active' status 전체를 불러와 ReachPolicy 필터를 메모리에서 적용.
        // 대규모 이전 단계에서는 SQL-level 조인이 바람직하지만 현재 rumor 개수 스케일에서는 충분.
        // Step C3에서 인덱스 추가 + 범위 쿼리화 계획.
        let conn = self.conn.lock().unwrap();
        let ids: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT id FROM rumors WHERE status = 'active'")
                .map_err(|e| MemoryError::StorageError(e.to_string()))?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| MemoryError::StorageError(e.to_string()))?;
            rows.filter_map(|r| r.ok()).collect()
        };
        let mut out = Vec::new();
        for id in ids {
            if let Some(r) = load_internal(&conn, &id)? {
                if reach_overlaps(reach, &r.reach_policy) {
                    out.push(r);
                }
            }
        }
        Ok(out)
    }
}

/// 주어진 두 reach가 "도달 중첩"을 가지는지 판정.
///
/// 한 축이라도 교집합이 있으면 overlap. 양쪽 모두 비어 있는 축은 "제한 없음"으로 취급.
/// min_significance는 문의 reach의 하한이 rumor의 min_significance 이하여야 통과.
fn reach_overlaps(query: &ReachPolicy, rumor: &ReachPolicy) -> bool {
    let region_ok = query.regions.is_empty()
        || rumor.regions.is_empty()
        || query.regions.iter().any(|r| rumor.regions.contains(r));
    let faction_ok = query.factions.is_empty()
        || rumor.factions.is_empty()
        || query.factions.iter().any(|f| rumor.factions.contains(f));
    let npc_ok = query.npc_ids.is_empty()
        || rumor.npc_ids.is_empty()
        || query.npc_ids.iter().any(|n| rumor.npc_ids.contains(n));
    let sig_ok = query.min_significance <= rumor.min_significance.max(f32::MIN_POSITIVE * 0.0)
        || query.min_significance <= 0.0;
    region_ok && faction_ok && npc_ok && sig_ok
}

fn load_internal(conn: &Connection, id: &str) -> Result<Option<Rumor>, MemoryError> {
    let row = conn
        .query_row(
            "SELECT id, topic, seed_content, origin_kind, origin_ref,
                    reach_regions, reach_factions, reach_npc_ids, reach_min_significance,
                    status, created_at
             FROM rumors WHERE id = ?",
            params![id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, Option<String>>(7)?,
                    r.get::<_, Option<f64>>(8)?,
                    r.get::<_, String>(9)?,
                    r.get::<_, i64>(10)?,
                ))
            },
        )
        .ok();

    let Some((
        id,
        topic,
        seed_content,
        origin_kind,
        origin_ref,
        reach_regions,
        reach_factions,
        reach_npc_ids,
        reach_min_significance,
        status,
        created_at,
    )) = row
    else {
        return Ok(None);
    };

    let origin = origin_from_columns(&origin_kind, origin_ref)?;
    let status = status_from_str(&status)?;
    let reach_policy = ReachPolicy {
        regions: json_array_parse(reach_regions),
        factions: json_array_parse(reach_factions),
        npc_ids: json_array_parse(reach_npc_ids),
        min_significance: reach_min_significance.unwrap_or(0.0) as f32,
    };

    // Hops — hop_index 오름차순
    let hops: Vec<RumorHop> = {
        let mut stmt = conn
            .prepare(
                "SELECT hop_index, content_version, recipients, spread_at
                 FROM rumor_hops WHERE rumor_id = ? ORDER BY hop_index ASC",
            )
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map(params![id], |r| {
                Ok(RumorHop {
                    hop_index: r.get::<_, i64>(0)? as u32,
                    content_version: r.get::<_, Option<String>>(1)?,
                    recipients: json_array_parse(r.get::<_, Option<String>>(2)?),
                    spread_at: r.get::<_, i64>(3)? as u64,
                })
            })
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // Distortions — created_at 오름차순 (DAG 순서 보장을 위해 부모가 먼저 저장되어야 함)
    let distortions: Vec<RumorDistortion> = {
        let mut stmt = conn
            .prepare(
                "SELECT id, parent, content, created_at
                 FROM rumor_distortions WHERE rumor_id = ? ORDER BY created_at ASC, id ASC",
            )
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map(params![id], |r| {
                Ok(RumorDistortion {
                    id: r.get::<_, String>(0)?,
                    parent: r.get::<_, Option<String>>(1)?,
                    content: r.get::<_, String>(2)?,
                    created_at: r.get::<_, i64>(3)? as u64,
                })
            })
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let rumor = Rumor::from_parts(
        id,
        topic,
        seed_content,
        origin,
        reach_policy,
        hops,
        distortions,
        created_at as u64,
        status,
    )
    .map_err(|e| MemoryError::StorageError(e.to_string()))?;

    Ok(Some(rumor))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 테스트용 샘플 rumor. distortion id는 rumor id로 prefix되어 전역 UNIQUE를 만족한다.
    fn sample_rumor(id: &str) -> Rumor {
        let d1 = format!("{id}:d1");
        let mut r = Rumor::with_forecast_content(
            id,
            "moorim-leader-change",
            "장문인이 바뀔 거라더라",
            RumorOrigin::Authored {
                by: Some("informant".into()),
            },
            ReachPolicy {
                regions: vec!["central".into()],
                factions: vec!["moorim".into()],
                npc_ids: vec![],
                min_significance: 0.3,
            },
            100,
        );
        r.add_distortion(RumorDistortion {
            id: d1.clone(),
            parent: None,
            content: "순한 버전".into(),
            created_at: 110,
        })
        .unwrap();
        r.add_hop(RumorHop {
            hop_index: 0,
            content_version: Some(d1),
            recipients: vec!["npc-a".into(), "npc-b".into()],
            spread_at: 120,
        })
        .unwrap();
        r
    }

    #[test]
    fn save_and_load_roundtrip_preserves_all_fields() {
        let store = SqliteRumorStore::in_memory().unwrap();
        let r = sample_rumor("r1");
        store.save(&r).unwrap();

        let back = store.load("r1").unwrap().expect("loaded");
        assert_eq!(back, r);
    }

    #[test]
    fn load_missing_returns_none() {
        let store = SqliteRumorStore::in_memory().unwrap();
        assert!(store.load("ghost").unwrap().is_none());
    }

    #[test]
    fn save_upsert_allows_adding_hops_and_distortions() {
        let store = SqliteRumorStore::in_memory().unwrap();
        let mut r = sample_rumor("r-upsert");
        store.save(&r).unwrap();

        let d1 = "r-upsert:d1".to_string();
        let d2 = "r-upsert:d2".to_string();
        // 새 홉 추가 후 재저장
        r.add_distortion(RumorDistortion {
            id: d2.clone(),
            parent: Some(d1),
            content: "더 과장된 버전".into(),
            created_at: 200,
        })
        .unwrap();
        r.add_hop(RumorHop {
            hop_index: 1,
            content_version: Some(d2),
            recipients: vec!["npc-c".into()],
            spread_at: 210,
        })
        .unwrap();
        store.save(&r).unwrap();

        let back = store.load("r-upsert").unwrap().expect("loaded");
        assert_eq!(back.hops().len(), 2);
        assert_eq!(back.distortions().len(), 2);
        assert_eq!(back, r);
    }

    #[test]
    fn find_by_topic_returns_all_matching_rumors() {
        let store = SqliteRumorStore::in_memory().unwrap();
        let r1 = sample_rumor("r1");
        let r2 = {
            let mut r = sample_rumor("r2");
            // 같은 topic이지만 다른 id
            r.reach_policy.regions = vec!["frontier".into()];
            r
        };
        let r3 = Rumor::new(
            "r3",
            "other-topic",
            RumorOrigin::Seeded,
            ReachPolicy::default(),
            100,
        );
        store.save(&r1).unwrap();
        store.save(&r2).unwrap();
        store.save(&r3).unwrap();

        let found = store.find_by_topic("moorim-leader-change").unwrap();
        assert_eq!(found.len(), 2);
        let ids: Vec<String> = found.iter().map(|r| r.id.clone()).collect();
        assert!(ids.contains(&"r1".to_string()));
        assert!(ids.contains(&"r2".to_string()));
    }

    #[test]
    fn find_active_in_reach_excludes_faded_and_applies_overlap() {
        let store = SqliteRumorStore::in_memory().unwrap();

        let mut active = sample_rumor("r-active");
        let mut faded = sample_rumor("r-faded");
        faded.transition_to(RumorStatus::Faded).unwrap();
        let mut elsewhere = sample_rumor("r-elsewhere");
        elsewhere.reach_policy.regions = vec!["frontier".into()];
        elsewhere.reach_policy.factions = vec!["sapa".into()];

        // active 소문 하나 더 — 지역은 중첩 없지만 faction은 중첩
        active.reach_policy.regions = vec!["central".into()];

        store.save(&active).unwrap();
        store.save(&faded).unwrap();
        store.save(&elsewhere).unwrap();

        let query = ReachPolicy {
            regions: vec!["central".into()],
            factions: vec![],
            npc_ids: vec![],
            min_significance: 0.0,
        };
        let found = store.find_active_in_reach(&query).unwrap();
        let ids: Vec<String> = found.iter().map(|r| r.id.clone()).collect();
        assert!(ids.contains(&"r-active".to_string()));
        assert!(
            !ids.contains(&"r-faded".to_string()),
            "faded rumor must be excluded"
        );
        assert!(
            !ids.contains(&"r-elsewhere".to_string()),
            "no region overlap → excluded"
        );
    }

    #[test]
    fn origin_roundtrip_all_kinds() {
        let store = SqliteRumorStore::in_memory().unwrap();
        let variants = vec![
            (
                "r-seeded",
                RumorOrigin::Seeded,
            ),
            (
                "r-world",
                RumorOrigin::FromWorldEvent { event_id: 42 },
            ),
            (
                "r-authored-some",
                RumorOrigin::Authored {
                    by: Some("informant".into()),
                },
            ),
            (
                "r-authored-none",
                RumorOrigin::Authored { by: None },
            ),
        ];
        for (id, origin) in variants {
            let r = Rumor::new(id, "topic", origin.clone(), ReachPolicy::default(), 0);
            store.save(&r).unwrap();
            let back = store.load(id).unwrap().expect("loaded");
            assert_eq!(back.origin, origin);
        }
    }

    #[test]
    fn orphan_rumor_roundtrip_with_seed_content() {
        let store = SqliteRumorStore::in_memory().unwrap();
        let r = Rumor::orphan(
            "r-orph",
            "뭔가 심상치 않은 일이 생긴다더라",
            RumorOrigin::Authored { by: None },
            ReachPolicy::default(),
            0,
        );
        store.save(&r).unwrap();
        let back = store.load("r-orph").unwrap().expect("loaded");
        assert!(back.is_orphan());
        assert_eq!(back.seed_content.as_deref(), Some("뭔가 심상치 않은 일이 생긴다더라"));
    }

    #[test]
    fn status_roundtrip() {
        let store = SqliteRumorStore::in_memory().unwrap();
        let mut r = sample_rumor("r-status");
        r.transition_to(RumorStatus::Fading).unwrap();
        store.save(&r).unwrap();
        assert_eq!(
            store.load("r-status").unwrap().unwrap().status(),
            RumorStatus::Fading
        );
        r.transition_to(RumorStatus::Faded).unwrap();
        store.save(&r).unwrap();
        assert_eq!(
            store.load("r-status").unwrap().unwrap().status(),
            RumorStatus::Faded
        );
    }
}
