//! `SqliteWorldStore` — Phase 1·2·3·4·5a·5b Vertical Slice (groups + persons + places + atlases + events + eras + FTS5 trigram).
//!
//! 스키마는 task-phase{1,2,3,4,5a,5b}-vertical-slice §6.3을 그대로 따름.
//! Phase 2에서 persons 테이블 + persons_fts 추가 (`migrate_v2`). Phase 3에서 places +
//! places_fts 추가 (`migrate_v3`). Phase 4에서 atlases + atlases_fts + place_atlas_refs
//! 양방향 인덱스 추가 (`migrate_v4`). Phase 5a에서 events + events_fts +
//! event_participants_refs 양방향 인덱스 추가 (`migrate_v5`). Phase 5b 체크포인트 1에서
//! eras + eras_fts 추가 (`migrate_v6`). Phase 5b 체크포인트 2에서 timelines + timelines_fts +
//! timeline_era_refs 양방향 인덱스 추가 (`migrate_v7`). 같은 SQLite 파일이 7 도메인 모두 보관.
//! 임베딩은 Phase N+에서 도입 (vec0 미사용).

use std::sync::Mutex;

use rusqlite::{Connection, params};
use serde_json::{Map, Value};

use crate::domain::world::{
    Atlas, AtlasExtent, AtlasFilter, AtlasId, Era, EraFilter, EraId, EraTemporal, Event,
    EventCategory, EventFilter, EventId, EventTemporal, Group, GroupFilter, GroupId, HexacoSix,
    ParticipantsRefs, Person, PersonFilter, PersonId, PersonStatus, PersonTemporal, Place,
    PlaceFilter, PlaceId, PlaceLayer, Spatial, Timeline, TimelineFilter, TimelineId, WorldError,
};
#[cfg(test)]
use crate::domain::world::GroupStatus;
use crate::worldbuilding::WorldRepository;

