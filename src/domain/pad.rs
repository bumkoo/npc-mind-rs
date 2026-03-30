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
pub const PLEASURE_ANCHORS: PadAxisAnchors = PadAxisAnchors {
    positive: &[
        "기쁘고 흐뭇하다",
        "마음이 따뜻하고 행복하다",
        "감사하고 만족스럽다",
    ],
    negative: &[
        "괴롭고 불쾌하다",
        "마음이 아프고 고통스럽다",
        "분하고 원통하다",
    ],
};

/// A축: 각성(Arousal) ↔ 이완
pub const AROUSAL_ANCHORS: PadAxisAnchors = PadAxisAnchors {
    positive: &[
        "격앙되어 흥분한다",
        "긴장되고 심장이 빠르게 뛴다",
        "흥분하여 가만히 있을 수 없다",
    ],
    negative: &[
        "차분하고 담담하다",
        "평온하고 고요하다",
        "편안하고 여유롭다",
    ],
};

/// D축: 지배(Dominance) ↔ 복종
/// pad_dot에서 D 격차 스케일러로 사용. PadAnalyzer에서 임베딩한다.
pub const DOMINANCE_ANCHORS: PadAxisAnchors = PadAxisAnchors {
    positive: &[
        "내가 주도한다, 물러서라",
        "상황을 장악하고 있다",
        "당당하고 자신감에 차 있다",
    ],
    negative: &[
        "어찌할 바를 모르겠다",
        "무력하고 속수무책이다",
        "위축되어 아무것도 할 수 없다",
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
