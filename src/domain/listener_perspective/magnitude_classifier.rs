//! Magnitude 축 k-NN 분류기 (Phase 4)
//!
//! 발화의 listener-perspective P 강도 (Weak / Normal / Strong) 를 분류한다.
//!
//! ## 구조
//!
//! `SignClassifier` 와 동일한 k-NN top-k 패턴. 3-way 확장:
//! - weak / normal / strong 프로토타입을 각각 보관
//! - 세 그룹의 top-k 평균 점수 중 최대값 채택
//! - margin = top1 − top2 (3-way 에서는 2-way 와 다름)
//!
//! 설계: `docs/emotion/sign-classifier-design.md` §3.1, §3.3
//!       `docs/emotion/phase7-converter-integration.md` §3

use super::classifier::{cosine_sim, top_k_mean_sorted};
use super::prototype::{load_prototypes_from_path, PrototypeSet};
use super::types::{ListenerPerspectiveError, Magnitude};
use crate::ports::TextEmbedder;
use std::path::Path;

// ============================================================
// 공개 타입
// ============================================================

/// Magnitude 분류 결과
#[derive(Debug, Clone)]
pub struct MagnitudeClassifyResult {
    pub predicted: Magnitude,
    pub weak_score: f32,
    pub normal_score: f32,
    pub strong_score: f32,
    /// top1 − top2 점수차 (3-way 신뢰도 지표)
    pub margin: f32,
}

impl MagnitudeClassifyResult {
    pub fn is_confident(&self, threshold: f32) -> bool {
        self.margin >= threshold
    }

    /// 그룹별 점수 (for 디버깅·리포트)
    pub fn scores(&self) -> [(Magnitude, f32); 3] {
        [
            (Magnitude::Weak, self.weak_score),
            (Magnitude::Normal, self.normal_score),
            (Magnitude::Strong, self.strong_score),
        ]
    }
}

// ============================================================
// 분류기 본체
// ============================================================

/// 3-way Magnitude 분류기 (weak / normal / strong)
pub struct MagnitudeClassifier {
    weak: PrototypeSet,
    normal: PrototypeSet,
    strong: PrototypeSet,
    weak_embeddings: Vec<Vec<f32>>,
    normal_embeddings: Vec<Vec<f32>>,
    strong_embeddings: Vec<Vec<f32>>,
    k: usize,
}

impl MagnitudeClassifier {
    pub const DEFAULT_K: usize = 3;

    /// 파일 경로로 분류기 초기화
    pub fn from_paths<P: AsRef<Path>>(
        embedder: &mut dyn TextEmbedder,
        weak_path: P,
        normal_path: P,
        strong_path: P,
        k: usize,
    ) -> Result<Self, ListenerPerspectiveError> {
        let weak = load_prototypes_from_path(weak_path, "magnitude_weak")?;
        let normal = load_prototypes_from_path(normal_path, "magnitude_normal")?;
        let strong = load_prototypes_from_path(strong_path, "magnitude_strong")?;
        Self::new(embedder, weak, normal, strong, k)
    }

    pub fn new(
        embedder: &mut dyn TextEmbedder,
        weak: PrototypeSet,
        normal: PrototypeSet,
        strong: PrototypeSet,
        k: usize,
    ) -> Result<Self, ListenerPerspectiveError> {
        let weak_texts = weak.texts();
        let weak_embeddings = embedder
            .embed(&weak_texts)
            .map_err(|e| ListenerPerspectiveError::Embed(format!("weak: {:?}", e)))?;

        let normal_texts = normal.texts();
        let normal_embeddings = embedder
            .embed(&normal_texts)
            .map_err(|e| ListenerPerspectiveError::Embed(format!("normal: {:?}", e)))?;

        let strong_texts = strong.texts();
        let strong_embeddings = embedder
            .embed(&strong_texts)
            .map_err(|e| ListenerPerspectiveError::Embed(format!("strong: {:?}", e)))?;

        Ok(Self {
            weak,
            normal,
            strong,
            weak_embeddings,
            normal_embeddings,
            strong_embeddings,
            k: k.max(1),
        })
    }
}

impl MagnitudeClassifier {
    /// 발화 임베딩으로 분류
    ///
    /// 기존 벤치 (magnitude_classifier_bench.rs) 와 수학적으로 동일.
    pub fn classify(&self, utterance_embedding: &[f32]) -> MagnitudeClassifyResult {
        let weak_score = self.group_top_k_score(utterance_embedding, &self.weak_embeddings);
        let normal_score = self.group_top_k_score(utterance_embedding, &self.normal_embeddings);
        let strong_score = self.group_top_k_score(utterance_embedding, &self.strong_embeddings);

        let (predicted, margin) = Self::pick_top(weak_score, normal_score, strong_score);

        MagnitudeClassifyResult {
            predicted,
            weak_score,
            normal_score,
            strong_score,
            margin,
        }
    }

    fn group_top_k_score(&self, query: &[f32], group_embeddings: &[Vec<f32>]) -> f32 {
        let mut sims: Vec<f32> = group_embeddings
            .iter()
            .map(|emb| cosine_sim(query, emb))
            .collect();
        sims.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        top_k_mean_sorted(&sims, self.k)
    }

    /// 3개 점수 중 최대값과 top1-top2 margin 반환
    ///
    /// 동점 시 우선순위: weak < normal < strong (강도 낮은 쪽 선호).
    /// 이는 기존 bench 의 내림차순 정렬 + `max_by` 동작과 일치.
    fn pick_top(weak: f32, normal: f32, strong: f32) -> (Magnitude, f32) {
        let mut arr = [
            (Magnitude::Weak, weak),
            (Magnitude::Normal, normal),
            (Magnitude::Strong, strong),
        ];
        arr.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let margin = arr[0].1 - arr[1].1;
        (arr[0].0, margin)
    }

