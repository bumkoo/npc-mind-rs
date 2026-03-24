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
//! - apply_stimulus에서 pad_dot(내적)으로 공명 계산

use serde::{Deserialize, Serialize};

use super::emotion::EmotionType;

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

/// PAD 단순 내적 — 같은 방향이면 양수, 반대면 음수.
/// 자극이 강할수록 내적 절대값이 커서 자극 크기도 자동 반영.
///
/// apply_stimulus에서 감정별 공명 계산에 사용:
/// - 양수 → 같은 방향 → 해당 감정 증폭
/// - 음수 → 반대 방향 → 해당 감정 감소
pub fn pad_dot(a: &Pad, b: &Pad) -> f32 {
    a.pleasure * b.pleasure
    + a.arousal * b.arousal
    + a.dominance * b.dominance
}

// ---------------------------------------------------------------------------
// OCC → PAD 매핑 테이블 (Gebhard 2005, ALMA 모델 참고)
// ---------------------------------------------------------------------------

/// OCC 22개 감정 유형을 PAD 좌표로 변환
///
/// 대표값이며 플레이테스트로 튜닝 대상.
/// apply_stimulus에서 감정별 내적 계산에 사용.
pub fn emotion_to_pad(emotion: EmotionType) -> Pad {
    match emotion {
        // --- Event: Well-being ---
        EmotionType::Joy             => Pad::new( 0.40,  0.20,  0.10),
        EmotionType::Distress        => Pad::new(-0.40,  0.20, -0.50),

        // --- Event: Fortune-of-others ---
        EmotionType::HappyFor        => Pad::new( 0.40,  0.20,  0.20),
        EmotionType::Pity            => Pad::new(-0.40, -0.20, -0.50),
        EmotionType::Gloating        => Pad::new( 0.30,  0.30,  0.30),
        EmotionType::Resentment      => Pad::new(-0.20,  0.30, -0.20),

        // --- Event: Prospect-based ---
        EmotionType::Hope            => Pad::new( 0.20,  0.20, -0.10),
        EmotionType::Fear            => Pad::new(-0.64,  0.60, -0.43),
        EmotionType::Satisfaction    => Pad::new( 0.30, -0.20,  0.40),
        EmotionType::Disappointment  => Pad::new(-0.30, -0.40, -0.40),
        EmotionType::Relief          => Pad::new( 0.20, -0.30,  0.20),
        EmotionType::FearsConfirmed  => Pad::new(-0.50,  0.30, -0.60),

        // --- Action: Attribution ---
        EmotionType::Pride           => Pad::new( 0.40,  0.30,  0.30),
        EmotionType::Shame           => Pad::new(-0.30,  0.10, -0.60),
        EmotionType::Admiration      => Pad::new( 0.50,  0.30, -0.20),
        EmotionType::Reproach        => Pad::new(-0.30,  0.20,  0.40),

        // --- Action: Compound ---
        EmotionType::Gratification   => Pad::new( 0.50,  0.40,  0.40),
        EmotionType::Remorse         => Pad::new(-0.30,  0.10, -0.60),
        EmotionType::Gratitude       => Pad::new( 0.40,  0.20, -0.30),
        EmotionType::Anger           => Pad::new(-0.51,  0.59,  0.25),

        // --- Object ---
        EmotionType::Love            => Pad::new( 0.30,  0.10,  0.20),
        EmotionType::Hate            => Pad::new(-0.60,  0.60,  0.30),
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
/// 초기화 시 TextEmbedder로 앵커 임베딩을 사전 계산하고,
/// analyze() 시 대사 벡터와 앵커 간 유사도로 PAD를 추출.
///
/// 임베딩 모델을 교체해도 이 코드는 변경 없음.
pub struct PadAnalyzer {
    embedder: Box<dyn TextEmbedder + Send>,
    pleasure: AxisEmbeddings,
    arousal: AxisEmbeddings,
    dominance: AxisEmbeddings,
}

impl PadAnalyzer {
    /// TextEmbedder로 앵커 임베딩을 사전 계산하여 생성
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
    fn analyze(&mut self, utterance: &str) -> Pad {
        let embeddings = self.embedder
            .embed(&[utterance])
            .unwrap_or_default();

        if embeddings.is_empty() {
            return Pad::neutral();
        }

        self.to_pad(&embeddings[0])
    }
}
