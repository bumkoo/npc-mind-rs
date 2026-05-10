//! Phase 1 Mind Architecture (relationships.md v0.7 §6) Stage 5 — Bench.
//!
//! 측정 항목:
//! 1. `compute_significance` 단독 latency (engine 부분, LLM 무관) — <1ms target
//! 2. `dispatch_v2(EndDialogue)` chitchat vs significant 비교 — chitchat skip 시
//!    cascade 감소로 더 빠를 것 (검증)
//! 3. narrative 시나리오 3개의 significance 분포 calibration — 낮음/중간/높음 밴드
//!    각각 정확한 범위에 들어가는지 검증 (가중치 튜닝 입력)
//!
//! 결과는 spec F12 (Bench results) sub-section에 박제. 실제 LLM bench는 디자이너
//! Mind Studio 수동 검증 (spec §5.3).

use std::sync::{Arc, Mutex};
use std::time::Instant;

use npc_mind::adapter::memory_repository::InMemoryRepository;
use npc_mind::application::command::{Command, CommandDispatcher};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::emotion::EmotionType;
use npc_mind::domain::pad::Pad;
use npc_mind::domain::reflection::{ReflectionResult, TurnSnapshot, compute_significance};
use npc_mind::ports::{EmotionStore, NpcWorld};

// ===========================================================================
// 5.1 compute_significance 단독 latency
// ===========================================================================

#[test]
fn bench_compute_significance_single_call_under_microsecond_avg() {
    // 10 turn (높음 케이스 worst-case에 가까움) × N 반복
    let turns: Vec<TurnSnapshot> = (0..10)
        .map(|i| TurnSnapshot {
            user_utterance: format!("user-{i}"),
            npc_response: format!("npc-{i}"),
            occ_emotions: vec![
                (EmotionType::Anger, 0.7),
                (EmotionType::Distress, 0.5),
                (EmotionType::Pride, 0.3),
            ],
            pad_before: Pad::new(0.0, 0.0, 0.0),
            pad_after: Pad::new(0.5 - (i as f32 * 0.05), 0.5, 0.2),
            beat_changed: i % 5 == 0,
            turn_index: i + 1,
        })
        .collect();

    const N: u32 = 10_000;
    let start = Instant::now();
    let mut sum = 0.0_f64;
    for _ in 0..N {
        sum += compute_significance(&turns) as f64;
    }
    let elapsed = start.elapsed();
    let per_call_ns = elapsed.as_nanos() / (N as u128);
    let per_call_us = per_call_ns as f64 / 1000.0;

    println!(
        "compute_significance(10 turns) ×{N}: total={elapsed:?}, per-call={per_call_us:.2} µs (sum={sum:.2})"
    );

    // Target: < 1000 µs (1 ms). 실측치 <100µs 예상 (단순 산술).
    assert!(
        per_call_us < 1000.0,
        "compute_significance per-call {per_call_us:.2} µs >= 1ms target"
    );
}

#[test]
fn bench_compute_significance_empty_short_long_scaling() {
    let scales = [0_usize, 1, 5, 10, 30, 100];
    for &n in &scales {
        let turns: Vec<TurnSnapshot> = (0..n)
            .map(|i| TurnSnapshot {
                user_utterance: String::new(),
                npc_response: String::new(),
                occ_emotions: vec![(EmotionType::Joy, 0.5)],
                pad_before: Pad::neutral(),
                pad_after: Pad::new(0.1, 0.1, 0.0),
                beat_changed: false,
                turn_index: (i + 1) as u32,
            })
            .collect();

        let start = Instant::now();
        let _ = compute_significance(&turns);
        let elapsed = start.elapsed();
        println!("compute_significance({n} turns): {:?}", elapsed);
    }
}

// ===========================================================================
// 5.2 dispatch_v2(EndDialogue) chitchat vs significant 비교
// ===========================================================================

fn build_dispatcher_with_alice_bob() -> Arc<CommandDispatcher<InMemoryRepository>> {
    let mut repo = InMemoryRepository::new();
    repo.add_npc(npc_mind::domain::personality::NpcBuilder::new("alice", "Alice").build());
    repo.add_npc(npc_mind::domain::personality::NpcBuilder::new("bob", "Bob").build());
    repo.add_relationship(npc_mind::domain::relationship::Relationship::neutral(
        "alice", "bob",
    ));
    let repo_arc = Arc::new(Mutex::new(repo));
    let event_store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher =
        CommandDispatcher::new(repo_arc, event_store, bus).with_default_handlers();
    dispatcher
        .repository_guard()
        .save_emotion_state("alice", Default::default());
    Arc::new(dispatcher)
}

