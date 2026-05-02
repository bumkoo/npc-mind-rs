//! `world-load` CLI의 Phase 5a Event 외래키 거부 경로 테스트 — 체크포인트 1
//! manual demo의 영구 자동화 가드.
//!
//! 검증 항목 (사양 §3.2 + Phase 5a 활성 외래키):
//! - `Event.participants.people` ↔ `persons.id` (결손 시 hard-fail + DB 미수정)
//! - `Event.participants.groups` ↔ `groups.id`
//! - `Event.participants.places` ↔ `places.id`
//! - `Event.related_events` ↔ `events.id` (자체 도메인)
//! - 카테고리 내 중복 (event_participants_refs composite PK 보호)
//! - 정상 흐름 — 모든 FK 정합 시 ingest 성공 + events_indexed = 1
//!
//! 실행: `cargo test --features embed --test world_load_fk_negative_event`

#![cfg(feature = "embed")]

use std::path::PathBuf;
use std::process::Command;

const WORLD_LOAD_BIN: &str = env!("CARGO_BIN_EXE_world-load");

/// Phase 5a 5 도메인 (group/person/place/atlas/event) 모두 작성 가능한 fixture.
/// 이전 `world_load_fk_negative.rs`의 places-only fixture를 확장한 형태.
struct Fixture {
    _tmp: tempfile::TempDir,
    project_dir: PathBuf,
    projects_root: PathBuf,
    project_name: String,
    db_path: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        let projects_root = tmp.path().to_path_buf();
        let project_dir = projects_root.join(name);
        for sub in ["group", "person", "place", "event"] {
            std::fs::create_dir_all(project_dir.join("world").join(sub)).unwrap();
        }
        let db_path = project_dir.join("build").join("world.sqlite");
        Self {
            _tmp: tmp,
            project_dir,
            projects_root,
            project_name: name.to_string(),
            db_path,
        }
    }

    fn write(&self, sub: &str, filename: &str, contents: &str) {
        std::fs::write(
            self.project_dir.join("world").join(sub).join(filename),
            contents,
        )
        .unwrap();
    }

    /// 이번 테스트 군의 baseline — 1 group + 1 person + 1 place. event가 참조할 수 있는
    /// 정상 ID 셋을 미리 깔아 둔다. 각 fixture에서 event 파일만 별도로 작성.
    fn seed_baseline(&self) {
        // place — 자족 (어떤 외래키도 없음)
        self.write(
            "place",
            "place-cap.md",
            "---\nid: place-cap\nlayer: settlement\nkind: nation\nname: Capital\n---\n",
        );
        // group — headquarters만 place-cap으로 (다른 외래키 없음)
        self.write(
            "group",
            "group-court.md",
            "---\nid: group-court\nkind: dynasty-court\nname: Court\nheadquarters: place-cap\n---\n",
        );
        // person — npc-01 (active이면 mind 변환 시도하나 --no-mind로 비활성)
        self.write(
            "person",
            "npc-01.md",
            "---\nid: npc-01\nkind: active\nname: A\nstatus: alive\n---\n",
        );
    }

    fn run(&self) -> std::process::Output {
        Command::new(WORLD_LOAD_BIN)
            .args([
                "--project",
                &self.project_name,
                "--projects-root",
                self.projects_root.to_str().unwrap(),
                "--no-mind",
                "--db",
                self.db_path.to_str().unwrap(),
            ])
            .output()
            .expect("spawn world-load")
    }
}

fn db_count_events(path: &PathBuf) -> i64 {
    let conn = rusqlite::Connection::open(path).expect("open sqlite");
    conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Negative paths — 외래키 결손 4종 + composite PK 위반 1종 = 5 hard-fail 케이스
// ---------------------------------------------------------------------------

#[test]
fn rejects_participants_people_pointing_at_missing_id() {
    // 체크포인트 1 manual demo의 자동화 — npc-99 같은 미등록 ID 주입 시 거부.
    let f = Fixture::new("test-event-missing-person");
    f.seed_baseline();
    f.write(
        "event",
        "event-x.md",
        "---\nid: event-x\nkind: betrayal\nname: X\nparticipants:\n  people:\n    - npc-01\n    - npc-99\n---\n",
    );

    let out = f.run();
    assert!(
        !out.status.success(),
        "world-load should fail on missing npc-99 — stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("participants.people") && stderr.contains("npc-99") && stderr.contains("결손"),
        "stderr가 participants.people npc-99 결손을 명시해야 함:\n{stderr}"
    );
    // partial commit 방지 — events 테이블 자체가 미생성이거나 row 0건이어야.
    assert!(
        !f.db_path.exists() || db_count_events(&f.db_path) == 0,
        "fail 시 events 테이블에 row가 남으면 안 됨"
    );
}

#[test]
fn rejects_participants_groups_pointing_at_missing_id() {
    let f = Fixture::new("test-event-missing-group");
    f.seed_baseline();
    f.write(
        "event",
        "event-x.md",
        "---\nid: event-x\nkind: war\nname: X\nparticipants:\n  groups:\n    - group-court\n    - group-ghost\n---\n",
    );

    let out = f.run();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("participants.groups") && stderr.contains("group-ghost"),
        "stderr가 participants.groups 결손을 명시해야 함:\n{stderr}"
    );
}

