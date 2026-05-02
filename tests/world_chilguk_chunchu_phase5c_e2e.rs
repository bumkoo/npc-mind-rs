//! Phase 5c.1 체크포인트 2 — 7 historical/active NPC 마크다운 계약 잠금 e2e 테스트.
//!
//! 사양 `task-phase5-followup-historical-npcs.md` v1.1 §3.4(직교 플래그) + §3.4b(extras.secret)
//! + §6.4(affiliation 정합) 자동 검증:
//! - 7 신규 person + 1 stub 승급 (npc-im-seoun [Phase 5c.1] + npc-08·09·10 + npc-11 + 3 historical)
//! - `extras.heritage_doc_pending` (bool) + `extras.hexaco_confidence` (enum) 라운드트립
//! - `extras.secret` (string block) 라운드트립
//! - `is_mind_eligible()` kind별 검증 (active=true, historical=false)
//! - npc-danun 단독 결정: heritage_doc_pending=false + hexaco_confidence=precise (★ 디렉터 사양과 다름)
//! - npc-11 stub 승급: legacy `source_status` 제거 + 새 플래그 존재
//! - HEXACO 6 dim 값 일치 (체크포인트 2 보고서 권장값 그대로)
//! - affiliation FK (사양 §6.4 정합): npc-jincheonmyeong·npc-danun = [group-daejin-court]
//!
//! 실행: `cargo test --features embed --test world_chilguk_chunchu_phase5c_e2e`

#![cfg(feature = "embed")]

use std::path::PathBuf;

use npc_mind::adapter::sqlite_world::SqliteWorldStore;
use npc_mind::domain::world::{Person, PersonStatus};
use npc_mind::worldbuilding::WorldRepository;
use npc_mind::worldbuilding::markdown::{group_from_markdown, person_from_markdown};

/// Phase 5c.1 신규/승급 8인 — 체크포인트 1·2 합산.
const PHASE5C_PERSON_IDS: &[&str] = &[
    "npc-08",
    "npc-09",
    "npc-10",
    "npc-11", // 기존 stub 승급 — Phase 5c.2에서 직교 플래그 적용
    "npc-im-seoun",
    "npc-chuyangjinin",
    "npc-jincheonmyeong",
    "npc-danun",
];

/// kind=active 4인 — mind eligible 대상.
const PHASE5C_ACTIVE_IDS: &[&str] = &["npc-08", "npc-09", "npc-10", "npc-11"];

/// kind=historical 4인 — mind 등록 X.
const PHASE5C_HISTORICAL_IDS: &[&str] = &[
    "npc-im-seoun",
    "npc-chuyangjinin",
    "npc-jincheonmyeong",
    "npc-danun",
];

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_phase5c_persons() -> Vec<Person> {
    let dir = project_root().join("projects/chilguk-chunchu/world/person");
    let mut out = Vec::new();
    for id in PHASE5C_PERSON_IDS {
        let path = dir.join(format!("{id}.md"));
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("missing person file: {}", path.display()));
        let mut person = person_from_markdown(&raw)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        person.source_path = Some(path.to_string_lossy().to_string());
        out.push(person);
    }
    out
}

fn load_all_groups() -> Vec<npc_mind::domain::world::Group> {
    let dir = project_root().join("projects/chilguk-chunchu/world/group");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|_| panic!("missing dir: {}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let raw = std::fs::read_to_string(&p).expect("read .md");
            group_from_markdown(&raw).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
        })
        .collect()
}

fn extras_bool(p: &Person, key: &str) -> bool {
    p.extras
        .get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| panic!("{}: extras.{key} (bool) 누락", p.id))
}

fn extras_str(p: &Person, key: &str) -> String {
    p.extras
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| panic!("{}: extras.{key} (string) 누락", p.id))
}

#[test]
fn phase5c_eight_persons_parse_and_load() {
    let persons = load_phase5c_persons();
    assert_eq!(persons.len(), PHASE5C_PERSON_IDS.len(), "Phase 5c.1 8인 모두 필요");

    for p in &persons {
        assert!(
            PHASE5C_PERSON_IDS.contains(&p.id.as_str()),
            "예상치 못한 person id: {}",
            p.id
        );
    }
}

