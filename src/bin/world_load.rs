//! `world-load` — Phase 1·2 Worldbuilding ingest CLI.
//!
//! 사용법:
//!   cargo run --features embed --bin world-load -- --project chilguk-chunchu
//!   cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload
//!   cargo run --features embed --bin world-load -- --project chilguk-chunchu --no-mind
//!
//! 환경변수:
//!   NPC_MIND_WORLD_DB        SQLite 경로 오버라이드 (기본 projects/<id>/build/world.sqlite)
//!   NPC_MIND_WORLD_PROJECTS  프로젝트 루트 (기본 ./projects)
//!
//! Phase 2 동작:
//!   - `world/person/*.md` 스캔 → persons 테이블 upsert + FTS5
//!   - 외래키 검증 활성: Group.members.person_id ↔ persons.id, Person.affiliation ↔ groups.id
//!     결손 시 ERROR로 승급 (Phase 1엔 경고였음)
//!   - npc-mind 변환 dry-run: kind in {active, player}인 Person을 `Npc`로 변환 시도
//!     (HEXACO Score VO 범위 검증). 실제 mind store 등록은 mind-studio 시작 시점에 발생.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{
    Group, GroupId, Person, WorldError, detect_parent_group_cycle,
};
use npc_mind::worldbuilding::WorldRepository;
use npc_mind::worldbuilding::markdown::{group_from_markdown, person_from_markdown};
use npc_mind::worldbuilding::mind_sync::person_to_npc;

#[derive(Debug)]
struct Args {
    project: String,
    /// `--reload`: 기존 SQLite 파일을 삭제 후 재생성.
    reload: bool,
    db_override: Option<PathBuf>,
    projects_root: Option<PathBuf>,
    /// `--no-mind`: Person → Npc 변환 dry-run을 끔.
    no_mind: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut project: Option<String> = None;
    let mut reload = false;
    let mut db_override = None;
    let mut projects_root = None;
    let mut no_mind = false;
    let mut iter = std::env::args().skip(1);
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--project" => {
                project = Some(iter.next().ok_or("--project requires a value")?);
            }
            "--reload" => reload = true,
            "--no-mind" => no_mind = true,
            "--db" => {
                db_override = Some(iter.next().ok_or("--db requires a value")?.into());
            }
            "--projects-root" => {
                projects_root = Some(iter.next().ok_or("--projects-root requires a value")?.into());
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("Unknown argument: {other}")),
        }
    }
    Ok(Args {
        project: project.ok_or("--project <id> is required")?,
        reload,
        db_override,
        projects_root,
        no_mind,
    })
}

