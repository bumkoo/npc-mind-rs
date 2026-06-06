//! Phase 2 Stage 5 narrative validation — S1~S3 정량 + S4 정성.
//!
//! 인계 spec: `docs/tasks/mind-architecture/task-rel-phase2-stage5-narrative-FROZEN.md` §2 작업 3.
//!
//! ## 박제 ground truth (S5-D3)
//! 진입 시점 *코드 실행 산출* 4축값이 ground truth. 회귀 가드 tolerance ±0.5.
//! EXPECTED는 디자이너 산정값이 아니라 본 commit 시점의 코드 출력. 어색 시
//! 게이트2 완화 *금지* (S5-D4); 게이트3에서 입력 JSON 조정 → EXPECTED 갱신.
//!
//! ## axis_modulation 부재 (S5-D1)
//! Phase 2.5 reflection LLM 출력 필드 — Stage 5엔 입력 부재.
//! `src/domain/reflection.rs::ReflectionResult` 7 필드에 해당 없음 → axis 변동에
//! ±5 가산 항 자체 없음.
//!
//! ## S4 (임충→고구) 정성 (S5-D2)
//! 정량 테스트 *제외*. 게이트3 디자이너 narrative 검토 핸드오프 — 어떤
//! `_s4_qualitative_handoff_note` 함수 자리(주석)만 표기, 실행 0.

use std::sync::{Arc, Mutex};

use npc_mind::adapter::memory_repository::InMemoryRepository;
use npc_mind::application::command::{Command, CommandDispatcher};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::emotion::{EmotionState, EmotionType};
use npc_mind::domain::event::EventKind;
use npc_mind::domain::personality::{Npc, NpcBuilder, Score};
use npc_mind::domain::reflection::ReflectionResult;
use npc_mind::domain::relationship::{AxisScore, RelationshipBuilder, WarinessScore};
use npc_mind::ports::{EmotionStore, NpcWorld};

const TOLERANCE: f32 = 0.5;

// ---------------------------------------------------------------------------
// HEXACO 픽스처 — 기존 phase1-validation 시나리오 정합화
// ---------------------------------------------------------------------------

/// 임충 — lin-chong-shanshenmiao.json 인계 (Stage 5 정합화).
/// 활성 modifier: sincerity 0.6 → trust × 1.2 (HIGH_THRESHOLD 0.5 초과).
fn lin_chong_npc() -> Npc {
    NpcBuilder::new("lin_chong", "임충")
        .description("팔십만금군 교두 출신 — 결단 직전")
        .with_inner_compass(
            "체제의 법도와 의리를 지킨다 — 단, 의리 없는 자는 더 이상 신의의 대상이 아니다",
        )
        .honesty_humility(|h| {
            h.sincerity = Score::new(0.6, "sincerity").unwrap();
            h.fairness = Score::new(0.7, "fairness").unwrap();
        })
        .emotionality(|e| {
            e.anxiety = Score::new(0.4, "anxiety").unwrap();
        })
        .agreeableness(|a| {
            a.patience = Score::new(0.3, "patience").unwrap();
        })
        .conscientiousness(|c| {
            c.prudence = Score::new(0.4, "prudence").unwrap();
        })
        .build()
}

/// 수련 — daily-training.json 인계.
/// 활성 modifier: sincerity 0.7 (trust × 1.2) + patience 0.9 (전체 × 0.7) + prudence 0.8 (전체 × 0.8).
fn yu_shulien_npc() -> Npc {
    NpcBuilder::new("yu_shulien", "수련")
        .description("청명검 전수자 — 절제와 인내의 화신")
        .with_inner_compass("공성명수신퇴(功成名遂身退) — 공을 이루었으니 물러난다")
        .honesty_humility(|h| {
            h.sincerity = Score::new(0.7, "sincerity").unwrap();
        })
        .agreeableness(|a| {
            a.patience = Score::new(0.9, "patience").unwrap();
            a.forgiveness = Score::new(0.6, "forgiveness").unwrap();
        })
        .conscientiousness(|c| {
            c.prudence = Score::new(0.8, "prudence").unwrap();
            c.diligence = Score::new(0.8, "diligence").unwrap();
        })
        .build()
}

/// 노지심 — S1 partner. HEXACO modifier 미적용 (partner는 mapping에 영향 0).
fn lu_zhishen_npc() -> Npc {
    NpcBuilder::new("lu_zhishen", "노지심")
        .description("화화상 노지심 — 임충의 결의 형제")
        .build()
}

/// 옥교룡 — S3 partner. HEXACO modifier 미적용.
fn yu_qiaolong_npc() -> Npc {
    NpcBuilder::new("yu_qiaolong", "옥교룡")
        .description("교만한 제자 — 사부의 가르침을 거부")
        .build()
}