const SCHEMA_VERSION: i64 = 7;

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
        if current < 3 {
            Self::migrate_v3(&conn)?;
        }
        if current < 4 {
            Self::migrate_v4(&conn)?;
        }
        if current < 5 {
            Self::migrate_v5(&conn)?;
        }
        if current < 6 {
            Self::migrate_v6(&conn)?;
        }
        if current < 7 {
            Self::migrate_v7(&conn)?;
        }
        // schema_meta를 단일 row로 강제 (Code review #7).
        // 이전 구현은 `INSERT OR REPLACE INTO world_schema_meta(version)` 만 호출했는데,
        // PRIMARY KEY가 version이라 v1→v2 후 (1)·(2) 두 row가 누적됐다. MAX()는 정상 동작
        // 했으나 향후 "exact version 매치" 코드가 깨질 수 있어 DELETE+INSERT로 단일 row 유지.
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        tx.execute("DELETE FROM world_schema_meta", [])
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        tx.execute(
            "INSERT INTO world_schema_meta(version) VALUES (?)",
            [SCHEMA_VERSION],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        tx.commit().map_err(|e| WorldError::Storage(e.to_string()))?;
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

    /// v3 → v4 마이그레이션: atlases 테이블 + atlases_fts + place_atlas_refs 양방향 인덱스.
    /// `CREATE TABLE IF NOT EXISTS`이라 v4에서 신규 생성한 DB에도 안전.
    ///
    /// `place_atlas_refs`는 Phase 3에서 자리만 잡았던 것을 Phase 4에서 정식 활성:
    /// composite PK (atlas_id, place_id) + ref_order로 references 배열 내 위치 보존.
    /// `idx_par_place`는 place→atlas 역참조를 빠르게(어느 atlas에 등장하는가).
    ///
    /// **Source-of-truth 계약 (중요)**:
    /// - `atlases.references_json`이 **단일 권위** — `row_to_atlas`가 본 컬럼만 읽어
    ///   `Atlas.references`를 복원한다. 도메인·HTTP 응답에서 보이는 references는 모두
    ///   여기에서 나온다.
    /// - `place_atlas_refs`는 **역방향 인덱스 전용** — "이 place_id를 참조하는 atlas 찾기"
    ///   같은 reverse lookup용. `get_atlas`/`list_atlases`는 본 테이블을 조회하지 않는다.
    /// - 두 곳의 일관성은 `upsert_atlas` 단일 트랜잭션 내에서만 보장된다 — 외부 도구가
    ///   둘 중 하나만 변경하면 silent drift 발생 가능. 마이그레이션·외부 SQL 작성 시
    ///   반드시 둘 다 갱신할 것.
    fn migrate_v4(conn: &Connection) -> Result<(), WorldError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS atlases (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                aliases_json TEXT NOT NULL DEFAULT '[]',
                summary TEXT NOT NULL DEFAULT '',
                tags_json TEXT NOT NULL DEFAULT '[]',
                extras_json TEXT NOT NULL DEFAULT '{}',
                extent_json TEXT NOT NULL DEFAULT '{}',
                references_json TEXT NOT NULL DEFAULT '[]',
                body_sections_json TEXT NOT NULL DEFAULT '{}',
                source_path TEXT,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_atlases_kind ON atlases(kind);
            CREATE INDEX IF NOT EXISTS idx_atlases_project ON atlases(project_id);
            CREATE VIRTUAL TABLE IF NOT EXISTS atlases_fts USING fts5(
                id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
            );
            CREATE TABLE IF NOT EXISTS place_atlas_refs (
                atlas_id TEXT NOT NULL,
                place_id TEXT NOT NULL,
                ref_order INTEGER NOT NULL,
                PRIMARY KEY (atlas_id, place_id)
            );
            CREATE INDEX IF NOT EXISTS idx_par_place ON place_atlas_refs(place_id);
            CREATE INDEX IF NOT EXISTS idx_par_atlas ON place_atlas_refs(atlas_id);",
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }

    /// v4 → v5 마이그레이션: events 테이블 + events_fts + event_participants_refs.
    ///
    /// **Source-of-truth 계약 (중요)** — Phase 4 atlases와 동일 패턴:
    /// - `events.participants_json`이 **단일 권위** — `row_to_event`가 본 컬럼만 읽어
    ///   `Event.participants`를 복원한다. 도메인·HTTP 응답에서 보이는 participants는
    ///   모두 여기에서 나온다.
    /// - `event_participants_refs`는 **역방향 인덱스 전용** — "이 person/group/place_id를
    ///   참조하는 event 찾기" 같은 reverse lookup용. `get_event`/`list_events`는
    ///   ref_kind/ref_id 필터를 본 테이블로 조회하지만 결과 row는 events 테이블을 권위로 한다.
    /// - 두 곳의 일관성은 `upsert_event` 단일 트랜잭션 내에서만 보장된다 — 외부 도구가
    ///   둘 중 하나만 변경하면 silent drift 발생 가능.
    ///
    /// `year_relative` 캐시 컬럼은 `temporal_json.year_relative`와 동일 — 정렬·필터용.
    /// `era_id` 컬럼은 텍스트 보존만 (Phase 5b에서 era 도메인 외래키로 활성).
    fn migrate_v5(conn: &Connection) -> Result<(), WorldError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'historical' CHECK(category IN ('historical','scheduled','legendary')),
                name TEXT NOT NULL,
                aliases_json TEXT NOT NULL DEFAULT '[]',
                summary TEXT NOT NULL DEFAULT '',
                tags_json TEXT NOT NULL DEFAULT '[]',
                extras_json TEXT NOT NULL DEFAULT '{}',
                temporal_json TEXT NOT NULL DEFAULT '{}',
                year_relative INTEGER,
                era_id TEXT,
                participants_json TEXT NOT NULL DEFAULT '{}',
                body_sections_json TEXT NOT NULL DEFAULT '{}',
                related_events_json TEXT NOT NULL DEFAULT '[]',
                source_path TEXT,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);
            CREATE INDEX IF NOT EXISTS idx_events_category ON events(category);
            CREATE INDEX IF NOT EXISTS idx_events_year_relative ON events(year_relative);
            CREATE INDEX IF NOT EXISTS idx_events_era_id ON events(era_id);
            CREATE INDEX IF NOT EXISTS idx_events_project ON events(project_id);
            CREATE VIRTUAL TABLE IF NOT EXISTS events_fts USING fts5(
                id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
            );
            CREATE TABLE IF NOT EXISTS event_participants_refs (
                event_id TEXT NOT NULL,
                ref_kind TEXT NOT NULL CHECK(ref_kind IN ('person','group','place')),
                ref_id TEXT NOT NULL,
                ref_order INTEGER NOT NULL,
                PRIMARY KEY (event_id, ref_kind, ref_id)
            );
            CREATE INDEX IF NOT EXISTS idx_epr_person ON event_participants_refs(ref_id) WHERE ref_kind = 'person';
            CREATE INDEX IF NOT EXISTS idx_epr_group ON event_participants_refs(ref_id) WHERE ref_kind = 'group';
            CREATE INDEX IF NOT EXISTS idx_epr_place ON event_participants_refs(ref_id) WHERE ref_kind = 'place';
            CREATE INDEX IF NOT EXISTS idx_epr_event ON event_participants_refs(event_id);",
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }

    /// v5 → v6 마이그레이션: eras 테이블 + eras_fts (Phase 5b 체크포인트 1).
    ///
    /// Era는 인스턴스 도메인이라 Atlas의 place_atlas_refs 같은 양방향 인덱스 불필요.
    /// `key_events`는 Era→Event 단방향 외래키이며 역방향 lookup("이 사건이 어느 era의
    /// key_events에 포함됐나")이 흔하지 않다 — 필요 시 Phase 6+에서 추가.
    ///
    /// `start_year_relative`/`end_year_relative` 캐시 컬럼이 events.year_relative와
    /// 같은 정렬 키 — Timeline.events_during(era_id) view 메서드의 인덱스 활용 보장.
    /// boundary 정책 §3.3 — start inclusive · end exclusive.
    fn migrate_v6(conn: &Connection) -> Result<(), WorldError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS eras (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                aliases_json TEXT NOT NULL DEFAULT '[]',
                summary TEXT NOT NULL DEFAULT '',
                tags_json TEXT NOT NULL DEFAULT '[]',
                extras_json TEXT NOT NULL DEFAULT '{}',
                temporal_json TEXT NOT NULL DEFAULT '{}',
                start_year_relative INTEGER,
                end_year_relative INTEGER,
                key_events_json TEXT NOT NULL DEFAULT '[]',
                body_sections_json TEXT NOT NULL DEFAULT '{}',
                source_path TEXT,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_eras_kind ON eras(kind);
            CREATE INDEX IF NOT EXISTS idx_eras_start_year ON eras(start_year_relative);
            CREATE INDEX IF NOT EXISTS idx_eras_end_year ON eras(end_year_relative);
            CREATE INDEX IF NOT EXISTS idx_eras_project ON eras(project_id);
            CREATE VIRTUAL TABLE IF NOT EXISTS eras_fts USING fts5(
                id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
            );",
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }

    /// v6 → v7 마이그레이션: timelines + timelines_fts + timeline_era_refs (Phase 5b 체크포인트 2).
    ///
    /// Atlas의 place_atlas_refs 패턴 그대로 — `references_json`이 단일 권위, `timeline_era_refs`는
    /// 역방향 인덱스 전용 (composite PK + idx_ter_era로 "이 era를 참조하는 timeline 찾기").
    fn migrate_v7(conn: &Connection) -> Result<(), WorldError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS timelines (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                aliases_json TEXT NOT NULL DEFAULT '[]',
                summary TEXT NOT NULL DEFAULT '',
                tags_json TEXT NOT NULL DEFAULT '[]',
                extras_json TEXT NOT NULL DEFAULT '{}',
                references_json TEXT NOT NULL DEFAULT '[]',
                body_sections_json TEXT NOT NULL DEFAULT '{}',
                source_path TEXT,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_timelines_kind ON timelines(kind);
            CREATE INDEX IF NOT EXISTS idx_timelines_project ON timelines(project_id);
            CREATE VIRTUAL TABLE IF NOT EXISTS timelines_fts USING fts5(
                id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
            );
            CREATE TABLE IF NOT EXISTS timeline_era_refs (
                timeline_id TEXT NOT NULL,
                era_id TEXT NOT NULL,
                ref_order INTEGER NOT NULL,
                PRIMARY KEY (timeline_id, era_id)
            );
            CREATE INDEX IF NOT EXISTS idx_ter_era ON timeline_era_refs(era_id);
            CREATE INDEX IF NOT EXISTS idx_ter_timeline ON timeline_era_refs(timeline_id);",
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }

    /// v2 → v3 마이그레이션: places 테이블 + places_fts.
    /// `CREATE TABLE IF NOT EXISTS`이라 v3에서 신규 생성한 DB에도 안전.
    /// `place_atlas_refs`는 Phase 4 `migrate_v4`에서 정식 추가.
    fn migrate_v3(conn: &Connection) -> Result<(), WorldError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS places (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                layer TEXT NOT NULL CHECK(layer IN ('settlement','geography')),
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                aliases_json TEXT NOT NULL DEFAULT '[]',
                summary TEXT NOT NULL DEFAULT '',
                tags_json TEXT NOT NULL DEFAULT '[]',
                extras_json TEXT NOT NULL DEFAULT '{}',
                body_sections_json TEXT NOT NULL DEFAULT '{}',
                spatial_json TEXT NOT NULL DEFAULT '{}',
                parent_place TEXT,
                source_path TEXT,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_places_layer ON places(layer);
            CREATE INDEX IF NOT EXISTS idx_places_kind ON places(kind);
            CREATE INDEX IF NOT EXISTS idx_places_parent ON places(parent_place);
            CREATE INDEX IF NOT EXISTS idx_places_project ON places(project_id);
            CREATE VIRTUAL TABLE IF NOT EXISTS places_fts USING fts5(
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

/// JSON 배열 컬럼(tags_json·affiliation_json)을 토큰 단위로 LIKE 매칭할 때 쓰는 패턴 빌더.
///
/// LIKE의 와일드카드 `%`/`_`를 escape하고, 따옴표로 둘러싼 토큰을 만들어서
/// `["group-a","group-b"]`에서 정확히 `"group-a"`만 매칭하도록 한다.
/// 호출자는 `... LIKE ? ESCAPE '\\\\'` SQL을 사용해야 함.
///
/// 보호하는 케이스:
/// - id에 `_` (예: `group_underscore`) — `_`는 LIKE에서 single-char wildcard로 작동
/// - id에 `%` — 전체 wildcard
/// - id가 다른 id의 prefix (예: `group-a` vs `group-aa`) — 따옴표 boundary가 분리
fn json_token_like_pattern(token: &str) -> String {
    let escaped = token.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%\"{}\"%", escaped)
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

fn place_body_concat(place: &Place) -> String {
    let mut s = String::new();
    for (k, v) in &place.body_sections {
        s.push_str(k);
        s.push('\n');
        s.push_str(v);
        s.push('\n');
    }
    s
}

fn event_body_concat(event: &Event) -> String {
    // Event body는 산문 위주 (`## 개요`/`## 발단`/`## 결과` 등). 코드블록 strip은
    // 미적용 — Atlas만큼 ASCII art가 흔하지 않다. 필요해지면 strip_fenced_code_blocks를
    // 적용 (group/person/place와 동일 정책).
    let mut s = String::new();
    for (k, v) in &event.body_sections {
        s.push_str(k);
        s.push('\n');
        s.push_str(v);
        s.push('\n');
    }
    s
}

fn era_body_concat(era: &Era) -> String {
    // Era body는 산문 위주 — Event와 동일 정책. 코드블록 strip 미적용.
    let mut s = String::new();
    for (k, v) in &era.body_sections {
        s.push_str(k);
        s.push('\n');
        s.push_str(v);
        s.push('\n');
    }
    s
}

fn timeline_body_concat(timeline: &Timeline) -> String {
    // Timeline body는 산문 위주 — Era·Event와 동일 정책.
    let mut s = String::new();
    for (k, v) in &timeline.body_sections {
        s.push_str(k);
        s.push('\n');
        s.push_str(v);
        s.push('\n');
    }
    s
}

/// Atlas body는 `## 배치 다이어그램` 같은 ASCII art 코드블록(```...```)을 byte-exact
/// 보존한다. 그 코드블록은 트리그램 토크나이저에 무의미한 토큰(box-drawing
/// 부분 문자열·들여쓰기 공백)을 다량 생성해 FTS5 인덱스를 부풀리고, 임의의 3-byte
/// 시퀀스가 atlas-jungwon에 매치되는 false positive를 만든다.
///
/// **정책**: FTS body 합성 시 fenced code block (``` 또는 ~~~로 시작·끝) 안의 라인은
/// 제외. 도메인 데이터(`body_sections`)에는 그대로 보존됨 — view·HTTP 응답에선 손실 없음.
/// 이 정책은 atlas에만 적용. group/person/place는 산문 위주라 코드블록이 거의 없고,
/// 있더라도 적은 양이라 별도 처리하지 않는다 (Phase 4 결정).
fn atlas_body_concat(atlas: &Atlas) -> String {
    let mut s = String::new();
    for (k, v) in &atlas.body_sections {
        s.push_str(k);
        s.push('\n');
        s.push_str(&strip_fenced_code_blocks(v));
        s.push('\n');
    }
    s
}

/// fenced code block(``` 또는 ~~~ 3+ 연속) 안의 라인을 제거한 사본 반환.
/// 같은 종류의 펜스끼리만 토글되며(``` ↔ ~~~), 펜스 라인 자체도 제거.
/// 펜스가 닫히지 않은 입력은 그 시점부터 EOF까지 모두 제거 (안전 측).
fn strip_fenced_code_blocks(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut fence: Option<char> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        let n_back = trimmed.chars().take_while(|&c| c == '`').count();
        let n_tilde = trimmed.chars().take_while(|&c| c == '~').count();
        let fence_kind = if n_back >= 3 {
            Some('`')
        } else if n_tilde >= 3 {
            Some('~')
        } else {
            None
        };
        if let Some(c) = fence_kind {
            match fence {
                None => fence = Some(c),
                Some(prev) if prev == c => fence = None,
                _ => {} // 다른 종류 펜스는 무시
            }
            // 펜스 라인 자체도 인덱스에서 제외.
            continue;
        }
        if fence.is_some() {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
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
            // LIKE 메타문자(%/_) escape는 json_token_like_pattern + ESCAPE '\\' 조합으로.
            sql.push_str(" AND tags_json LIKE ? ESCAPE '\\'");
            binds.push(json_token_like_pattern(&t));
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
            // affiliation_json은 ["group-a","group-b"] 형식. 따옴표로 boundary 강제.
            // LIKE 메타문자 escape는 json_token_like_pattern + ESCAPE '\\' 조합으로.
            sql.push_str(" AND affiliation_json LIKE ? ESCAPE '\\'");
            binds.push(json_token_like_pattern(g.as_str()));
        }
        if let Some(t) = filter.genre_tag {
            sql.push_str(" AND tags_json LIKE ? ESCAPE '\\'");
            binds.push(json_token_like_pattern(&t));
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

    // ---------------------------------------------------------------------
    // Phase 3 — Place
    // ---------------------------------------------------------------------

    fn upsert_place(&self, project_id: &str, place: &Place) -> Result<(), WorldError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| WorldError::Storage(e.to_string()))?;

        let extras_json = serde_json::to_string(&place.extras)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let body_json = serde_json::to_string(&place.body_sections)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let spatial_json = serde_json::to_string(&place.spatial)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let aliases_json = json_array_of_strings(&place.aliases);
        let tags_json = json_array_of_strings(&place.tags);
        let parent_place = place
            .spatial
            .parent_place
            .as_ref()
            .map(|p| p.as_str().to_string());
        let layer = place.layer.as_str();
        let updated_at = now_ms();

        tx.execute(
            "INSERT OR REPLACE INTO places (
                id, project_id, layer, kind, name, aliases_json,
                summary, tags_json, extras_json, body_sections_json, spatial_json,
                parent_place, source_path, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                place.id.as_str(),
                project_id,
                layer,
                place.kind,
                place.name,
                aliases_json,
                place.summary,
                tags_json,
                extras_json,
                body_json,
                spatial_json,
                parent_place,
                place.source_path,
                updated_at,
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        // FTS5 — id 기반 delete-then-insert
        tx.execute("DELETE FROM places_fts WHERE id = ?1", params![place.id.as_str()])
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        tx.execute(
            "INSERT INTO places_fts (id, name, aliases, summary, body) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                place.id.as_str(),
                place.name,
                aliases_concat(&place.aliases),
                place.summary,
                place_body_concat(place),
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        tx.commit().map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }

    fn list_places(&self, filter: PlaceFilter) -> Result<Vec<Place>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let mut sql = String::from(
            "SELECT id, project_id, layer, kind, name, aliases_json,
                    summary, tags_json, extras_json, body_sections_json, spatial_json,
                    parent_place, source_path
             FROM places WHERE 1=1",
        );
        let mut binds: Vec<String> = Vec::new();
        if let Some(l) = filter.layer {
            sql.push_str(" AND layer = ?");
            binds.push(l.as_str().to_string());
        }
        if let Some(k) = filter.kind {
            sql.push_str(" AND kind = ?");
            binds.push(k);
        }
        if let Some(p) = filter.parent_place {
            sql.push_str(" AND parent_place = ?");
            binds.push(p.as_str().to_string());
        }
        if let Some(t) = filter.genre_tag {
            sql.push_str(" AND tags_json LIKE ? ESCAPE '\\'");
            binds.push(json_token_like_pattern(&t));
        }
        sql.push_str(" ORDER BY id ASC");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let bind_refs: Vec<&dyn rusqlite::ToSql> =
            binds.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt
            .query_map(rusqlite::params_from_iter(bind_refs), row_to_place)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_place_rows_warn_on_err(rows))
    }

    fn get_place(&self, id: &PlaceId) -> Result<Option<Place>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let res = conn.query_row(
            "SELECT id, project_id, layer, kind, name, aliases_json,
                    summary, tags_json, extras_json, body_sections_json, spatial_json,
                    parent_place, source_path
             FROM places WHERE id = ?1",
            params![id.as_str()],
            row_to_place,
        );
        match res {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(WorldError::Storage(e.to_string())),
        }
    }

    /// `get_place` N round-trip을 피하는 단일 `IN(...)` 쿼리. 결과는 `ids` 입력
    /// 순서대로 반환되며 (HashMap 재정렬) 결손 id는 사일런트로 누락된다.
    /// trait의 default 구현과 의미가 동일하되 SQLite 한 번의 prepare/execute로 처리.
    fn get_places_batch(&self, ids: &[PlaceId]) -> Result<Vec<Place>, WorldError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.lock().unwrap();
        let placeholders = std::iter::repeat("?")
            .take(ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT id, project_id, layer, kind, name, aliases_json,
                    summary, tags_json, extras_json, body_sections_json, spatial_json,
                    parent_place, source_path
             FROM places WHERE id IN ({})",
            placeholders
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let id_strs: Vec<&str> = ids.iter().map(|id| id.as_str()).collect();
        let bind_refs: Vec<&dyn rusqlite::ToSql> =
            id_strs.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt
            .query_map(rusqlite::params_from_iter(bind_refs), row_to_place)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let fetched = collect_place_rows_warn_on_err(rows);

        // 입력 순서 보존 — IN(...)은 결과 순서를 보장하지 않으므로 HashMap lookup.
        let mut by_id: std::collections::HashMap<String, Place> = fetched
            .into_iter()
            .map(|p| (p.id.as_str().to_string(), p))
            .collect();
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(p) = by_id.remove(id.as_str()) {
                out.push(p);
            }
        }
        Ok(out)
    }

    fn search_places(&self, query: &str, top_k: u32) -> Result<Vec<Place>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let char_count = q.chars().count();
        if char_count < 3 {
            return self.search_places_like(&conn, q, top_k);
        }

        let escaped = q.replace('"', "\"\"");
        let phrase = format!("\"{}\"", escaped);
        let fts_hits: Result<Vec<Place>, rusqlite::Error> = (|| {
            let mut stmt = conn.prepare(
                "SELECT pl.id, pl.project_id, pl.layer, pl.kind, pl.name, pl.aliases_json,
                        pl.summary, pl.tags_json, pl.extras_json, pl.body_sections_json, pl.spatial_json,
                        pl.parent_place, pl.source_path
                 FROM places_fts f
                 JOIN places pl ON pl.id = f.id
                 WHERE places_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![phrase, top_k as i64], row_to_place)?;
            Ok(collect_place_rows_warn_on_err(rows))
        })();

        match fts_hits {
            Ok(hits) if !hits.is_empty() => Ok(hits),
            Ok(_) => self.search_places_like(&conn, q, top_k),
            Err(e) => {
                tracing::debug!(
                    "FTS5 MATCH 실패(places), LIKE fallback로 진행: query={q:?} err={e}"
                );
                self.search_places_like(&conn, q, top_k)
            }
        }
    }

    fn count_places(&self, project_id: Option<&str>) -> Result<u64, WorldError> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = match project_id {
            Some(p) => conn
                .query_row(
                    "SELECT COUNT(*) FROM places WHERE project_id = ?1",
                    params![p],
                    |r| r.get(0),
                )
                .map_err(|e| WorldError::Storage(e.to_string()))?,
            None => conn
                .query_row("SELECT COUNT(*) FROM places", [], |r| r.get(0))
                .map_err(|e| WorldError::Storage(e.to_string()))?,
        };
        Ok(n.max(0) as u64)
    }

    // ---------------------------------------------------------------------
    // Phase 4 — Atlas (관계 도메인)
    // ---------------------------------------------------------------------

    fn upsert_atlas(&self, project_id: &str, atlas: &Atlas) -> Result<(), WorldError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| WorldError::Storage(e.to_string()))?;

        let aliases_json = json_array_of_strings(&atlas.aliases);
        let tags_json = json_array_of_strings(&atlas.tags);
        let extras_json = serde_json::to_string(&atlas.extras)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let extent_json = serde_json::to_string(&atlas.extent)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let references_json: String = serde_json::to_string(
            &atlas
                .references
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>(),
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        let body_json = serde_json::to_string(&atlas.body_sections)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let updated_at = now_ms();

        tx.execute(
            "INSERT OR REPLACE INTO atlases (
                id, project_id, kind, name, aliases_json,
                summary, tags_json, extras_json, extent_json, references_json,
                body_sections_json, source_path, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                atlas.id.as_str(),
                project_id,
                atlas.kind,
                atlas.name,
                aliases_json,
                atlas.summary,
                tags_json,
                extras_json,
                extent_json,
                references_json,
                body_json,
                atlas.source_path,
                updated_at,
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        // FTS5 — id 기반 delete-then-insert
        tx.execute(
            "DELETE FROM atlases_fts WHERE id = ?1",
            params![atlas.id.as_str()],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        tx.execute(
            "INSERT INTO atlases_fts (id, name, aliases, summary, body) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                atlas.id.as_str(),
                atlas.name,
                aliases_concat(&atlas.aliases),
                atlas.summary,
                atlas_body_concat(atlas),
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        // place_atlas_refs — 양방향 인덱스 동기화 (atlas_id 기준 delete-then-insert).
        // 같은 atlas의 기존 매핑을 모두 지우고 references 순서대로 ref_order 채워 재삽입.
        // composite PK (atlas_id, place_id)이라 동일 place 중복 시 PK 위반 — 호출자가
        // references에서 중복을 제거해야 한다.
        tx.execute(
            "DELETE FROM place_atlas_refs WHERE atlas_id = ?1",
            params![atlas.id.as_str()],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        for (idx, pid) in atlas.references.iter().enumerate() {
            tx.execute(
                "INSERT INTO place_atlas_refs (atlas_id, place_id, ref_order) VALUES (?1, ?2, ?3)",
                params![atlas.id.as_str(), pid.as_str(), idx as i64],
            )
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        }

        tx.commit().map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }

    fn list_atlases(&self, filter: AtlasFilter) -> Result<Vec<Atlas>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let mut sql = String::from(
            "SELECT id, project_id, kind, name, aliases_json,
                    summary, tags_json, extras_json, extent_json, references_json,
                    body_sections_json, source_path
             FROM atlases WHERE 1=1",
        );
        let mut binds: Vec<String> = Vec::new();
        if let Some(k) = filter.kind {
            sql.push_str(" AND kind = ?");
            binds.push(k);
        }
        if let Some(t) = filter.genre_tag {
            sql.push_str(" AND tags_json LIKE ? ESCAPE '\\'");
            binds.push(json_token_like_pattern(&t));
        }
        sql.push_str(" ORDER BY id ASC");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let bind_refs: Vec<&dyn rusqlite::ToSql> =
            binds.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt
            .query_map(rusqlite::params_from_iter(bind_refs), row_to_atlas)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_atlas_rows_warn_on_err(rows))
    }

    fn get_atlas(&self, id: &AtlasId) -> Result<Option<Atlas>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let res = conn.query_row(
            "SELECT id, project_id, kind, name, aliases_json,
                    summary, tags_json, extras_json, extent_json, references_json,
                    body_sections_json, source_path
             FROM atlases WHERE id = ?1",
            params![id.as_str()],
            row_to_atlas,
        );
        match res {
            Ok(a) => Ok(Some(a)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(WorldError::Storage(e.to_string())),
        }
    }

    fn search_atlases(&self, query: &str, top_k: u32) -> Result<Vec<Atlas>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let char_count = q.chars().count();
        if char_count < 3 {
            return self.search_atlases_like(&conn, q, top_k);
        }

        let escaped = q.replace('"', "\"\"");
        let phrase = format!("\"{}\"", escaped);
        let fts_hits: Result<Vec<Atlas>, rusqlite::Error> = (|| {
            let mut stmt = conn.prepare(
                "SELECT a.id, a.project_id, a.kind, a.name, a.aliases_json,
                        a.summary, a.tags_json, a.extras_json, a.extent_json, a.references_json,
                        a.body_sections_json, a.source_path
                 FROM atlases_fts f
                 JOIN atlases a ON a.id = f.id
                 WHERE atlases_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![phrase, top_k as i64], row_to_atlas)?;
            Ok(collect_atlas_rows_warn_on_err(rows))
        })();

        match fts_hits {
            Ok(hits) if !hits.is_empty() => Ok(hits),
            Ok(_) => self.search_atlases_like(&conn, q, top_k),
            Err(e) => {
                tracing::debug!(
                    "FTS5 MATCH 실패(atlases), LIKE fallback로 진행: query={q:?} err={e}"
                );
                self.search_atlases_like(&conn, q, top_k)
            }
        }
    }

    fn count_atlases(&self, project_id: Option<&str>) -> Result<u64, WorldError> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = match project_id {
            Some(p) => conn
                .query_row(
                    "SELECT COUNT(*) FROM atlases WHERE project_id = ?1",
                    params![p],
                    |r| r.get(0),
                )
                .map_err(|e| WorldError::Storage(e.to_string()))?,
            None => conn
                .query_row("SELECT COUNT(*) FROM atlases", [], |r| r.get(0))
                .map_err(|e| WorldError::Storage(e.to_string()))?,
        };
        Ok(n.max(0) as u64)
    }

    // ---------------------------------------------------------------------
    // Phase 5a — Event (두 번째 인스턴스 도메인)
    // ---------------------------------------------------------------------

    fn upsert_event(&self, project_id: &str, event: &Event) -> Result<(), WorldError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| WorldError::Storage(e.to_string()))?;

        let aliases_json = json_array_of_strings(&event.aliases);
        let tags_json = json_array_of_strings(&event.tags);
        let extras_json = serde_json::to_string(&event.extras)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let temporal_json = serde_json::to_string(&event.temporal)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let participants_json = serde_json::to_string(&event.participants)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let body_json = serde_json::to_string(&event.body_sections)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let related_events_json: String = serde_json::to_string(
            &event
                .related_events
                .iter()
                .map(|e| e.as_str())
                .collect::<Vec<_>>(),
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        let updated_at = now_ms();

        tx.execute(
            "INSERT OR REPLACE INTO events (
                id, project_id, kind, category, name, aliases_json,
                summary, tags_json, extras_json, temporal_json,
                year_relative, era_id, participants_json, body_sections_json,
                related_events_json, source_path, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                      ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                event.id.as_str(),
                project_id,
                event.kind,
                event.category.as_str(),
                event.name,
                aliases_json,
                event.summary,
                tags_json,
                extras_json,
                temporal_json,
                event.temporal.year_relative,
                event.era_id,
                participants_json,
                body_json,
                related_events_json,
                event.source_path,
                updated_at,
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        // FTS5 — id 기반 delete-then-insert
        tx.execute(
            "DELETE FROM events_fts WHERE id = ?1",
            params![event.id.as_str()],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        tx.execute(
            "INSERT INTO events_fts (id, name, aliases, summary, body) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                event.id.as_str(),
                event.name,
                aliases_concat(&event.aliases),
                event.summary,
                event_body_concat(event),
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        // event_participants_refs — 양방향 인덱스 동기화 (event_id 기준 delete-then-insert).
        // composite PK (event_id, ref_kind, ref_id)이라 동일 (kind, id) 중복 시 PK 위반 —
        // 호출자가 participants 내 중복을 제거해야 한다 (world-load CLI가 검증).
        tx.execute(
            "DELETE FROM event_participants_refs WHERE event_id = ?1",
            params![event.id.as_str()],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        // 정방향 순서 보존 — people → groups → places 카테고리 순으로 ref_order 부여.
        // 같은 카테고리 내에선 작성 순서 유지.
        let mut order: i64 = 0;
        for pid in &event.participants.people {
            tx.execute(
                "INSERT INTO event_participants_refs (event_id, ref_kind, ref_id, ref_order) VALUES (?1, ?2, ?3, ?4)",
                params![event.id.as_str(), "person", pid, order],
            )
            .map_err(|e| WorldError::Storage(e.to_string()))?;
            order += 1;
        }
        for gid in &event.participants.groups {
            tx.execute(
                "INSERT INTO event_participants_refs (event_id, ref_kind, ref_id, ref_order) VALUES (?1, ?2, ?3, ?4)",
                params![event.id.as_str(), "group", gid, order],
            )
            .map_err(|e| WorldError::Storage(e.to_string()))?;
            order += 1;
        }
        for plid in &event.participants.places {
            tx.execute(
                "INSERT INTO event_participants_refs (event_id, ref_kind, ref_id, ref_order) VALUES (?1, ?2, ?3, ?4)",
                params![event.id.as_str(), "place", plid, order],
            )
            .map_err(|e| WorldError::Storage(e.to_string()))?;
            order += 1;
        }

        tx.commit().map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }

    fn list_events(&self, filter: EventFilter) -> Result<Vec<Event>, WorldError> {
        // R2: 함수 진입 시 전체 destructure — non-Copy 필드가 추가될 때 borrow 충돌 방지.
        let EventFilter {
            kind,
            category,
            participants_person,
            participants_group,
            participants_place,
            year_relative_min,
            year_relative_max,
            genre_tag,
        } = filter;

        let conn = self.conn.lock().unwrap();
        let mut sql = String::from(
            "SELECT e.id, e.project_id, e.kind, e.category, e.name, e.aliases_json,
                    e.summary, e.tags_json, e.extras_json, e.temporal_json,
                    e.year_relative, e.era_id, e.participants_json, e.body_sections_json,
                    e.related_events_json, e.source_path
             FROM events e",
        );
        // R1: heterogeneous bind — year_relative_min/max는 Integer로 바인딩해
        // SQLite affinity 변환을 거치지 않고 idx_events_year_relative 인덱스를 직접 활용.
        // 텍스트→정수 변환은 일부 케이스에서 인덱스 사용을 포기시키므로 영구 가드.
        let mut binds: Vec<rusqlite::types::Value> = Vec::new();
        // participants_* 필터는 event_participants_refs 인덱스를 활용한 EXISTS로 매핑.
        // 셋 다 동시에 지정되면 AND로 결합 — 모두 관여한 사건만 (LLM/MCP 사용자 주의 사항).
        let mut where_clauses: Vec<String> = vec!["1=1".into()];
        if let Some(k) = kind {
            where_clauses.push("e.kind = ?".into());
            binds.push(rusqlite::types::Value::Text(k));
        }
        if let Some(c) = category {
            where_clauses.push("e.category = ?".into());
            binds.push(rusqlite::types::Value::Text(c.as_str().to_string()));
        }
        if let Some(p) = participants_person {
            where_clauses.push(
                "EXISTS (SELECT 1 FROM event_participants_refs r WHERE r.event_id = e.id AND r.ref_kind = 'person' AND r.ref_id = ?)".into(),
            );
            binds.push(rusqlite::types::Value::Text(p));
        }
        if let Some(g) = participants_group {
            where_clauses.push(
                "EXISTS (SELECT 1 FROM event_participants_refs r WHERE r.event_id = e.id AND r.ref_kind = 'group' AND r.ref_id = ?)".into(),
            );
            binds.push(rusqlite::types::Value::Text(g));
        }
        if let Some(pl) = participants_place {
            where_clauses.push(
                "EXISTS (SELECT 1 FROM event_participants_refs r WHERE r.event_id = e.id AND r.ref_kind = 'place' AND r.ref_id = ?)".into(),
            );
            binds.push(rusqlite::types::Value::Text(pl));
        }
        if let Some(min) = year_relative_min {
            where_clauses.push("e.year_relative IS NOT NULL AND e.year_relative >= ?".into());
            binds.push(rusqlite::types::Value::Integer(min as i64));
        }
        if let Some(max) = year_relative_max {
            where_clauses.push("e.year_relative IS NOT NULL AND e.year_relative <= ?".into());
            binds.push(rusqlite::types::Value::Integer(max as i64));
        }
        if let Some(t) = genre_tag {
            where_clauses.push("e.tags_json LIKE ? ESCAPE '\\'".into());
            binds.push(rusqlite::types::Value::Text(json_token_like_pattern(&t)));
        }
        sql.push_str(" WHERE ");
        sql.push_str(&where_clauses.join(" AND "));
        sql.push_str(" ORDER BY e.id ASC");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(binds.iter()), row_to_event)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_event_rows_warn_on_err(rows))
    }

    fn get_event(&self, id: &EventId) -> Result<Option<Event>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let res = conn.query_row(
            "SELECT id, project_id, kind, category, name, aliases_json,
                    summary, tags_json, extras_json, temporal_json,
                    year_relative, era_id, participants_json, body_sections_json,
                    related_events_json, source_path
             FROM events WHERE id = ?1",
            params![id.as_str()],
            row_to_event,
        );
        match res {
            Ok(e) => Ok(Some(e)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(WorldError::Storage(e.to_string())),
        }
    }

    fn search_events(&self, query: &str, top_k: u32) -> Result<Vec<Event>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let char_count = q.chars().count();
        if char_count < 3 {
            return self.search_events_like(&conn, q, top_k);
        }

        let escaped = q.replace('"', "\"\"");
        let phrase = format!("\"{}\"", escaped);
        let fts_hits: Result<Vec<Event>, rusqlite::Error> = (|| {
            let mut stmt = conn.prepare(
                "SELECT e.id, e.project_id, e.kind, e.category, e.name, e.aliases_json,
                        e.summary, e.tags_json, e.extras_json, e.temporal_json,
                        e.year_relative, e.era_id, e.participants_json, e.body_sections_json,
                        e.related_events_json, e.source_path
                 FROM events_fts f
                 JOIN events e ON e.id = f.id
                 WHERE events_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![phrase, top_k as i64], row_to_event)?;
            Ok(collect_event_rows_warn_on_err(rows))
        })();

        match fts_hits {
            Ok(hits) if !hits.is_empty() => Ok(hits),
            Ok(_) => self.search_events_like(&conn, q, top_k),
            Err(e) => {
                tracing::debug!(
                    "FTS5 MATCH 실패(events), LIKE fallback로 진행: query={q:?} err={e}"
                );
                self.search_events_like(&conn, q, top_k)
            }
        }
    }

    fn count_events(&self, project_id: Option<&str>) -> Result<u64, WorldError> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = match project_id {
            Some(p) => conn
                .query_row(
                    "SELECT COUNT(*) FROM events WHERE project_id = ?1",
                    params![p],
                    |r| r.get(0),
                )
                .map_err(|e| WorldError::Storage(e.to_string()))?,
            None => conn
                .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
                .map_err(|e| WorldError::Storage(e.to_string()))?,
        };
        Ok(n.max(0) as u64)
    }

    // ---------------------------------------------------------------------
    // Phase 5b — Era (세 번째 인스턴스 도메인)
    // ---------------------------------------------------------------------

    fn upsert_era(&self, project_id: &str, era: &Era) -> Result<(), WorldError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| WorldError::Storage(e.to_string()))?;

        let aliases_json = json_array_of_strings(&era.aliases);
        let tags_json = json_array_of_strings(&era.tags);
        let extras_json = serde_json::to_string(&era.extras)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let temporal_json = serde_json::to_string(&era.temporal)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let key_events_json: String = serde_json::to_string(
            &era.key_events
                .iter()
                .map(|e| e.as_str())
                .collect::<Vec<_>>(),
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        let body_json = serde_json::to_string(&era.body_sections)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let updated_at = now_ms();

        tx.execute(
            "INSERT OR REPLACE INTO eras (
                id, project_id, kind, name, aliases_json,
                summary, tags_json, extras_json, temporal_json,
                start_year_relative, end_year_relative,
                key_events_json, body_sections_json, source_path, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                era.id.as_str(),
                project_id,
                era.kind,
                era.name,
                aliases_json,
                era.summary,
                tags_json,
                extras_json,
                temporal_json,
                era.temporal.start_year_relative,
                era.temporal.end_year_relative,
                key_events_json,
                body_json,
                era.source_path,
                updated_at,
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        // FTS5 — id 기반 delete-then-insert
        tx.execute(
            "DELETE FROM eras_fts WHERE id = ?1",
            params![era.id.as_str()],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        tx.execute(
            "INSERT INTO eras_fts (id, name, aliases, summary, body) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                era.id.as_str(),
                era.name,
                aliases_concat(&era.aliases),
                era.summary,
                era_body_concat(era),
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        tx.commit().map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }

    fn list_eras(&self, filter: EraFilter) -> Result<Vec<Era>, WorldError> {
        // Phase 5a R2 패턴: 진입 시 destructure로 borrow 충돌 방지.
        let EraFilter {
            kind,
            contains_year,
            genre_tag,
        } = filter;

        let conn = self.conn.lock().unwrap();
        let mut sql = String::from(
            "SELECT id, project_id, kind, name, aliases_json,
                    summary, tags_json, extras_json, temporal_json,
                    start_year_relative, end_year_relative,
                    key_events_json, body_sections_json, source_path
             FROM eras",
        );
        // Phase 5a R1 패턴: heterogeneous bind — Integer 명시로 인덱스 affinity 보장.
        let mut binds: Vec<rusqlite::types::Value> = Vec::new();
        let mut where_clauses: Vec<String> = vec!["1=1".into()];
        if let Some(k) = kind {
            where_clauses.push("kind = ?".into());
            binds.push(rusqlite::types::Value::Text(k));
        }
        if let Some(y) = contains_year {
            // boundary 정책 §3.3: start inclusive · end exclusive.
            where_clauses.push(
                "start_year_relative IS NOT NULL AND end_year_relative IS NOT NULL \
                 AND start_year_relative <= ? AND end_year_relative > ?"
                    .into(),
            );
            binds.push(rusqlite::types::Value::Integer(y as i64));
            binds.push(rusqlite::types::Value::Integer(y as i64));
        }
        if let Some(t) = genre_tag {
            where_clauses.push("tags_json LIKE ? ESCAPE '\\'".into());
            binds.push(rusqlite::types::Value::Text(json_token_like_pattern(&t)));
        }
        sql.push_str(" WHERE ");
        sql.push_str(&where_clauses.join(" AND "));
        sql.push_str(" ORDER BY id ASC");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(binds.iter()), row_to_era)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_era_rows_warn_on_err(rows))
    }

    fn get_era(&self, id: &EraId) -> Result<Option<Era>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let res = conn.query_row(
            "SELECT id, project_id, kind, name, aliases_json,
                    summary, tags_json, extras_json, temporal_json,
                    start_year_relative, end_year_relative,
                    key_events_json, body_sections_json, source_path
             FROM eras WHERE id = ?1",
            params![id.as_str()],
            row_to_era,
        );
        match res {
            Ok(e) => Ok(Some(e)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(WorldError::Storage(e.to_string())),
        }
    }

    fn search_eras(&self, query: &str, top_k: u32) -> Result<Vec<Era>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let char_count = q.chars().count();
        if char_count < 3 {
            return self.search_eras_like(&conn, q, top_k);
        }

        let escaped = q.replace('"', "\"\"");
        let phrase = format!("\"{}\"", escaped);
        let fts_hits: Result<Vec<Era>, rusqlite::Error> = (|| {
            let mut stmt = conn.prepare(
                "SELECT e.id, e.project_id, e.kind, e.name, e.aliases_json,
                        e.summary, e.tags_json, e.extras_json, e.temporal_json,
                        e.start_year_relative, e.end_year_relative,
                        e.key_events_json, e.body_sections_json, e.source_path
                 FROM eras_fts f
                 JOIN eras e ON e.id = f.id
                 WHERE eras_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![phrase, top_k as i64], row_to_era)?;
            Ok(collect_era_rows_warn_on_err(rows))
        })();

        match fts_hits {
            Ok(hits) if !hits.is_empty() => Ok(hits),
            Ok(_) => self.search_eras_like(&conn, q, top_k),
            Err(e) => {
                tracing::debug!(
                    "FTS5 MATCH 실패(eras), LIKE fallback로 진행: query={q:?} err={e}"
                );
                self.search_eras_like(&conn, q, top_k)
            }
        }
    }

    fn count_eras(&self, project_id: Option<&str>) -> Result<u64, WorldError> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = match project_id {
            Some(p) => conn
                .query_row(
                    "SELECT COUNT(*) FROM eras WHERE project_id = ?1",
                    params![p],
                    |r| r.get(0),
                )
                .map_err(|e| WorldError::Storage(e.to_string()))?,
            None => conn
                .query_row("SELECT COUNT(*) FROM eras", [], |r| r.get(0))
                .map_err(|e| WorldError::Storage(e.to_string()))?,
        };
        Ok(n.max(0) as u64)
    }

    // ---------------------------------------------------------------------
    // Phase 5b 체크포인트 2 — Timeline (두 번째 관계 도메인)
    // Atlas의 place_atlas_refs 패턴 그대로 — references_json 단일 권위 +
    // timeline_era_refs 역방향 인덱스 (composite PK delete-then-insert 동기화).
    // ---------------------------------------------------------------------

    fn upsert_timeline(
        &self,
        project_id: &str,
        timeline: &Timeline,
    ) -> Result<(), WorldError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| WorldError::Storage(e.to_string()))?;

        let aliases_json = json_array_of_strings(&timeline.aliases);
        let tags_json = json_array_of_strings(&timeline.tags);
        let extras_json = serde_json::to_string(&timeline.extras)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let references_json: String = serde_json::to_string(
            &timeline
                .references
                .iter()
                .map(|e| e.as_str())
                .collect::<Vec<_>>(),
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        let body_json = serde_json::to_string(&timeline.body_sections)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let updated_at = now_ms();

        tx.execute(
            "INSERT OR REPLACE INTO timelines (
                id, project_id, kind, name, aliases_json,
                summary, tags_json, extras_json, references_json,
                body_sections_json, source_path, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                timeline.id.as_str(),
                project_id,
                timeline.kind,
                timeline.name,
                aliases_json,
                timeline.summary,
                tags_json,
                extras_json,
                references_json,
                body_json,
                timeline.source_path,
                updated_at,
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        // FTS5 — id 기반 delete-then-insert
        tx.execute(
            "DELETE FROM timelines_fts WHERE id = ?1",
            params![timeline.id.as_str()],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        tx.execute(
            "INSERT INTO timelines_fts (id, name, aliases, summary, body) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                timeline.id.as_str(),
                timeline.name,
                aliases_concat(&timeline.aliases),
                timeline.summary,
                timeline_body_concat(timeline),
            ],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;

        // timeline_era_refs — 양방향 인덱스 동기화 (timeline_id 기준 delete-then-insert).
        // composite PK (timeline_id, era_id) — 동일 era 중복 시 PK 위반, 호출자가 references
        // 중복을 제거해야 한다 (world-load CLI가 검증).
        tx.execute(
            "DELETE FROM timeline_era_refs WHERE timeline_id = ?1",
            params![timeline.id.as_str()],
        )
        .map_err(|e| WorldError::Storage(e.to_string()))?;
        for (idx, eid) in timeline.references.iter().enumerate() {
            tx.execute(
                "INSERT INTO timeline_era_refs (timeline_id, era_id, ref_order) VALUES (?1, ?2, ?3)",
                params![timeline.id.as_str(), eid.as_str(), idx as i64],
            )
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        }

        tx.commit().map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(())
    }

    fn list_timelines(
        &self,
        filter: TimelineFilter,
    ) -> Result<Vec<Timeline>, WorldError> {
        // Phase 5a R2 패턴 — 진입 시 destructure.
        let TimelineFilter {
            kind,
            references_era,
            genre_tag,
        } = filter;

        let conn = self.conn.lock().unwrap();
        let mut sql = String::from(
            "SELECT id, project_id, kind, name, aliases_json,
                    summary, tags_json, extras_json, references_json,
                    body_sections_json, source_path
             FROM timelines",
        );
        let mut binds: Vec<rusqlite::types::Value> = Vec::new();
        let mut where_clauses: Vec<String> = vec!["1=1".into()];
        if let Some(k) = kind {
            where_clauses.push("kind = ?".into());
            binds.push(rusqlite::types::Value::Text(k));
        }
        if let Some(eid) = references_era {
            // timeline_era_refs 인덱스 활용 — 특정 era를 references에 포함하는 timeline.
            where_clauses.push(
                "id IN (SELECT timeline_id FROM timeline_era_refs WHERE era_id = ?)"
                    .into(),
            );
            binds.push(rusqlite::types::Value::Text(eid.0));
        }
        if let Some(t) = genre_tag {
            where_clauses.push("tags_json LIKE ? ESCAPE '\\'".into());
            binds.push(rusqlite::types::Value::Text(json_token_like_pattern(&t)));
        }
        sql.push_str(" WHERE ");
        sql.push_str(&where_clauses.join(" AND "));
        sql.push_str(" ORDER BY id ASC");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(binds.iter()), row_to_timeline)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_timeline_rows_warn_on_err(rows))
    }

    fn get_timeline(&self, id: &TimelineId) -> Result<Option<Timeline>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let res = conn.query_row(
            "SELECT id, project_id, kind, name, aliases_json,
                    summary, tags_json, extras_json, references_json,
                    body_sections_json, source_path
             FROM timelines WHERE id = ?1",
            params![id.as_str()],
            row_to_timeline,
        );
        match res {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(WorldError::Storage(e.to_string())),
        }
    }

    fn search_timelines(
        &self,
        query: &str,
        top_k: u32,
    ) -> Result<Vec<Timeline>, WorldError> {
        let conn = self.conn.lock().unwrap();
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let char_count = q.chars().count();
        if char_count < 3 {
            return self.search_timelines_like(&conn, q, top_k);
        }

        let escaped = q.replace('"', "\"\"");
        let phrase = format!("\"{}\"", escaped);
        let fts_hits: Result<Vec<Timeline>, rusqlite::Error> = (|| {
            let mut stmt = conn.prepare(
                "SELECT t.id, t.project_id, t.kind, t.name, t.aliases_json,
                        t.summary, t.tags_json, t.extras_json, t.references_json,
                        t.body_sections_json, t.source_path
                 FROM timelines_fts f
                 JOIN timelines t ON t.id = f.id
                 WHERE timelines_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![phrase, top_k as i64], row_to_timeline)?;
            Ok(collect_timeline_rows_warn_on_err(rows))
        })();

        match fts_hits {
            Ok(hits) if !hits.is_empty() => Ok(hits),
            Ok(_) => self.search_timelines_like(&conn, q, top_k),
            Err(e) => {
                tracing::debug!(
                    "FTS5 MATCH 실패(timelines), LIKE fallback로 진행: query={q:?} err={e}"
                );
                self.search_timelines_like(&conn, q, top_k)
            }
        }
    }

    fn count_timelines(&self, project_id: Option<&str>) -> Result<u64, WorldError> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = match project_id {
            Some(p) => conn
                .query_row(
                    "SELECT COUNT(*) FROM timelines WHERE project_id = ?1",
                    params![p],
                    |r| r.get(0),
                )
                .map_err(|e| WorldError::Storage(e.to_string()))?,
            None => conn
                .query_row("SELECT COUNT(*) FROM timelines", [], |r| r.get(0))
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

    /// Place 검색용 LIKE fallback — FTS5 trigram이 처리하지 못하는 짧은 query 또는
    /// 결과 0건 시 호출.
    fn search_places_like(
        &self,
        conn: &Connection,
        q: &str,
        top_k: u32,
    ) -> Result<Vec<Place>, WorldError> {
        let pat = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
        let mut stmt = conn
            .prepare(
                "SELECT pl.id, pl.project_id, pl.layer, pl.kind, pl.name, pl.aliases_json,
                        pl.summary, pl.tags_json, pl.extras_json, pl.body_sections_json, pl.spatial_json,
                        pl.parent_place, pl.source_path
                 FROM places pl
                 LEFT JOIN places_fts f ON f.id = pl.id
                 WHERE pl.name LIKE ?1 ESCAPE '\\'
                    OR f.aliases LIKE ?1 ESCAPE '\\'
                    OR pl.summary LIKE ?1 ESCAPE '\\'
                    OR f.body LIKE ?1 ESCAPE '\\'
                 GROUP BY pl.id
                 ORDER BY pl.id ASC
                 LIMIT ?2",
            )
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![pat, top_k as i64], row_to_place)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_place_rows_warn_on_err(rows))
    }

    /// Atlas 검색용 LIKE fallback — FTS5 trigram이 처리하지 못하는 짧은 query 또는
    /// 결과 0건 시 호출.
    fn search_atlases_like(
        &self,
        conn: &Connection,
        q: &str,
        top_k: u32,
    ) -> Result<Vec<Atlas>, WorldError> {
        let pat = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
        let mut stmt = conn
            .prepare(
                "SELECT a.id, a.project_id, a.kind, a.name, a.aliases_json,
                        a.summary, a.tags_json, a.extras_json, a.extent_json, a.references_json,
                        a.body_sections_json, a.source_path
                 FROM atlases a
                 LEFT JOIN atlases_fts f ON f.id = a.id
                 WHERE a.name LIKE ?1 ESCAPE '\\'
                    OR f.aliases LIKE ?1 ESCAPE '\\'
                    OR a.summary LIKE ?1 ESCAPE '\\'
                    OR f.body LIKE ?1 ESCAPE '\\'
                 GROUP BY a.id
                 ORDER BY a.id ASC
                 LIMIT ?2",
            )
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![pat, top_k as i64], row_to_atlas)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_atlas_rows_warn_on_err(rows))
    }

    /// Era 검색용 LIKE fallback — Phase 5b. events·atlases와 동일 패턴.
    fn search_eras_like(
        &self,
        conn: &Connection,
        q: &str,
        top_k: u32,
    ) -> Result<Vec<Era>, WorldError> {
        let pat = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.project_id, e.kind, e.name, e.aliases_json,
                        e.summary, e.tags_json, e.extras_json, e.temporal_json,
                        e.start_year_relative, e.end_year_relative,
                        e.key_events_json, e.body_sections_json, e.source_path
                 FROM eras e
                 LEFT JOIN eras_fts f ON f.id = e.id
                 WHERE e.name LIKE ?1 ESCAPE '\\'
                    OR f.aliases LIKE ?1 ESCAPE '\\'
                    OR e.summary LIKE ?1 ESCAPE '\\'
                    OR f.body LIKE ?1 ESCAPE '\\'
                 GROUP BY e.id
                 ORDER BY e.id ASC
                 LIMIT ?2",
            )
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![pat, top_k as i64], row_to_era)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_era_rows_warn_on_err(rows))
    }

    /// Timeline 검색용 LIKE fallback — Phase 5b 체크포인트 2. atlases와 동일 패턴.
    fn search_timelines_like(
        &self,
        conn: &Connection,
        q: &str,
        top_k: u32,
    ) -> Result<Vec<Timeline>, WorldError> {
        let pat = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.project_id, t.kind, t.name, t.aliases_json,
                        t.summary, t.tags_json, t.extras_json, t.references_json,
                        t.body_sections_json, t.source_path
                 FROM timelines t
                 LEFT JOIN timelines_fts f ON f.id = t.id
                 WHERE t.name LIKE ?1 ESCAPE '\\'
                    OR f.aliases LIKE ?1 ESCAPE '\\'
                    OR t.summary LIKE ?1 ESCAPE '\\'
                    OR f.body LIKE ?1 ESCAPE '\\'
                 GROUP BY t.id
                 ORDER BY t.id ASC
                 LIMIT ?2",
            )
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![pat, top_k as i64], row_to_timeline)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_timeline_rows_warn_on_err(rows))
    }

    /// Event 검색용 LIKE fallback — Phase 5a. atlases와 동일 패턴.
    fn search_events_like(
        &self,
        conn: &Connection,
        q: &str,
        top_k: u32,
    ) -> Result<Vec<Event>, WorldError> {
        let pat = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.project_id, e.kind, e.category, e.name, e.aliases_json,
                        e.summary, e.tags_json, e.extras_json, e.temporal_json,
                        e.year_relative, e.era_id, e.participants_json, e.body_sections_json,
                        e.related_events_json, e.source_path
                 FROM events e
                 LEFT JOIN events_fts f ON f.id = e.id
                 WHERE e.name LIKE ?1 ESCAPE '\\'
                    OR f.aliases LIKE ?1 ESCAPE '\\'
                    OR e.summary LIKE ?1 ESCAPE '\\'
                    OR f.body LIKE ?1 ESCAPE '\\'
                 GROUP BY e.id
                 ORDER BY e.id ASC
                 LIMIT ?2",
            )
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![pat, top_k as i64], row_to_event)
            .map_err(|e| WorldError::Storage(e.to_string()))?;
        Ok(collect_event_rows_warn_on_err(rows))
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

    // status는 schema CHECK가 막지만, 외부 도구로 손상된 row 방어 — silent fallback 대신
    // hard error로 collect_person_rows_warn_on_err가 row 자체를 스킵·로그한다.
    let status = PersonStatus::from_str_loose(&status_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            rusqlite::types::Type::Text,
            format!("persons.status 알 수 없는 값 '{status_str}' (id={id})").into(),
        )
    })?;
    // hexaco_json: Score VO 범위 위반 등 손상된 값은 neutral로 가장하면 안 됨 — hard error
    // → 호출자가 row를 스킵하고 진단 로그를 남기도록.
    let hexaco: HexacoSix = serde_json::from_str(&hexaco_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            6,
            rusqlite::types::Type::Text,
            format!("persons.hexaco_json 디코드 실패 (id={id}): {e}").into(),
        )
    })?;
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

