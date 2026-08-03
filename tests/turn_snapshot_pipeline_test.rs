//! ② 배선 검증 — 대화 흐름이 `TurnSnapshot`을 제대로 채우는가
//!
//! `compute_significance`가 먹는 재료(`TurnSnapshot`)가 **실제 대화 파이프라인을
//! 통과해** 만들어지는지 검증한다. 수식 자체가 아니라 배선이 관심사다.
//!
//! ## 왜 별도 테스트가 필요한가
//!
//! 커버리지가 이렇게 갈린다:
//!
//! | | 질문 | 담당 |
//! |---|---|---|
//! | ① | 대사에서 PAD가 제대로 뽑히나 | `embed_test` (실 ONNX) |
//! | **②** | **PAD가 파이프라인을 타고 TurnSnapshot에 실리나** | **본 테스트** |
//! | ③ | TurnSnapshot으로 점수가 맞게 계산되나 | `phase1_bench_test` |
//!
//! ③은 `TurnSnapshot`을 리터럴로 직접 주입하므로 **배선이 끊겨도 통과한다.**
//! 실제로 `phase1_real_llm_test`가 analyzer를 붙이지 않아 모든 `pad_after`가
//! `Pad::neutral()`로 폴백되고 있었는데, ③은 멀쩡히 통과해 아무도 눈치채지 못했다.
//! 본 테스트는 발화만 주고 파이프라인이 재료를 만들게 하므로 그런 고장을 잡는다.
//!
//! ONNX는 필요 없다 — analyzer가 *무엇을* 돌려주는지가 아니라 그 값이 제대로
//! *실려 가는지*가 검증 대상이므로 mock으로 충분하다. 그래서 정본 회귀에 포함된다.
//!
//! 배경: `docs/tasks/mind-architecture/reflection-test-restructure-handoff.md`

#![cfg(feature = "chat")]

mod common;

use common::TestContext;
use common::mock_chat::MockConversationPort;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use npc_mind::application::dto::{EventInput, SituationInput};
use npc_mind::application::reflection_service::ReflectionRunner;
use npc_mind::domain::pad::{Pad, UtteranceEmbedding};
use npc_mind::domain::personality::Npc;
use npc_mind::domain::reflection::{ReflectionResult, TurnSnapshot, compute_significance};
use npc_mind::ports::{EmbedError, GuideFormatter, UtteranceAnalyzer};
use npc_mind::presentation::builtin_toml;
use npc_mind::presentation::formatter::LocaleFormatter;
use npc_mind::DialogueOrchestrator;

// ============================================================
// Mock — 호출 순서대로 다른 PAD를 돌려주는 분석기
// ============================================================

/// 턴마다 다른 PAD를 반환한다. 큐가 마르면 마지막 값을 반복한다.
///
/// `pad_magnitude`(turn 사이 delta 누적)가 살아나려면 연속한 턴의 PAD가 서로
/// 달라야 하므로, 고정 PAD를 주는 `ScriptedAnalyzer`류로는 이 배선을 검증할 수 없다.
struct SequencedAnalyzer {
    pads: Vec<Pad>,
    cursor: usize,
}

impl SequencedAnalyzer {
    fn new(pads: Vec<Pad>) -> Self {
        Self { pads, cursor: 0 }
    }

    fn next_pad(&mut self) -> Pad {
        let pad = self.pads[self.cursor.min(self.pads.len() - 1)];
        self.cursor += 1;
        pad
    }
}

impl UtteranceAnalyzer for SequencedAnalyzer {
    fn analyze(&mut self, _utterance: &str) -> Result<Pad, EmbedError> {
        Ok(self.next_pad())
    }

    fn analyze_with_embedding(
        &mut self,
        _utterance: &str,
    ) -> Result<(Pad, Option<UtteranceEmbedding>), EmbedError> {
        // 임베딩은 None — listener_perspective 변환기가 미주입이면 화자 PAD가 그대로 쓰인다.
        Ok((self.next_pad(), None))
    }
}

// ============================================================
// Mock — reflect()가 받은 TurnSnapshot을 그대로 캡처
// ============================================================

/// `turn_buffers`는 `DialogueOrchestrator`의 private 필드라 밖에서 못 본다.
/// `ReflectionRunner`가 `end_session` 시점에 그 버퍼를 인자로 받으므로,
/// 여기서 낚아채는 것이 유일하게 깔끔한 관측 지점이다.
struct CapturingReflectionRunner {
    captured: Arc<Mutex<Vec<TurnSnapshot>>>,
}

#[async_trait]
impl ReflectionRunner for CapturingReflectionRunner {
    async fn reflect(
        &self,
        _sid: &str,
        turns: &[TurnSnapshot],
        _npc: &Npc,
        _partner: &Npc,
    ) -> ReflectionResult {
        *self.captured.lock().unwrap() = turns.to_vec();
        ReflectionResult {
            is_chitchat: false,
            summary: "(captured)".into(),
            significance_score: compute_significance(turns),
            declarative_events: vec![],
            partnership_event: None,
            turn_count: turns.len(),
            llm_reasoning: None,
        }
    }
}

// ============================================================
// 공통 셋업
// ============================================================

