//! `world-load` — Phase 1·2·3·4 Worldbuilding ingest CLI.
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
//! Phase 3 동작:
//!   - `world/place/*.md` 스캔 → places 테이블 upsert + FTS5
//!   - 외래키 검증 활성 (에러 승급, Phase 1·2의 경고/보류에서 승급):
//!     - `Group.headquarters` ↔ `places.id`
//!     - `Person.birthplace`/`current_location` ↔ `places.id`
//!     - `Place.spatial.parent_place` cycle (같은 도메인 내)
//!     - `Place.spatial.bordering_places`/`geography_refs` 존재
//!     - `Place.spatial.geography_refs` layer 일치 (target이 `Geography`이어야)
//!     - `Place.extras.controlling_group` (sect kind만) ↔ `groups.id`
//!   - 결손 시 partial commit 방지 — DB 미수정 유지 (Phase 1·2 정책 그대로)
//!
//! Phase 4 동작 (Atlas — 첫 관계 도메인):
//!   - `world/atlas/*.md` 스캔 → atlases 테이블 upsert + FTS5 + place_atlas_refs 양방향 인덱스
//!   - 외래키 검증 활성 (에러):
//!     - `Atlas.references` ↔ `places.id` (모두 존재해야 — references = atlas의 핵심)
//!     - `Atlas.references` 중복 금지 (place_atlas_refs composite PK 위반 방지)
//!   - `Atlas.extras.era_id`는 텍스트만 보존 (Phase 5 Era 도메인 진입 시 외래키 활성)

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{
    Atlas, Group, GroupId, Person, Place, PlaceLayer, WorldError, detect_parent_group_cycle,
    detect_parent_place_cycle,
};
use npc_mind::worldbuilding::WorldRepository;
use npc_mind::worldbuilding::markdown::{
    atlas_from_markdown, group_from_markdown, person_from_markdown, place_from_markdown,
};
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
    let place_dir = project_dir.join("world").join("place");
    let atlas_dir = project_dir.join("world").join("atlas");
    if !group_dir.is_dir() && !person_dir.is_dir() && !place_dir.is_dir() && !atlas_dir.is_dir() {
        eprintln!(
            "[world-load] warning: world/group/ · world/person/ · world/place/ · world/atlas/ 모두 없음. 빈 인덱스로 마침."
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

    // Phase 3 — Place 스캔
    let mut places: Vec<Place> = Vec::new();
    if place_dir.is_dir() {
        for entry in walk_md(&place_dir).map_err(|e| e.to_string())? {
            let path = entry;
            let raw = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    errors.push(format!("{}: read failed — {e}", path.display()));
                    continue;
                }
            };
            match place_from_markdown(&raw) {
                Ok(mut p) => {
                    p.source_path = Some(path_relative_str(&projects_root, &path));
                    places.push(p);
                }
                Err(e) => {
                    errors.push(format!("{}: {e}", path.display()));
                }
            }
        }
    } else {
        eprintln!(
            "[world-load] ℹ world/place/ 없음 ({}). Place 인덱싱 스킵.",
            place_dir.display()
        );
    }

    // Phase 4 — Atlas 스캔
    let mut atlases: Vec<Atlas> = Vec::new();
    if atlas_dir.is_dir() {
        for entry in walk_md(&atlas_dir).map_err(|e| e.to_string())? {
            let path = entry;
            let raw = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    errors.push(format!("{}: read failed — {e}", path.display()));
                    continue;
                }
            };
            match atlas_from_markdown(&raw) {
                Ok(mut a) => {
                    a.source_path = Some(path_relative_str(&projects_root, &path));
                    atlases.push(a);
                }
                Err(e) => {
                    errors.push(format!("{}: {e}", path.display()));
                }
            }
        }
    } else {
        eprintln!(
            "[world-load] ℹ world/atlas/ 없음 ({}). Atlas 인덱싱 스킵.",
            atlas_dir.display()
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

    // Phase 3 — Place parent_place cycle 검증 (에러)
    let place_cycles = detect_parent_place_cycle(&places);
    if !place_cycles.is_empty() {
        eprintln!("[world-load] ✗ parent_place cycle {} 건:", place_cycles.len());
        for c in &place_cycles {
            let path = c
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(" → ");
            eprintln!("  - {path} → ({})", c[0]);
        }
    }

    // 외래키 결손 — 같은 도메인 내 (Phase 1 경고 그대로)
    let id_set: HashSet<&str> = groups.iter().map(|g| g.id.as_str()).collect();
    let person_id_set: HashSet<&str> = persons.iter().map(|p| p.id.as_str()).collect();
    // Phase 3 신규: place id 집합 + layer lookup (geography_refs 검증용)
    let place_id_set: HashSet<&str> = places.iter().map(|p| p.id.as_str()).collect();
    let place_layer_by_id: HashMap<&str, PlaceLayer> =
        places.iter().map(|p| (p.id.as_str(), p.layer)).collect();

    let mut missing_parents: Vec<(String, String)> = Vec::new();
    let mut missing_allied: Vec<(String, String)> = Vec::new();
    let mut missing_rival: Vec<(String, String)> = Vec::new();
    // Phase 2 — 외래키 활성 (에러)
    let mut missing_member_persons: Vec<(String, String)> = Vec::new();
    let mut missing_affiliations: Vec<(String, String)> = Vec::new();
    // Phase 3 — 외래키 활성 (에러로 승급)
    let mut missing_hq: Vec<(String, String)> = Vec::new();
    let mut missing_birthplace: Vec<(String, String)> = Vec::new();
    let mut missing_current_location: Vec<(String, String)> = Vec::new();
    let mut missing_place_parent: Vec<(String, String)> = Vec::new();
    let mut missing_bordering: Vec<(String, String)> = Vec::new();
    let mut missing_geography: Vec<(String, String)> = Vec::new();
    // geography_refs target이 settlement layer면 layer mismatch (해당 ref가 자연 지형이 아님).
    let mut geography_layer_mismatch: Vec<(String, String)> = Vec::new();
    let mut missing_controlling_group: Vec<(String, String)> = Vec::new();
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
        // Phase 3 외래키 활성: Group.headquarters ↔ places.id
        if let Some(hq) = &g.headquarters
            && !hq.is_empty()
            && !place_id_set.contains(hq.as_str())
        {
            missing_hq.push((g.id.0.clone(), hq.clone()));
        }
        let allied: HashSet<&GroupId> = g.allied_groups.iter().collect();
        for r in &g.rival_groups {
            if allied.contains(r) {
                allied_rival_overlap.push((g.id.0.clone(), r.0.clone()));
            }
        }
    }
    // Phase 2 외래키 활성: Person.affiliation ↔ groups.id
    // Phase 3 외래키 활성: Person.birthplace/current_location ↔ places.id
    for p in &persons {
        for a in &p.affiliation {
            if !id_set.contains(a.as_str()) {
                missing_affiliations.push((p.id.0.clone(), a.0.clone()));
            }
        }
        if let Some(bp) = &p.birthplace
            && !bp.is_empty()
            && !place_id_set.contains(bp.as_str())
        {
            missing_birthplace.push((p.id.0.clone(), bp.clone()));
        }
        if let Some(cl) = &p.current_location
            && !cl.is_empty()
            && !place_id_set.contains(cl.as_str())
        {
            missing_current_location.push((p.id.0.clone(), cl.clone()));
        }
    }
    // Phase 3 외래키 활성: Place.spatial.* + extras.controlling_group
    for pl in &places {
        if let Some(parent) = &pl.spatial.parent_place
            && !place_id_set.contains(parent.as_str())
        {
            missing_place_parent.push((pl.id.0.clone(), parent.0.clone()));
        }
        for b in &pl.spatial.bordering_places {
            if !place_id_set.contains(b.as_str()) {
                missing_bordering.push((pl.id.0.clone(), b.0.clone()));
            }
        }
        for gref in &pl.spatial.geography_refs {
            match place_layer_by_id.get(gref.as_str()) {
                None => missing_geography.push((pl.id.0.clone(), gref.0.clone())),
                Some(PlaceLayer::Settlement) => {
                    geography_layer_mismatch.push((pl.id.0.clone(), gref.0.clone()));
                }
                Some(PlaceLayer::Geography) => {}
            }
        }
        // sect kind만 controlling_group 외래키 검증. 다른 kind(예: nation)에서
        // controlling_group 텍스트가 명시되어도 Phase 3에선 검증하지 않는다 — sect
        // 이중 등록이 명확한 양방향 외래키(Place ↔ Group)인 반면 nation의
        // controlling_group은 통치 주체 메모 성격이라 Phase 1·2 텍스트 보존과 결이
        // 같다. 비-sect 검증 확장은 Phase 5+에서 검토.
        if pl.kind == "sect"
            && let Some(cg) = pl.controlling_group()
            && !cg.is_empty()
            && !id_set.contains(cg)
        {
            missing_controlling_group.push((pl.id.0.clone(), cg.to_string()));
        }
    }

    // Phase 4 — Atlas 외래키 활성: references ↔ places.id (모두 존재) + 중복 금지.
    let mut missing_atlas_refs: Vec<(String, String)> = Vec::new();
    let mut duplicate_atlas_refs: Vec<(String, String)> = Vec::new();
    for at in &atlases {
        let mut seen: HashSet<&str> = HashSet::new();
        for pid in &at.references {
            if !place_id_set.contains(pid.as_str()) {
                missing_atlas_refs.push((at.id.0.clone(), pid.0.clone()));
            }
            if !seen.insert(pid.as_str()) {
                duplicate_atlas_refs.push((at.id.0.clone(), pid.0.clone()));
            }
        }
    }

    print_warnings("parent_group", &missing_parents);
    print_warnings("allied_groups", &missing_allied);
    print_warnings("rival_groups", &missing_rival);

    // Phase 2 외래키 활성: 에러로 승급 — 결손 시 hard-fail.
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
    // Phase 3 외래키 활성 — 모두 hard-fail.
    if !missing_hq.is_empty() {
        eprintln!(
            "[world-load] ✗ Phase 3 외래키 활성: groups.headquarters 결손 {} 건:",
            missing_hq.len()
        );
        for (g, missing) in &missing_hq {
            eprintln!("  - {g}: headquarters '{missing}' (places.id에 없음)");
        }
    }
    if !missing_birthplace.is_empty() {
        eprintln!(
            "[world-load] ✗ Phase 3 외래키 활성: persons.birthplace 결손 {} 건:",
            missing_birthplace.len()
        );
        for (p, missing) in &missing_birthplace {
            eprintln!("  - {p}: birthplace '{missing}' (places.id에 없음)");
        }
    }
    if !missing_current_location.is_empty() {
        eprintln!(
            "[world-load] ✗ Phase 3 외래키 활성: persons.current_location 결손 {} 건:",
            missing_current_location.len()
        );
        for (p, missing) in &missing_current_location {
            eprintln!("  - {p}: current_location '{missing}' (places.id에 없음)");
        }
    }
    if !missing_place_parent.is_empty() {
        eprintln!(
            "[world-load] ✗ Phase 3 외래키 활성: places.spatial.parent_place 결손 {} 건:",
            missing_place_parent.len()
        );
        for (p, missing) in &missing_place_parent {
            eprintln!("  - {p}: parent_place '{missing}' (places.id에 없음)");
        }
    }
    if !missing_bordering.is_empty() {
        eprintln!(
            "[world-load] ✗ Phase 3 외래키 활성: places.spatial.bordering_places 결손 {} 건:",
            missing_bordering.len()
        );
        for (p, missing) in &missing_bordering {
            eprintln!("  - {p}: bordering_places '{missing}' (places.id에 없음)");
        }
    }
    if !missing_geography.is_empty() {
        eprintln!(
            "[world-load] ✗ Phase 3 외래키 활성: places.spatial.geography_refs 결손 {} 건:",
            missing_geography.len()
        );
        for (p, missing) in &missing_geography {
            eprintln!("  - {p}: geography_refs '{missing}' (places.id에 없음)");
        }
    }
    if !geography_layer_mismatch.is_empty() {
        eprintln!(
            "[world-load] ✗ Phase 3 외래키 활성: places.spatial.geography_refs layer 불일치 {} 건 (target이 geography이어야):",
            geography_layer_mismatch.len()
        );
        for (p, target) in &geography_layer_mismatch {
            eprintln!("  - {p}: geography_refs '{target}' (layer=settlement, 자연 지형 아님)");
        }
    }
    if !missing_controlling_group.is_empty() {
        eprintln!(
            "[world-load] ✗ Phase 3 외래키 활성: places.extras.controlling_group(sect) 결손 {} 건:",
            missing_controlling_group.len()
        );
        for (p, missing) in &missing_controlling_group {
            eprintln!("  - {p}: controlling_group '{missing}' (groups.id에 없음)");
        }
    }
    if !missing_atlas_refs.is_empty() {
        eprintln!(
            "[world-load] ✗ Phase 4 외래키 활성: atlases.references 결손 {} 건:",
            missing_atlas_refs.len()
        );
        for (a, missing) in &missing_atlas_refs {
            eprintln!("  - {a}: references '{missing}' (places.id에 없음)");
        }
    }
    if !duplicate_atlas_refs.is_empty() {
        eprintln!(
            "[world-load] ✗ Phase 4 데이터 결함: atlases.references 중복 {} 건 (place_atlas_refs PK 위반):",
            duplicate_atlas_refs.len()
        );
        for (a, dup) in &duplicate_atlas_refs {
            eprintln!("  - {a}: references '{dup}' (배열 내 중복)");
        }
    }

    let fk_errors_total = missing_member_persons.len()
        + missing_affiliations.len()
        + missing_hq.len()
        + missing_birthplace.len()
        + missing_current_location.len()
        + missing_place_parent.len()
        + missing_bordering.len()
        + missing_geography.len()
        + geography_layer_mismatch.len()
        + missing_controlling_group.len()
        + missing_atlas_refs.len()
        + duplicate_atlas_refs.len();
    let cycle_errors_total = place_cycles.len();
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
    let fatal_cycle = cycle_errors_total > 0;
    let fatal_mind = !args.no_mind && !mind_failures.is_empty();

    if fatal_parse || fatal_fk || fatal_cycle || fatal_mind {
        // 진단 위주 result 블록 — DB가 미수정임을 명시.
        println!("\n=== 결과 (DB 미수정) ===");
        println!("project           = {}", args.project);
        println!("groups parsed     = {}", groups.len());
        println!("persons parsed    = {}", persons.len());
        println!("places parsed     = {}", places.len());
        println!("atlases parsed    = {}", atlases.len());
        println!("errors            = {}", errors.len());
        println!("group cycles      = {}", cycles.len());
        println!("place cycles      = {}", place_cycles.len());
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
        if fatal_cycle {
            return Err(format!(
                "{} parent_place cycle — Phase 3 활성. DB 미수정. .md 수정 후 재실행하세요.",
                cycle_errors_total
            ));
        }
        if fatal_fk {
            return Err(format!(
                "{} 외래키 결손 — Phase 2·3 활성. DB 미수정. .md 수정 후 재실행하세요.",
                fk_errors_total
            ));
        }
        if fatal_mind {
            return Err(format!(
                "{} 인물의 npc-mind 변환 실패 — DB 미수정. HEXACO 범위 점검 후 재실행.",
                mind_failures.len()
            ));
        }
        unreachable!("fatal_* 위 네 분기가 모든 case를 cover");
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
    for pl in &places {
        store
            .upsert_place(&args.project, pl)
            .map_err(|e: WorldError| format!("upsert place {}: {e}", pl.id))?;
    }
    for at in &atlases {
        store
            .upsert_atlas(&args.project, at)
            .map_err(|e: WorldError| format!("upsert atlas {}: {e}", at.id))?;
    }

    // 최종 카운트 + 결과 출력 — upsert 완료 후의 인덱스 상태.
    let group_total = store
        .count_groups(Some(&args.project))
        .map_err(|e| format!("count groups: {e:?}"))?;
    let person_total = store
        .count_persons(Some(&args.project))
        .map_err(|e| format!("count persons: {e:?}"))?;
    let place_total = store
        .count_places(Some(&args.project))
        .map_err(|e| format!("count places: {e:?}"))?;
    let atlas_total = store
        .count_atlases(Some(&args.project))
        .map_err(|e| format!("count atlases: {e:?}"))?;

    println!("\n=== 결과 ===");
    println!("project           = {}", args.project);
    println!("groups indexed    = {group_total}");
    println!("persons indexed   = {person_total}");
    println!("places indexed    = {place_total}");
    println!("atlases indexed   = {atlas_total}");
    println!("groups parsed     = {}", groups.len());
    println!("persons parsed    = {}", persons.len());
    println!("places parsed     = {}", places.len());
    println!("atlases parsed    = {}", atlases.len());
    println!("errors            = {}", errors.len());
    println!("group cycles      = {}", cycles.len());
    println!("place cycles      = {}", place_cycles.len());
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
