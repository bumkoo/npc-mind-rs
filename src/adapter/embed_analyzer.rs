//! fastembed 기반 대사 감정 분석기
//!
//! bge-m3 임베딩으로 대사 텍스트를 PAD 3축 좌표로 변환.
//! 3축 × 양극단 앵커 텍스트의 평균 임베딩과 코사인 유사도를 계산.

use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

use crate::domain::pad::{
    Pad, PLEASURE_ANCHORS, AROUSAL_ANCHORS, DOMINANCE_ANCHORS,
};
use crate::ports::UtteranceAnalyzer;

/// 사전 계산된 앵커 임베딩 (축 하나의 양극단)
struct AxisEmbeddings {
    /// 양극단 평균 벡터
    positive: Vec<f32>,
    /// 음극단 평균 벡터
    negative: Vec<f32>,
}

/// fastembed(bge-m3) 기반 대사 감정 분석기
///
/// 초기화 시 앵커 임베딩을 사전 계산하고,
/// analyze() 호출 시 대사와 앵커 간 유사도로 PAD를 추출.
pub struct EmbedAnalyzer {
    model: TextEmbedding,
    pleasure: AxisEmbeddings,
    arousal: AxisEmbeddings,
    dominance: AxisEmbeddings,
}

impl EmbedAnalyzer {
    /// bge-m3 모델로 분석기 생성 (앵커 임베딩 사전 계산)
    pub fn new() -> Result<Self, anyhow::Error> {
        Self::with_model(EmbeddingModel::BGEM3)
    }

    /// 지정 모델로 분석기 생성
    pub fn with_model(model_type: EmbeddingModel) -> Result<Self, anyhow::Error> {
        let mut model = TextEmbedding::try_new(
            InitOptions::new(model_type).with_show_download_progress(true),
        )?;

        let pleasure = Self::embed_axis(&mut model, &PLEASURE_ANCHORS)?;
        let arousal = Self::embed_axis(&mut model, &AROUSAL_ANCHORS)?;
        let dominance = Self::embed_axis(&mut model, &DOMINANCE_ANCHORS)?;

        Ok(Self { model, pleasure, arousal, dominance })
    }

    /// 앵커 텍스트 → 평균 임베딩 벡터 계산
    fn embed_axis(
        model: &mut TextEmbedding,
        anchors: &crate::domain::pad::PadAxisAnchors,
    ) -> Result<AxisEmbeddings, anyhow::Error> {
        let pos_texts: Vec<&str> = anchors.positive.to_vec();
        let neg_texts: Vec<&str> = anchors.negative.to_vec();

        let pos_embeddings = model.embed(pos_texts, None)?;
        let neg_embeddings = model.embed(neg_texts, None)?;

        Ok(AxisEmbeddings {
            positive: Self::mean_vector(&pos_embeddings),
            negative: Self::mean_vector(&neg_embeddings),
        })
    }

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
    fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
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
        // 차이를 -1.0~1.0으로 클램핑
        (sim_pos - sim_neg).clamp(-1.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// UtteranceAnalyzer 포트 구현
// ---------------------------------------------------------------------------

impl UtteranceAnalyzer for EmbedAnalyzer {
    fn analyze(&mut self, utterance: &str) -> Pad {
        let embeddings = self.model
            .embed(vec![utterance], None)
            .unwrap_or_default();

        if embeddings.is_empty() {
            return Pad::neutral();
        }

        let emb = &embeddings[0];
        Pad::new(
            Self::axis_score(emb, &self.pleasure),
            Self::axis_score(emb, &self.arousal),
            Self::axis_score(emb, &self.dominance),
        )
    }
}
