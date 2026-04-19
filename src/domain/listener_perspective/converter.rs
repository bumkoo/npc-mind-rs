//! Listener-perspective Converter — Phase 7 통합 API
//!
//! Prefilter + Sign + Magnitude 를 조합해 화자 PAD → 청자 PAD 변환을 수행한다.
//!
//! ## 파이프라인
//!
//! ```text
//!                       ┌─ Prefilter hit ──→ (sign, magnitude, p_s_default)
//! utterance ─→ classify ┤
//!                       └─ Prefilter miss ─→ Sign classifier + Magnitude classifier
//!                                                   │
//!                                        (sign, magnitude)
//!                                                   │
//!                              P_L = sign × coef_p[magnitude] × P_S
//!                              A_L = coef_a[magnitude] × A_S
//!                              D_L = coef_d[magnitude] × D_S
//! ```
//!
//! ## 설계
//!
//! - `docs/emotion/phase7-converter-integration.md` §3
//! - `docs/emotion/sign-classifier-design.md` §3.1, §3.1.2, §3.5

use super::magnitude_classifier::{MagnitudeClassifier, MagnitudeClassifyResult};
use super::magnitude_coef::MagnitudeCoefTable;
use super::prefilter::Prefilter;
use super::prototype::PrototypeSet;
use super::sign_classifier::{SignClassifier, SignClassifyResult};
use super::types::{ListenerPerspectiveError, Magnitude, Sign};
use crate::domain::pad::Pad;
use crate::ports::TextEmbedder;

// ============================================================
// 결과 타입
// ============================================================

/// 변환이 어떤 경로로 이루어졌는가
#[derive(Debug, Clone)]
pub enum ConvertPath {
    /// 정규식 프리필터가 매칭 (Phase 3)
    Prefilter {
        category: String,
        pattern: String,
    },
    /// 분류기 경로 (sign + magnitude k-NN)
    Classifier {
        sign_margin: f32,
        magnitude_margin: f32,
    },
}

impl ConvertPath {
    pub fn is_prefilter(&self) -> bool {
        matches!(self, ConvertPath::Prefilter { .. })
    }

    pub fn is_classifier(&self) -> bool {
        matches!(self, ConvertPath::Classifier { .. })
    }
}

/// 변환 메타 — 디버깅·리포트·회귀 감시용
#[derive(Debug, Clone)]
pub struct ConvertMeta {
    pub path: ConvertPath,
    pub sign: Sign,
    pub magnitude: Magnitude,
    /// 적용된 P축 계수 (sign × coef_p[magnitude])
    pub applied_p_coef: f32,
    /// 적용된 A축 계수
    pub applied_a_coef: f32,
    /// 적용된 D축 계수
    pub applied_d_coef: f32,
}

/// 변환 결과
#[derive(Debug, Clone)]
pub struct ConvertResult {
    pub listener_pad: Pad,
    pub meta: ConvertMeta,
}

// ============================================================
// Trait — Listener-perspective 변환 추상화
// ============================================================

/// 화자 PAD → 청자 PAD 변환 추상화
///
/// 구현체는 자체 분류 로직을 가지며, 호출자는 utterance 와 speaker_pad 만 넘긴다.
/// 발화 임베딩은 호출자가 미리 계산하여 넘긴다 (PadAnalyzer 와 공유 가능).
pub trait ListenerPerspectiveConverter: Send + Sync {
    /// 발화 임베딩이 이미 계산된 상태에서 변환
    ///
    /// # 인자
    /// - `utterance`: 원본 발화 텍스트 (프리필터가 정규식 매칭에 사용)
    /// - `speaker_pad`: PadAnalyzer 가 이미 추출한 화자 PAD
    /// - `utterance_embedding`: 발화 임베딩 (분류기가 사용)
    fn convert(
        &self,
        utterance: &str,
        speaker_pad: &Pad,
        utterance_embedding: &[f32],
    ) -> Result<ConvertResult, ListenerPerspectiveError>;
}

// ============================================================
// 기본 구현 — EmbeddedConverter
// ============================================================

/// Prefilter + SignClassifier + MagnitudeClassifier 조합 구현체
///
/// 초기화 후 `TextEmbedder` 의존 없음. `convert()` 는 순수 수학.
/// `convert_from_text()` 는 편의 메서드 (내부 임베딩 실행).
pub struct EmbeddedConverter {
    prefilter: Prefilter,
    sign_classifier: SignClassifier,
    magnitude_classifier: MagnitudeClassifier,
    coef_table: MagnitudeCoefTable,
}

impl EmbeddedConverter {
    /// 세 구성 요소 + 기본 계수 테이블로 초기화
    pub fn new(
        prefilter: Prefilter,
        sign_classifier: SignClassifier,
        magnitude_classifier: MagnitudeClassifier,
    ) -> Self {
        Self {
            prefilter,
            sign_classifier,
            magnitude_classifier,
            coef_table: MagnitudeCoefTable::default(),
        }
    }

