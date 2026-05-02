use crate::domain::pad::{Pad, UtteranceEmbedding};

/// 인프라 포트: 텍스트 → 벡터 변환 (임베딩)
pub trait TextEmbedder {
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError>;
}

/// 임베딩 오류
#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("임베딩 모델 초기화 실패: {0}")]
    InitError(String),
    #[error("임베딩 추론 실패: {0}")]
    InferenceError(String),
}

/// 도메인 포트: 대사 → PAD 변환
pub trait UtteranceAnalyzer {
    /// 대사 텍스트 → PAD (Pleasure, Arousal, Dominance)
    fn analyze(&mut self, utterance: &str) -> Result<Pad, EmbedError>;

    /// 대사 텍스트 → (PAD, 발화 임베딩) — 후속 단계와 임베딩 공유용
    fn analyze_with_embedding(
        &mut self,
        utterance: &str,
    ) -> Result<(Pad, Option<UtteranceEmbedding>), EmbedError> {
        self.analyze(utterance).map(|pad| (pad, None))
    }
}
