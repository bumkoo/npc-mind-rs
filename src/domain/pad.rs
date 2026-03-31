//! PAD 감정 공간 모델 (Mehrabian & Russell, 1974)
//!
//! 3축 연속 좌표로 감정 상태를 표현한다:
//! - P (Pleasure):  쾌 ↔ 불쾌     (-1.0 ~ 1.0)
//! - A (Arousal):   각성 ↔ 이완   (-1.0 ~ 1.0)
//! - D (Dominance): 지배 ↔ 복종   (-1.0 ~ 1.0)
//!
//! 용도:
//! - OCC 감정 → PAD 변환 (매핑 테이블, Gebhard 2005 ALMA 모델 참고)
//! - 대사 자극의 감정적 방향/강도 표현
//! - apply_stimulus에서 pad_dot으로 공명 계산
//!
//! ## pad_dot 공식: P·A 방향 × D 격차 스케일러
//!
//! D축은 관계적 차원(상보적)이라 내적 기반 공명에 적합하지 않다.
//! 복종적 감정(Shame, Fear)에 지배적 자극이 증폭해야 하지만,
//! 내적은 "같은 방향=공명"이라 반대로 작동한다.
//!
//! 해결: P·A가 공명 방향(증폭/감소)을 정하고,
//! D축 차이(|D_n - D_o|)가 그 효과의 강도를 스케일링한다.
//!
//! 직관: 상대와 나의 권력 격차가 클수록, 그 사람의 말이 나에게 더 강하게 작용한다.
//!
//! 상세: docs/pad-stimulus-design-decisions.md

use serde::{Deserialize, Serialize};

use super::emotion::EmotionType;
use super::tuning::PAD_D_SCALE_WEIGHT;

// ---------------------------------------------------------------------------
// PAD 구조체
// ---------------------------------------------------------------------------

/// 감정의 3차원 좌표 (Pleasure, Arousal, Dominance)
///
/// 대사 자극으로 사용될 때: 대사가 주는 감정적 방향과 강도
/// OCC 감정 매핑으로 사용될 때: 특정 감정의 PAD 공간 위치
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pad {
    /// 쾌-불쾌 (-1.0=불쾌, +1.0=쾌)
    pub pleasure: f32,
    /// 각성-이완 (-1.0=이완, +1.0=각성)
    pub arousal: f32,
    /// 지배-복종 (-1.0=복종, +1.0=지배)
    /// pad_dot에서 P·A 결과의 강도 스케일러로 사용 (내적 항으로는 사용하지 않음)
    pub dominance: f32,
}

impl Pad {
    pub fn new(pleasure: f32, arousal: f32, dominance: f32) -> Self {
        Self { pleasure, arousal, dominance }
    }

    /// 중립 (0, 0, 0)
    pub fn neutral() -> Self {
        Self { pleasure: 0.0, arousal: 0.0, dominance: 0.0 }
    }
}

// ---------------------------------------------------------------------------
// PAD 연산
// ---------------------------------------------------------------------------

/// P·A 공명 × D 격차 스케일러
///
/// P·A 내적이 공명 방향(증폭/감소)을 결정하고,
/// D축 차이(|D_n - D_o|)가 그 효과의 강도를 배율로 조절한다.
///
/// 공식: (P_a × P_b + A_a × A_b) × (1.0 + |D_a - D_b| × 0.3)
///
/// D축을 내적 항에서 분리한 이유:
/// D축은 관계적 차원(상보적)이라 "같은 방향=공명"이 성립하지 않는다.
/// Shame(D:-0.60) + 비난(D:+0.5) → 내적은 반발이지만 실제로는 증폭이어야 한다.
/// D 격차를 스케일러로 쓰면 "권력 격차가 클수록 자극이 세게 먹힌다"는
/// 직관을 방향 왜곡 없이 반영할 수 있다.
///
/// 상세: docs/pad-stimulus-design-decisions.md
pub fn pad_dot(a: &Pad, b: &Pad) -> f32 {
    let pa = a.pleasure * b.pleasure + a.arousal * b.arousal;
    let d_gap = (a.dominance - b.dominance).abs();
    pa * (1.0 + d_gap * PAD_D_SCALE_WEIGHT)
}