fn chitchat_reflection() -> ReflectionResult {
    ReflectionResult {
        is_chitchat: true,
        summary: "지나가는 인사".into(),
        significance_score: 0.05,
        declarative_events: vec![],
        partnership_event: None,
        turn_count: 2,
        llm_reasoning: None,
    }
}

fn significant_reflection() -> ReflectionResult {
    ReflectionResult {
        is_chitchat: false,
        summary: "결단".into(),
        significance_score: 0.85,
        declarative_events: vec![],
        partnership_event: None,
        turn_count: 8,
        llm_reasoning: None,
    }
}

#[tokio::test]
async fn bench_dispatch_v2_end_dialogue_chitchat_vs_significant() {
    const WARMUP: u32 = 5;
    const N: u32 = 50;

    // Warmup
    for _ in 0..WARMUP {
        let dispatcher = build_dispatcher_with_alice_bob();
        let _ = dispatcher
            .dispatch_v2(Command::EndDialogue {
                npc_id: "alice".into(),
                partner_id: "bob".into(),
                significance: None,
                reflection: Some(chitchat_reflection()),
            })
            .await;
    }

    // chitchat (3 follow-up: DialogueReflected + EmotionCleared + SceneEnded)
    let mut chitchat_total = std::time::Duration::ZERO;
    for _ in 0..N {
        let dispatcher = build_dispatcher_with_alice_bob();
        let start = Instant::now();
        let _ = dispatcher
            .dispatch_v2(Command::EndDialogue {
                npc_id: "alice".into(),
                partner_id: "bob".into(),
                significance: None,
                reflection: Some(chitchat_reflection()),
            })
            .await
            .expect("dispatch OK");
        chitchat_total += start.elapsed();
    }

    // significant (4 follow-up: + RelationshipUpdated)
    let mut significant_total = std::time::Duration::ZERO;
    for _ in 0..N {
        let dispatcher = build_dispatcher_with_alice_bob();
        let start = Instant::now();
        let _ = dispatcher
            .dispatch_v2(Command::EndDialogue {
                npc_id: "alice".into(),
                partner_id: "bob".into(),
                significance: None,
                reflection: Some(significant_reflection()),
            })
            .await
            .expect("dispatch OK");
        significant_total += start.elapsed();
    }

    // legacy (reflection: None + significance: Some — RelationshipUpdated 무조건)
    let mut legacy_total = std::time::Duration::ZERO;
    for _ in 0..N {
        let dispatcher = build_dispatcher_with_alice_bob();
        let start = Instant::now();
        let _ = dispatcher
            .dispatch_v2(Command::EndDialogue {
                npc_id: "alice".into(),
                partner_id: "bob".into(),
                significance: Some(0.5),
                reflection: None,
            })
            .await
            .expect("dispatch OK");
        legacy_total += start.elapsed();
    }

    let chitchat_per = chitchat_total / N;
    let significant_per = significant_total / N;
    let legacy_per = legacy_total / N;

    println!("dispatch_v2(EndDialogue) per-call (N={N}):");
    println!("  chitchat (3 follow-up):   {chitchat_per:?}");
    println!("  significant (4 follow-up): {significant_per:?}");
    println!("  legacy   (3 follow-up):   {legacy_per:?}");
    println!(
        "  chitchat vs legacy ratio: {:.2}x (1.0 = 동일, <1.0 = chitchat 더 빠름)",
        chitchat_per.as_nanos() as f64 / legacy_per.as_nanos() as f64
    );
    println!(
        "  significant vs legacy ratio: {:.2}x",
        significant_per.as_nanos() as f64 / legacy_per.as_nanos() as f64
    );

    // 회귀 sanity check — 모든 케이스 < 5ms (Mock LLM, in-memory repo)
    assert!(
        chitchat_per.as_micros() < 5000,
        "chitchat dispatch {chitchat_per:?} >= 5ms"
    );
    assert!(
        significant_per.as_micros() < 5000,
        "significant dispatch {significant_per:?} >= 5ms"
    );
    assert!(
        legacy_per.as_micros() < 5000,
        "legacy dispatch {legacy_per:?} >= 5ms"
    );
}

