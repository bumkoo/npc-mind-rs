//! `world-load` CLI의 외래키 거부 경로(negative paths) 테스트.
//!
//! Phase 3에서 활성된 hard-fail 검증들이 실제로 fail로 떨어지는지 — 그리고 DB가
//! 미수정 상태로 남는지 — 실제 바이너리를 spawn해 검증한다. 도메인 모델·SqliteWorldStore의
//! invariant는 라이브러리 단위 테스트가 책임지지만, 통합 진입점인 world-load CLI의
//! 거부 경로는 별도로 가드.
//!
//! 실행: `cargo test --features embed --test world_load_fk_negative`

#![cfg(feature = "embed")]

use std::path::PathBuf;
use std::process::Command;

/// `cargo` 가 통합 테스트 빌드 시 자동으로 주입하는 world-load 바이너리 절대 경로.
/// `Cargo.toml`의 `[[bin]] name = "world-load"`와 매칭.
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
        std::fs::create_dir_all(project_dir.join("world").join("place")).unwrap();
        let db_path = project_dir.join("build").join("world.sqlite");
        Self {
            _tmp: tmp,
            project_dir,
            projects_root,
            project_name: name.to_string(),
            db_path,
        }
    }

    fn write_place(&self, filename: &str, contents: &str) {
        std::fs::write(
            self.project_dir.join("world").join("place").join(filename),
            contents,
        )
        .unwrap();
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

#[test]
fn rejects_geography_refs_pointing_at_settlement_layer() {
    // settlement → geography_refs target이 settlement layer면 거부.
    // (의미상 "정치체가 자연 지형 위에 layered"여야 하므로 target은 Geography 강제.)
    let f = Fixture::new("test-layer-mismatch");
    f.write_place(
        "place-a.md",
        "---\nid: place-a\nlayer: settlement\nkind: nation\nname: A\n---\n",
    );
    f.write_place(
        "place-b.md",
        "---\nid: place-b\nlayer: settlement\nkind: nation\nname: B\nspatial:\n  geography_refs:\n    - place-a\n---\n",
    );

    let out = f.run();
    assert!(
        !out.status.success(),
        "world-load should fail on layer mismatch — stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("geography_refs") && stderr.contains("layer 불일치"),
        "stderr가 geography_refs layer 불일치를 명시해야 함:\n{stderr}"
    );
    // 거부 후 DB는 미수정 — places 테이블 자체가 미생성이어야 함.
    // (검증 단계 중 fatal_fk가 발생하면 upsert 자체를 건너뛴다.)
    assert!(
        !f.db_path.exists() || db_count_places(&f.db_path) == 0,
        "fail 시 places 테이블에 row가 없어야 함"
    );
}

#[test]
fn rejects_geography_refs_pointing_at_missing_id() {
    // settlement → geography_refs target이 places에 없으면 거부.
    let f = Fixture::new("test-missing-geo");
    f.write_place(
        "place-x.md",
        "---\nid: place-x\nlayer: settlement\nkind: nation\nname: X\nspatial:\n  geography_refs:\n    - place-nope\n---\n",
    );

    let out = f.run();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("geography_refs") && stderr.contains("결손"),
        "stderr가 geography_refs 결손을 명시해야 함:\n{stderr}"
    );
}

#[test]
fn accepts_geography_refs_pointing_at_geography_layer() {
    // 정상 흐름 — settlement → geography_refs target이 Geography이면 통과.
    let f = Fixture::new("test-layer-ok");
    f.write_place(
        "place-mt.md",
        "---\nid: place-mt\nlayer: geography\nkind: mountain-range\nname: Mt\n---\n",
    );
    f.write_place(
        "place-s.md",
        "---\nid: place-s\nlayer: settlement\nkind: nation\nname: S\nspatial:\n  geography_refs:\n    - place-mt\n---\n",
    );

    let out = f.run();
    assert!(
        out.status.success(),
        "올바른 layer reference는 통과해야 함 — stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(db_count_places(&f.db_path), 2);
}

fn db_count_places(path: &PathBuf) -> i64 {
    let conn = rusqlite::Connection::open(path).expect("open sqlite");
    conn.query_row("SELECT COUNT(*) FROM places", [], |r| r.get(0))
        .unwrap_or(0)
}
