//! `world-load` — Phase 1 Worldbuilding ingest CLI.
//!
//! 사용법:
//!   cargo run --features embed --bin world-load -- --project chilguk-chunchu
//!   cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload
//!
//! 환경변수:
//!   NPC_MIND_WORLD_DB        SQLite 경로 오버라이드 (기본 projects/<id>/build/world.sqlite)
//!   NPC_MIND_WORLD_PROJECTS  프로젝트 루트 (기본 ./projects)

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{Group, GroupId, WorldError, detect_parent_group_cycle};
use npc_mind::worldbuilding::WorldRepository;
use npc_mind::worldbuilding::markdown::group_from_markdown;

#[derive(Debug)]
struct Args {
    project: String,
    /// `--reload`: 기존 SQLite 파일을 삭제 후 재생성.
    reload: bool,
    db_override: Option<PathBuf>,
    projects_root: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut project: Option<String> = None;
    let mut reload = false;
    let mut db_override = None;
    let mut projects_root = None;
    let mut iter = std::env::args().skip(1);
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--project" => {
                project = Some(iter.next().ok_or("--project requires a value")?);
            }
            "--reload" => reload = true,
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
    })
}

fn print_help() {
    println!(
        "world-load — Phase 1 Worldbuilding 인덱싱\n\n\
        USAGE:\n\
        \tcargo run --features embed --bin world-load -- --project <id> [--reload] [--db <path>]\n\n\
        OPTIONS:\n\
        \t--project <id>       projects/<id>/ 하위를 ingest\n\
        \t--reload             기존 SQLite를 삭제 후 재생성\n\
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
    if !group_dir.is_dir() {
        eprintln!(
            "[world-load] warning: world/group/ 없음 ({}). 빈 인덱스로 마침.",
            group_dir.display()
        );
        return Ok(());
    }

    let mut groups: Vec<Group> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
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

    if !errors.is_empty() {
        eprintln!("[world-load] {} 파일 파싱 실패:", errors.len());
        for e in &errors {
            eprintln!("  - {e}");
        }
    }

    // upsert
    for g in &groups {
        store
            .upsert_group(&args.project, g)
            .map_err(|e: WorldError| format!("upsert {}: {e}", g.id))?;
    }

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

    // 외래키 결손 경고 (Phase 1 — 같은 도메인 내만 검증)
    let id_set: HashSet<&str> = groups.iter().map(|g| g.id.as_str()).collect();
    let mut missing_parents: Vec<(String, String)> = Vec::new();
    let mut missing_allied: Vec<(String, String)> = Vec::new();
    let mut missing_rival: Vec<(String, String)> = Vec::new();
    // 도메인 외 외래키(Person/Place)는 Phase 2/3에서 활성. Phase 1엔 전체 카운트만 보고.
    // — `pending_member_refs`/`pending_hq_refs`는 누락 검출이 아니라 **모든** 참조 카운트다.
    let mut pending_member_refs: u64 = 0;
    let mut pending_hq_refs: u64 = 0;
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
        for m in &g.members {
            if let Some(pid) = &m.person_id
                && !pid.is_empty()
            {
                pending_member_refs += 1;
                let _ = pid; // Phase 2에서 person 도메인 활성 시 검증 활성
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
    print_warnings("parent_group", &missing_parents);
    print_warnings("allied_groups", &missing_allied);
    print_warnings("rival_groups", &missing_rival);
    if pending_member_refs > 0 {
        eprintln!(
            "[world-load] ℹ Phase 2(Person) 도입 예정 — members.person_id {} 건은 텍스트 보존 (검증 비활성)",
            pending_member_refs
        );
    }
    if pending_hq_refs > 0 {
        eprintln!(
            "[world-load] ℹ Phase 3(Place) 도입 예정 — headquarters {} 건은 텍스트 보존 (검증 비활성)",
            pending_hq_refs
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

    let total = store
        .count_groups(Some(&args.project))
        .map_err(|e| format!("count: {e:?}"))?;
    println!("\n=== 결과 ===");
    println!("project           = {}", args.project);
    println!("groups indexed    = {total}");
    println!("parsed (this run) = {}", groups.len());
    println!("errors            = {}", errors.len());
    println!("cycles            = {}", cycles.len());

    if !errors.is_empty() {
        // 부분 실패 시 SQLite는 성공한 파일만 적재된 partial 상태이다.
        // 사용자가 일관된 인덱스를 원하면 `--reload`로 재실행하거나 `.md` 오류를 고친 뒤 재실행.
        eprintln!(
            "\n[world-load] ⚠ {} 파일 파싱 실패 — DB는 partial 상태일 수 있음. \
             오류 수정 후 `--reload`로 재실행하면 일관된 인덱스 보장.",
            errors.len()
        );
        return Err(format!(
            "{} 파일 파싱 실패 (DB partial; 위 가이드 참조)",
            errors.len()
        ));
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