#[test]
fn rejects_participants_places_pointing_at_missing_id() {
    let f = Fixture::new("test-event-missing-place");
    f.seed_baseline();
    f.write(
        "event",
        "event-x.md",
        "---\nid: event-x\nkind: disaster\nname: X\nparticipants:\n  places:\n    - place-cap\n    - place-nowhere\n---\n",
    );

    let out = f.run();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("participants.places") && stderr.contains("place-nowhere"),
        "stderr가 participants.places 결손을 명시해야 함:\n{stderr}"
    );
}

#[test]
fn rejects_related_events_pointing_at_missing_id() {
    // 자체 도메인 외래키 — 같은 ingest 안에 미정의된 event id 참조 시 거부.
    let f = Fixture::new("test-event-missing-related");
    f.seed_baseline();
    f.write(
        "event",
        "event-x.md",
        "---\nid: event-x\nkind: betrayal\nname: X\nrelated_events:\n  - event-y\n---\n",
    );

    let out = f.run();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("related_events") && stderr.contains("event-y"),
        "stderr가 related_events 결손을 명시해야 함:\n{stderr}"
    );
}

#[test]
fn rejects_duplicate_participants_within_category() {
    // event_participants_refs composite PK (event_id, ref_kind, ref_id) 보호 —
    // 같은 카테고리 내 동일 ID 중복은 PK 위반 전에 검증 단계에서 차단.
    let f = Fixture::new("test-event-dup-participant");
    f.seed_baseline();
    f.write(
        "event",
        "event-x.md",
        "---\nid: event-x\nkind: betrayal\nname: X\nparticipants:\n  people:\n    - npc-01\n    - npc-01\n---\n",
    );

    let out = f.run();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("중복") && stderr.contains("npc-01"),
        "stderr가 카테고리 내 중복을 명시해야 함:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Positive path — 모든 FK 정합 + related_events 양방향 시연
// ---------------------------------------------------------------------------

#[test]
fn accepts_event_with_all_foreign_keys_satisfied() {
    // 정상 흐름 — participants 셋 + related_events 양방향 모두 정합.
    let f = Fixture::new("test-event-ok");
    f.seed_baseline();
    // 두 사건이 서로를 related_events로 참조 — 양방향 시연 (Phase 5a는 cycle 검증 비활성).
    f.write(
        "event",
        "event-a.md",
        "---\nid: event-a\nkind: betrayal\nname: A\nparticipants:\n  people:\n    - npc-01\n  groups:\n    - group-court\n  places:\n    - place-cap\nrelated_events:\n  - event-b\n---\n",
    );
    f.write(
        "event",
        "event-b.md",
        "---\nid: event-b\nkind: war\nname: B\nrelated_events:\n  - event-a\n---\n",
    );

    let out = f.run();
    assert!(
        out.status.success(),
        "정합 사건은 통과해야 함 — stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(db_count_events(&f.db_path), 2);
}

// ---------------------------------------------------------------------------
// Recovery path — 체크포인트 1 manual demo의 마지막 단계
// ---------------------------------------------------------------------------

#[test]
fn recovers_after_fixing_missing_id() {
    // 체크포인트 1 manual demo의 정확한 자동화: 결손 → fail → 수정 → 통과.
    let f = Fixture::new("test-event-recovery");
    f.seed_baseline();

    // Step 1: 결손 ID로 빌드 시도 → 실패.
    f.write(
        "event",
        "event-x.md",
        "---\nid: event-x\nkind: betrayal\nname: X\nparticipants:\n  people:\n    - npc-99\n---\n",
    );
    let out_fail = f.run();
    assert!(!out_fail.status.success(), "결손 ID는 실패해야 함");
    let pre_count = if f.db_path.exists() {
        db_count_events(&f.db_path)
    } else {
        0
    };
    assert_eq!(pre_count, 0, "fail 시 DB 미수정 — events 0건");

    // Step 2: ID 정정 후 재실행 → 성공.
    f.write(
        "event",
        "event-x.md",
        "---\nid: event-x\nkind: betrayal\nname: X\nparticipants:\n  people:\n    - npc-01\n---\n",
    );
    let out_ok = f.run();
    assert!(
        out_ok.status.success(),
        "복구 후 통과해야 함 — stderr:\n{}",
        String::from_utf8_lossy(&out_ok.stderr)
    );
    assert_eq!(db_count_events(&f.db_path), 1, "복구 후 events 1건");
}