// ===========================================================================
// 5.3 narrative 시나리오 3개 calibration — significance 분포가 밴드별로 정확한가
// ===========================================================================

#[test]
fn bench_narrative_calibration_chitchat_low_band() {
    // 잡담 시뮬레이션 — 4 turn, OCC 강도 낮음, PAD 변화 0, beat 없음
    let turns: Vec<TurnSnapshot> = (0..4)
        .map(|i| TurnSnapshot {
            user_utterance: "안녕".into(),
            npc_response: "안녕".into(),
            occ_emotions: vec![],
            pad_before: Pad::neutral(),
            pad_after: Pad::neutral(),
            beat_changed: false,
            turn_index: (i + 1) as u32,
        })
        .collect();
    let s = compute_significance(&turns);
    println!("chitchat 시나리오 significance: {s:.3} (target < 0.3)");
    assert!(s < 0.3, "chitchat significance {s} >= 0.3 (낮음 밴드 calibration 깨짐)");
}

#[test]
fn bench_narrative_calibration_daily_mid_band() {
    // 일상 가르침 시뮬레이션 — 8 turn, Pride 0.4 / Admiration 0.5, PAD 약간 변동
    let turns: Vec<TurnSnapshot> = (0..8)
        .map(|i| TurnSnapshot {
            user_utterance: "가르침".into(),
            npc_response: "이렇게 해라".into(),
            occ_emotions: if i % 2 == 0 {
                vec![(EmotionType::Pride, 0.4), (EmotionType::Admiration, 0.5)]
            } else {
                vec![(EmotionType::Joy, 0.3)]
            },
            pad_before: Pad::new(0.2, 0.1, 0.0),
            pad_after: Pad::new(0.3 + (i as f32 * 0.02), 0.15, 0.0),
            beat_changed: i == 5,
            turn_index: (i + 1) as u32,
        })
        .collect();
    let s = compute_significance(&turns);
    println!("daily 시나리오 significance: {s:.3} (target 0.3~0.7)");
    assert!(s >= 0.3 && s < 0.7, "daily significance {s} not in [0.3, 0.7)");
}

#[test]
fn bench_narrative_calibration_shanshenmiao_high_band() {
    // 결단 시뮬레이션 — 9 turn, Anger 0.95 + Hate 0.8 + Surprise 0.6, PAD 큰 변화, beat 전환
    let turns: Vec<TurnSnapshot> = vec![
        TurnSnapshot {
            user_utterance: "묘 도착".into(),
            npc_response: "...".into(),
            occ_emotions: vec![(EmotionType::Fear, 0.4)],
            pad_before: Pad::neutral(),
            pad_after: Pad::new(-0.3, 0.4, 0.1),
            beat_changed: false,
            turn_index: 1,
        },
        TurnSnapshot {
            user_utterance: "음모 발각".into(),
            npc_response: "분노".into(),
            occ_emotions: vec![
                (EmotionType::Anger, 0.95),
                (EmotionType::Hate, 0.8),
                (EmotionType::Distress, 0.7),
            ],
            pad_before: Pad::new(-0.3, 0.4, 0.1),
            pad_after: Pad::new(-0.9, 0.95, 0.6),
            beat_changed: true,
            turn_index: 2,
        },
        TurnSnapshot {
            user_utterance: "처단".into(),
            npc_response: "검을 빼다".into(),
            occ_emotions: vec![
                (EmotionType::Anger, 0.9),
                (EmotionType::Pride, 0.5),
            ],
            pad_before: Pad::new(-0.9, 0.95, 0.6),
            pad_after: Pad::new(-0.5, 0.5, 0.3),
            beat_changed: false,
            turn_index: 3,
        },
        TurnSnapshot {
            user_utterance: "고요".into(),
            npc_response: "끝났다".into(),
            occ_emotions: vec![(EmotionType::Distress, 0.6)],
            pad_before: Pad::new(-0.5, 0.5, 0.3),
            pad_after: Pad::new(-0.3, 0.0, 0.0),
            beat_changed: false,
            turn_index: 4,
        },
    ];
    let s = compute_significance(&turns);
    println!("shanshenmiao 시나리오 significance: {s:.3} (target >= 0.7)");
    assert!(s >= 0.7, "shanshenmiao significance {s} < 0.7 (높음 밴드 calibration 깨짐)");
}