#[test]
fn phase5c_kind_status_per_director_spec() {
    let persons = load_phase5c_persons();
    let by_id: std::collections::HashMap<&str, &Person> =
        persons.iter().map(|p| (p.id.as_str(), p)).collect();

    // active 4인 — alive
    for id in PHASE5C_ACTIVE_IDS {
        let p = by_id.get(id).unwrap();
        assert_eq!(p.kind, "active", "{id}: kind=active 기대");
        assert_eq!(p.status, PersonStatus::Alive, "{id}: alive 기대");
    }

    // historical 4인 — kind=historical
    for id in PHASE5C_HISTORICAL_IDS {
        let p = by_id.get(id).unwrap();
        assert_eq!(p.kind, "historical", "{id}: kind=historical 기대");
    }

    // 세부 status:
    // - 임서운(missing): 행방불명, 메인 퀘스트 후반 분기
    // - 추양진인(dead): 260년 멸문 시 전사
    // - 진천명(dead): 30년차 즈음 사망 (270년 전 건국 황제)
    // - 단운(missing): 255년차 사망/행방불명, 공식 사망 처리
    assert_eq!(by_id["npc-im-seoun"].status, PersonStatus::Missing);
    assert_eq!(by_id["npc-chuyangjinin"].status, PersonStatus::Dead);
    assert_eq!(by_id["npc-jincheonmyeong"].status, PersonStatus::Dead);
    assert_eq!(by_id["npc-danun"].status, PersonStatus::Missing);
}

#[test]
fn phase5c_mind_eligibility_per_kind() {
    let persons = load_phase5c_persons();
    let by_id: std::collections::HashMap<&str, &Person> =
        persons.iter().map(|p| (p.id.as_str(), p)).collect();

    // active 4인 — mind eligible
    for id in PHASE5C_ACTIVE_IDS {
        let p = by_id.get(id).unwrap();
        assert!(
            p.is_mind_eligible(),
            "{id}: active이므로 is_mind_eligible() = true 기대"
        );
    }

    // historical 4인 — mind 등록 X (정책 정합)
    for id in PHASE5C_HISTORICAL_IDS {
        let p = by_id.get(id).unwrap();
        assert!(
            !p.is_mind_eligible(),
            "{id}: historical이므로 is_mind_eligible() = false 기대 (정책 §3.2)"
        );
    }
}

#[test]
fn phase5c_orthogonal_flags_present_on_all_eight() {
    // 사양 §3.4 직교 플래그: heritage_doc_pending (bool) + hexaco_confidence (enum)
    let persons = load_phase5c_persons();
    for p in &persons {
        let _ = extras_bool(p, "heritage_doc_pending");
        let conf = extras_str(p, "hexaco_confidence");
        assert!(
            ["precise", "pending", "unknown"].contains(&conf.as_str()),
            "{}: hexaco_confidence='{conf}' 비정상 (precise|pending|unknown 기대)",
            p.id
        );
    }
}

#[test]
fn phase5c_extras_secret_block_present_on_all_eight() {
    // 사양 §3.4b extras.secret = `## 비밀` H2의 머신 리더블 미러
    let persons = load_phase5c_persons();
    for p in &persons {
        let secret = extras_str(p, "secret");
        // 비밀 라벨 블록은 numbered list 형식 — 최소 한 항목("1.") 포함.
        assert!(
            secret.contains("1."),
            "{}: extras.secret이 numbered 형식 아님 — '1.' 누락\n실제: {secret}",
            p.id
        );
    }
}

#[test]
fn phase5c_npc_danun_is_precise_with_heritage_doc_present() {
    // ★ 디렉터 사양과 다른 결정 (보고서 §결정 1):
    //   wuxia-core/docs/characters/npc-11-taemuje.md 본기(本紀) 존재
    //   → heritage_doc_pending = false + hexaco_confidence = precise
    let persons = load_phase5c_persons();
    let danun = persons.iter().find(|p| p.id.as_str() == "npc-danun").unwrap();

    assert_eq!(
        extras_bool(danun, "heritage_doc_pending"),
        false,
        "npc-danun: wuxia-core 본기 존재라 heritage_doc_pending=false (사양 §3.4 직교 플래그)"
    );
    assert_eq!(
        extras_str(danun, "hexaco_confidence"),
        "precise",
        "npc-danun: 본기 Big Five 명시라 hexaco_confidence=precise"
    );
}