// ---------------------------------------------------------------------------
// dispatcher 셋업 헬퍼
// ---------------------------------------------------------------------------

type Repo = InMemoryRepository;

fn build_dispatcher(repo: Repo) -> (Arc<Mutex<Repo>>, CommandDispatcher<Repo>) {
    let repo_arc = Arc::new(Mutex::new(repo));
    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher =
        CommandDispatcher::new(repo_arc.clone(), event_store, bus).with_default_handlers();
    (repo_arc, dispatcher)
}

fn axes(repo: &Repo, owner: &str, target: &str) -> (f32, f32, f32, f32) {
    let r = repo.get_relationship(owner, target).expect("relationship");
    (
        r.trust().value(),
        r.affinity().value(),
        r.respect().value(),
        r.wariness().value(),
    )
}

fn assert_axes_match(actual: (f32, f32, f32, f32), expected: (f32, f32, f32, f32), case: &str) {
    let (a_t, a_a, a_r, a_w) = actual;
    let (e_t, e_a, e_r, e_w) = expected;
    assert!(
        (a_t - e_t).abs() < TOLERANCE,
        "{case} trust: actual={a_t} expected={e_t} (tol ±{TOLERANCE})"
    );
    assert!(
        (a_a - e_a).abs() < TOLERANCE,
        "{case} affinity: actual={a_a} expected={e_a} (tol ±{TOLERANCE})"
    );
    assert!(
        (a_r - e_r).abs() < TOLERANCE,
        "{case} respect: actual={a_r} expected={e_r} (tol ±{TOLERANCE})"
    );
    assert!(
        (a_w - e_w).abs() < TOLERANCE,
        "{case} wariness: actual={a_w} expected={e_w} (tol ±{TOLERANCE})"
    );
}

// ---------------------------------------------------------------------------
// S1 — 임충 → 노지심 (Admiration + Gratitude, Joy=0)
// 서사: 노지심이 야저림에서 임충을 구함. 임충의 사후 감정 = 우러르고(존경) + 감사.
// ---------------------------------------------------------------------------

fn s1_reflection() -> ReflectionResult {
    ReflectionResult {
        is_chitchat: false,
        summary: "노지심이 야저림에서 임충을 구하고 떠남 — 임충의 깊은 감사·존경".into(),
        significance_score: 0.78,
        declarative_events: vec![],
        partnership_event: None,
        turn_count: 7,
        llm_reasoning: Some("우러르는 인정(Admiration) + 보은 의지(Gratitude). beat=구원".into()),
    }
}

#[tokio::test]
async fn s1_lin_chong_admires_lu_zhishen_admiration_gratitude() {
    // S1 박제: lin_chong (sincerity 0.6) → lu_zhishen, Admiration 0.6 + Gratitude 0.6
    let mut repo = InMemoryRepository::new();
    repo.add_npc(lin_chong_npc());
    repo.add_npc(lu_zhishen_npc());
    repo.add_relationship(
        RelationshipBuilder::new("lin_chong", "lu_zhishen")
            .trust(AxisScore::new(50.0))
            .affinity(AxisScore::new(40.0))
            .respect(AxisScore::new(20.0))
            .wariness(WarinessScore::new(0.0))
            .type_text("결의 형제")
            .build(),
    );

    let (repo_arc, dispatcher) = build_dispatcher(repo);

    // emotion 사전 설정 — 우러름(Admiration) + 감사(Gratitude)
    {
        let mut state = EmotionState::default();
        state.set_intensity(EmotionType::Admiration, 0.6);
        state.set_intensity(EmotionType::Gratitude, 0.6);
        dispatcher
            .repository_guard()
            .save_emotion_state("lin_chong", state);
    }

    let output = dispatcher
        .dispatch_v2(Command::EndDialogue {
            npc_id: "lin_chong".into(),
            partner_id: "lu_zhishen".into(),
            significance: None,
            reflection: Some(s1_reflection()),
        })
        .await
        .expect("dispatch OK");

    let kinds: Vec<EventKind> = output.events.iter().map(|e| e.kind()).collect();
    assert!(kinds.contains(&EventKind::DialogueReflected));
    assert!(
        kinds.contains(&EventKind::RelationshipUpdated),
        "S1 outer loop 진입 — RelationshipUpdated 발행 (significance 0.78)"
    );

    let actual = axes(&repo_arc.lock().unwrap(), "lin_chong", "lu_zhishen");

    // 박제 EXPECTED — Stage 5 진입 시점 코드 실행 산출 (S5-D3). tol ±0.5 회귀 가드.
    //
    // 손계산 검증:
    // - 초기 {trust:50, affinity:40, respect:20, wariness:0}.
    // - HEXACO modifier (lin_chong sincerity 0.6): trust × 1.2, 나머지 × 1.0.
    // - Admiration 0.6 × base{0,0,+20,0} = {0, 0, +12, 0}
    // - Gratitude  0.6 × base{+15,+10,0,-10} × {1.2,1,1,1} = {+10.8, +6, 0, -6}
    // - sum delta = {+10.8, +6, +12, -6} → after {60.8, 46, 32, -6}
    // - WarinessScore clamps to [0,100] → wariness=0.
    let expected = (60.8_f32, 46.0_f32, 32.0_f32, 0.0_f32);
    assert_axes_match(actual, expected, "S1");
}