// ---------------------------------------------------------------------------
// OCC → PAD 매핑 테이블 (Gebhard 2005, ALMA 모델)
// ---------------------------------------------------------------------------
//
// 좌표 값은 pad_table.rs에서 중앙 관리한다.

use super::pad_table::*;

/// OCC 22개 감정 유형을 PAD 좌표로 변환
///
/// 좌표 값은 `pad_table` 모듈의 상수를 참조한다.
/// P·A는 pad_dot 내적에, D는 격차 스케일러에 사용된다.
pub fn emotion_to_pad(emotion: EmotionType) -> Pad {
    match emotion {
        EmotionType::Joy             => JOY_PAD,
        EmotionType::Distress        => DISTRESS_PAD,
        EmotionType::HappyFor        => HAPPY_FOR_PAD,
        EmotionType::Pity            => PITY_PAD,
        EmotionType::Gloating        => GLOATING_PAD,
        EmotionType::Resentment      => RESENTMENT_PAD,
        EmotionType::Hope            => HOPE_PAD,
        EmotionType::Fear            => FEAR_PAD,
        EmotionType::Satisfaction    => SATISFACTION_PAD,
        EmotionType::Disappointment  => DISAPPOINTMENT_PAD,
        EmotionType::Relief          => RELIEF_PAD,
        EmotionType::FearsConfirmed  => FEARS_CONFIRMED_PAD,
        EmotionType::Pride           => PRIDE_PAD,
        EmotionType::Shame           => SHAME_PAD,
        EmotionType::Admiration      => ADMIRATION_PAD,
        EmotionType::Reproach        => REPROACH_PAD,
        EmotionType::Gratification   => GRATIFICATION_PAD,
        EmotionType::Remorse         => REMORSE_PAD,
        EmotionType::Gratitude       => GRATITUDE_PAD,
        EmotionType::Anger           => ANGER_PAD,
        EmotionType::Love            => LOVE_PAD,
        EmotionType::Hate            => HATE_PAD,
    }
}


// ---------------------------------------------------------------------------
// PAD 앵커 텍스트 (3축 × 양극단 × 변형)
// ---------------------------------------------------------------------------

/// PAD 축 하나의 양극단 앵커 텍스트
pub struct PadAxisAnchors {
    /// 양극단 (+1.0 방향) 텍스트 변형들
    pub positive: &'static [&'static str],
    /// 음극단 (-1.0 방향) 텍스트 변형들
    pub negative: &'static [&'static str],
}

/// P축: 쾌(Pleasure) ↔ 불쾌
///
/// 앵커 설계 원칙:
/// - 모든 문장을 1인칭 대사/독백 톤으로 통일 (임베딩 결집력)
/// - positive: 기쁨, 감사, 호의, 위로/격려 포함
/// - negative: 고통, 분노, 비난, 경멸 포함
pub const PLEASURE_ANCHORS: PadAxisAnchors = PadAxisAnchors {
    positive: &[
        "참으로 기쁘고 흐뭇하구려.",
        "마음이 따뜻해지는군. 이런 게 행복이란 것이지.",
        "이토록 훌륭하니 진심으로 만족스럽소.",
        "괜찮소, 걱정하지 마시오. 모든 것이 잘 될 것이오.",
        "은혜를 잊지 않겠소. 정말 고맙소.",
        "오랜만이오, 반갑소! 그간 무고하셨소?",
    ],
    negative: &[
        "정말 괴롭고 불쾌하기 짝이 없군.",
        "마음이 아프고 고통스러워 견딜 수가 없구나.",
        "이리 당하다니 참으로 분하고 원통하다!",
        "배은망덕한 놈! 네놈이 어찌 그럴 수 있느냐!",
        "꺼져라. 네 꼴을 보기도 싫다.",
        "모든 것이 끝이다. 아무런 희망이 없다.",
    ],
};

