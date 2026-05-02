//! `world-load` CLI의 Phase 5b Era 외래키 거부 경로 테스트 — Phase 5a N1 패턴 미러.
//!
//! 검증 항목 (사양 §3.2 + Phase 5b 활성 외래키):
//! - `Era.key_events` ↔ `events.id` (Era → Event 단방향)
//! - `Event.era_id` ↔ `eras.id` (Phase 5a 텍스트 → Phase 5b 활성)
//! - `Atlas.era_id` ↔ `eras.id` (Phase 4 텍스트 → Phase 5b 활성, extras 안)
//! - 정상 흐름 — 모든 era 외래키 정합 시 ingest 성공
//!
//! 실행: `cargo test --features embed --test world_load_fk_negative_era`

#![cfg(feature = "embed")]

use std::path::PathBuf;
use std::process::Command;

const WORLD_LOAD_BIN: &str = env!("CARGO_BIN_EXE_world-load");

/// Phase 5b 6 도메인 (group/person/place/atlas/event/era) 모두 작성 가능한 fixture.
/// Phase 5a `world_load_fk_negative_event.rs` fixture를 era로 확장.
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
        for sub in ["group", "person", "place", "atlas", "event", "era"] {
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

    /// 이번 테스트 군의 baseline — Phase 5a 시드 일부 + 1 era. 각 fixture에서 검증 대상만
    /// 별도 작성. era-test가 깔려 있고 event-baseline이 era-test에 매핑된 정합 상태에서
    /// 시작.
    fn seed_baseline(&self) {
        // place + group + person — 최소.
        self.write(
            "place",
            "place-cap.md",
            "---\nid: place-cap\nlayer: settlement\nkind: nation\nname: Capital\n---\n",
        );
        self.write(
            "group",
            "group-court.md",
            "---\nid: group-court\nkind: dynasty-court\nname: Court\nheadquarters: place-cap\n---\n",
        );
        self.write(
            "person",
            "npc-01.md",
            "---\nid: npc-01\nkind: active\nname: A\nstatus: alive\n---\n",
        );
        // event-baseline은 era-test에 매핑됨 — 정합.
        self.write(
            "event",
            "event-baseline.md",
            "---\nid: event-baseline\nkind: betrayal\nname: Baseline\nera_id: era-test\n---\n",
        );
        self.write(
            "era",
            "era-test.md",
            "---\nid: era-test\nkind: fall\nname: Test Era\ntemporal:\n  start_year_relative: -30\n  end_year_relative: 0\nkey_events:\n  - event-baseline\n---\n",
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

fn db_count_eras(path: &PathBuf) -> i64 {
    let conn = rusqlite::Connection::open(path).expect("open sqlite");
    conn.query_row("SELECT COUNT(*) FROM eras", [], |r| r.get(0))
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Negative paths — Phase 5b 외래키 활성 3종
// ---------------------------------------------------------------------------

#[test]
fn rejects_era_key_events_pointing_at_missing_id() {
    // Era.key_events 결손 시 hard-fail + DB 미수정.
    let f = Fixture::new("test-era-missing-key-event");
    f.seed_baseline();
    // era-bad가 미존재 event-99를 key_events로 참조.
    f.write(
        "era",
        "era-bad.md",
        "---\nid: era-bad\nkind: founding\nname: Bad\ntemporal:\n  start_year_relative: -270\n  end_year_relative: -220\nkey_events:\n  - event-baseline\n  - event-99\n---\n",
    );

    let out = f.run();
    assert!(
        !out.status.success(),
        "world-load should fail on missing event-99 in key_events — stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("eras.key_events") && stderr.contains("event-99") && stderr.contains("결손"),
        "stderr가 eras.key_events event-99 결손을 명시해야 함:\n{stderr}"
    );
    assert!(
        !f.db_path.exists() || db_count_eras(&f.db_path) == 0,
        "fail 시 eras 테이블에 row가 남으면 안 됨"
    );
}

#[test]
fn rejects_event_era_id_pointing_at_missing_id() {
    // Event.era_id 결손 — Phase 5a 텍스트 → Phase 5b 활성 회귀 가드.
    let f = Fixture::new("test-event-missing-era");
    f.seed_baseline();
    // event-x가 era-99 (미존재)를 참조.
    f.write(
        "event",
        "event-x.md",
        "---\nid: event-x\nkind: war\nname: X\nera_id: era-99\n---\n",
    );

    let out = f.run();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("events.era_id") && stderr.contains("era-99"),
        "stderr가 events.era_id 결손을 명시해야 함:\n{stderr}"
    );
}

#[test]
fn rejects_atlas_era_id_pointing_at_missing_id() {
    // Atlas.extras.era_id 결손 — Phase 4 텍스트 → Phase 5b 활성 회귀 가드.
    let f = Fixture::new("test-atlas-missing-era");
    f.seed_baseline();
    // atlas-x가 era-99 (미존재)를 extras.era_id로 참조.
    f.write(
        "atlas",
        "atlas-x.md",
        "---\nid: atlas-x\nkind: continent\nname: X\nextras:\n  era_id: era-99\nreferences: []\n---\n",
    );

    let out = f.run();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("atlases.extras.era_id") && stderr.contains("era-99"),
        "stderr가 atlases.extras.era_id 결손을 명시해야 함:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Positive path — boundary 정책 §3.3 적용한 정합 시드 (체크포인트 1 자동화)
// ---------------------------------------------------------------------------

#[test]
fn accepts_canonical_5_era_with_boundary_event() {
    // Phase 5b 체크포인트 1의 정합 시드를 fixture로 재현 — boundary -30(start inclusive)
    // event가 era-fall에 매핑.
    let f = Fixture::new("test-era-canonical");
    // Phase 5b 5 era 중 2개 + boundary event 1개.
    f.write(
        "place",
        "place-cap.md",
        "---\nid: place-cap\nlayer: settlement\nkind: nation\nname: Capital\n---\n",
    );
    f.write(
        "person",
        "npc-01.md",
        "---\nid: npc-01\nkind: active\nname: A\nstatus: alive\n---\n",
    );
    f.write(
        "era",
        "era-decline.md",
        "---\nid: era-decline\nkind: decline\nname: Decline\ntemporal:\n  start_year_relative: -70\n  end_year_relative: -30\n---\n",
    );
    f.write(
        "era",
        "era-fall.md",
        "---\nid: era-fall\nkind: fall\nname: Fall\ntemporal:\n  start_year_relative: -30\n  end_year_relative: 0\nkey_events:\n  - event-boundary\n---\n",
    );
    // boundary 케이스 — year_relative=-30은 era-fall(start inclusive)에 매핑.
    f.write(
        "event",
        "event-boundary.md",
        "---\nid: event-boundary\nkind: war\nname: Boundary\ntemporal:\n  year_relative: -30\nera_id: era-fall\n---\n",
    );

    let out = f.run();
    assert!(
        out.status.success(),
        "boundary 정합 시드는 통과해야 함 — stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(db_count_eras(&f.db_path), 2);
}

// ---------------------------------------------------------------------------
// Recovery path — Phase 5a 패턴 미러 (era-99 → 정정 → 통과)
// ---------------------------------------------------------------------------

#[test]
fn recovers_after_fixing_missing_era_id() {
    let f = Fixture::new("test-era-recovery");
    f.seed_baseline();

    // Step 1: 결손 era_id로 빌드 시도 → 실패.
    f.write(
        "event",
        "event-x.md",
        "---\nid: event-x\nkind: war\nname: X\nera_id: era-99\n---\n",
    );
    let out_fail = f.run();
    assert!(!out_fail.status.success(), "결손 era-99는 실패해야 함");
    let pre_count = if f.db_path.exists() {
        db_count_eras(&f.db_path)
    } else {
        0
    };
    assert_eq!(pre_count, 0, "fail 시 DB 미수정 — eras 0건");

    // Step 2: era_id 정정 후 재실행 → 성공.
    f.write(
        "event",
        "event-x.md",
        "---\nid: event-x\nkind: war\nname: X\nera_id: era-test\n---\n",
    );
    let out_ok = f.run();
    assert!(
        out_ok.status.success(),
        "복구 후 통과해야 함 — stderr:\n{}",
        String::from_utf8_lossy(&out_ok.stderr)
    );
    // baseline era-test 1건만 (era-bad는 만들지 않았음).
    assert_eq!(db_count_eras(&f.db_path), 1, "복구 후 eras 1건");
}