// ---------------------------------------------------------------------------
// S2 — 임충 → 육겸 (Reproach + Hate + Anger, Distress·FC=0)
// 서사: 산신묘에서 육겸의 음모 발각 → 처단. (mapping.rs L471/L564 인프라.)
// ---------------------------------------------------------------------------

fn s2_reflection() -> ReflectionResult {
    ReflectionResult {
        is_chitchat: false,
        summary: "임충이 산신묘에서 육겸을 처단 — 옛 친구의 배신·결단".into(),
        significance_score: 0.92,
        declarative_events: vec![],
        partnership_event: None,
        turn_count: 9,
        llm_reasoning: Some(
            "OCC peak 0.9+ (Anger·Hate·Reproach), PAD trajectory 큼, beat=처단".into(),
        ),
    }
}

#[tokio::test]
async fn s2_lin_chong_breaks_with_lu_qian_reproach_hate_anger() {
    // S2 박제: lin_chong (sincerity 0.6) → lu_qian, Reproach 0.8 + Hate 0.8 + Anger 0.9.
    // mapping.rs L471 (base_delta sum) / L564 (lin_chong HEXACO modifier — full profile은
    // 본 테스트의 단순 프로파일과 다르나, 진입 시점 *코드 산출*이 ground truth).
    let mut repo = InMemoryRepository::new();
    repo.add_npc(lin_chong_npc());
    repo.add_npc(
        NpcBuilder::new("lu_qian", "육겸")
            .description("임충의 옛 친구 — 고구의 명령으로 암살 음모")
            .build(),
    );
    repo.add_relationship(
        RelationshipBuilder::new("lin_chong", "lu_qian")
            .trust(AxisScore::new(50.0))
            .affinity(AxisScore::new(40.0))
            .respect(AxisScore::new(20.0))
            .wariness(WarinessScore::new(0.0))
            .type_text("옛 친구")
            .build(),
    );

    let (repo_arc, dispatcher) = build_dispatcher(repo);

    {
        let mut state = EmotionState::default();
        state.set_intensity(EmotionType::Reproach, 0.8);
        state.set_intensity(EmotionType::Hate, 0.8);
        state.set_intensity(EmotionType::Anger, 0.9);
        dispatcher
            .repository_guard()
            .save_emotion_state("lin_chong", state);
    }

    let output = dispatcher
        .dispatch_v2(Command::EndDialogue {
            npc_id: "lin_chong".into(),
            partner_id: "lu_qian".into(),
            significance: None,
            reflection: Some(s2_reflection()),
        })
        .await
        .expect("dispatch OK");

    let kinds: Vec<EventKind> = output.events.iter().map(|e| e.kind()).collect();
    assert!(kinds.contains(&EventKind::DialogueReflected));
    assert!(kinds.contains(&EventKind::RelationshipUpdated));

    let actual = axes(&repo_arc.lock().unwrap(), "lin_chong", "lu_qian");

    // 박제 EXPECTED (S5-D3). 손계산 검증:
    // - 초기 {50, 40, 20, 0}. HEXACO modifier (lin_chong, forgiveness=0.0 not <-0.5):
    //   trust × 1.2, 나머지 × 1.0.
    // - Reproach 0.8 × {-10,-10,-25,+15} × {1.2,1,1,1} = {-9.6, -8, -20, +12}
    // - Hate     0.8 × {-10,-25, -5,+20} × {1.2,1,1,1} = {-9.6,-20, -4, +16}
    // - Anger    0.9 × {-25,-10,-10,+25} × {1.2,1,1,1} = {-27, -9, -9, +22.5}
    // - sum delta = {-46.2, -37, -33, +50.5}
    // - after = {3.8, 3, -13, 50.5} (4축 clamp 범위 내).
    //
    // mapping.rs L471 base_delta sum: trust:-45/affinity:-45/respect:-40/wariness:+60
    // (intensity=1.0). 본 케이스 intensity는 0.8~0.9 + trust×1.2 → 위 sum 유도.
    let expected = (3.8_f32, 3.0_f32, -13.0_f32, 50.5_f32);
    assert_axes_match(actual, expected, "S2");
}