/// A축: 각성(Arousal) ↔ 이완
///
/// 앵커 설계 원칙:
/// - positive: 격앙, 긴급, 전투, 흥분 (에너지 폭발)
/// - negative: 차분, 관조, 위로, 여유 (에너지 가라앉음)
/// - 대사 스타일은 에너지 수준을 직접 전달하는 문장
pub const AROUSAL_ANCHORS: PadAxisAnchors = PadAxisAnchors {
    positive: &[
        "피가 끓어오르고 주체할 수 없이 흥분되는군!",
        "헉, 헉... 긴장해서 심장이 터질 것 같다.",
        "도저히 가만히 있을 수가 없다! 몸이 달아오른다!",
        "적이 쳐들어 온다! 모두 무기를 들어라!",
        "검이 부딪히는 굉음에 온몸의 피가 역류하는구나!",
        "어서! 지금 당장 움직여야 한다!",
    ],
    negative: &[
        "마음이 한없이 차분하고 담담해지는구려.",
        "주변이 참으로 평온하고 고요하군.",
        "몸도 마음도 편안하고 여유롭소.",
        "천천히 하시오. 서두를 것 없소.",
        "편히 쉬시게. 차 한잔 드시오.",
        "강물처럼 흘러가는 대로 두면 되오.",
    ],
};

/// D축: 지배(Dominance) ↔ 복종
///
/// 앵커 설계 원칙:
/// - D축은 화자의 권력 위치/태도를 측정
/// - positive: 명령, 질타, 위압, 주도적 선언 (화자가 위)
/// - negative: 경어 겸양, 애걸, 자책, 위임, 주저 (화자가 아래)
/// - pad_dot에서 D 격차 스케일러로 사용
pub const DOMINANCE_ANCHORS: PadAxisAnchors = PadAxisAnchors {
    positive: &[
        "내가 주도한다, 물러서라!",
        "이곳의 모든 상황은 내 통제 아래에 있다.",
        "내가 해내지 못할 일은 천하에 없다.",
        "감히! 무릎 꿇어라! 이것은 명이다!",
        "내 결정에 이의를 달 자가 있느냐?",
        "이 일은 내가 책임지겠소. 뒤로 물러나시오.",
    ],
    negative: &[
        "눈앞이 캄캄하군... 어찌해야 할지 모르겠어.",
        "아무것도 할 수 없다니, 이리도 무력할 수가...",
        "위축되어 숨조차 제대로 쉴 수가 없구나.",
        "소인은 감히 거역할 수 없습니다. 처분을 기다리겠습니다.",
        "살려주십시오... 무엇이든 하겠습니다.",
        "저... 혹시 괜찮으시다면... 따라가도 될까요?",
    ],
};

/// 전체 PAD 앵커 세트 (3축)
pub const PAD_ANCHORS: [&PadAxisAnchors; 3] = [
    &PLEASURE_ANCHORS,
    &AROUSAL_ANCHORS,
    &DOMINANCE_ANCHORS,
];


// ---------------------------------------------------------------------------
// PadAnalyzer — 도메인 서비스 (업무 규칙)
// ---------------------------------------------------------------------------
//
// TextEmbedder(인프라 포트)에만 의존하며, 구체적 임베딩 모델을 모른다.
// cosine_sim, mean_vector, axis_score는 순수 수학 — 인프라 무관.

use crate::ports::{TextEmbedder, EmbedError};

/// 사전 계산된 앵커 임베딩 (축 하나의 양극단)
pub struct AxisEmbeddings {
    /// 양극단 평균 벡터
    pub positive: Vec<f32>,
    /// 음극단 평균 벡터
    pub negative: Vec<f32>,
}