    // === 메타 조회 ===

    pub fn weak_set(&self) -> &PrototypeSet { &self.weak }
    pub fn normal_set(&self) -> &PrototypeSet { &self.normal }
    pub fn strong_set(&self) -> &PrototypeSet { &self.strong }
    pub fn k(&self) -> usize { self.k }
}

// ============================================================
// 단위 테스트
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::EmbedError;

    struct MockEmbedder {
        table: Vec<(String, Vec<f32>)>,
    }

    impl MockEmbedder {
        fn new(pairs: Vec<(&str, Vec<f32>)>) -> Self {
            Self {
                table: pairs.into_iter().map(|(s, v)| (s.to_string(), v)).collect(),
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

    fn build_three_sets() -> (PrototypeSet, PrototypeSet, PrototypeSet) {
        use crate::domain::listener_perspective::prototype::load_prototypes_from_toml;
        let weak_toml = r#"
[meta]
version = "t"
group = "magnitude_weak"
[prototypes]
items = [ { text = "W1", subtype = "apology" }, { text = "W2", subtype = "plea" } ]
"#;
        let normal_toml = r#"
[meta]
version = "t"
group = "magnitude_normal"
[prototypes]
items = [ { text = "N1", subtype = "gratitude" }, { text = "N2", subtype = "praise" } ]
"#;
        let strong_toml = r#"
[meta]
version = "t"
group = "magnitude_strong"
[prototypes]
items = [ { text = "S1", subtype = "extreme_praise" }, { text = "S2", subtype = "threat" } ]
"#;
        (
            load_prototypes_from_toml(weak_toml, "magnitude_weak").unwrap(),
            load_prototypes_from_toml(normal_toml, "magnitude_normal").unwrap(),
            load_prototypes_from_toml(strong_toml, "magnitude_strong").unwrap(),
        )
    }

    #[test]
    fn predicts_strong_when_closer_to_strong_protos() {
        let mut embedder = MockEmbedder::new(vec![
            ("W1", vec![1.0, 0.0, 0.0]),
            ("W2", vec![0.9, 0.1, 0.0]),
            ("N1", vec![0.0, 1.0, 0.0]),
            ("N2", vec![0.1, 0.9, 0.0]),
            ("S1", vec![0.0, 0.0, 1.0]),
            ("S2", vec![0.1, 0.0, 0.9]),
        ]);
        let (w, n, s) = build_three_sets();
        let clf = MagnitudeClassifier::new(&mut embedder, w, n, s, 2).unwrap();

        // 발화 임베딩이 strong 축에 가까움
        let result = clf.classify(&[0.0, 0.0, 1.0]);
        assert_eq!(result.predicted, Magnitude::Strong);
        assert!(result.strong_score > result.normal_score);
        assert!(result.strong_score > result.weak_score);
        assert!(result.margin > 0.3);
    }

    #[test]
    fn predicts_weak_when_closer_to_weak_protos() {
        let mut embedder = MockEmbedder::new(vec![
            ("W1", vec![1.0, 0.0, 0.0]),
            ("W2", vec![0.9, 0.1, 0.0]),
            ("N1", vec![0.0, 1.0, 0.0]),
            ("N2", vec![0.1, 0.9, 0.0]),
            ("S1", vec![0.0, 0.0, 1.0]),
            ("S2", vec![0.1, 0.0, 0.9]),
        ]);
        let (w, n, s) = build_three_sets();
        let clf = MagnitudeClassifier::new(&mut embedder, w, n, s, 2).unwrap();

        let result = clf.classify(&[1.0, 0.0, 0.0]);
        assert_eq!(result.predicted, Magnitude::Weak);
    }

    #[test]
    fn predicts_normal_when_closer_to_normal_protos() {
        let mut embedder = MockEmbedder::new(vec![
            ("W1", vec![1.0, 0.0, 0.0]),
            ("W2", vec![0.9, 0.1, 0.0]),
            ("N1", vec![0.0, 1.0, 0.0]),
            ("N2", vec![0.1, 0.9, 0.0]),
            ("S1", vec![0.0, 0.0, 1.0]),
            ("S2", vec![0.1, 0.0, 0.9]),
        ]);
        let (w, n, s) = build_three_sets();
        let clf = MagnitudeClassifier::new(&mut embedder, w, n, s, 2).unwrap();

        let result = clf.classify(&[0.0, 1.0, 0.0]);
        assert_eq!(result.predicted, Magnitude::Normal);
    }

    #[test]
    fn pick_top_tie_breaks_lowest_intensity() {
        // weak = normal = strong 인 경우, Weak 가 첫 배열 위치라 우선
        // 실용적 의미: 모든 그룹 동점이면 "약한 쪽" 선택 (안전한 기본값)
        let (picked, margin) = MagnitudeClassifier::pick_top(0.5, 0.5, 0.5);
        assert_eq!(picked, Magnitude::Weak);
        assert_eq!(margin, 0.0);
    }

    #[test]
    fn scores_method_returns_all_three() {
        let result = MagnitudeClassifyResult {
            predicted: Magnitude::Normal,
            weak_score: 0.3,
            normal_score: 0.7,
            strong_score: 0.5,
            margin: 0.2,
        };
        let scores = result.scores();
        assert_eq!(scores.len(), 3);
        assert_eq!(scores[0], (Magnitude::Weak, 0.3));
        assert_eq!(scores[1], (Magnitude::Normal, 0.7));
        assert_eq!(scores[2], (Magnitude::Strong, 0.5));
    }
}
