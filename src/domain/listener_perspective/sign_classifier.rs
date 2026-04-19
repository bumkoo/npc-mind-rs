//! Sign 축 k-NN 분류기 (Phase 1)
//!
//! 발화의 listener-perspective P 부호 (Keep / Invert) 를 분류한다.
//!
//! ## 파이프라인
//!
//! ```text
//! utterance embedding  ──┐
//!                        │
//! keep 프로토타입 (사전) ─┼─→ cosine_sim (각 프로토별)
//! invert 프로토타입 (사전)─┘
//!                        │
//!                  그룹별 top-k 평균
//!                        │
//!                   최대 점수 → Sign
//! ```
//!
//! ## 초기화 시 1회
//!
//! TextEmbedder 로 프로토타입 임베딩을 미리 계산하여 내부 보관.
//! 이후 `classify()` 는 발화 임베딩만 받아 순수 수학으로 분류.
//!
//! 설계: `docs/emotion/sign-classifier-design.md` §3.2

use super::classifier::{cosine_sim, top_k_mean_sorted};
use super::prototype::{load_prototypes_from_path, PrototypeSet};
use super::types::{ListenerPerspectiveError, Sign};
use crate::ports::TextEmbedder;
use std::path::Path;

// ============================================================
// 공개 타입
// ============================================================

/// Sign 분류 결과
#[derive(Debug, Clone)]
pub struct SignClassifyResult {
    /// 예측된 부호
    pub predicted: Sign,
    /// keep 그룹 top-k 평균
    pub keep_score: f32,
    /// invert 그룹 top-k 평균
    pub invert_score: f32,
    /// |keep − invert| — 신뢰도 지표
    pub margin: f32,
}

impl SignClassifyResult {
    /// margin 이 threshold 이상이면 신뢰 가능한 분류로 간주
    pub fn is_confident(&self, threshold: f32) -> bool {
        self.margin >= threshold
    }
}

// ============================================================
// 분류기 본체
// ============================================================

/// 2-way Sign 분류기 (keep vs invert)
///
/// 초기화 시 keep/invert 프로토타입 임베딩을 내부에 보관.
/// 이후 `classify()` 는 순수 수학 — TextEmbedder 의존 없음.
pub struct SignClassifier {
    keep: PrototypeSet,
    invert: PrototypeSet,
    keep_embeddings: Vec<Vec<f32>>,
    invert_embeddings: Vec<Vec<f32>>,
    k: usize,
}

impl SignClassifier {
    /// 기본값 k=3 으로 생성
    pub const DEFAULT_K: usize = 3;

    /// 파일 경로로 분류기 초기화
    ///
    /// 각 프로토타입 TOML 파일을 읽고, `embedder` 로 사전 임베딩을 계산한다.
    pub fn from_paths<P: AsRef<Path>>(
        embedder: &mut dyn TextEmbedder,
        keep_path: P,
        invert_path: P,
        k: usize,
    ) -> Result<Self, ListenerPerspectiveError> {
        let keep = load_prototypes_from_path(keep_path, "sign_keep")?;
        let invert = load_prototypes_from_path(invert_path, "sign_invert")?;
        Self::new(embedder, keep, invert, k)
    }

    /// 이미 로드된 PrototypeSet 으로 분류기 초기화
    pub fn new(
        embedder: &mut dyn TextEmbedder,
        keep: PrototypeSet,
        invert: PrototypeSet,
        k: usize,
    ) -> Result<Self, ListenerPerspectiveError> {
        let keep_texts = keep.texts();
        let keep_embeddings = embedder
            .embed(&keep_texts)
            .map_err(|e| ListenerPerspectiveError::Embed(format!("keep: {:?}", e)))?;

        let invert_texts = invert.texts();
        let invert_embeddings = embedder
            .embed(&invert_texts)
            .map_err(|e| ListenerPerspectiveError::Embed(format!("invert: {:?}", e)))?;

        Ok(Self {
            keep,
            invert,
            keep_embeddings,
            invert_embeddings,
            k: k.max(1),
        })
    }
}

impl SignClassifier {
    /// 발화 임베딩으로 분류
    ///
    /// 기존 벤치와 수학적으로 동일한 결과 반환 (회귀 감시 포인트).
    pub fn classify(&self, utterance_embedding: &[f32]) -> SignClassifyResult {
        let keep_score = self.group_top_k_score(utterance_embedding, &self.keep_embeddings);
        let invert_score = self.group_top_k_score(utterance_embedding, &self.invert_embeddings);

        let predicted = if keep_score >= invert_score {
            Sign::Keep
        } else {
            Sign::Invert
        };
        let margin = (keep_score - invert_score).abs();

        SignClassifyResult {
            predicted,
            keep_score,
            invert_score,
            margin,
        }
    }