/// 대사 → PAD 변환 도메인 서비스
///
/// 초기화 시 TextEmbedder로 3축 앵커 임베딩을 사전 계산하고,
/// analyze() 시 대사 벡터와 앵커 간 유사도로 PAD를 추출.
///
/// P·A는 pad_dot 내적에, D는 격차 스케일러에 사용된다.
/// 임베딩 모델을 교체해도 이 코드는 변경 없음.
pub struct PadAnalyzer {
    embedder: Box<dyn TextEmbedder + Send>,
    pleasure: AxisEmbeddings,
    arousal: AxisEmbeddings,
    dominance: AxisEmbeddings,
}

impl PadAnalyzer {
    /// TextEmbedder로 3축 앵커 임베딩을 사전 계산하여 생성
    pub fn new(mut embedder: Box<dyn TextEmbedder + Send>) -> Result<Self, EmbedError> {
        let pleasure = Self::embed_axis(&mut *embedder, &PLEASURE_ANCHORS)?;
        let arousal = Self::embed_axis(&mut *embedder, &AROUSAL_ANCHORS)?;
        let dominance = Self::embed_axis(&mut *embedder, &DOMINANCE_ANCHORS)?;

        Ok(Self { embedder, pleasure, arousal, dominance })
    }

    /// 앵커 텍스트 → 평균 임베딩 벡터 계산
    fn embed_axis(
        embedder: &mut dyn TextEmbedder,
        anchors: &PadAxisAnchors,
    ) -> Result<AxisEmbeddings, EmbedError> {
        let pos_vecs = embedder.embed(anchors.positive)?;
        let neg_vecs = embedder.embed(anchors.negative)?;

        Ok(AxisEmbeddings {
            positive: Self::mean_vector(&pos_vecs),
            negative: Self::mean_vector(&neg_vecs),
        })
    }

    /// 대사 벡터를 앵커와 비교하여 PAD 계산 (순수 도메인 로직, 인프라 호출 없음)
    pub fn to_pad(&self, utterance_embedding: &[f32]) -> Pad {
        Pad::new(
            Self::axis_score(utterance_embedding, &self.pleasure),
            Self::axis_score(utterance_embedding, &self.arousal),
            Self::axis_score(utterance_embedding, &self.dominance),
        )
    }

    // --- 순수 수학 함수 (인프라 무관) ---

    /// 여러 벡터의 평균
    fn mean_vector(vectors: &[Vec<f32>]) -> Vec<f32> {
        if vectors.is_empty() { return Vec::new(); }
        let dim = vectors[0].len();
        let n = vectors.len() as f32;
        let mut mean = vec![0.0_f32; dim];
        for v in vectors {
            for (i, val) in v.iter().enumerate() {
                mean[i] += val;
            }
        }
        for val in mean.iter_mut() {
            *val /= n;
        }
        mean
    }

    /// 코사인 유사도
    pub fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na == 0.0 || nb == 0.0 { return 0.0; }
        dot / (na * nb)
    }

    /// 대사 임베딩과 축 앵커 유사도 차이 → -1.0~1.0
    fn axis_score(utterance_emb: &[f32], axis: &AxisEmbeddings) -> f32 {
        let sim_pos = Self::cosine_sim(utterance_emb, &axis.positive);
        let sim_neg = Self::cosine_sim(utterance_emb, &axis.negative);
        (sim_pos - sim_neg).clamp(-1.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// UtteranceAnalyzer 포트 구현
// ---------------------------------------------------------------------------

impl crate::ports::UtteranceAnalyzer for PadAnalyzer {
    fn analyze(&mut self, utterance: &str) -> Result<Pad, EmbedError> {
        let embeddings = self.embedder.embed(&[utterance])?;

        if embeddings.is_empty() {
            return Ok(Pad::neutral());
        }

        Ok(self.to_pad(&embeddings[0]))
    }
}