fn situation() -> SituationInput {
    SituationInput {
        description: "배신 상황".into(),
        event: Some(EventInput {
            description: "사건".into(),
            desirability_for_self: -0.6,
            other: None,
            prospect: None,
        }),
        action: None,
        object: None,
    }
}

const UTTERANCES: [&str; 3] = ["첫 마디요.", "두 번째 마디요.", "마지막 마디요."];

/// 주어진 PAD 시퀀스로 3턴 대화를 돌리고, reflection이 받은 스냅샷을 돌려준다.
async fn run_dialogue(pads: Vec<Pad>) -> Vec<TurnSnapshot> {
    let ctx = TestContext::new();
    let (dispatcher, _store, _bus) = common::v2_dispatcher_with_defaults(ctx.repo);

    let toml = builtin_toml("ko").expect("ko locale");
    let formatter: Arc<dyn GuideFormatter> =
        Arc::new(LocaleFormatter::from_toml(toml).expect("formatter"));

    let captured = Arc::new(Mutex::new(Vec::new()));
    let runner: Arc<dyn ReflectionRunner> = Arc::new(CapturingReflectionRunner {
        captured: Arc::clone(&captured),
    });

    let mut agent = DialogueOrchestrator::new(dispatcher, MockConversationPort::new(), formatter)
        .with_analyzer(SequencedAnalyzer::new(pads))
        .with_reflection(runner);

    agent
        .start_session("s1", "mu_baek", "gyo_ryong", Some(situation()))
        .await
        .expect("start_session");

    for u in UTTERANCES {
        agent.turn("s1", u, None, None).await.expect("turn");
    }

    agent
        .end_session("s1", Some(0.5))
        .await
        .expect("end_session");

    let snapshots = captured.lock().unwrap().clone();
    snapshots
}

// ============================================================
// 검증
// ============================================================

#[tokio::test]
async fn 대화_턴이_turn_snapshot으로_축적된다() {
    let snapshots = run_dialogue(vec![
        Pad::new(0.5, 0.2, 0.1),
        Pad::new(-0.4, 0.6, -0.2),
        Pad::new(0.3, -0.3, 0.5),
    ])
    .await;

    assert_eq!(snapshots.len(), UTTERANCES.len(), "턴 수만큼 스냅샷이 쌓여야 함");

    for (i, s) in snapshots.iter().enumerate() {
        assert_eq!(s.turn_index, (i + 1) as u32, "turn_index는 1-based 연속");
        assert_eq!(s.user_utterance, UTTERANCES[i], "발화가 순서대로 실려야 함");
        assert!(!s.npc_response.is_empty(), "NPC 응답이 실려야 함");
    }
}

#[tokio::test]
async fn analyzer_pad가_turn_snapshot에_실린다() {
    let snapshots = run_dialogue(vec![
        Pad::new(0.5, 0.2, 0.1),
        Pad::new(-0.4, 0.6, -0.2),
        Pad::new(0.3, -0.3, 0.5),
    ])
    .await;

    // 배선이 끊기면 전부 Pad::neutral()로 폴백해 서로 같아진다 — 그 고장을 잡는다.
    assert_ne!(
        snapshots[0].pad_after, snapshots[1].pad_after,
        "턴 사이 PAD가 변해야 pad_magnitude 신호가 살아난다"
    );
    assert_ne!(snapshots[1].pad_after, snapshots[2].pad_after);

    assert!(
        snapshots.iter().all(|s| s.pad_after != Pad::neutral()),
        "analyzer가 준 PAD가 중립으로 뭉개지면 안 된다"
    );
}

#[tokio::test]
async fn 감정_상태가_turn_snapshot에_실린다() {
    let snapshots = run_dialogue(vec![
        Pad::new(0.5, 0.2, 0.1),
        Pad::new(-0.4, 0.6, -0.2),
        Pad::new(0.3, -0.3, 0.5),
    ])
    .await;

    assert!(
        snapshots.iter().any(|s| !s.occ_emotions.is_empty()),
        "appraise/stimulus를 거친 감정이 스냅샷에 실려야 peak_occ·diversity 신호가 산다"
    );
}

/// 배선이 살아 있으면 실제 대화 흐름에서도 유의미한 점수가 나온다.
///
/// 값의 정확성이 아니라 **신호가 죽어있지 않음**을 보는 단언이다.
/// (수식 자체의 골든값 회귀는 `phase1_bench_test` 소관)
#[tokio::test]
async fn 실제_흐름에서_significance가_죽지_않는다() {
    let turbulent = run_dialogue(vec![
        Pad::new(0.7, 0.3, 0.2),
        Pad::new(-0.6, 0.8, -0.4),
        Pad::new(0.5, -0.5, 0.6),
    ])
    .await;
    let flat = run_dialogue(vec![Pad::new(0.1, 0.1, 0.0)]).await; // 큐가 말라 전 턴 동일

    let s_turbulent = compute_significance(&turbulent);
    let s_flat = compute_significance(&flat);

    assert!(
        s_turbulent > 0.0,
        "격동적인 대화의 significance가 0이면 배선이 끊긴 것 (실제={s_turbulent})"
    );
    assert!(
        s_turbulent > s_flat,
        "PAD가 크게 흔들린 대화가 평탄한 대화보다 높아야 한다 \
         (turbulent={s_turbulent}, flat={s_flat})"
    );
}