#[test]
fn phase5c_other_seven_have_heritage_doc_pending_true() {
    // npc-danun 외 7인은 모두 단독 열전 .md 부재.
    let persons = load_phase5c_persons();
    for p in &persons {
        if p.id.as_str() == "npc-danun" {
            continue;
        }
        assert_eq!(
            extras_bool(p, "heritage_doc_pending"),
            true,
            "{}: 단독 열전 .md 부재라 heritage_doc_pending=true 기대",
            p.id
        );
    }
}

#[test]
fn phase5c_npc_11_stub_upgrade_removed_legacy_source_status() {
    // npc-11 소풍자 stub 승급: legacy `source_status: heritage-pending` 제거 + 새 플래그.
    let persons = load_phase5c_persons();
    let npc11 = persons.iter().find(|p| p.id.as_str() == "npc-11").unwrap();

    assert!(
        !npc11.extras.contains_key("source_status"),
        "npc-11: legacy source_status 키 제거됐어야 함 (사양 §3.4 마이그레이션 사례 1번)"
    );
    // 새 직교 플래그 존재 + precise 등급 (npc-05 다중 묘사 근거)
    assert_eq!(extras_bool(npc11, "heritage_doc_pending"), true);
    assert_eq!(extras_str(npc11, "hexaco_confidence"), "precise");
}

#[test]
fn phase5c_active_four_use_precise_confidence() {
    // 필수 4 active (npc-08·09·10·11) 모두 precise — 다중 출처 근거.
    let persons = load_phase5c_persons();
    let by_id: std::collections::HashMap<&str, &Person> =
        persons.iter().map(|p| (p.id.as_str(), p)).collect();
    for id in PHASE5C_ACTIVE_IDS {
        let p = by_id.get(id).unwrap();
        assert_eq!(
            extras_str(p, "hexaco_confidence"),
            "precise",
            "{id}: 필수 4 active은 precise 등급 (체크포인트 2 §결정)"
        );
    }
}

#[test]
fn phase5c_historical_three_have_pending_confidence() {
    // 핵심 historical 3 중 단운 외 둘은 pending — 단편 출처만이라 잠정.
    // 임서운은 다중 출처라 precise (체크포인트 1 §결정).
    let persons = load_phase5c_persons();
    let by_id: std::collections::HashMap<&str, &Person> =
        persons.iter().map(|p| (p.id.as_str(), p)).collect();

    assert_eq!(
        extras_str(by_id["npc-chuyangjinin"], "hexaco_confidence"),
        "pending"
    );
    assert_eq!(
        extras_str(by_id["npc-jincheonmyeong"], "hexaco_confidence"),
        "pending"
    );
    // 임서운은 player.md + 다중 NPC 회상으로 precise (Phase 5c.1 체크포인트 1 결정)
    assert_eq!(
        extras_str(by_id["npc-im-seoun"], "hexaco_confidence"),
        "precise"
    );
}