fn print_help() {
    println!(
        "world-load — Phase 1·2 Worldbuilding 인덱싱\n\n\
        USAGE:\n\
        \tcargo run --features embed --bin world-load -- --project <id> [--reload] [--no-mind] [--db <path>]\n\n\
        OPTIONS:\n\
        \t--project <id>       projects/<id>/ 하위를 ingest\n\
        \t--reload             기존 SQLite를 삭제 후 재생성\n\
        \t--no-mind            Person → Npc 변환 dry-run 비활성 (Phase 2)\n\
        \t--db <path>          SQLite 경로 오버라이드\n\
        \t--projects-root <p>  projects 디렉토리 루트 (기본 ./projects)"
    );
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("world-load 실패: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let projects_root = args
        .projects_root
        .or_else(|| std::env::var_os("NPC_MIND_WORLD_PROJECTS").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("projects"));
    let project_dir = projects_root.join(&args.project);
    if !project_dir.is_dir() {
        return Err(format!(
            "projects/<id> not found: {}",
            project_dir.display()
        ));
    }

    let project_toml = project_dir.join("project.toml");
    let project_meta = if project_toml.is_file() {
        std::fs::read_to_string(&project_toml).map_err(|e| e.to_string())?
    } else {
        eprintln!(
            "[world-load] warning: project.toml 없음 ({}). 기본 메타로 진행.",
            project_toml.display()
        );
        String::new()
    };
    let project_meta_genre = parse_project_genre(&project_meta);

    let db_path = args
        .db_override
        .clone()
        .or_else(|| std::env::var_os("NPC_MIND_WORLD_DB").map(PathBuf::from))
        .unwrap_or_else(|| project_dir.join("build").join("world.sqlite"));

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }

    if args.reload && db_path.is_file() {
        std::fs::remove_file(&db_path)
            .map_err(|e| format!("remove {}: {e}", db_path.display()))?;
    }

    println!("[world-load] project    = {}", args.project);
    println!("[world-load] genre      = {}", project_meta_genre.as_deref().unwrap_or("(unset)"));
    println!("[world-load] project_dir= {}", project_dir.display());
    println!("[world-load] db         = {}", db_path.display());

    let store = SqliteWorldStore::new(
        db_path
            .to_str()
            .ok_or_else(|| format!("UTF-8 변환 실패: {}", db_path.display()))?,
    )
    .map_err(|e| format!("{e:?}"))?;

    let group_dir = project_dir.join("world").join("group");
    let person_dir = project_dir.join("world").join("person");
    if !group_dir.is_dir() && !person_dir.is_dir() {
        eprintln!(
            "[world-load] warning: world/group/ · world/person/ 둘 다 없음 ({}, {}). 빈 인덱스로 마침.",
            group_dir.display(),
            person_dir.display()
        );
        return Ok(());
    }

    let mut groups: Vec<Group> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    if group_dir.is_dir() {
        for entry in walk_md(&group_dir).map_err(|e| e.to_string())? {
            let path = entry;
            let raw = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    errors.push(format!("{}: read failed — {e}", path.display()));
                    continue;
                }
            };
            match group_from_markdown(&raw) {
                Ok(mut g) => {
                    g.source_path = Some(path_relative_str(&projects_root, &path));
                    groups.push(g);
                }
                Err(e) => {
                    errors.push(format!("{}: {e}", path.display()));
                }
            }
        }
    } else {
        eprintln!(
            "[world-load] ℹ world/group/ 없음 ({}). Group 인덱싱 스킵.",
            group_dir.display()
        );
    }

    // Phase 2 — Person 스캔
    let mut persons: Vec<Person> = Vec::new();
    if person_dir.is_dir() {
        for entry in walk_md(&person_dir).map_err(|e| e.to_string())? {
            let path = entry;
            let raw = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    errors.push(format!("{}: read failed — {e}", path.display()));
                    continue;
                }
            };
            match person_from_markdown(&raw) {
                Ok(mut p) => {
                    p.source_path = Some(path_relative_str(&projects_root, &path));
                    persons.push(p);
                }
                Err(e) => {
                    errors.push(format!("{}: {e}", path.display()));
                }
            }
        }
    } else {
        eprintln!(
            "[world-load] ℹ world/person/ 없음 ({}). Person 인덱싱 스킵.",
            person_dir.display()
        );
    }

    if !errors.is_empty() {
        eprintln!("[world-load] {} 파일 파싱 실패:", errors.len());
        for e in &errors {
            eprintln!("  - {e}");
        }
    }

    // 검증을 upsert 앞으로 — 치명적 결함이 있으면 DB를 건드리지 않는다.
    // (이전 동작: upsert → 검증 순서로 partial-state DB가 남았음. Code review #3.)
    // parent_group cycle 검증 (경고)
    let cycles = detect_parent_group_cycle(&groups);
    if !cycles.is_empty() {
        eprintln!("[world-load] ⚠ parent_group cycle {} 건:", cycles.len());
        for c in &cycles {
            let path = c
                .iter()
                .map(|g| g.as_str())
                .collect::<Vec<_>>()
                .join(" → ");
            eprintln!("  - {path} → ({})", c[0]);
        }
    }

    // 외래키 결손 — Phase 1 같은 도메인 내 (경고)
    let id_set: HashSet<&str> = groups.iter().map(|g| g.id.as_str()).collect();
    let person_id_set: HashSet<&str> = persons.iter().map(|p| p.id.as_str()).collect();
    let mut missing_parents: Vec<(String, String)> = Vec::new();
    let mut missing_allied: Vec<(String, String)> = Vec::new();
    let mut missing_rival: Vec<(String, String)> = Vec::new();
    // Phase 2 — 외래키 활성화 (에러로 승급)
    let mut missing_member_persons: Vec<(String, String)> = Vec::new();
    let mut missing_affiliations: Vec<(String, String)> = Vec::new();
    // Phase 3 활성 예정 (Place) — 카운트만
    let mut pending_hq_refs: u64 = 0;
    let mut pending_birthplace_refs: u64 = 0;
    let mut pending_current_location_refs: u64 = 0;
    let mut allied_rival_overlap: Vec<(String, String)> = Vec::new();
    for g in &groups {
        if let Some(p) = &g.parent_group
            && !id_set.contains(p.as_str())
        {
            missing_parents.push((g.id.0.clone(), p.0.clone()));
        }
        for a in &g.allied_groups {
            if !id_set.contains(a.as_str()) {
                missing_allied.push((g.id.0.clone(), a.0.clone()));
            }
        }
        for r in &g.rival_groups {
            if !id_set.contains(r.as_str()) {
                missing_rival.push((g.id.0.clone(), r.0.clone()));
            }
        }
        // Phase 2 외래키 활성: members.person_id ↔ persons.id
        for m in &g.members {
            if let Some(pid) = &m.person_id
                && !pid.is_empty()
                && !person_id_set.contains(pid.as_str())
            {
                missing_member_persons.push((g.id.0.clone(), pid.clone()));
            }
        }
        if let Some(hq) = &g.headquarters
            && !hq.is_empty()
        {
            pending_hq_refs += 1;
            let _ = hq; // Phase 3에서 place 도메인 활성 시 검증 활성
        }
        let allied: HashSet<&GroupId> = g.allied_groups.iter().collect();
        for r in &g.rival_groups {
            if allied.contains(r) {
                allied_rival_overlap.push((g.id.0.clone(), r.0.clone()));
            }
        }
    }
    // Phase 2 외래키 활성: Person.affiliation ↔ groups.id
    for p in &persons {
        for a in &p.affiliation {
            if !id_set.contains(a.as_str()) {
                missing_affiliations.push((p.id.0.clone(), a.0.clone()));
            }
        }
        if p.birthplace.as_ref().is_some_and(|s| !s.is_empty()) {
            pending_birthplace_refs += 1;
        }
        if p.current_location.as_ref().is_some_and(|s| !s.is_empty()) {
            pending_current_location_refs += 1;
        }
    }

    print_warnings("parent_group", &missing_parents);
    print_warnings("allied_groups", &missing_allied);
    print_warnings("rival_groups", &missing_rival);

    // Phase 2 외래키 활성: 에러로 승급 — 결손 시 hard-fail.
    let fk_errors_total = missing_member_persons.len() + missing_affiliations.len();
    if !missing_member_persons.is_empty() {
        eprintln!(
            "[world-load] ✗ Phase 2 외래키 활성: groups.members.person_id 결손 {} 건:",
            missing_member_persons.len()
        );
        for (g, missing) in &missing_member_persons {
            eprintln!("  - {g}: person_id '{missing}' (persons.id에 없음)");
        }
    }
    if !missing_affiliations.is_empty() {
        eprintln!(
            "[world-load] ✗ Phase 2 외래키 활성: persons.affiliation 결손 {} 건:",
            missing_affiliations.len()
        );
        for (p, missing) in &missing_affiliations {
            eprintln!("  - {p}: affiliation '{missing}' (groups.id에 없음)");
        }
    }

    if pending_hq_refs > 0 {
        eprintln!(
            "[world-load] ℹ Phase 3(Place) 도입 예정 — headquarters {} 건은 텍스트 보존 (검증 비활성)",
            pending_hq_refs
        );
    }
    if pending_birthplace_refs > 0 || pending_current_location_refs > 0 {
        eprintln!(
            "[world-load] ℹ Phase 3(Place) 도입 예정 — birthplace {} 건, current_location {} 건은 텍스트 보존 (검증 비활성)",
            pending_birthplace_refs, pending_current_location_refs
        );
    }
    if !allied_rival_overlap.is_empty() {
        eprintln!(
            "[world-load] ⚠ allied/rival 모순 {} 건:",
            allied_rival_overlap.len()
        );
        for (g, r) in &allied_rival_overlap {
            eprintln!("  - {g}: {r}이(가) allied와 rival 둘 다에 있음");
        }
    }

    // rival 대칭성 경고
    let by_id: HashMap<&str, &Group> = groups.iter().map(|g| (g.id.as_str(), g)).collect();
    let mut asym: Vec<(String, String)> = Vec::new();
    for g in &groups {
        for r in &g.rival_groups {
            if let Some(other) = by_id.get(r.as_str()) {
                let other_rivals: HashSet<&GroupId> = other.rival_groups.iter().collect();
                if !other_rivals.contains(&g.id) {
                    asym.push((g.id.0.clone(), r.0.clone()));
                }
            }
        }
    }
    if !asym.is_empty() {
        eprintln!(
            "[world-load] ℹ rival 비대칭 {} 건 (일방적 적대 — 무협에서 흔함):",
            asym.len()
        );
        for (a, b) in &asym {
            eprintln!("  - {a} → {b} (역방향 미선언)");
        }
    }

    // Phase 2 — npc-mind 변환 dry-run.
    // active/player Person을 Npc로 변환 가능한지 검증 (HEXACO Score VO 범위 등).
    // 실제 mind store 등록은 mind-studio 부착 시점에 발생.
    let mut mind_eligible = 0u64;
    let mut mind_failures: Vec<(String, String)> = Vec::new();
    if !args.no_mind {
        for p in &persons {
            if p.is_mind_eligible() {
                match person_to_npc(p) {
                    Some(_) => mind_eligible += 1,
                    None => {
                        // is_mind_eligible() 통과 후에는 Some이어야 — 방어적.
                        mind_failures.push((p.id.0.clone(), "person_to_npc 실패".into()));
                    }
                }
            }
        }
        if !mind_failures.is_empty() {
            eprintln!(
                "[world-load] ✗ npc-mind 변환 실패 {} 건:",
                mind_failures.len()
            );
            for (id, e) in &mind_failures {
                eprintln!("  - {id}: {e}");
            }
        }
    }

    // 치명적 결함 조기 종료 — DB 미터치 유지.
    // (이전 동작은 upsert를 먼저 수행해 SQLite에 partial row가 남았다. Code review #3.)
    let fatal_parse = !errors.is_empty();
    let fatal_fk = fk_errors_total > 0;
    let fatal_mind = !args.no_mind && !mind_failures.is_empty();

    if fatal_parse || fatal_fk || fatal_mind {
        // 진단 위주 result 블록 — DB가 미수정임을 명시.
        println!("\n=== 결과 (DB 미수정) ===");
        println!("project           = {}", args.project);
        println!("groups parsed     = {}", groups.len());
        println!("persons parsed    = {}", persons.len());
        println!("errors            = {}", errors.len());
        println!("cycles            = {}", cycles.len());
        println!("fk errors (활성)  = {fk_errors_total}");
        if !args.no_mind {
            println!("mind failures     = {}", mind_failures.len());
        } else {
            println!("mind eligible     = (--no-mind: 비활성)");
        }

        if fatal_parse {
            eprintln!(
                "\n[world-load] ✗ {} 파일 파싱 실패 — DB는 미수정 상태이며 기존 인덱스가 그대로 유지됩니다. \
                 오류 수정 후 재실행하세요.",
                errors.len()
            );
            return Err(format!("{} 파일 파싱 실패 (DB unchanged)", errors.len()));
        }
        if fatal_fk {
            return Err(format!(
                "{} 외래키 결손 — Phase 2 활성. DB 미수정. .md 수정 후 재실행하세요.",
                fk_errors_total
            ));
        }
        if fatal_mind {
            return Err(format!(
                "{} 인물의 npc-mind 변환 실패 — DB 미수정. HEXACO 범위 점검 후 재실행.",
                mind_failures.len()
            ));
        }
        unreachable!("fatal_* 위 세 분기가 모든 case를 cover");
    }

    // 모든 검증 통과 — 이제 upsert. 부분 실패가 발생하면(SQLite IO 오류 등) 그 시점까지의
    // row가 DB에 남으나 검증 단계 자체는 통과한 상태이므로 partial-state는 SQLite IO 자체의
    // 신뢰성에 의존한다 (atomic ingest는 별도 task로 분리 예정).
    for g in &groups {
        store
            .upsert_group(&args.project, g)
            .map_err(|e: WorldError| format!("upsert group {}: {e}", g.id))?;
    }
    for p in &persons {
        store
            .upsert_person(&args.project, p)
            .map_err(|e: WorldError| format!("upsert person {}: {e}", p.id))?;
    }

    // 최종 카운트 + 결과 출력 — upsert 완료 후의 인덱스 상태.
    let group_total = store
        .count_groups(Some(&args.project))
        .map_err(|e| format!("count groups: {e:?}"))?;
    let person_total = store
        .count_persons(Some(&args.project))
        .map_err(|e| format!("count persons: {e:?}"))?;

    println!("\n=== 결과 ===");
    println!("project           = {}", args.project);
    println!("groups indexed    = {group_total}");
    println!("persons indexed   = {person_total}");
    println!("groups parsed     = {}", groups.len());
    println!("persons parsed    = {}", persons.len());
    println!("errors            = {}", errors.len());
    println!("cycles            = {}", cycles.len());
    println!("fk errors (활성)  = {fk_errors_total}");
    if !args.no_mind {
        println!("mind eligible     = {mind_eligible}");
    } else {
        println!("mind eligible     = (--no-mind: 비활성)");
    }
    Ok(())
}