    /// 한 그룹에 대한 top-k 평균 유사도
    fn group_top_k_score(&self, query: &[f32], group_embeddings: &[Vec<f32>]) -> f32 {
        let mut sims: Vec<f32> = group_embeddings
            .iter()
            .map(|emb| cosine_sim(query, emb))
            .collect();
        sims.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        top_k_mean_sorted(&sims, self.k)
    }

    // === 메타 조회 ===

    pub fn keep_set(&self) -> &PrototypeSet {
        &self.keep
    }

    pub fn invert_set(&self) -> &PrototypeSet {
        &self.invert
    }

    pub fn k(&self) -> usize {
        self.k
    }
}

// ============================================================
// 단위 테스트 — Mock Embedder 로 분류 로직 검증
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::EmbedError;

    /// 결정론적 mock — 텍스트를 고정 임베딩에 매핑
    struct MockEmbedder {
        table: Vec<(String, Vec<f32>)>,
    }

    impl MockEmbedder {
        fn new(pairs: Vec<(&str, Vec<f32>)>) -> Self {
            Self {
                table: pairs
                    .into_iter()
                    .map(|(s, v)| (s.to_string(), v))
                    .collect(),
            }
        }
    }

    impl TextEmbedder for MockEmbedder {
        fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
            texts
                .iter()
                .map(|t| {
                    self.table
                        .iter()
                        .find(|(k, _)| k == t)
                        .map(|(_, v)| v.clone())
                        .ok_or_else(|| EmbedError::InferenceError(format!("mock miss: {}", t)))
                })
                .collect()
        }
    }

    fn sample_keep() -> PrototypeSet {
        let toml = r#"
[meta]
version = "t"
group = "sign_keep"
[prototypes]
items = [
    { text = "K1", subtype = "gratitude" },
    { text = "K2", subtype = "praise" },
]
"#;
        crate::domain::listener_perspective::prototype::load_prototypes_from_toml(toml, "sign_keep")
            .unwrap()
    }

    fn sample_invert() -> PrototypeSet {
        let toml = r#"
[meta]
version = "t"
group = "sign_invert"
[prototypes]
items = [
    { text = "I1", subtype = "apology" },
    { text = "I2", subtype = "plea" },
]
"#;
        crate::domain::listener_perspective::prototype::load_prototypes_from_toml(toml, "sign_invert")
            .unwrap()
    }

    #[test]
    fn predicts_keep_when_closer_to_keep() {
        let mut embedder = MockEmbedder::new(vec![
            ("K1", vec![1.0, 0.0]),
            ("K2", vec![0.9, 0.1]),
            ("I1", vec![-1.0, 0.0]),
            ("I2", vec![-0.9, -0.1]),
        ]);
        let clf = SignClassifier::new(&mut embedder, sample_keep(), sample_invert(), 2).unwrap();

        // 발화 임베딩이 [1.0, 0.0] — keep 쪽과 가까움
        let result = clf.classify(&[1.0, 0.0]);
        assert_eq!(result.predicted, Sign::Keep);
        assert!(result.keep_score > result.invert_score);
        assert!(result.margin > 0.5);
    }

    #[test]
    fn predicts_invert_when_closer_to_invert() {
        let mut embedder = MockEmbedder::new(vec![
            ("K1", vec![1.0, 0.0]),
            ("K2", vec![0.9, 0.1]),
            ("I1", vec![-1.0, 0.0]),
            ("I2", vec![-0.9, -0.1]),
        ]);
        let clf = SignClassifier::new(&mut embedder, sample_keep(), sample_invert(), 2).unwrap();

        let result = clf.classify(&[-1.0, 0.0]);
        assert_eq!(result.predicted, Sign::Invert);
        assert!(result.invert_score > result.keep_score);
    }

    #[test]
    fn tie_breaks_to_keep() {
        // 동점 시 keep 우선 (>=) — 기존 bench 와 동일 규칙
        let mut embedder = MockEmbedder::new(vec![
            ("K1", vec![1.0, 0.0]),
            ("K2", vec![0.0, 1.0]),
            ("I1", vec![1.0, 0.0]),
            ("I2", vec![0.0, 1.0]),
        ]);
        let clf = SignClassifier::new(&mut embedder, sample_keep(), sample_invert(), 2).unwrap();

        let result = clf.classify(&[0.5, 0.5]);
        assert_eq!(result.predicted, Sign::Keep);
        assert_eq!(result.margin, 0.0);
    }

    #[test]
    fn confidence_threshold() {
        let result = SignClassifyResult {
            predicted: Sign::Keep,
            keep_score: 0.7,
            invert_score: 0.65,
            margin: 0.05,
        };
        assert!(result.is_confident(0.02));
        assert!(!result.is_confident(0.10));
    }
}