    /// Builder — 커스텀 계수 테이블 주입
    pub fn with_coef_table(mut self, coef_table: MagnitudeCoefTable) -> Self {
        self.coef_table = coef_table;
        self
    }

    // === 메타 조회 ===

    pub fn coef_table(&self) -> &MagnitudeCoefTable {
        &self.coef_table
    }

    pub fn prefilter(&self) -> &Prefilter {
        &self.prefilter
    }

    pub fn sign_classifier(&self) -> &SignClassifier {
        &self.sign_classifier
    }

    pub fn magnitude_classifier(&self) -> &MagnitudeClassifier {
        &self.magnitude_classifier
    }
}

// ============================================================
// 편의 빌더 — 파일 경로에서 일괄 초기화
// ============================================================

impl EmbeddedConverter {
    /// 파일 경로 세트로 일괄 초기화
    ///
    /// # 인자
    /// - `embedder`: 프로토타입 임베딩 계산용 (초기화 시 1회만 사용)
    /// - `patterns_path`: Prefilter 패턴 TOML
    /// - `sign_keep_path`, `sign_invert_path`: 부호 분류기 프로토타입
    /// - `mag_weak_path`, `mag_normal_path`, `mag_strong_path`: 강도 분류기 프로토타입
    #[allow(clippy::too_many_arguments)]
    pub fn from_paths<P: AsRef<std::path::Path>>(
        embedder: &mut dyn TextEmbedder,
        patterns_path: P,
        sign_keep_path: P,
        sign_invert_path: P,
        mag_weak_path: P,
        mag_normal_path: P,
        mag_strong_path: P,
    ) -> Result<Self, ListenerPerspectiveError> {
        let prefilter = Prefilter::from_path(patterns_path)?;
        let sign = SignClassifier::from_paths(
            embedder,
            sign_keep_path,
            sign_invert_path,
            SignClassifier::DEFAULT_K,
        )?;
        let magnitude = MagnitudeClassifier::from_paths(
            embedder,
            mag_weak_path,
            mag_normal_path,
            mag_strong_path,
            MagnitudeClassifier::DEFAULT_K,
        )?;
        Ok(Self::new(prefilter, sign, magnitude))
    }

    /// 이미 로드된 PrototypeSet 들로 초기화
    pub fn from_sets(
        embedder: &mut dyn TextEmbedder,
        prefilter: Prefilter,
        sign_keep: PrototypeSet,
        sign_invert: PrototypeSet,
        mag_weak: PrototypeSet,
        mag_normal: PrototypeSet,
        mag_strong: PrototypeSet,
    ) -> Result<Self, ListenerPerspectiveError> {
        let sign = SignClassifier::new(
            embedder,
            sign_keep,
            sign_invert,
            SignClassifier::DEFAULT_K,
        )?;
        let magnitude = MagnitudeClassifier::new(
            embedder,
            mag_weak,
            mag_normal,
            mag_strong,
            MagnitudeClassifier::DEFAULT_K,
        )?;
        Ok(Self::new(prefilter, sign, magnitude))
    }
}

// ============================================================
// 내부 변환 로직
// ============================================================

impl EmbeddedConverter {
    /// 내부용 — sign/magnitude 및 입력 PAD 로부터 listener PAD 및 meta 생성
    ///
    /// `p_s_override`: prefilter hit 경로에서 `p_s_default` 를 쓰기 위함.
    ///                 None 이면 `speaker_pad.pleasure` 사용.
    fn build_result(
        &self,
        sign: Sign,
        magnitude: Magnitude,
        speaker_pad: &Pad,
        p_s_override: Option<f32>,
        path: ConvertPath,
    ) -> ConvertResult {
        let p_coef = self.coef_table.p_coef(magnitude);
        let a_coef = self.coef_table.a_coef(magnitude);
        let d_coef = self.coef_table.d_coef(magnitude);

        let p_s = p_s_override.unwrap_or(speaker_pad.pleasure);

        let listener_pad = Pad::new(
            sign.as_f32() * p_coef * p_s,
            a_coef * speaker_pad.arousal,
            d_coef * speaker_pad.dominance,
        );

        let meta = ConvertMeta {
            path,
            sign,
            magnitude,
            applied_p_coef: sign.as_f32() * p_coef,
            applied_a_coef: a_coef,
            applied_d_coef: d_coef,
        };

        ConvertResult { listener_pad, meta }
    }

