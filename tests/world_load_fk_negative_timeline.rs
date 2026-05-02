//! `world-load` CLI의 Phase 5b 체크포인트 2 Timeline 외래키 거부 경로 테스트.
//! Phase 5a N1 패턴 미러 + Phase 5b 체크포인트 1 era 패턴 미러.
//!
//! 검증 항목 (사양 §3.2 + Phase 5b 체크포인트 2 활성 외래키):
//! - `Timeline.references` ↔ `eras.id` (모두 존재해야)
//! - `Timeline.references` 중복 금지 (timeline_era_refs composite PK 보호)
//! - 정상 흐름 — 5 era references 정합 시 ingest 성공
//! - recovery — 결손 → DB 미수정 → 정정 → 통과
//!
//! 실행: `cargo test --features embed --test world_load_fk_negative_timeline`

#![cfg(feature = "embed")]

use std::path::PathBuf;
use std::process::Command;

const WORLD_LOAD_BIN: &str = env!("CARGO_BIN_EXE_world-load");

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
        for sub in ["group", "person", "place", "atlas", "event", "era", "timeline"] {
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

    /// baseline — 1 era만. 각 fixture에서 timeline 파일을 작성.
    fn seed_baseline(&self) {
        self.write(
            "era",
            "era-test.md",
            "---\nid: era-test\nkind: fall\nname: Test Era\ntemporal:\n  start_year_relative: -30\n  end_year_relative: 0\n---\n",
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

fn db_count_timelines(path: &PathBuf) -> i64 {
    let conn = rusqlite::Connection::open(path).expect("open sqlite");
    conn.query_row("SELECT COUNT(*) FROM timelines", [], |r| r.get(0))
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Negative paths — references 결손 + 중복 (composite PK 보호)
// ---------------------------------------------------------------------------

#[test]
fn rejects_timeline_references_pointing_at_missing_era() {
    let f = Fixture::new("test-timeline-missing-era");
    f.seed_baseline();
    f.write(
        "timeline",
        "timeline-x.md",
        "---\nid: timeline-x\nkind: history\nname: X\nreferences:\n  - era-test\n  - era-99\n---\n",
    );

    let out = f.run();
    assert!(
        !out.status.success(),
        "world-load should fail on missing era-99 — stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("timelines.references") && stderr.contains("era-99") && stderr.contains("결손"),
        "stderr가 timelines.references era-99 결손을 명시해야 함:\n{stderr}"
    );
    assert!(
        !f.db_path.exists() || db_count_timelines(&f.db_path) == 0,
        "fail 시 timelines 테이블에 row가 남으면 안 됨"
    );
}

#[test]
fn rejects_duplicate_references() {
    // composite PK (timeline_id, era_id) 보호 — 같은 era 중복 시 차단.
    let f = Fixture::new("test-timeline-duplicate-refs");
    f.seed_baseline();
    f.write(
        "timeline",
        "timeline-x.md",
        "---\nid: timeline-x\nkind: history\nname: X\nreferences:\n  - era-test\n  - era-test\n---\n",
    );

    let out = f.run();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("timelines.references") && stderr.contains("중복") && stderr.contains("era-test"),
        "stderr가 references 중복을 명시해야 함:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Positive path — Phase 5b 5 era references 정합
// ---------------------------------------------------------------------------

#[test]
fn accepts_timeline_with_all_five_era_references() {
    let f = Fixture::new("test-timeline-canonical");
    // Phase 5b 5 era 정합 시드.
    for (id, kind, start, end) in [
        ("era-founding", "founding", -270, -220),
        ("era-prosperity", "prosperity", -220, -150),
        ("era-turning", "turning", -150, -70),
        ("era-decline", "decline", -70, -30),
        ("era-fall-of-empire", "fall", -30, 0),
    ] {
        f.write(
            "era",
            &format!("{id}.md"),
            &format!("---\nid: {id}\nkind: {kind}\nname: {id}\ntemporal:\n  start_year_relative: {start}\n  end_year_relative: {end}\n---\n"),
        );
    }
    f.write(
        "timeline",
        "timeline-jungwon.md",
        "---\nid: timeline-jungwon\nkind: history\nname: 270년사\nreferences:\n  - era-founding\n  - era-prosperity\n  - era-turning\n  - era-decline\n  - era-fall-of-empire\n---\n",
    );

    let out = f.run();
    assert!(
        out.status.success(),
        "정합 5 era references는 통과해야 함 — stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(db_count_timelines(&f.db_path), 1);
}

// ---------------------------------------------------------------------------
// Recovery path — Phase 5a·5b N1 패턴 일관
// ---------------------------------------------------------------------------

#[test]
fn recovers_after_fixing_missing_reference() {
    let f = Fixture::new("test-timeline-recovery");
    f.seed_baseline();

    // Step 1: 결손 era로 빌드 시도 → 실패.
    f.write(
        "timeline",
        "timeline-x.md",
        "---\nid: timeline-x\nkind: history\nname: X\nreferences:\n  - era-99\n---\n",
    );
    let out_fail = f.run();
    assert!(!out_fail.status.success(), "결손 era-99는 실패해야 함");
    let pre_count = if f.db_path.exists() {
        db_count_timelines(&f.db_path)
    } else {
        0
    };
    assert_eq!(pre_count, 0, "fail 시 DB 미수정 — timelines 0건");

    // Step 2: era_id 정정 후 재실행 → 성공.
    f.write(
        "timeline",
        "timeline-x.md",
        "---\nid: timeline-x\nkind: history\nname: X\nreferences:\n  - era-test\n---\n",
    );
    let out_ok = f.run();
    assert!(
        out_ok.status.success(),
        "복구 후 통과해야 함 — stderr:\n{}",
        String::from_utf8_lossy(&out_ok.stderr)
    );
    assert_eq!(db_count_timelines(&f.db_path), 1, "복구 후 timelines 1건");
}