fn print_warnings(label: &str, items: &[(String, String)]) {
    if items.is_empty() {
        return;
    }
    eprintln!("[world-load] ⚠ {} 결손 {} 건:", label, items.len());
    for (g, missing) in items {
        eprintln!("  - {g}: {missing} (미정의)");
    }
}

/// `world/group/` 하위 .md 파일을 (한 단계만) 수집. 재귀 X — Phase 1 단순.
/// 하위 디렉토리가 발견되면 stderr로 경고 (재귀가 아니라 무시되었음을 알림).
fn walk_md(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut subdirs: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            out.push(path);
        } else if path.is_dir() {
            subdirs.push(path);
        }
    }
    out.sort();
    if !subdirs.is_empty() {
        eprintln!(
            "[world-load] ℹ {} 하위 디렉토리 {} 개 무시 (Phase 1 walk_md 비-재귀):",
            dir.display(),
            subdirs.len()
        );
        for d in &subdirs {
            eprintln!("  - {}", d.display());
        }
    }
    Ok(out)
}

fn path_relative_str(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

/// project.toml에서 `genre = "..."` 한 줄만 추출. TOML 파싱 실패는 stderr로 보고하되
/// fatal로 취급하지 않는다 (CLI는 빈 메타로 진행).
fn parse_project_genre(raw: &str) -> Option<String> {
    if raw.trim().is_empty() {
        return None;
    }
    match toml::from_str::<toml::Value>(raw) {
        Ok(v) => v
            .as_table()?
            .get("genre")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        Err(e) => {
            eprintln!("[world-load] ⚠ project.toml 파싱 실패 — genre 추출 불가: {e}");
            None
        }
    }
}