#[test]
fn phase5c_hexaco_values_match_recommended() {
    // 체크포인트 2 보고서 권장값 (소수점 1자리 정확 일치).
    let persons = load_phase5c_persons();
    let by_id: std::collections::HashMap<&str, &Person> =
        persons.iter().map(|p| (p.id.as_str(), p)).collect();

    // (id, h, e, x, a, c, o)
    let expected = [
        ("npc-08", 0.0, 0.4, 0.4, -0.2, 0.6, 0.3),
        ("npc-09", -0.3, -0.2, 0.3, 0.0, 0.7, 0.4),
        ("npc-10", -0.4, -0.3, 0.0, -0.5, 0.5, 0.7),
        ("npc-11", 0.6, 0.3, 0.4, 0.5, 0.4, 0.5),
        ("npc-im-seoun", 0.7, 0.5, -0.2, 0.5, 0.7, 0.4),
        ("npc-chuyangjinin", 0.6, 0.4, 0.0, 0.3, 0.7, 0.3),
        ("npc-jincheonmyeong", 0.5, 0.0, 0.4, 0.4, 0.7, 0.5),
        ("npc-danun", -0.4, 0.4, 0.0, -0.2, 0.6, 0.8),
    ];
    for (id, h, e, x, a, c, o) in expected {
        let p = by_id.get(id).unwrap();
        assert!(
            (p.hexaco.honesty_humility.value() - h).abs() < 1e-6,
            "{id}: H={} 기대={h}",
            p.hexaco.honesty_humility.value()
        );
        assert!(
            (p.hexaco.emotionality.value() - e).abs() < 1e-6,
            "{id}: E={} 기대={e}",
            p.hexaco.emotionality.value()
        );
        assert!(
            (p.hexaco.extraversion.value() - x).abs() < 1e-6,
            "{id}: X={} 기대={x}",
            p.hexaco.extraversion.value()
        );
        assert!(
            (p.hexaco.agreeableness.value() - a).abs() < 1e-6,
            "{id}: A={} 기대={a}",
            p.hexaco.agreeableness.value()
        );
        assert!(
            (p.hexaco.conscientiousness.value() - c).abs() < 1e-6,
            "{id}: C={} 기대={c}",
            p.hexaco.conscientiousness.value()
        );
        assert!(
            (p.hexaco.openness.value() - o).abs() < 1e-6,
            "{id}: O={} 기대={o}",
            p.hexaco.openness.value()
        );
    }
}

#[test]
fn phase5c_historical_emperors_affiliated_with_daejin_court() {
    // 사양 §6.4 정합 — npc-jincheonmyeong·npc-danun = [group-daejin-court].
    // (Phase 5c.2 체크포인트 2 보고서 §결정에서 합의된 처리.)
    let persons = load_phase5c_persons();
    let by_id: std::collections::HashMap<&str, &Person> =
        persons.iter().map(|p| (p.id.as_str(), p)).collect();

    let jincheonmyeong = by_id["npc-jincheonmyeong"];
    assert_eq!(
        jincheonmyeong.affiliation.len(),
        1,
        "npc-jincheonmyeong: affiliation 1건 기대 ([group-daejin-court])"
    );
    assert_eq!(
        jincheonmyeong.affiliation[0].as_str(),
        "group-daejin-court"
    );

    let danun = by_id["npc-danun"];
    assert_eq!(danun.affiliation.len(), 1);
    assert_eq!(danun.affiliation[0].as_str(), "group-daejin-court");
}

#[test]
fn phase5c_npc_10_uses_active_cheonma_group_affiliation() {
    // npc-10 3대 천마 = group-cheonma-shingyo (Phase 1 등록). 직접 외래키.
    let persons = load_phase5c_persons();
    let npc10 = persons.iter().find(|p| p.id.as_str() == "npc-10").unwrap();
    assert_eq!(npc10.affiliation.len(), 1);
    assert_eq!(npc10.affiliation[0].as_str(), "group-cheonma-shingyo");
}

#[test]
fn phase5c_pending_groups_for_unregistered_clans() {
    // npc-08·09·im-seoun·chuyangjinin은 affiliation 빈 + extras.pending_groups.
    let persons = load_phase5c_persons();
    let by_id: std::collections::HashMap<&str, &Person> =
        persons.iter().map(|p| (p.id.as_str(), p)).collect();

    for id in &["npc-08", "npc-09", "npc-im-seoun", "npc-chuyangjinin"] {
        let p = by_id[id];
        assert!(
            p.affiliation.is_empty(),
            "{id}: affiliation 빈 기대 (Phase 1 미등록 group + pending_groups 메타)"
        );
        assert!(
            p.extras.contains_key("pending_groups"),
            "{id}: extras.pending_groups 메타 필요"
        );
    }
}