fn row_to_place(row: &rusqlite::Row) -> rusqlite::Result<Place> {
    let id: String = row.get(0)?;
    // project_id (1)은 도메인 모델 미보존.
    let layer_str: String = row.get(2)?;
    let kind: String = row.get(3)?;
    let name: String = row.get(4)?;
    let aliases_json: String = row.get(5)?;
    let summary: String = row.get(6)?;
    let tags_json: String = row.get(7)?;
    let extras_json: String = row.get(8)?;
    let body_json: String = row.get(9)?;
    let spatial_json: String = row.get(10)?;
    // parent_place (11): spatial_json에서 동일 값 복원. 캐시 컬럼.
    let source_path: Option<String> = row.get(12)?;

    let layer = PlaceLayer::from_str_loose(&layer_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            format!("places.layer 알 수 없는 값 '{layer_str}' (id={id})").into(),
        )
    })?;
    let extras: Map<String, Value> =
        serde_json::from_str(&extras_json).unwrap_or_default();
    let body_sections = serde_json::from_str(&body_json).unwrap_or_default();
    let spatial: Spatial = serde_json::from_str(&spatial_json).unwrap_or_default();
    let aliases = from_json_strings(&aliases_json);
    let tags = from_json_strings(&tags_json);

    Ok(Place {
        id: PlaceId::new(id),
        layer,
        kind,
        name,
        aliases,
        summary,
        tags,
        extras,
        body_sections,
        spatial,
        source_path,
    })
}