// ---------------------------------------------------------------------------
// S3 — 수련 → 옥교룡 (Pity + Reproach + Anger, Distress=0)
// 서사: 수련이 교만한 제자의 폭주를 마주함 — 안타까움 + 책망 + 분노.
// ---------------------------------------------------------------------------

fn s3_reflection() -> ReflectionResult {
    ReflectionResult {
        is_chitchat: false,
        summary: "수련이 옥교룡의 폭주를 직시 — 안타까움·책망·분노".into(),
        significance_score: 0.72,
        declarative_events: vec![],
        partnership_event: None,
        turn_count: 8,
        llm_reasoning: Some("Pity (제자의 길 vs 폭주) + Reproach + Anger (사부의 한)".into()),
    }
}

#[tokio::test]
async fn s3_yu_shulien_pities_yu_qiaolong_pity_reproach_anger() {
    // S3 박제: yu_shulien (sincerity 0.7 + patience 0.9 + prudence 0.8) → yu_qiaolong,
    // Pity 0.7 + Reproach 0.7 + Anger 0.6.
    let mut repo = InMemoryRepository::new();
    repo.add_npc(yu_shulien_npc());
    repo.add_npc(yu_qiaolong_npc());
    repo.add_relationship(
        RelationshipBuilder::new("yu_shulien", "yu_qiaolong")
            .trust(AxisScore::new(40.0))
            .affinity(AxisScore::new(30.0))
            .respect(AxisScore::new(10.0))
            .wariness(WarinessScore::new(20.0))
            .type_text("교만한 제자")
            .build(),
    );

    let (repo_arc, dispatcher) = build_dispatcher(repo);

    {
        let mut state = EmotionState::default();
        state.set_intensity(EmotionType::Pity, 0.7);
        state.set_intensity(EmotionType::Reproach, 0.7);
        state.set_intensity(EmotionType::Anger, 0.6);
        dispatcher
            .repository_guard()
            .save_emotion_state("yu_shulien", state);
    }

    let output = dispatcher
        .dispatch_v2(Command::EndDialogue {
            npc_id: "yu_shulien".into(),
            partner_id: "yu_qiaolong".into(),
            significance: None,
            reflection: Some(s3_reflection()),
        })
        .await
        .expect("dispatch OK");

    let kinds: Vec<EventKind> = output.events.iter().map(|e| e.kind()).collect();
    assert!(kinds.contains(&EventKind::DialogueReflected));
    assert!(kinds.contains(&EventKind::RelationshipUpdated));

    let actual = axes(&repo_arc.lock().unwrap(), "yu_shulien", "yu_qiaolong");

    // 박제 EXPECTED (S5-D3). 손계산 검증:
    // - 초기 {40, 30, 10, 20}. HEXACO 3 rules fire: sincerity(0.7)>0.5 trust×1.2,
    //   patience(0.9)>0.5 all×0.7, prudence(0.8)>0.5 all×0.8. forgiveness=0.6 not <-0.5.
    // - per-axis 누적 modifier: trust = 1.2 × 0.7 × 0.8 = 0.672 / others = 0.7 × 0.8 = 0.56.
    // - Pity     0.7 × {0,+10,-5,0} × {0.672,0.56,0.56,0.56} = {0, +3.92, -1.96, 0}
    // - Reproach 0.7 × {-10,-10,-25,+15} × {…} = {-4.704, -3.92, -9.8, +5.88}
    // - Anger    0.6 × {-25,-10,-10,+25} × {…} = {-10.08, -3.36, -3.36, +8.4}
    //   (Reproach, Anger: forgiveness=0.6 not <-0.5 → A− Forgiveness 룰 미발동.)
    // - sum delta = {-14.784, -3.36, -15.12, +14.28}
    // - after = {25.216, 26.64, -5.12, 34.28}.
    let expected = (25.216_f32, 26.64_f32, -5.12_f32, 34.28_f32);
    assert_axes_match(actual, expected, "S3");
}

// ---------------------------------------------------------------------------
// S4 — 임충 → 고구 (정성 케이스, 게이트3 디자이너 핸드오프) — S5-D2
//
// 본 시나리오는 spec §3.6에서 focus 수치가 *의도적으로 부재*하다 → 게이트3 흡수.
// "3 layer separation"(서사·인지·정서) 정성 검증은 디자이너 narrative 검토에서
// 처리. 본 파일에 함수 미작성 (자리만 표기).
//
// 디자이너 핸드오프 노트:
// - 임충의 對고구 감정은 *체제 정점에 대한 누적 분노*. 단일 시점 EmotionState
//   mock으로 박제하기 어려움 (시간 분산 + 권력 거리).
// - 4축 변동은 시뮬레이션이 아니라 *서사 직관*과 정합해야 함 → 게이트3.
// - 향후 Phase 2.3+에서 시간 분산 + axis_modulation 활성화 시 정량 가능.
// ---------------------------------------------------------------------------