    /// 편의 메서드 — 발화 문자열과 embedder 가 있을 때 내부에서 임베딩 후 변환
    pub fn convert_from_text(
        &self,
        utterance: &str,
        speaker_pad: &Pad,
        embedder: &mut dyn TextEmbedder,
    ) -> Result<ConvertResult, ListenerPerspectiveError> {
        let embeddings = embedder
            .embed(&[utterance])
            .map_err(|e| ListenerPerspectiveError::Embed(format!("utterance: {:?}", e)))?;
        if embeddings.is_empty() {
            return Err(ListenerPerspectiveError::Embed(
                "empty result".to_string(),
            ));
        }
        self.convert(utterance, speaker_pad, &embeddings[0])
    }
}

// ============================================================
// 공유 fallback 헬퍼 — 호출 사이트 통합용
// ============================================================

/// Converter 변환을 시도하고 실패/입력 부족 시 화자 PAD를 그대로 반환.
///
/// DialogueAgent와 Mind Studio(StudioService)가 동일 분기 로직을 가져 path-for-path
/// drift 위험이 있던 부분을 도메인 레벨 단일 헬퍼로 통합한다.
///
/// 변환 조건: `converter`와 `embedding`이 모두 `Some`일 때만 변환 시도.
/// 변환 실패 시 `tracing::warn!`을 남기고 화자 PAD 그대로 반환 (silent failure 방지).
/// 변환 성공 시 호출자가 추가 디버깅 로그를 원하면 결과 PAD를 사용해 별도 처리.
///
/// 입력이 부족한 경로(converter 미주입, 임베딩 부재)는 silent passthrough — 정상 동작.
pub fn convert_or_fallback(
    converter: Option<&dyn ListenerPerspectiveConverter>,
    utterance: &str,
    speaker_pad: Pad,
    embedding: Option<&[f32]>,
) -> Pad {
    let (Some(converter), Some(emb)) = (converter, embedding) else {
        return speaker_pad;
    };
    match converter.convert(utterance, &speaker_pad, emb) {
        Ok(result) => result.listener_pad,
        Err(e) => {
            tracing::warn!(
                error = ?e,
                utterance = utterance,
                "listener-perspective conversion failed; falling back to speaker PAD"
            );
            speaker_pad
        }
    }
}

// ============================================================
// Trait 구현 — 핵심 변환
// ============================================================

impl ListenerPerspectiveConverter for EmbeddedConverter {
    fn convert(
        &self,
        utterance: &str,
        speaker_pad: &Pad,
        utterance_embedding: &[f32],
    ) -> Result<ConvertResult, ListenerPerspectiveError> {
        // 1. Prefilter 먼저
        if let Some(hit) = self.prefilter.classify(utterance) {
            let path = ConvertPath::Prefilter {
                category: hit.matched_category.clone(),
                pattern: hit.matched_pattern.clone(),
            };
            return Ok(self.build_result(
                hit.sign,
                hit.magnitude,
                speaker_pad,
                Some(hit.p_s_default),
                path,
            ));
        }

        // 2. Classifier 경로
        let sign_result: SignClassifyResult =
            self.sign_classifier.classify(utterance_embedding);
        let mag_result: MagnitudeClassifyResult =
            self.magnitude_classifier.classify(utterance_embedding);

        let path = ConvertPath::Classifier {
            sign_margin: sign_result.margin,
            magnitude_margin: mag_result.margin,
        };

        Ok(self.build_result(
            sign_result.predicted,
            mag_result.predicted,
            speaker_pad,
            None,
            path,
        ))
    }
}

// ============================================================
// 단위 테스트 (Mock embedder 기반)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::listener_perspective::prototype::load_prototypes_from_toml;
    use crate::ports::EmbedError;

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

    const PREFILTER_TOML: &str = r#"
[meta]
version = "t"

[[category]]
name = "sarcasm_interjection"
sign = "invert"
magnitude = "strong"
p_s_default = 0.6
description = "감탄사 빈정"
patterns = [
    "^(허허|아이고)",
]
"#;

    const KEEP_TOML: &str = r#"
[meta]
version = "t"
group = "sign_keep"
[prototypes]
items = [
    { text = "K1", subtype = "gratitude" },
    { text = "K2", subtype = "praise" },
]
"#;

    const INVERT_TOML: &str = r#"
[meta]
version = "t"
group = "sign_invert"
[prototypes]
items = [
    { text = "I1", subtype = "apology" },
    { text = "I2", subtype = "plea" },
]
"#;

    const MAG_WEAK_TOML: &str = r#"
[meta]
version = "t"
group = "magnitude_weak"
[prototypes]
items = [
    { text = "W1", subtype = "apology" },
    { text = "W2", subtype = "plea" },
]
"#;

    const MAG_NORMAL_TOML: &str = r#"
[meta]
version = "t"
group = "magnitude_normal"
[prototypes]
items = [
    { text = "N1", subtype = "gratitude" },
    { text = "N2", subtype = "praise" },
]
"#;

    const MAG_STRONG_TOML: &str = r#"