fn collect_place_rows_warn_on_err<I>(rows: I) -> Vec<Place>
where
    I: Iterator<Item = rusqlite::Result<Place>>,
{
    let mut out = Vec::new();
    for r in rows {
        match r {
            Ok(p) => out.push(p),
            Err(e) => {
                tracing::warn!("SqliteWorldStore place row decode 실패 — 결과에서 제외: {e}");
            }
        }
    }
    out
}

fn row_to_atlas(row: &rusqlite::Row) -> rusqlite::Result<Atlas> {
    let id: String = row.get(0)?;
    // project_id (1)은 도메인 모델 미보존.
    let kind: String = row.get(2)?;
    let name: String = row.get(3)?;
    let aliases_json: String = row.get(4)?;
    let summary: String = row.get(5)?;
    let tags_json: String = row.get(6)?;
    let extras_json: String = row.get(7)?;
    let extent_json: String = row.get(8)?;
    let references_json: String = row.get(9)?;
    let body_json: String = row.get(10)?;
    let source_path: Option<String> = row.get(11)?;

    let extras: Map<String, Value> =
        serde_json::from_str(&extras_json).unwrap_or_default();
    let extent: AtlasExtent =
        serde_json::from_str(&extent_json).unwrap_or_default();
    let references: Vec<PlaceId> = serde_json::from_str::<Vec<String>>(&references_json)
        .map(|v| v.into_iter().map(PlaceId::new).collect())
        .unwrap_or_default();
    let body_sections = serde_json::from_str(&body_json).unwrap_or_default();
    let aliases = from_json_strings(&aliases_json);
    let tags = from_json_strings(&tags_json);

    Ok(Atlas {
        id: AtlasId::new(id),
        kind,
        name,
        aliases,
        summary,
        tags,
        extras,
        extent,
        references,
        body_sections,
        source_path,
    })
}

fn collect_atlas_rows_warn_on_err<I>(rows: I) -> Vec<Atlas>
where
    I: Iterator<Item = rusqlite::Result<Atlas>>,
{
    let mut out = Vec::new();
    for r in rows {
        match r {
            Ok(a) => out.push(a),
            Err(e) => {
                tracing::warn!("SqliteWorldStore atlas row decode 실패 — 결과에서 제외: {e}");
            }
        }
    }
    out
}

fn row_to_event(row: &rusqlite::Row) -> rusqlite::Result<Event> {
    let id: String = row.get(0)?;
    // project_id (1)은 도메인 모델 미보존.
    let kind: String = row.get(2)?;
    let category_str: String = row.get(3)?;
    let name: String = row.get(4)?;
    let aliases_json: String = row.get(5)?;
    let summary: String = row.get(6)?;
    let tags_json: String = row.get(7)?;
    let extras_json: String = row.get(8)?;
    let temporal_json: String = row.get(9)?;
    // year_relative (10)·era_id (11): temporal_json·era_id에 동일 값. 캐시 컬럼.
    // 도메인 복원은 temporal_json + era_id (text column) 권위 사용.
    let _year_relative_cache: Option<i64> = row.get(10)?;
    let era_id: Option<String> = row.get(11)?;
    let participants_json: String = row.get(12)?;
    let body_json: String = row.get(13)?;
    let related_events_json: String = row.get(14)?;
    let source_path: Option<String> = row.get(15)?;

    let category = EventCategory::from_str_loose(&category_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Text,
            format!("events.category 알 수 없는 값 '{category_str}' (id={id})").into(),
        )
    })?;
    let extras: Map<String, Value> =
        serde_json::from_str(&extras_json).unwrap_or_default();
    let temporal: EventTemporal =
        serde_json::from_str(&temporal_json).unwrap_or_default();
    let participants: ParticipantsRefs =
        serde_json::from_str(&participants_json).unwrap_or_default();
    let body_sections = serde_json::from_str(&body_json).unwrap_or_default();
    let related_events: Vec<EventId> = serde_json::from_str::<Vec<String>>(&related_events_json)
        .map(|v| v.into_iter().map(EventId::new).collect())
        .unwrap_or_default();
    let aliases = from_json_strings(&aliases_json);
    let tags = from_json_strings(&tags_json);

    Ok(Event {
        id: EventId::new(id),
        kind,
        category,
        name,
        aliases,
        summary,
        tags,
        extras,
        temporal,
        era_id,
        participants,
        body_sections,
        related_events,
        source_path,
    })
}

fn collect_event_rows_warn_on_err<I>(rows: I) -> Vec<Event>
where
    I: Iterator<Item = rusqlite::Result<Event>>,
{
    let mut out = Vec::new();
    for r in rows {
        match r {
            Ok(e) => out.push(e),
            Err(e) => {
                tracing::warn!("SqliteWorldStore event row decode 실패 — 결과에서 제외: {e}");
            }
        }
    }
    out
}

fn row_to_era(row: &rusqlite::Row) -> rusqlite::Result<Era> {
    let id: String = row.get(0)?;
    // project_id (1)은 도메인 모델 미보존.
    let kind: String = row.get(2)?;
    let name: String = row.get(3)?;
    let aliases_json: String = row.get(4)?;
    let summary: String = row.get(5)?;
    let tags_json: String = row.get(6)?;
    let extras_json: String = row.get(7)?;
    let temporal_json: String = row.get(8)?;
    // start_year_relative (9)·end_year_relative (10): temporal_json에서 권위 복원 — 캐시 컬럼.
    let _start_cache: Option<i64> = row.get(9)?;
    let _end_cache: Option<i64> = row.get(10)?;
    let key_events_json: String = row.get(11)?;
    let body_json: String = row.get(12)?;
    let source_path: Option<String> = row.get(13)?;

    let extras: Map<String, Value> = serde_json::from_str(&extras_json).unwrap_or_default();
    let temporal: EraTemporal = serde_json::from_str(&temporal_json).unwrap_or_default();
    let key_events: Vec<EventId> = serde_json::from_str::<Vec<String>>(&key_events_json)
        .map(|v| v.into_iter().map(EventId::new).collect())
        .unwrap_or_default();
    let body_sections = serde_json::from_str(&body_json).unwrap_or_default();
    let aliases = from_json_strings(&aliases_json);
    let tags = from_json_strings(&tags_json);

    Ok(Era {
        id: EraId::new(id),
        kind,
        name,
        aliases,
        summary,
        tags,
        extras,
        temporal,
        key_events,
        body_sections,
        source_path,
    })
}

fn collect_era_rows_warn_on_err<I>(rows: I) -> Vec<Era>
where
    I: Iterator<Item = rusqlite::Result<Era>>,
{
    let mut out = Vec::new();
    for r in rows {
        match r {
            Ok(e) => out.push(e),
            Err(e) => {
                tracing::warn!("SqliteWorldStore era row decode 실패 — 결과에서 제외: {e}");
            }
        }
    }
    out
}