#[test]
fn phase5c_sqlite_roundtrip_preserves_orthogonal_flags() {
    // SqliteWorldStore 라운드트립으로 새 플래그가 보존되는지 검증.
    let persons = load_phase5c_persons();
    let groups = load_all_groups();
    let store = SqliteWorldStore::in_memory().expect("sqlite in-memory");
    for g in &groups {
        store.upsert_group("chilguk-chunchu", g).expect("upsert group");
    }
    for p in &persons {
        store.upsert_person("chilguk-chunchu", p).expect("upsert person");
    }

    for original in &persons {
        let stored = store
            .get_person(&original.id)
            .expect("get_person")
            .unwrap_or_else(|| panic!("{}: 라운드트립 None", original.id));

        // 직교 플래그
        assert_eq!(
            extras_bool(&stored, "heritage_doc_pending"),
            extras_bool(original, "heritage_doc_pending"),
            "{}: heritage_doc_pending 라운드트립",
            original.id
        );
        assert_eq!(
            extras_str(&stored, "hexaco_confidence"),
            extras_str(original, "hexaco_confidence"),
            "{}: hexaco_confidence 라운드트립",
            original.id
        );
        // extras.secret
        assert_eq!(
            extras_str(&stored, "secret"),
            extras_str(original, "secret"),
            "{}: secret 라운드트립",
            original.id
        );
        // affiliation
        assert_eq!(
            stored.affiliation, original.affiliation,
            "{}: affiliation 라운드트립",
            original.id
        );
    }
}

#[test]
fn phase5c_no_legacy_source_status_on_new_persons() {
    // Phase 5c.1+ 신규 person은 legacy `source_status: heritage-pending` 사용 안 함.
    // npc-11 stub 승급 시점에 legacy 키 제거 (npc-07 천순제는 본 Phase 외, 마이그레이션 비강제).
    let persons = load_phase5c_persons();
    for p in &persons {
        assert!(
            !p.extras.contains_key("source_status"),
            "{}: Phase 5c.1+ 신규/승급 person은 legacy source_status 키 사용 금지 (사양 §3.4)",
            p.id
        );
    }
}

#[test]
fn phase5c_im_seoun_player_relevance_max() {
    // 임서운은 player 메인 비밀 4종의 핵심 → player_relevance = 5
    let persons = load_phase5c_persons();
    let im_seoun = persons.iter().find(|p| p.id.as_str() == "npc-im-seoun").unwrap();
    let pr = im_seoun
        .extras
        .get("player_relevance")
        .and_then(|v| v.as_i64())
        .expect("npc-im-seoun: extras.player_relevance 필요");
    assert_eq!(pr, 5, "임서운은 player 메인 비밀의 핵심 (★★★★★)");
}

#[test]
fn phase5c_aliases_unique_within_each_person() {
    let persons = load_phase5c_persons();
    for p in &persons {
        let mut seen = std::collections::HashSet::new();
        for alias in &p.aliases {
            assert!(
                seen.insert(alias.as_str()),
                "{}: alias '{alias}' 중복",
                p.id
            );
        }
    }
}

#[test]
fn phase5c_persons_count_in_world_load_is_sixteen() {
    // Phase 5c.1 종결 시점: 기존 9 + 임서운 + 6 신규 = 16.
    let dir = project_root().join("projects/chilguk-chunchu/world/person");
    let count = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().and_then(|s| s.to_str()) == Some("md")
        })
        .count();
    assert_eq!(count, 16, "Phase 5c.1 종결 시 person 파일 16개 기대");
}

#[test]
fn phase5c_active_count_supports_mind_eligible_twelve() {
    // mind eligible 12 = 11 active + 1 player. historical 4명 + 기존 npc-07(이미 active)은
    // 본 슈트 외에서 검증되나, 본 슈트는 Phase 5c.1 신규 8인 중 active=4를 시연.
    let persons = load_phase5c_persons();
    let active_count = persons.iter().filter(|p| p.is_mind_eligible()).count();
    assert_eq!(
        active_count,
        4,
        "Phase 5c.1 신규/승급 8인 중 mind eligible(active 4) = npc-08·09·10·11"
    );
    let historical_count = persons.iter().filter(|p| !p.is_mind_eligible()).count();
    assert_eq!(
        historical_count, 4,
        "Phase 5c.1 신규 8인 중 historical 4 (mind 등록 X)"
    );
}