[meta]
version = "t"
group = "magnitude_strong"
[prototypes]
items = [
    { text = "S1", subtype = "extreme_praise" },
    { text = "S2", subtype = "threat" },
]
"#;

    fn build_converter() -> EmbeddedConverter {
        let mut embedder = MockEmbedder::new(vec![
            // sign prototypes
            ("K1", vec![1.0, 0.0, 0.0]),
            ("K2", vec![0.9, 0.1, 0.0]),
            ("I1", vec![-1.0, 0.0, 0.0]),
            ("I2", vec![-0.9, -0.1, 0.0]),
            // magnitude prototypes (orthogonal axes)
            ("W1", vec![0.0, 1.0, 0.0]),
            ("W2", vec![0.0, 0.9, 0.1]),
            ("N1", vec![0.0, 0.0, 1.0]),
            ("N2", vec![0.1, 0.0, 0.9]),
            ("S1", vec![1.0, 1.0, 1.0]),
            ("S2", vec![0.9, 0.9, 1.0]),
        ]);

        let prefilter = Prefilter::from_toml(PREFILTER_TOML).unwrap();
        let keep = load_prototypes_from_toml(KEEP_TOML, "sign_keep").unwrap();
        let invert = load_prototypes_from_toml(INVERT_TOML, "sign_invert").unwrap();
        let weak = load_prototypes_from_toml(MAG_WEAK_TOML, "magnitude_weak").unwrap();
        let normal = load_prototypes_from_toml(MAG_NORMAL_TOML, "magnitude_normal").unwrap();
        let strong = load_prototypes_from_toml(MAG_STRONG_TOML, "magnitude_strong").unwrap();

        EmbeddedConverter::from_sets(
            &mut embedder, prefilter, keep, invert, weak, normal, strong,
        )
        .unwrap()
    }

    #[test]
    fn prefilter_hit_uses_category_values() {
        let conv = build_converter();
        // 화자 PAD: P_S=+0.3, A_S=+0.2, D_S=-0.1
        let speaker = Pad::new(0.3, 0.2, -0.1);

        let result = conv.convert("허허, 참 훌륭하시다", &speaker, &[0.0, 0.0, 0.0])
            .unwrap();

        // prefilter sarcasm_interjection: sign=invert, magnitude=strong, p_s_default=0.6
        assert!(result.meta.path.is_prefilter());
        assert_eq!(result.meta.sign, Sign::Invert);
        assert_eq!(result.meta.magnitude, Magnitude::Strong);

        // P_L = -1 × 1.5 × 0.6 = -0.9 (speaker.pleasure 무시, p_s_default 사용)
        assert!((result.listener_pad.pleasure - (-0.9)).abs() < 1e-5);
        // A_L = 1.3 × 0.2 = 0.26 (화자 A 사용)
        assert!((result.listener_pad.arousal - 0.26).abs() < 1e-5);
        // D_L = 1.3 × -0.1 = -0.13
        assert!((result.listener_pad.dominance - (-0.13)).abs() < 1e-5);
    }

    #[test]
    fn classifier_path_uses_speaker_p() {
        let conv = build_converter();
        // 발화 임베딩이 keep 쪽에 가까움 ([1,0,0] — K1,K2 쪽)
        // magnitude는 strong 축 ([1,1,1] — S1,S2 쪽) 에 가깝게 유도 위해
        // 발화 임베딩 [0.8, 0.3, 0.3] — keep + 약간의 strong 성분
        let speaker = Pad::new(0.4, 0.5, 0.2);
        let utt_emb = vec![0.8, 0.3, 0.3];

        let result = conv.convert("일반 발화", &speaker, &utt_emb).unwrap();

        assert!(result.meta.path.is_classifier());
        assert_eq!(result.meta.sign, Sign::Keep);

        // 변환식 검증: P_L = +1 × p_coef × 0.4
        let expected_p = 1.0 * conv.coef_table.p_coef(result.meta.magnitude) * 0.4;
        assert!((result.listener_pad.pleasure - expected_p).abs() < 1e-5);
    }

    #[test]
    fn custom_coef_table_overrides_default() {
        let custom = MagnitudeCoefTable {
            strong_p: 2.0,  // default 1.5 → 2.0
            ..Default::default()
        };
        let conv = build_converter().with_coef_table(custom);

        let speaker = Pad::new(0.0, 0.0, 0.0);
        let result = conv.convert("허허, 빈정", &speaker, &[0.0, 0.0, 0.0]).unwrap();

        // prefilter strong 경로, sign=invert, p_s_default=0.6
        // P_L = -1 × 2.0 × 0.6 = -1.2
        assert!((result.listener_pad.pleasure - (-1.2)).abs() < 1e-5);
        assert_eq!(conv.coef_table().strong_p, 2.0);
    }
}