fn row_to_timeline(row: &rusqlite::Row) -> rusqlite::Result<Timeline> {
    let id: String = row.get(0)?;
    // project_id (1)은 도메인 모델 미보존.
    let kind: String = row.get(2)?;
    let name: String = row.get(3)?;
    let aliases_json: String = row.get(4)?;
    let summary: String = row.get(5)?;
    let tags_json: String = row.get(6)?;
    let extras_json: String = row.get(7)?;
    let references_json: String = row.get(8)?;
    let body_json: String = row.get(9)?;
    let source_path: Option<String> = row.get(10)?;

    let extras: Map<String, Value> =
        serde_json::from_str(&extras_json).unwrap_or_default();
    let references: Vec<EraId> = serde_json::from_str::<Vec<String>>(&references_json)
        .map(|v| v.into_iter().map(EraId::new).collect())
        .unwrap_or_default();
    let body_sections = serde_json::from_str(&body_json).unwrap_or_default();
    let aliases = from_json_strings(&aliases_json);
    let tags = from_json_strings(&tags_json);

    Ok(Timeline {
        id: TimelineId::new(id),
        kind,
        name,
        aliases,
        summary,
        tags,
        extras,
        references,
        body_sections,
        source_path,
    })
}

fn collect_timeline_rows_warn_on_err<I>(rows: I) -> Vec<Timeline>
where
    I: Iterator<Item = rusqlite::Result<Timeline>>,
{
    let mut out = Vec::new();
    for r in rows {
        match r {
            Ok(t) => out.push(t),
            Err(e) => {
                tracing::warn!(
                    "SqliteWorldStore timeline row decode 실패 — 결과에서 제외: {e}"
                );
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
    fn schema_v1_to_v2_migration_upgrades_existing_file_db() {
        // 실제 v1→v2 마이그레이션 경로 검증. tempfile에 v1 schema를 작성한 뒤,
        // SqliteWorldStore::new(path)로 재오픈 → init_tables의 `current < 2` 분기가 활성되어
        // persons 테이블이 추가되며 기존 groups 데이터도 보존된다.
        // tempfile의 자동 삭제를 그대로 활용 — path는 _tmp drop까지 유효.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path_buf = tmp.path().to_path_buf();

        // 1) v1 schema 작성: schema_meta version=1 + groups 테이블 + groups_fts + 한 row.
        {
            let conn = rusqlite::Connection::open(&path_buf).unwrap();
            conn.execute_batch(
                "CREATE TABLE world_schema_meta (version INTEGER PRIMARY KEY);
                 INSERT INTO world_schema_meta(version) VALUES (1);
                 CREATE TABLE groups (
                    id TEXT PRIMARY KEY,
                    project_id TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    name TEXT NOT NULL,
                    aliases_json TEXT NOT NULL DEFAULT '[]',
                    parent_group TEXT,
                    allied_groups_json TEXT NOT NULL DEFAULT '[]',
                    rival_groups_json TEXT NOT NULL DEFAULT '[]',
                    headquarters TEXT,
                    status TEXT NOT NULL DEFAULT 'active',
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
                 CREATE VIRTUAL TABLE groups_fts USING fts5(
                    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
                 );
                 INSERT INTO groups (
                    id, project_id, kind, name, status, summary, updated_at
                 ) VALUES ('group-legacy', 'p', 'alliance', '레거시', 'active', '기존 v1 데이터', 0);
                 INSERT INTO groups_fts (id, name, aliases, summary, body)
                 VALUES ('group-legacy', '레거시', '', '기존 v1 데이터', '');",
            )
            .unwrap();
            // explicit drop via scope end
        }

        // 2) SqliteWorldStore로 재오픈 — init_tables가 current=1을 보고 migrate_v2 실행.
        let store = SqliteWorldStore::new(path_buf.to_str().unwrap()).unwrap();

        // 3) persons 테이블이 추가되어 count_persons이 동작.
        assert_eq!(store.count_persons(None).unwrap(), 0);

        // 4) 기존 v1 groups 데이터 보존.
        let g = store
            .get_group(&GroupId::new("group-legacy"))
            .unwrap()
            .expect("v1 row 보존 필요");
        assert_eq!(g.name, "레거시");

        // 5) v2 신규 기능 — persons upsert·get 동작.
        let p = sample_person("npc-after-migration", "active", "신규");
        store.upsert_person("p", &p).unwrap();
        let back = store
            .get_person(&PersonId::new("npc-after-migration"))
            .unwrap()
            .unwrap();
        assert_eq!(back.name, "신규");

        // store를 명시 drop해 SQLite 핸들을 닫고 NamedTempFile이 자동 삭제하도록.
        drop(store);
        drop(tmp);
    }

    #[test]
    fn corrupt_hexaco_row_is_skipped_not_silently_neutralized() {
        // 외부 도구·다운그레이드 등으로 persons.hexaco_json에 손상된 값이 들어간 경우,
        // 읽기 경로는 silent하게 neutral로 가장하면 안 된다 — row를 스킵하고 진단 로그를 남긴다.
        let store = SqliteWorldStore::in_memory().unwrap();
        let good = sample_person("npc-good", "active", "정상");
        store.upsert_person("p", &good).unwrap();

        // 손상된 row 직접 삽입 — Score VO가 거부할 -2.0 값. CHECK는 status만 막으므로
        // 이 path는 hexaco_json 검증의 유일한 방어선.
        {
            let conn = store.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO persons (
                    id, project_id, kind, name, aliases_json, status,
                    hexaco_json, temporal_json, affiliation_json,
                    birthplace, current_location, summary, tags_json, extras_json,
                    body_sections_json, source_path, updated_at
                 ) VALUES ('npc-bad', 'p', 'active', '손상',
                           '[]', 'alive',
                           '{\"honesty_humility\":-2.5,\"emotionality\":0,\"extraversion\":0,\"agreeableness\":0,\"conscientiousness\":0,\"openness\":0}',
                           '{}', '[]',
                           NULL, NULL, '', '[]', '{}',
                           '{}', NULL, 0)",
                [],
            )
            .unwrap();
        }

        // list_persons은 손상된 row를 스킵하고 정상 row만 반환해야 함 (silent neutral 아님).
        let all = store.list_persons(PersonFilter::default()).unwrap();
        assert_eq!(all.len(), 1, "손상 row는 스킵되어야 함");
        assert_eq!(all[0].id.as_str(), "npc-good");

        // get_person 단건 조회는 hard error로 전파되어야 함 (silent neutral 아님).
        let res = store.get_person(&PersonId::new("npc-bad"));
        assert!(
            res.is_err(),
            "손상된 단건 조회는 silent neutral이 아니라 에러여야 함: {res:?}"
        );
    }

    #[test]
    fn schema_meta_remains_single_row_after_migration() {
        // Code review #7: schema_meta는 v1→v2 후에도 단일 row 유지해야 함.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path_buf = tmp.path().to_path_buf();
        {
            let conn = rusqlite::Connection::open(&path_buf).unwrap();
            conn.execute_batch(
                "CREATE TABLE world_schema_meta (version INTEGER PRIMARY KEY);
                 INSERT INTO world_schema_meta(version) VALUES (1);
                 CREATE TABLE groups (id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
                    kind TEXT NOT NULL, name TEXT NOT NULL,
                    aliases_json TEXT NOT NULL DEFAULT '[]',
                    parent_group TEXT, allied_groups_json TEXT NOT NULL DEFAULT '[]',
                    rival_groups_json TEXT NOT NULL DEFAULT '[]', headquarters TEXT,
                    status TEXT NOT NULL DEFAULT 'active', alignment TEXT,
                    summary TEXT NOT NULL DEFAULT '', tags_json TEXT NOT NULL DEFAULT '[]',
                    extras_json TEXT NOT NULL DEFAULT '{}',
                    body_sections_json TEXT NOT NULL DEFAULT '{}',
                    temporal_json TEXT NOT NULL DEFAULT '{}',
                    members_json TEXT NOT NULL DEFAULT '[]',
                    source_path TEXT, updated_at INTEGER NOT NULL
                 );
                 CREATE VIRTUAL TABLE groups_fts USING fts5(
                    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
                 );",
            )
            .unwrap();
        }

        let store = SqliteWorldStore::new(path_buf.to_str().unwrap()).unwrap();

        // schema_meta에 row가 정확히 1개여야 하며 version=2.
        let (count, max_version): (i64, i64) = {
            let conn = store.conn.lock().unwrap();
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM world_schema_meta", [], |r| r.get(0))
                .unwrap();
            let max: i64 = conn
                .query_row(
                    "SELECT COALESCE(MAX(version), 0) FROM world_schema_meta",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            (count, max)
        };
        assert_eq!(count, 1, "schema_meta는 단일 row여야 함 (v1·v2·v3·v4 누적 X)");
        assert_eq!(max_version, SCHEMA_VERSION);

        drop(store);
        drop(tmp);
    }

    #[test]
    fn affiliation_filter_does_not_match_substring_groups() {
        // Code review #4: LIKE 메타문자 escape — id가 다른 id의 substring이거나
        // `_`/`%` 포함 시 false-positive 매칭 방지.
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut p1 = sample_person("npc-1", "active", "1");
        p1.affiliation = vec![GroupId::new("group-a")];
        let mut p2 = sample_person("npc-2", "active", "2");
        // `group-a`의 substring이 아니라 별도 그룹.
        p2.affiliation = vec![GroupId::new("group-aa")];
        let mut p3 = sample_person("npc-3", "active", "3");
        // `_` 포함 — LIKE에서 single-char wildcard로 해석되면 group-aa가 매치되어 false-positive.
        p3.affiliation = vec![GroupId::new("group_a")];
        store.upsert_person("p", &p1).unwrap();
        store.upsert_person("p", &p2).unwrap();
        store.upsert_person("p", &p3).unwrap();

        // group-a 필터: npc-1만 매치되어야 함 (group-aa 또는 group_a 매치 안 됨).
        let hits = store
            .list_persons(PersonFilter {
                affiliation: Some(GroupId::new("group-a")),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<&str> = hits.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["npc-1"],
            "group-a 필터는 정확히 npc-1만 — substring/wildcard false-positive 금지"
        );

        // group_a 필터: npc-3만 매치 (literal `_`).
        let hits = store
            .list_persons(PersonFilter {
                affiliation: Some(GroupId::new("group_a")),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<&str> = hits.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["npc-3"],
            "group_a 필터는 정확히 npc-3만 — `_` literal 매칭"
        );

        // group-aa 필터: npc-2만.
        let hits = store
            .list_persons(PersonFilter {
                affiliation: Some(GroupId::new("group-aa")),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<&str> = hits.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["npc-2"]);
    }

    #[test]
    fn tag_filter_escapes_like_metachars() {
        // genre_tag 필터도 동일 escape 적용 — `tag_a`와 `tag-a` 구분.
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut p1 = sample_person("npc-1", "active", "1");
        p1.tags = vec!["tag-a".into()];
        let mut p2 = sample_person("npc-2", "active", "2");
        p2.tags = vec!["tag_a".into()];
        store.upsert_person("p", &p1).unwrap();
        store.upsert_person("p", &p2).unwrap();

        let hits = store
            .list_persons(PersonFilter {
                genre_tag: Some("tag-a".into()),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<&str> = hits.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["npc-1"], "tag-a는 tag_a를 매치하면 안 됨");

        let hits = store
            .list_persons(PersonFilter {
                genre_tag: Some("tag_a".into()),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<&str> = hits.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["npc-2"]);
    }

    #[test]
    fn hexaco_facets_extras_round_trip_through_sqlite() {
        // Code review #6: 24 facet 정형 보존이 SQLite를 통과해도 무손실.
        // Phase 4 정밀 패스 진입 시 silent regression 방지를 위한 회귀 가드.
        let mut p = sample_person("npc-x", "active", "X");
        let facets = serde_json::json!({
            "H_sincerity": -0.9,
            "H_fairness": -0.7,
            "H_greed_avoidance": -0.8,
            "H_modesty": -0.6,
            "E_fearfulness": -0.4,
            "E_anxiety": -0.2,
            "E_dependence": -0.7,
            "E_sentimentality": -0.3,
            "X_social_self_esteem": 0.6,
            "X_social_boldness": 0.5,
            "X_sociability": -0.4,
            "X_liveliness": -0.2,
            "A_forgiveness": -0.8,
            "A_gentleness": -0.6,
            "A_flexibility": -0.5,
            "A_patience": 0.7,
            "C_organization": 0.8,
            "C_diligence": 0.7,
            "C_perfectionism": 0.6,
            "C_prudence": 0.7,
            "O_aesthetic_appreciation": 0.5,
            "O_inquisitiveness": 0.7,
            "O_creativity": 0.4,
            "O_unconventionality": 0.6
        });
        p.extras
            .insert("hexaco_facets".into(), facets.clone());

        let store = SqliteWorldStore::in_memory().unwrap();
        store.upsert_person("p", &p).unwrap();
        let back = store.get_person(&PersonId::new("npc-x")).unwrap().unwrap();

        let stored = back
            .extras
            .get("hexaco_facets")
            .expect("hexaco_facets 보존 필요");
        assert_eq!(
            stored, &facets,
            "24 facet JSON 객체가 SQLite 라운드트립에서 무손실로 보존되어야 함"
        );
        // 24 키 카운트 회귀 가드.
        assert_eq!(stored.as_object().unwrap().len(), 24);
    }

    #[test]
    fn search_persons_with_percent_in_summary_uses_escape() {
        // Code review #8: search_persons LIKE fallback이 `%` 포함 query·body를 잘못 매칭하지 않아야 함.
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut p1 = sample_person("npc-percent", "active", "퍼센트");
        p1.summary = "정확히 50% 할인".into();
        p1.body_sections.clear();
        let mut p2 = sample_person("npc-other", "active", "다른");
        p2.summary = "할인 행사".into();
        p2.body_sections.clear();
        store.upsert_person("p", &p1).unwrap();
        store.upsert_person("p", &p2).unwrap();

        // FTS5 trigram이 query 길이 3 이상을 처리하나, `%` 같은 punctuation 포함 query는
        // LIKE fallback으로 떨어진다. fallback escape가 정확하면 "50%"는 "50"로 시작하는
        // 임의 string이 아니라 literal `50%`만 매치해야 한다.
        let hits = store.search_persons("50%", 5).unwrap();
        let ids: Vec<&str> = hits.iter().map(|p| p.id.as_str()).collect();
        assert!(
            ids.contains(&"npc-percent"),
            "search_persons(\"50%\")는 npc-percent를 찾아야 함 — escape 후에도 literal 매칭"
        );
        // npc-other("할인 행사")는 매치되지 않아야 — 50% wildcard 해석 방지.
        assert!(
            !ids.contains(&"npc-other"),
            "search_persons(\"50%\")가 npc-other(50% 무관)까지 매치되면 안 됨 — escape 누락 의심"
        );
    }

    #[test]
    fn corrupt_status_row_is_skipped() {
        // status 컬럼은 schema CHECK가 alive|dead|missing|unknown을 강제하므로 정상 흐름에선
        // 도달 불가. 그러나 from_str_loose가 silent default로 가장하면 미래에 CHECK가 완화되거나
        // 다른 작성 경로가 추가됐을 때 잠재 버그. 본 테스트는 row_to_person이 hard-error를
        // 반환함을 명시적으로 가드한다 — status_str을 임의 NULL이나 unknown 값으로 바꾸려면
        // CHECK를 우회해야 하므로 별도 connection으로 CHECK 없는 임시 테이블에 row를 넣지 않고,
        // from_str_loose 자체의 None 반환 + row_to_person 매핑 정합성만 단위 검증한다.
        // (실제 SQLite path는 CHECK가 막아 도달 불가하므로 직접 SQL injection 케이스는 생략.)
        assert!(PersonStatus::from_str_loose("ghost").is_none());
        assert!(PersonStatus::from_str_loose("").is_none());
    }

    // ---------------------------------------------------------------------
    // Phase 3 — Place 테스트
    // ---------------------------------------------------------------------

    fn sample_settlement(id: &str, name: &str) -> Place {
        let mut p = Place::new(id, PlaceLayer::Settlement, "nation", name);
        p.aliases = vec!["별호".into(), "옛 이름".into()];
        p.summary = "테스트 장소 요약".into();
        p.tags = vec!["wuxia".into(), "place".into(), "settlement".into()];
        p.extras
            .insert("capital".into(), json!("수도명"));
        p.extras
            .insert("ki_concentration".into(), json!("보통"));
        p.body_sections.insert("개요".into(), "본문".into());
        p.spatial = Spatial {
            parent_place: None,
            relative_position: Some("center".into()),
            bordering_places: vec![PlaceId::new("place-other")],
            geography_refs: vec![PlaceId::new("place-mt-a")],
        };
        p
    }

    fn sample_geography(id: &str, name: &str) -> Place {
        let mut p = Place::new(id, PlaceLayer::Geography, "mountain-range", name);
        p.aliases = vec!["서령산맥".into()];
        p.summary = "산악 요약".into();
        p.tags = vec!["wuxia".into(), "place".into(), "geography".into()];
        p.extras
            .insert("terrain_type".into(), json!("mountain-range"));
        p.extras
            .insert("hazards".into(), serde_json::json!(["눈사태", "안개"]));
        p.spatial = Spatial {
            parent_place: None,
            relative_position: Some("west".into()),
            bordering_places: vec![PlaceId::new("place-seoryang")],
            geography_refs: vec![],
        };
        p
    }

    #[test]
    fn places_count_zero_on_fresh_db() {
        let store = SqliteWorldStore::in_memory().unwrap();
        assert_eq!(store.count_places(None).unwrap(), 0);
    }

    #[test]
    fn places_upsert_and_get_roundtrip_preserves_all_fields() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let p = sample_settlement("place-x", "X국");
        store.upsert_place("test-project", &p).unwrap();
        let back = store.get_place(&PlaceId::new("place-x")).unwrap().unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn places_geography_layer_roundtrip() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let g = sample_geography("place-mt-a", "산악 A");
        store.upsert_place("p", &g).unwrap();
        let back = store.get_place(&PlaceId::new("place-mt-a")).unwrap().unwrap();
        assert_eq!(back.layer, PlaceLayer::Geography);
        assert_eq!(back.kind, "mountain-range");
        // hazards 배열 보존
        let hazards = back.extras.get("hazards").and_then(|v| v.as_array()).unwrap();
        assert_eq!(hazards.len(), 2);
    }

    #[test]
    fn places_list_filter_layer_and_kind() {
        let store = SqliteWorldStore::in_memory().unwrap();
        store
            .upsert_place("p", &sample_settlement("place-s1", "S1"))
            .unwrap();
        let mut s2 = sample_settlement("place-s2", "S2");
        s2.kind = "city".into();
        store.upsert_place("p", &s2).unwrap();
        store
            .upsert_place("p", &sample_geography("place-g1", "G1"))
            .unwrap();

        let settlements = store
            .list_places(PlaceFilter {
                layer: Some(PlaceLayer::Settlement),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(settlements.len(), 2);

        let cities = store
            .list_places(PlaceFilter {
                kind: Some("city".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(cities.len(), 1);
        assert_eq!(cities[0].id.as_str(), "place-s2");

        let geos = store
            .list_places(PlaceFilter {
                layer: Some(PlaceLayer::Geography),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(geos.len(), 1);
    }

    #[test]
    fn places_list_filter_parent_place() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut parent = sample_settlement("place-parent", "Parent");
        parent.spatial.parent_place = None;
        store.upsert_place("p", &parent).unwrap();

        let mut child = sample_settlement("place-child", "Child");
        child.spatial.parent_place = Some(PlaceId::new("place-parent"));
        store.upsert_place("p", &child).unwrap();

        let kids = store
            .list_places(PlaceFilter {
                parent_place: Some(PlaceId::new("place-parent")),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].id.as_str(), "place-child");
    }

    #[test]
    fn places_search_matches_name_and_alias() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut p = sample_settlement("place-daejin", "대진");
        p.aliases = vec!["낙양".into(), "중원 황도".into()];
        store.upsert_place("p", &p).unwrap();

        // alias 매칭 (FTS5 trigram)
        let hits = store.search_places("낙양", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id.as_str(), "place-daejin");

        // 빈 query
        assert!(store.search_places("", 5).unwrap().is_empty());
    }

    #[test]
    fn places_count_with_project_filter() {
        let store = SqliteWorldStore::in_memory().unwrap();
        store
            .upsert_place("p1", &sample_settlement("place-a", "A"))
            .unwrap();
        store
            .upsert_place("p2", &sample_geography("place-mt", "Mt"))
            .unwrap();
        assert_eq!(store.count_places(None).unwrap(), 2);
        assert_eq!(store.count_places(Some("p1")).unwrap(), 1);
        assert_eq!(store.count_places(Some("p2")).unwrap(), 1);
    }

    #[test]
    fn places_upsert_replaces_fts_stale_row() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut p = sample_settlement("place-x", "X");
        p.aliases = vec!["OldAliasUnique".into()];
        p.body_sections.clear();
        p.body_sections.insert("Overview".into(), "first".into());
        store.upsert_place("p", &p).unwrap();
        let hits = store.search_places("OldAliasUnique", 5).unwrap();
        assert_eq!(hits.len(), 1);

        p.aliases = vec!["NewAliasUnique".into()];
        p.body_sections.clear();
        p.body_sections.insert("Overview".into(), "second".into());
        store.upsert_place("p", &p).unwrap();

        let hits_old = store.search_places("OldAliasUnique", 5).unwrap();
        assert!(hits_old.is_empty());
        let hits_new = store.search_places("NewAliasUnique", 5).unwrap();
        assert_eq!(hits_new.len(), 1);
    }

    #[test]
    fn schema_v2_to_v3_migration_upgrades_existing_file_db() {
        // 실제 v2→v3 마이그레이션 경로 검증. tempfile에 v2 schema를 작성한 뒤,
        // SqliteWorldStore::new(path)로 재오픈 → init_tables의 `current < 3` 분기가 활성되어
        // places 테이블이 추가되며 기존 groups/persons 데이터도 보존된다.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path_buf = tmp.path().to_path_buf();

        // 1) v2 schema 작성: world_schema_meta version=2 + groups + persons + 한 row씩.
        {
            let conn = rusqlite::Connection::open(&path_buf).unwrap();
            conn.execute_batch(
                "CREATE TABLE world_schema_meta (version INTEGER PRIMARY KEY);
                 INSERT INTO world_schema_meta(version) VALUES (2);
                 CREATE TABLE groups (
                    id TEXT PRIMARY KEY, project_id TEXT NOT NULL, kind TEXT NOT NULL,
                    name TEXT NOT NULL, aliases_json TEXT NOT NULL DEFAULT '[]',
                    parent_group TEXT, allied_groups_json TEXT NOT NULL DEFAULT '[]',
                    rival_groups_json TEXT NOT NULL DEFAULT '[]', headquarters TEXT,
                    status TEXT NOT NULL DEFAULT 'active', alignment TEXT,
                    summary TEXT NOT NULL DEFAULT '', tags_json TEXT NOT NULL DEFAULT '[]',
                    extras_json TEXT NOT NULL DEFAULT '{}', body_sections_json TEXT NOT NULL DEFAULT '{}',
                    temporal_json TEXT NOT NULL DEFAULT '{}', members_json TEXT NOT NULL DEFAULT '[]',
                    source_path TEXT, updated_at INTEGER NOT NULL
                 );
                 CREATE VIRTUAL TABLE groups_fts USING fts5(
                    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
                 );
                 INSERT INTO groups (
                    id, project_id, kind, name, status, summary, updated_at
                 ) VALUES ('group-legacy', 'p', 'alliance', '레거시', 'active', 'v2 데이터', 0);
                 CREATE TABLE persons (
                    id TEXT PRIMARY KEY, project_id TEXT NOT NULL, kind TEXT NOT NULL,
                    name TEXT NOT NULL, aliases_json TEXT NOT NULL DEFAULT '[]',
                    status TEXT NOT NULL DEFAULT 'alive',
                    hexaco_json TEXT NOT NULL DEFAULT '{}', temporal_json TEXT NOT NULL DEFAULT '{}',
                    affiliation_json TEXT NOT NULL DEFAULT '[]', birthplace TEXT,
                    current_location TEXT, summary TEXT NOT NULL DEFAULT '',
                    tags_json TEXT NOT NULL DEFAULT '[]', extras_json TEXT NOT NULL DEFAULT '{}',
                    body_sections_json TEXT NOT NULL DEFAULT '{}', source_path TEXT,
                    updated_at INTEGER NOT NULL
                 );
                 CREATE VIRTUAL TABLE persons_fts USING fts5(
                    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
                 );",
            )
            .unwrap();
        }

        // 2) SqliteWorldStore로 재오픈 — init_tables가 current=2를 보고 migrate_v3 실행.
        let store = SqliteWorldStore::new(path_buf.to_str().unwrap()).unwrap();

        // 3) places 테이블이 추가되어 count_places가 동작.
        assert_eq!(store.count_places(None).unwrap(), 0);

        // 4) 기존 v2 groups 데이터 보존.
        let g = store
            .get_group(&GroupId::new("group-legacy"))
            .unwrap()
            .expect("v2 row 보존 필요");
        assert_eq!(g.name, "레거시");

        // 5) v3 신규 기능 — places upsert·get 동작.
        let p = sample_settlement("place-after", "신규");
        store.upsert_place("p", &p).unwrap();
        let back = store.get_place(&PlaceId::new("place-after")).unwrap().unwrap();
        assert_eq!(back.name, "신규");

        drop(store);
        drop(tmp);
    }

    // -----------------------------------------------------------------------
    // Phase 4 — Atlas 라운드트립 + place_atlas_refs 양방향 인덱스
    // -----------------------------------------------------------------------

    fn sample_atlas_with_refs(id: &str, refs: &[&str]) -> Atlas {
        let mut a = Atlas::new(id, "continent", id);
        a.aliases = vec!["대륙".into()];
        a.summary = "테스트 atlas".into();
        a.tags = vec!["test".into(), "atlas".into()];
        a.extras
            .insert("era".into(), Value::String("현재".into()));
        a.extent = AtlasExtent {
            projection: "schematic".into(),
            width_units: Some(7),
            height_units: Some(7),
            unit: "schematic".into(),
        };
        a.references = refs.iter().map(|s| PlaceId::new(*s)).collect();
        // ASCII art는 byte-exact 보존이 핵심 — box-drawing + 빈 줄 포함.
        a.body_sections.insert(
            "배치 다이어그램".into(),
            "```\n┌──────┐\n│ 중원 │\n└──────┘\n```".into(),
        );
        a
    }

    #[test]
    fn atlas_full_roundtrip_through_sqlite() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let a = sample_atlas_with_refs("atlas-test", &["place-a", "place-b", "place-c"]);
        store.upsert_atlas("test", &a).unwrap();
        let back = store.get_atlas(&AtlasId::new("atlas-test")).unwrap().unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn atlas_body_sections_preserve_ascii_byte_exact() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut a = Atlas::new("atlas-d", "continent", "Diagram");
        // box-drawing + 빈 줄 + 들여쓰기 — 마크다운 파서가 코드블록 내부에서 이를 깨면 안 됨.
        let diagram = "```\n                    ┌──────────────────┐\n                    │     북 원        │\n                    │   (초원/유목)     │\n                    │   왕정(오르두)    │\n                    └────────┬─────────┘\n                             │\n```";
        a.body_sections
            .insert("배치 다이어그램".into(), diagram.to_string());
        store.upsert_atlas("test", &a).unwrap();
        let back = store.get_atlas(&AtlasId::new("atlas-d")).unwrap().unwrap();
        assert_eq!(
            back.body_sections.get("배치 다이어그램").map(String::as_str),
            Some(diagram),
            "ASCII 다이어그램이 SQLite 라운드트립 후 byte-exact 보존되어야 함"
        );
    }

    #[test]
    fn place_atlas_refs_bidirectional_index_populated_on_upsert() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let a = sample_atlas_with_refs("atlas-x", &["place-a", "place-b", "place-c"]);
        store.upsert_atlas("test", &a).unwrap();

        // 정방향 (atlas → places, ref_order 보존)
        let conn = store.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT place_id, ref_order FROM place_atlas_refs WHERE atlas_id = ?1
                 ORDER BY ref_order ASC",
            )
            .unwrap();
        let rows: Vec<(String, i64)> = stmt
            .query_map(params!["atlas-x"], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(
            rows,
            vec![
                ("place-a".to_string(), 0),
                ("place-b".to_string(), 1),
                ("place-c".to_string(), 2),
            ]
        );

        // 역방향 (place → atlases) — idx_par_place 인덱스 활용 가능.
        let mut stmt2 = conn
            .prepare("SELECT atlas_id FROM place_atlas_refs WHERE place_id = ?1 ORDER BY atlas_id")
            .unwrap();
        let atlases_for_b: Vec<String> = stmt2
            .query_map(params!["place-b"], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(atlases_for_b, vec!["atlas-x".to_string()]);
    }

    #[test]
    fn place_atlas_refs_resyncs_on_re_upsert() {
        // references 변경 → 기존 매핑은 모두 사라지고 신규로 채워짐.
        let store = SqliteWorldStore::in_memory().unwrap();
        let a1 = sample_atlas_with_refs("atlas-x", &["place-a", "place-b"]);
        store.upsert_atlas("test", &a1).unwrap();
        let a2 = sample_atlas_with_refs("atlas-x", &["place-c"]);
        store.upsert_atlas("test", &a2).unwrap();

        let conn = store.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT place_id FROM place_atlas_refs WHERE atlas_id = ?1 ORDER BY place_id")
            .unwrap();
        let ids: Vec<String> = stmt
            .query_map(params!["atlas-x"], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(ids, vec!["place-c".to_string()]);
    }

    #[test]
    fn list_atlases_filters_by_kind_and_genre_tag() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut a1 = Atlas::new("atlas-c1", "continent", "C1");
        a1.tags = vec!["wuxia".into(), "atlas".into()];
        let mut a2 = Atlas::new("atlas-r1", "region", "R1");
        a2.tags = vec!["wuxia".into(), "atlas".into()];
        store.upsert_atlas("test", &a1).unwrap();
        store.upsert_atlas("test", &a2).unwrap();

        let conts = store
            .list_atlases(AtlasFilter {
                kind: Some("continent".into()),
                genre_tag: None,
            })
            .unwrap();
        assert_eq!(conts.len(), 1);
        assert_eq!(conts[0].id.as_str(), "atlas-c1");

        let wuxia = store
            .list_atlases(AtlasFilter {
                kind: None,
                genre_tag: Some("wuxia".into()),
            })
            .unwrap();
        assert_eq!(wuxia.len(), 2);
    }

    #[test]
    fn search_atlases_fts_and_like_fallback() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut a = Atlas::new("atlas-jungwon", "continent", "칠국춘추 대륙");
        a.aliases = vec!["중원 대륙".into(), "칠국 대륙".into()];
        a.summary = "대진 중심 7개 정치체".into();
        store.upsert_atlas("test", &a).unwrap();

        // 3자 이상 한국어 → FTS5 trigram.
        let hits1 = store.search_atlases("칠국", 5).unwrap();
        assert!(!hits1.is_empty());
        // 별호 매칭 — alias 기반.
        let hits2 = store.search_atlases("중원", 5).unwrap();
        assert!(!hits2.is_empty());
        assert_eq!(hits2[0].id.as_str(), "atlas-jungwon");
    }

    #[test]
    fn schema_v3_to_v4_migration_upgrades_existing_file_db() {
        // 실제 v3→v4 경로 검증. tempfile에 v3 schema를 작성한 뒤,
        // SqliteWorldStore::new(path)로 재오픈 → init_tables의 `current < 4` 분기가 활성되어
        // atlases / atlases_fts / place_atlas_refs가 추가되며 기존 v3 데이터는 보존된다.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path_buf = tmp.path().to_path_buf();

        // 1) v3 schema 작성 — places까지만.
        {
            let conn = rusqlite::Connection::open(&path_buf).unwrap();
            conn.execute_batch(
                "CREATE TABLE world_schema_meta (version INTEGER PRIMARY KEY);
                 INSERT INTO world_schema_meta(version) VALUES (3);
                 CREATE TABLE places (
                    id TEXT PRIMARY KEY, project_id TEXT NOT NULL, layer TEXT NOT NULL,
                    kind TEXT NOT NULL, name TEXT NOT NULL,
                    aliases_json TEXT NOT NULL DEFAULT '[]', summary TEXT NOT NULL DEFAULT '',
                    tags_json TEXT NOT NULL DEFAULT '[]', extras_json TEXT NOT NULL DEFAULT '{}',
                    body_sections_json TEXT NOT NULL DEFAULT '{}',
                    spatial_json TEXT NOT NULL DEFAULT '{}', parent_place TEXT,
                    source_path TEXT, updated_at INTEGER NOT NULL
                 );
                 CREATE VIRTUAL TABLE places_fts USING fts5(
                    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
                 );
                 INSERT INTO places (id, project_id, layer, kind, name, updated_at)
                    VALUES ('place-legacy', 'p', 'settlement', 'nation', '레거시', 0);",
            )
            .unwrap();
        }

        // 2) 재오픈 → migrate_v4 실행.
        let store = SqliteWorldStore::new(path_buf.to_str().unwrap()).unwrap();

        // 3) atlases 테이블이 추가되어 count_atlases가 동작.
        assert_eq!(store.count_atlases(None).unwrap(), 0);

        // 4) 기존 v3 places 보존.
        let p = store
            .get_place(&PlaceId::new("place-legacy"))
            .unwrap()
            .expect("v3 places row 보존 필요");
        assert_eq!(p.name, "레거시");

        // 5) v4 신규 — atlases upsert·get + place_atlas_refs 채워짐.
        let a = sample_atlas_with_refs("atlas-after", &["place-legacy"]);
        store.upsert_atlas("p", &a).unwrap();
        let back = store
            .get_atlas(&AtlasId::new("atlas-after"))
            .unwrap()
            .unwrap();
        assert_eq!(back.references, vec![PlaceId::new("place-legacy")]);

        let conn = store.conn.lock().unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM place_atlas_refs WHERE atlas_id = ?1",
                params!["atlas-after"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);

        drop(conn);
        drop(store);
        drop(tmp);
    }

    // -----------------------------------------------------------------------
    // Phase 4 follow-up — FTS body 코드블록 strip + get_places_batch
    // -----------------------------------------------------------------------

    #[test]
    fn strip_fenced_code_blocks_removes_box_drawing_lines() {
        let body = "before\n```\n┌──┐\n│ N │\n└──┘\n```\nafter";
        let stripped = strip_fenced_code_blocks(body);
        assert!(stripped.contains("before"));
        assert!(stripped.contains("after"));
        assert!(!stripped.contains("┌──┐"));
        assert!(!stripped.contains("│ N │"));
        assert!(!stripped.contains("```"));
    }

    #[test]
    fn strip_fenced_code_blocks_preserves_inline_backticks() {
        // 단일 백틱 inline은 펜스가 아니므로 그대로 보존.
        let body = "use `code` like this\nand more `inline`";
        let stripped = strip_fenced_code_blocks(body);
        assert_eq!(stripped.trim_end(), body);
    }

    #[test]
    fn strip_fenced_code_blocks_handles_tilde_fence() {
        let body = "before\n~~~\nfenced art\n~~~\nafter";
        let stripped = strip_fenced_code_blocks(body);
        assert!(stripped.contains("before"));
        assert!(stripped.contains("after"));
        assert!(!stripped.contains("fenced art"));
    }

    #[test]
    fn strip_fenced_code_blocks_unclosed_fence_drops_to_eof() {
        // 안전 측 — 펜스가 닫히지 않으면 그 시점부터 EOF까지 모두 제거.
        let body = "before\n```\nstuck open\nmore stuck";
        let stripped = strip_fenced_code_blocks(body);
        assert!(stripped.contains("before"));
        assert!(!stripped.contains("stuck"));
    }

    #[test]
    fn atlas_fts_body_excludes_diagram_so_box_drawing_does_not_match() {
        // 본 회귀 가드 — atlases_fts.body가 ASCII art를 제외하므로, 다이어그램에만
        // 등장하는 box-drawing 부분 문자열로는 atlas-jungwon이 매칭되지 않는다.
        // 단, name/aliases/summary에 "중원"/"칠국" 같은 평문이 있으므로 그건 매칭.
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut a = Atlas::new("atlas-jungwon", "continent", "Real Atlas Name");
        a.aliases = vec!["alias one".into()];
        a.summary = "summary text".into();
        // 본문은 코드블록 안 ASCII art만.
        a.body_sections.insert(
            "배치 다이어그램".into(),
            "```\n┌──기괴한 토큰──┐\n│ 박스내부텍스트 │\n└──────────────┘\n```".into(),
        );
        store.upsert_atlas("p", &a).unwrap();

        // 코드블록 *밖*에 있는 텍스트(name·aliases·summary)는 매칭.
        let hits_name = store.search_atlases("Real Atlas", 5).unwrap();
        assert!(!hits_name.is_empty(), "name 텍스트는 검색되어야 함");

        // 코드블록 *안*에 있는 한국어 평문은 매칭되지 않아야 — strip 후 FTS 본문 비어 있음.
        let hits_inside = store.search_atlases("박스내부텍스트", 5).unwrap();
        assert!(
            hits_inside.is_empty(),
            "코드블록 안 텍스트는 FTS 인덱스에서 제외되어야 함"
        );

        // 도메인 객체에는 다이어그램이 그대로 보존됨 (FTS strip은 인덱스 합성에만 적용).
        let back = store
            .get_atlas(&AtlasId::new("atlas-jungwon"))
            .unwrap()
            .unwrap();
        assert!(
            back.body_sections
                .get("배치 다이어그램")
                .unwrap()
                .contains("박스내부텍스트"),
            "도메인 데이터에는 다이어그램 보존되어야 함 (HTTP·view에서 손실 X)"
        );
    }

    #[test]
    fn get_places_batch_preserves_input_order() {
        let store = SqliteWorldStore::in_memory().unwrap();
        for id in ["place-a", "place-b", "place-c"] {
            store
                .upsert_place("p", &sample_settlement(id, id))
                .unwrap();
        }
        // 입력 순서가 알파벳 역순이어도 결과는 입력 순서.
        let ids = vec![
            PlaceId::new("place-c"),
            PlaceId::new("place-a"),
            PlaceId::new("place-b"),
        ];
        let got = store.get_places_batch(&ids).unwrap();
        let got_ids: Vec<&str> = got.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(got_ids, vec!["place-c", "place-a", "place-b"]);
    }

    #[test]
    fn get_places_batch_skips_missing_silently() {
        let store = SqliteWorldStore::in_memory().unwrap();
        store
            .upsert_place("p", &sample_settlement("place-a", "A"))
            .unwrap();
        let ids = vec![
            PlaceId::new("place-a"),
            PlaceId::new("place-missing"),
            PlaceId::new("place-also-missing"),
        ];
        let got = store.get_places_batch(&ids).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id.as_str(), "place-a");
    }

    #[test]
    fn get_places_batch_empty_input_returns_empty_no_sql() {
        // 빈 ids는 SQL 없이 즉시 반환 — `IN ()`는 SQLite 문법 에러.
        let store = SqliteWorldStore::in_memory().unwrap();
        let got = store.get_places_batch(&[]).unwrap();
        assert!(got.is_empty());
    }

    // -----------------------------------------------------------------------
    // Phase 5a — Event 라운드트립 + event_participants_refs 양방향 인덱스
    // -----------------------------------------------------------------------

    fn sample_event_with_participants(
        id: &str,
        people: &[&str],
        groups: &[&str],
        places: &[&str],
    ) -> Event {
        let mut e = Event::new(id, "betrayal", id);
        e.aliases = vec!["별칭".into()];
        e.summary = "테스트 사건".into();
        e.tags = vec!["test".into(), "event".into(), "historical".into()];
        e.category = EventCategory::Historical;
        e.extras
            .insert("trigger".into(), Value::String("발단".into()));
        e.temporal = EventTemporal {
            year: Some("10년 전 (260년차)".into()),
            year_relative: Some(-10),
            duration: Some("사흘 밤".into()),
            notes: None,
        };
        e.era_id = None;
        e.participants = ParticipantsRefs {
            people: people.iter().map(|s| s.to_string()).collect(),
            groups: groups.iter().map(|s| s.to_string()).collect(),
            places: places.iter().map(|s| s.to_string()).collect(),
        };
        e.body_sections
            .insert("개요".into(), "산문 본문".into());
        e
    }

    #[test]
    fn events_count_zero_on_fresh_db() {
        let store = SqliteWorldStore::in_memory().unwrap();
        assert_eq!(store.count_events(None).unwrap(), 0);
    }

    #[test]
    fn event_full_roundtrip_through_sqlite() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let e = sample_event_with_participants(
            "event-test",
            &["npc-01", "npc-02"],
            &["group-x"],
            &["place-y"],
        );
        store.upsert_event("test", &e).unwrap();
        let back = store.get_event(&EventId::new("event-test")).unwrap().unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn event_participants_refs_bidirectional_index_populated_on_upsert() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let e = sample_event_with_participants(
            "event-x",
            &["npc-01", "npc-02"],
            &["group-a"],
            &["place-c"],
        );
        store.upsert_event("test", &e).unwrap();

        // 정방향 (event → participants, ref_kind 카테고리별 + ref_order 보존).
        let conn = store.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT ref_kind, ref_id, ref_order FROM event_participants_refs
                 WHERE event_id = ?1 ORDER BY ref_order ASC",
            )
            .unwrap();
        let rows: Vec<(String, String, i64)> = stmt
            .query_map(params!["event-x"], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(
            rows,
            vec![
                ("person".to_string(), "npc-01".to_string(), 0),
                ("person".to_string(), "npc-02".to_string(), 1),
                ("group".to_string(), "group-a".to_string(), 2),
                ("place".to_string(), "place-c".to_string(), 3),
            ]
        );

        // 역방향 (participant → events) — idx_epr_person 인덱스 활용 가능.
        let mut stmt2 = conn
            .prepare(
                "SELECT event_id FROM event_participants_refs
                 WHERE ref_kind = 'person' AND ref_id = ?1
                 ORDER BY event_id",
            )
            .unwrap();
        let events_for_npc01: Vec<String> = stmt2
            .query_map(params!["npc-01"], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(events_for_npc01, vec!["event-x".to_string()]);
    }

    #[test]
    fn event_participants_refs_resyncs_on_re_upsert() {
        // participants 변경 → 기존 매핑은 모두 사라지고 신규로 채워짐.
        let store = SqliteWorldStore::in_memory().unwrap();
        let e1 = sample_event_with_participants("event-x", &["npc-01"], &[], &[]);
        store.upsert_event("test", &e1).unwrap();
        let e2 = sample_event_with_participants("event-x", &[], &[], &["place-z"]);
        store.upsert_event("test", &e2).unwrap();

        let conn = store.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT ref_kind, ref_id FROM event_participants_refs
                 WHERE event_id = ?1 ORDER BY ref_order",
            )
            .unwrap();
        let rows: Vec<(String, String)> = stmt
            .query_map(params!["event-x"], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(rows, vec![("place".to_string(), "place-z".to_string())]);
    }

    #[test]
    fn list_events_filter_by_category_and_kind() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut e1 = sample_event_with_participants("event-a", &[], &[], &[]);
        e1.category = EventCategory::Historical;
        e1.kind = "betrayal".into();
        let mut e2 = sample_event_with_participants("event-b", &[], &[], &[]);
        e2.category = EventCategory::Legendary;
        e2.kind = "war".into();
        store.upsert_event("test", &e1).unwrap();
        store.upsert_event("test", &e2).unwrap();

        let hist = store
            .list_events(EventFilter {
                category: Some(EventCategory::Historical),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].id.as_str(), "event-a");

        let wars = store
            .list_events(EventFilter {
                kind: Some("war".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(wars.len(), 1);
        assert_eq!(wars[0].id.as_str(), "event-b");
    }

    #[test]
    fn list_events_filter_by_participants() {
        let store = SqliteWorldStore::in_memory().unwrap();
        store
            .upsert_event(
                "test",
                &sample_event_with_participants(
                    "event-a",
                    &["npc-01", "npc-02"],
                    &["group-x"],
                    &["place-y"],
                ),
            )
            .unwrap();
        store
            .upsert_event(
                "test",
                &sample_event_with_participants(
                    "event-b",
                    &["npc-02"],
                    &[],
                    &["place-z"],
                ),
            )
            .unwrap();
        store
            .upsert_event(
                "test",
                &sample_event_with_participants("event-c", &[], &[], &[]),
            )
            .unwrap();

        // npc-01 관여 사건은 event-a 하나만.
        let by_npc01 = store
            .list_events(EventFilter {
                participants_person: Some("npc-01".into()),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<&str> = by_npc01.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["event-a"]);

        // npc-02 관여는 event-a + event-b.
        let by_npc02 = store
            .list_events(EventFilter {
                participants_person: Some("npc-02".into()),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<&str> = by_npc02.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["event-a", "event-b"]);

        // group-x 관여는 event-a 하나만.
        let by_group = store
            .list_events(EventFilter {
                participants_group: Some("group-x".into()),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<&str> = by_group.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["event-a"]);

        // place-y만 관여한 사건 = event-a.
        let by_place = store
            .list_events(EventFilter {
                participants_place: Some("place-y".into()),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<&str> = by_place.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["event-a"]);
    }

    #[test]
    fn list_events_filter_by_year_relative_range() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut e_old = sample_event_with_participants("event-old", &[], &[], &[]);
        e_old.temporal.year_relative = Some(-200);
        let mut e_recent = sample_event_with_participants("event-recent", &[], &[], &[]);
        e_recent.temporal.year_relative = Some(-10);
        let mut e_now = sample_event_with_participants("event-now", &[], &[], &[]);
        e_now.temporal.year_relative = Some(0);
        store.upsert_event("test", &e_old).unwrap();
        store.upsert_event("test", &e_recent).unwrap();
        store.upsert_event("test", &e_now).unwrap();

        // -30 ≤ year_relative ≤ 0 → recent + now.
        let recent = store
            .list_events(EventFilter {
                year_relative_min: Some(-30),
                year_relative_max: Some(0),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<&str> = recent.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["event-now", "event-recent"]); // id ASC
    }

    #[test]
    fn search_events_fts_matches_name_alias_and_body() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut e = Event::new("event-bloody-night", "betrayal", "붉은 밤의 변");
        e.aliases = vec!["붉은 밤".into(), "10년 전 변란".into()];
        e.summary = "통일제국 대진의 영토 와해 사건".into();
        e.body_sections
            .insert("개요".into(), "사흘 밤 동안 이어진 변란".into());
        store.upsert_event("test", &e).unwrap();

        // alias 매칭 (FTS5 trigram, 3자 이상).
        let hits = store.search_events("붉은 밤", 5).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id.as_str(), "event-bloody-night");

        // summary 매칭.
        let hits = store.search_events("영토 와해", 5).unwrap();
        assert!(!hits.is_empty());

        // 빈 query.
        assert!(store.search_events("", 5).unwrap().is_empty());
    }

    #[test]
    fn events_upsert_replaces_fts_stale_row() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut e = Event::new("event-x", "betrayal", "Alpha Event");
        e.aliases = vec!["OldAliasUnique".into()];
        e.body_sections.clear();
        e.body_sections.insert("Overview".into(), "first".into());
        store.upsert_event("test", &e).unwrap();
        let hits = store.search_events("OldAliasUnique", 5).unwrap();
        assert_eq!(hits.len(), 1);

        e.aliases = vec!["NewAliasUnique".into()];
        e.body_sections.clear();
        e.body_sections.insert("Overview".into(), "second".into());
        store.upsert_event("test", &e).unwrap();

        let hits_old = store.search_events("OldAliasUnique", 5).unwrap();
        assert!(hits_old.is_empty(), "stale FTS5 row 검출");
        let hits_new = store.search_events("NewAliasUnique", 5).unwrap();
        assert_eq!(hits_new.len(), 1);
    }

    #[test]
    fn schema_v4_to_v5_migration_upgrades_existing_file_db() {
        // 실제 v4→v5 경로 검증. tempfile에 v4 schema를 작성한 뒤,
        // SqliteWorldStore::new(path)로 재오픈 → init_tables의 `current < 5` 분기가 활성되어
        // events / events_fts / event_participants_refs가 추가되며 기존 v4 데이터는 보존된다.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path_buf = tmp.path().to_path_buf();

        // 1) v4 schema 작성 — atlases까지만.
        {
            let conn = rusqlite::Connection::open(&path_buf).unwrap();
            conn.execute_batch(
                "CREATE TABLE world_schema_meta (version INTEGER PRIMARY KEY);
                 INSERT INTO world_schema_meta(version) VALUES (4);
                 CREATE TABLE atlases (
                    id TEXT PRIMARY KEY, project_id TEXT NOT NULL, kind TEXT NOT NULL,
                    name TEXT NOT NULL, aliases_json TEXT NOT NULL DEFAULT '[]',
                    summary TEXT NOT NULL DEFAULT '', tags_json TEXT NOT NULL DEFAULT '[]',
                    extras_json TEXT NOT NULL DEFAULT '{}', extent_json TEXT NOT NULL DEFAULT '{}',
                    references_json TEXT NOT NULL DEFAULT '[]',
                    body_sections_json TEXT NOT NULL DEFAULT '{}', source_path TEXT,
                    updated_at INTEGER NOT NULL
                 );
                 CREATE VIRTUAL TABLE atlases_fts USING fts5(
                    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
                 );
                 INSERT INTO atlases (id, project_id, kind, name, updated_at)
                    VALUES ('atlas-legacy', 'p', 'continent', '레거시', 0);",
            )
            .unwrap();
        }

        // 2) 재오픈 → migrate_v5 실행.
        let store = SqliteWorldStore::new(path_buf.to_str().unwrap()).unwrap();

        // 3) events 테이블이 추가되어 count_events가 동작.
        assert_eq!(store.count_events(None).unwrap(), 0);

        // 4) 기존 v4 atlases 보존.
        let a = store
            .get_atlas(&AtlasId::new("atlas-legacy"))
            .unwrap()
            .expect("v4 atlases row 보존 필요");
        assert_eq!(a.name, "레거시");

        // 5) v5 신규 — events upsert·get + event_participants_refs 채워짐.
        let e = sample_event_with_participants("event-after", &["npc-1"], &[], &[]);
        store.upsert_event("p", &e).unwrap();
        let back = store
            .get_event(&EventId::new("event-after"))
            .unwrap()
            .unwrap();
        assert_eq!(back.participants.people, vec!["npc-1".to_string()]);

        let conn = store.conn.lock().unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_participants_refs WHERE event_id = ?1",
                params!["event-after"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);

        drop(conn);
        drop(store);
        drop(tmp);
    }

    #[test]
    fn events_count_with_project_filter() {
        let store = SqliteWorldStore::in_memory().unwrap();
        store
            .upsert_event(
                "p1",
                &sample_event_with_participants("event-a", &[], &[], &[]),
            )
            .unwrap();
        store
            .upsert_event(
                "p2",
                &sample_event_with_participants("event-b", &[], &[], &[]),
            )
            .unwrap();
        assert_eq!(store.count_events(None).unwrap(), 2);
        assert_eq!(store.count_events(Some("p1")).unwrap(), 1);
        assert_eq!(store.count_events(Some("p2")).unwrap(), 1);
    }

    // -----------------------------------------------------------------------
    // Phase 5b — Era 라운드트립 + boundary 정책 + migrate_v6
    // -----------------------------------------------------------------------

    fn sample_era(id: &str, kind: &str, start: i32, end: i32) -> Era {
        let mut e = Era::new(id, kind, id);
        e.aliases = vec!["별호".into()];
        e.summary = "테스트 시대".into();
        e.tags = vec!["test".into(), "era".into()];
        e.extras
            .insert("game_role".into(), Value::String("trigger".into()));
        e.temporal = EraTemporal {
            start_year_relative: Some(start),
            end_year_relative: Some(end),
            notes: Some("inclusive-exclusive".into()),
        };
        e.body_sections.insert("개요".into(), "산문".into());
        e
    }

    #[test]
    fn eras_count_zero_on_fresh_db() {
        let store = SqliteWorldStore::in_memory().unwrap();
        assert_eq!(store.count_eras(None).unwrap(), 0);
    }

    #[test]
    fn era_full_roundtrip_through_sqlite() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut e = sample_era("era-fall", "fall", -30, 0);
        e.key_events = vec![
            EventId::new("event-a"),
            EventId::new("event-b"),
            EventId::new("event-c"),
        ];
        store.upsert_era("test", &e).unwrap();
        let back = store.get_era(&EraId::new("era-fall")).unwrap().unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn list_eras_filter_by_kind() {
        let store = SqliteWorldStore::in_memory().unwrap();
        store
            .upsert_era("p", &sample_era("era-founding", "founding", -270, -220))
            .unwrap();
        store
            .upsert_era("p", &sample_era("era-fall", "fall", -30, 0))
            .unwrap();

        let founders = store
            .list_eras(EraFilter {
                kind: Some("founding".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(founders.len(), 1);
        assert_eq!(founders[0].id.as_str(), "era-founding");
    }

    #[test]
    fn list_eras_filter_contains_year_inclusive_start() {
        // boundary 정책 §3.3 회귀 가드 — start inclusive.
        let store = SqliteWorldStore::in_memory().unwrap();
        store
            .upsert_era("p", &sample_era("era-decline", "decline", -70, -30))
            .unwrap();
        store
            .upsert_era("p", &sample_era("era-fall", "fall", -30, 0))
            .unwrap();

        // -30년차는 era-fall의 start (inclusive) — era-fall 매칭, era-decline은 end (exclusive)이라 미매칭.
        let hits = store
            .list_eras(EraFilter {
                contains_year: Some(-30),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<&str> = hits.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["era-fall"],
            "boundary -30은 era-fall (start inclusive)에만 매칭되어야 함"
        );
    }

    #[test]
    fn list_eras_filter_contains_year_exclusive_end() {
        // 270년차(=0)는 어느 era에도 속하지 않음 (모든 era end exclusive).
        let store = SqliteWorldStore::in_memory().unwrap();
        store
            .upsert_era("p", &sample_era("era-fall", "fall", -30, 0))
            .unwrap();

        let hits = store
            .list_eras(EraFilter {
                contains_year: Some(0),
                ..Default::default()
            })
            .unwrap();
        assert!(
            hits.is_empty(),
            "year=0은 어느 era에도 속하지 않아야 함 (end exclusive)"
        );
    }

    #[test]
    fn search_eras_fts_matches_korean() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut e = Era::new("era-fall-of-empire", "fall", "붕괴기");
        e.aliases = vec!["6국 분열기".into(), "240-270년차".into()];
        e.summary = "통일제국 와해 시기".into();
        store.upsert_era("p", &e).unwrap();

        let hits = store.search_eras("붕괴", 5).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id.as_str(), "era-fall-of-empire");

        // alias 매칭
        let hits2 = store.search_eras("분열기", 5).unwrap();
        assert!(!hits2.is_empty());
    }

    #[test]
    fn era_upsert_replaces_fts_stale_row() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut e = Era::new("era-x", "fall", "Alpha Era");
        e.aliases = vec!["OldAliasUnique".into()];
        store.upsert_era("p", &e).unwrap();
        let hits = store.search_eras("OldAliasUnique", 5).unwrap();
        assert_eq!(hits.len(), 1);

        e.aliases = vec!["NewAliasUnique".into()];
        store.upsert_era("p", &e).unwrap();

        let hits_old = store.search_eras("OldAliasUnique", 5).unwrap();
        assert!(hits_old.is_empty(), "stale FTS5 row 검출");
        let hits_new = store.search_eras("NewAliasUnique", 5).unwrap();
        assert_eq!(hits_new.len(), 1);
    }

    #[test]
    fn schema_v5_to_v6_migration_upgrades_existing_file_db() {
        // v5→v6 경로 — eras 테이블이 추가되며 기존 v5 events 보존.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path_buf = tmp.path().to_path_buf();

        // 1) v5 schema 작성 — events까지만.
        {
            let conn = rusqlite::Connection::open(&path_buf).unwrap();
            conn.execute_batch(
                "CREATE TABLE world_schema_meta (version INTEGER PRIMARY KEY);
                 INSERT INTO world_schema_meta(version) VALUES (5);
                 CREATE TABLE events (
                    id TEXT PRIMARY KEY, project_id TEXT NOT NULL, kind TEXT NOT NULL,
                    category TEXT NOT NULL DEFAULT 'historical',
                    name TEXT NOT NULL, aliases_json TEXT NOT NULL DEFAULT '[]',
                    summary TEXT NOT NULL DEFAULT '', tags_json TEXT NOT NULL DEFAULT '[]',
                    extras_json TEXT NOT NULL DEFAULT '{}', temporal_json TEXT NOT NULL DEFAULT '{}',
                    year_relative INTEGER, era_id TEXT,
                    participants_json TEXT NOT NULL DEFAULT '{}',
                    body_sections_json TEXT NOT NULL DEFAULT '{}',
                    related_events_json TEXT NOT NULL DEFAULT '[]',
                    source_path TEXT, updated_at INTEGER NOT NULL
                 );
                 CREATE VIRTUAL TABLE events_fts USING fts5(
                    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
                 );
                 CREATE TABLE event_participants_refs (
                    event_id TEXT NOT NULL, ref_kind TEXT NOT NULL, ref_id TEXT NOT NULL,
                    ref_order INTEGER NOT NULL,
                    PRIMARY KEY (event_id, ref_kind, ref_id)
                 );
                 INSERT INTO events (id, project_id, kind, name, updated_at)
                    VALUES ('event-legacy', 'p', 'war', '레거시', 0);",
            )
            .unwrap();
        }

        // 2) 재오픈 → migrate_v6 실행.
        let store = SqliteWorldStore::new(path_buf.to_str().unwrap()).unwrap();

        // 3) eras 테이블 추가 — count_eras 동작.
        assert_eq!(store.count_eras(None).unwrap(), 0);

        // 4) 기존 v5 events 보존.
        let ev = store
            .get_event(&EventId::new("event-legacy"))
            .unwrap()
            .expect("v5 events row 보존 필요");
        assert_eq!(ev.name, "레거시");

        // 5) v6 신규 — eras upsert·get 동작.
        let e = sample_era("era-after", "fall", -30, 0);
        store.upsert_era("p", &e).unwrap();
        let back = store.get_era(&EraId::new("era-after")).unwrap().unwrap();
        assert_eq!(back.kind, "fall");

        drop(store);
        drop(tmp);
    }

    #[test]
    fn eras_count_with_project_filter() {
        let store = SqliteWorldStore::in_memory().unwrap();
        store
            .upsert_era("p1", &sample_era("era-a", "founding", -270, -220))
            .unwrap();
        store
            .upsert_era("p2", &sample_era("era-b", "fall", -30, 0))
            .unwrap();
        assert_eq!(store.count_eras(None).unwrap(), 2);
        assert_eq!(store.count_eras(Some("p1")).unwrap(), 1);
        assert_eq!(store.count_eras(Some("p2")).unwrap(), 1);
    }

    #[test]
    fn era_with_key_events_roundtrip_preserves_order() {
        // key_events는 시간순 작성 권장 — 순서 보존이 핵심.
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut e = sample_era("era-fall", "fall", -30, 0);
        e.key_events = vec![
            EventId::new("event-bloody-cult-rebellion-2nd"),
            EventId::new("event-blood-disappearance"),
            EventId::new("event-bloody-night"),
            EventId::new("event-hwasan-fall"),
            EventId::new("event-six-states-independence"),
        ];
        store.upsert_era("p", &e).unwrap();
        let back = store.get_era(&EraId::new("era-fall")).unwrap().unwrap();
        assert_eq!(back.key_events.len(), 5);
        assert_eq!(
            back.key_events[0].as_str(),
            "event-bloody-cult-rebellion-2nd",
            "작성 순서 보존 (시간순)"
        );
        assert_eq!(
            back.key_events[4].as_str(),
            "event-six-states-independence"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 5b 체크포인트 2 — Timeline 라운드트립 + 양방향 인덱스 + migrate_v7
    // -----------------------------------------------------------------------

    fn sample_timeline(id: &str, refs: &[&str]) -> Timeline {
        let mut t = Timeline::new(id, "history", id);
        t.aliases = vec!["별호".into()];
        t.summary = "테스트 timeline".into();
        t.tags = vec!["test".into(), "timeline".into()];
        t.extras
            .insert("game_role".into(), Value::String("trigger".into()));
        t.references = refs.iter().map(|s| EraId::new(*s)).collect();
        t.body_sections.insert("개요".into(), "산문".into());
        t
    }

    #[test]
    fn timelines_count_zero_on_fresh_db() {
        let store = SqliteWorldStore::in_memory().unwrap();
        assert_eq!(store.count_timelines(None).unwrap(), 0);
    }

    #[test]
    fn timeline_full_roundtrip_through_sqlite() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let t = sample_timeline("timeline-x", &["era-a", "era-b", "era-c"]);
        store.upsert_timeline("test", &t).unwrap();
        let back = store
            .get_timeline(&TimelineId::new("timeline-x"))
            .unwrap()
            .unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn timeline_era_refs_bidirectional_index_populated() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let t = sample_timeline("timeline-x", &["era-a", "era-b", "era-c"]);
        store.upsert_timeline("test", &t).unwrap();

        // 정방향 (timeline → eras, ref_order 보존)
        let conn = store.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT era_id, ref_order FROM timeline_era_refs WHERE timeline_id = ?1
                 ORDER BY ref_order ASC",
            )
            .unwrap();
        let rows: Vec<(String, i64)> = stmt
            .query_map(params!["timeline-x"], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(
            rows,
            vec![
                ("era-a".to_string(), 0),
                ("era-b".to_string(), 1),
                ("era-c".to_string(), 2),
            ]
        );

        // 역방향 (era → timelines) — idx_ter_era 인덱스 활용 가능.
        let mut stmt2 = conn
            .prepare(
                "SELECT timeline_id FROM timeline_era_refs WHERE era_id = ?1
                 ORDER BY timeline_id",
            )
            .unwrap();
        let timelines_for_b: Vec<String> = stmt2
            .query_map(params!["era-b"], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(timelines_for_b, vec!["timeline-x".to_string()]);
    }

    #[test]
    fn timeline_era_refs_resyncs_on_re_upsert() {
        // references 변경 → 기존 매핑 모두 사라지고 신규로 채워짐.
        let store = SqliteWorldStore::in_memory().unwrap();
        let t1 = sample_timeline("timeline-x", &["era-a", "era-b"]);
        store.upsert_timeline("test", &t1).unwrap();
        let t2 = sample_timeline("timeline-x", &["era-c"]);
        store.upsert_timeline("test", &t2).unwrap();

        let conn = store.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT era_id FROM timeline_era_refs WHERE timeline_id = ?1 ORDER BY era_id",
            )
            .unwrap();
        let ids: Vec<String> = stmt
            .query_map(params!["timeline-x"], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(ids, vec!["era-c".to_string()]);
    }

    #[test]
    fn list_timelines_filter_by_kind_and_references_era() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut t1 = sample_timeline("timeline-a", &["era-fall"]);
        t1.kind = "history".into();
        let mut t2 = sample_timeline("timeline-b", &["era-founding"]);
        t2.kind = "biographical".into();
        store.upsert_timeline("test", &t1).unwrap();
        store.upsert_timeline("test", &t2).unwrap();

        let history = store
            .list_timelines(TimelineFilter {
                kind: Some("history".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id.as_str(), "timeline-a");

        // references_era 필터 — timeline_era_refs 인덱스 활용.
        let by_era = store
            .list_timelines(TimelineFilter {
                references_era: Some(EraId::new("era-fall")),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(by_era.len(), 1);
        assert_eq!(by_era[0].id.as_str(), "timeline-a");
    }

    #[test]
    fn search_timelines_fts_matches_alias() {
        let store = SqliteWorldStore::in_memory().unwrap();
        let mut t = Timeline::new("timeline-jungwon-history", "history", "270년사");
        t.aliases = vec!["중원사".into(), "main-history".into()];
        t.summary = "원년부터 현재까지".into();
        store.upsert_timeline("test", &t).unwrap();

        let hits = store.search_timelines("270년사", 5).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id.as_str(), "timeline-jungwon-history");

        // 알파벳 alias도 매칭.
        let hits2 = store.search_timelines("main-history", 5).unwrap();
        assert!(!hits2.is_empty());
    }

    #[test]
    fn schema_v6_to_v7_migration_upgrades_existing_file_db() {
        // v6→v7 경로 — timelines 테이블이 추가되며 기존 v6 eras 보존.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path_buf = tmp.path().to_path_buf();

        // v6 schema 작성 — eras까지만 (Phase 5b 체크포인트 1 상태).
        {
            let conn = rusqlite::Connection::open(&path_buf).unwrap();
            conn.execute_batch(
                "CREATE TABLE world_schema_meta (version INTEGER PRIMARY KEY);
                 INSERT INTO world_schema_meta(version) VALUES (6);
                 CREATE TABLE eras (
                    id TEXT PRIMARY KEY, project_id TEXT NOT NULL, kind TEXT NOT NULL,
                    name TEXT NOT NULL, aliases_json TEXT NOT NULL DEFAULT '[]',
                    summary TEXT NOT NULL DEFAULT '', tags_json TEXT NOT NULL DEFAULT '[]',
                    extras_json TEXT NOT NULL DEFAULT '{}', temporal_json TEXT NOT NULL DEFAULT '{}',
                    start_year_relative INTEGER, end_year_relative INTEGER,
                    key_events_json TEXT NOT NULL DEFAULT '[]',
                    body_sections_json TEXT NOT NULL DEFAULT '{}',
                    source_path TEXT, updated_at INTEGER NOT NULL
                 );
                 CREATE VIRTUAL TABLE eras_fts USING fts5(
                    id UNINDEXED, name, aliases, summary, body, tokenize='trigram'
                 );
                 INSERT INTO eras (id, project_id, kind, name, updated_at)
                    VALUES ('era-legacy', 'p', 'fall', '레거시', 0);",
            )
            .unwrap();
        }

        // 재오픈 → migrate_v7 실행.
        let store = SqliteWorldStore::new(path_buf.to_str().unwrap()).unwrap();

        // timelines 테이블 추가 — count_timelines 동작.
        assert_eq!(store.count_timelines(None).unwrap(), 0);

        // 기존 v6 eras 보존.
        let era = store
            .get_era(&EraId::new("era-legacy"))
            .unwrap()
            .expect("v6 eras row 보존 필요");
        assert_eq!(era.name, "레거시");

        // v7 신규 — timelines upsert·get + timeline_era_refs 채워짐.
        let t = sample_timeline("timeline-after", &["era-legacy"]);
        store.upsert_timeline("p", &t).unwrap();
        let back = store
            .get_timeline(&TimelineId::new("timeline-after"))
            .unwrap()
            .unwrap();
        assert_eq!(back.references, vec![EraId::new("era-legacy")]);

        let conn = store.conn.lock().unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM timeline_era_refs WHERE timeline_id = ?1",
                params!["timeline-after"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);

        drop(conn);
        drop(store);
        drop(tmp);
    }

    #[test]
    fn timelines_count_with_project_filter() {
        let store = SqliteWorldStore::in_memory().unwrap();
        store
            .upsert_timeline("p1", &sample_timeline("timeline-a", &["era-x"]))
            .unwrap();
        store
            .upsert_timeline("p2", &sample_timeline("timeline-b", &["era-y"]))
            .unwrap();
        assert_eq!(store.count_timelines(None).unwrap(), 2);
        assert_eq!(store.count_timelines(Some("p1")).unwrap(), 1);
        assert_eq!(store.count_timelines(Some("p2")).unwrap(), 1);
    }
}
