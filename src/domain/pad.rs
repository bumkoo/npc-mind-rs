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
use super::tuning::{PAD_AXIS_DEAD_ZONE, PAD_AXIS_SCALE, PAD_D_SCALE_WEIGHT};

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
        Self {
            pleasure,
            arousal,
            dominance,
        }
    }

    /// 중립 (0, 0, 0)
    pub fn neutral() -> Self {
        Self {
            pleasure: 0.0,
            arousal: 0.0,
            dominance: 0.0,
        }
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
        EmotionType::Joy => JOY_PAD,
        EmotionType::Distress => DISTRESS_PAD,
        EmotionType::HappyFor => HAPPY_FOR_PAD,
        EmotionType::Pity => PITY_PAD,
        EmotionType::Gloating => GLOATING_PAD,
        EmotionType::Resentment => RESENTMENT_PAD,
        EmotionType::Hope => HOPE_PAD,
        EmotionType::Fear => FEAR_PAD,
        EmotionType::Satisfaction => SATISFACTION_PAD,
        EmotionType::Disappointment => DISAPPOINTMENT_PAD,
        EmotionType::Relief => RELIEF_PAD,
        EmotionType::FearsConfirmed => FEARS_CONFIRMED_PAD,
        EmotionType::Pride => PRIDE_PAD,
        EmotionType::Shame => SHAME_PAD,
        EmotionType::Admiration => ADMIRATION_PAD,
        EmotionType::Reproach => REPROACH_PAD,
        EmotionType::Gratification => GRATIFICATION_PAD,
        EmotionType::Remorse => REMORSE_PAD,
        EmotionType::Gratitude => GRATITUDE_PAD,
        EmotionType::Anger => ANGER_PAD,
        EmotionType::Love => LOVE_PAD,
        EmotionType::Hate => HATE_PAD,
    }
}

// ---------------------------------------------------------------------------
// PadAnalyzer — 도메인 서비스 (업무 규칙)
// ---------------------------------------------------------------------------
//
// TextEmbedder(인프라 포트)에만 의존하며, 구체적 임베딩 모델을 모른다.
// cosine_sim, mean_vector, axis_score는 순수 수학 — 인프라 무관.

use crate::ports::{EmbedError, PadAnchorSource, TextEmbedder};

// ---------------------------------------------------------------------------
// 외부 로드용 도메인 타입
// ---------------------------------------------------------------------------

/// 발화 임베딩 — 임베딩 모델 출력의 도메인 newtype.
///
/// `UtteranceAnalyzer`가 PAD 추출에 사용한 동일 벡터를 후속 변환 단계
/// (`ListenerPerspectiveConverter` 등)에 공유 전달하는 데 쓰인다. newtype은
/// "이것은 발화 임베딩이다"라는 의미를 전달하고 임의 `Vec<f32>`와 구분되게 한다.
///
/// `Deref<Target = [f32]>` / `AsRef<[f32]>` 구현으로 분류기가 요구하는
/// `&[f32]` 타입에 자연스럽게 강제 변환된다.
#[derive(Debug, Clone, PartialEq)]
pub struct UtteranceEmbedding(pub Vec<f32>);

impl UtteranceEmbedding {
    pub fn new(values: Vec<f32>) -> Self {
        Self(values)
    }

    pub fn into_inner(self) -> Vec<f32> {
        self.0
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }
}

impl std::ops::Deref for UtteranceEmbedding {
    type Target = [f32];
    fn deref(&self) -> &[f32] {
        &self.0
    }
}

impl AsRef<[f32]> for UtteranceEmbedding {
    fn as_ref(&self) -> &[f32] {
        &self.0
    }
}

impl From<Vec<f32>> for UtteranceEmbedding {
    fn from(v: Vec<f32>) -> Self {
        Self(v)
    }
}

/// 외부 로드된 앵커 텍스트 (축 하나)
pub struct PadAxisAnchorsOwned {
    pub positive: Vec<String>,
    pub negative: Vec<String>,
}

/// 3축 전체 앵커 세트 (외부 소스에서 로드)
pub struct PadAnchorSet {
    pub pleasure: PadAxisAnchorsOwned,
    pub arousal: PadAxisAnchorsOwned,
    pub dominance: PadAxisAnchorsOwned,
}

/// 캐싱된 축 임베딩 (사전 계산된 평균 벡터)
#[derive(Clone, Serialize, Deserialize)]
pub struct CachedAxisEmbeddings {
    pub positive_mean: Vec<f32>,
    pub negative_mean: Vec<f32>,
}

/// 3축 전체 캐싱 임베딩
#[derive(Clone, Serialize, Deserialize)]
pub struct CachedPadEmbeddings {
    pub model_id: String,
    pub dimension: usize,
    pub pleasure: CachedAxisEmbeddings,
    pub arousal: CachedAxisEmbeddings,
    pub dominance: CachedAxisEmbeddings,
}

// ---------------------------------------------------------------------------
// 사전 계산된 앵커 임베딩
// ---------------------------------------------------------------------------

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
    /// 앵커 소스에서 로드하여 생성 (캐싱 지원)
    ///
    /// 1. 캐시된 임베딩이 있으면 바로 사용
    /// 2. 없으면 앵커 텍스트 로드 → 임베딩 계산 → 캐시 저장
    pub fn new(
        mut embedder: Box<dyn TextEmbedder + Send>,
        source: &dyn PadAnchorSource,
    ) -> Result<Self, EmbedError> {
        // 1. 캐시된 임베딩 시도
        if let Ok(Some(cached)) = source.load_cached_embeddings() {
            return Ok(Self {
                embedder,
                pleasure: AxisEmbeddings {
                    positive: cached.pleasure.positive_mean,
                    negative: cached.pleasure.negative_mean,
                },
                arousal: AxisEmbeddings {
                    positive: cached.arousal.positive_mean,
                    negative: cached.arousal.negative_mean,
                },
                dominance: AxisEmbeddings {
                    positive: cached.dominance.positive_mean,
                    negative: cached.dominance.negative_mean,
                },
            });
        }

        // 2. 앵커 텍스트 로드 → 임베딩 계산
        let anchors = source
            .load_anchors()
            .map_err(|e| EmbedError::InitError(e.to_string()))?;
        let pleasure = Self::embed_axis(&mut *embedder, &anchors.pleasure)?;
        let arousal = Self::embed_axis(&mut *embedder, &anchors.arousal)?;
        let dominance = Self::embed_axis(&mut *embedder, &anchors.dominance)?;

        // 3. 캐시 저장 (best-effort, 실패해도 무시)
        let dim = pleasure.positive.len();
        let cached = CachedPadEmbeddings {
            model_id: String::new(),
            dimension: dim,
            pleasure: CachedAxisEmbeddings {
                positive_mean: pleasure.positive.clone(),
                negative_mean: pleasure.negative.clone(),
            },
            arousal: CachedAxisEmbeddings {
                positive_mean: arousal.positive.clone(),
                negative_mean: arousal.negative.clone(),
            },
            dominance: CachedAxisEmbeddings {
                positive_mean: dominance.positive.clone(),
                negative_mean: dominance.negative.clone(),
            },
        };
        let _ = source.save_cached_embeddings(&cached);

        Ok(Self {
            embedder,
            pleasure,
            arousal,
            dominance,
        })
    }

    /// 앵커 텍스트 → 평균 임베딩 벡터 계산
    fn embed_axis(
        embedder: &mut dyn TextEmbedder,
        anchors: &PadAxisAnchorsOwned,
    ) -> Result<AxisEmbeddings, EmbedError> {
        let pos_refs: Vec<&str> = anchors.positive.iter().map(|s| s.as_str()).collect();
        let neg_refs: Vec<&str> = anchors.negative.iter().map(|s| s.as_str()).collect();
        let pos_vecs = embedder.embed(&pos_refs)?;
        let neg_vecs = embedder.embed(&neg_refs)?;

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
        if vectors.is_empty() {
            return Vec::new();
        }
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
        if na == 0.0 || nb == 0.0 {
            return 0.0;
        }
        dot / (na * nb)
    }

    /// 대사 임베딩과 축 앵커 유사도 차이 → -1.0~1.0
    ///
    /// 보정: |차이| < PAD_AXIS_DEAD_ZONE(0.02) → 0.0, 이후 ×PAD_AXIS_SCALE(3.0)
    fn axis_score(utterance_emb: &[f32], axis: &AxisEmbeddings) -> f32 {
        let sim_pos = Self::cosine_sim(utterance_emb, &axis.positive);
        let sim_neg = Self::cosine_sim(utterance_emb, &axis.negative);
        let raw = sim_pos - sim_neg;
        if raw.abs() < PAD_AXIS_DEAD_ZONE {
            return 0.0;
        }
        (raw * PAD_AXIS_SCALE).clamp(-1.0, 1.0)
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

    fn analyze_with_embedding(
        &mut self,
        utterance: &str,
    ) -> Result<(Pad, Option<UtteranceEmbedding>), EmbedError> {
        let mut embeddings = self.embedder.embed(&[utterance])?;

        if embeddings.is_empty() {
            return Ok((Pad::neutral(), None));
        }

        let embedding = embeddings.remove(0);
        let pad = self.to_pad(&embedding);
        Ok((pad, Some(UtteranceEmbedding::new(embedding))))
    }
}

// ---------------------------------------------------------------------------
// PadAnalyzer::analyze_with_embedding 단위 테스트 (Phase 7 Step 5)
// ---------------------------------------------------------------------------
//
// load-bearing invariant: 반환되는 임베딩이 to_pad에 사용된 것과 동일.
// 이 invariant가 깨지면 ListenerPerspectiveConverter가 PadAnalyzer 추출 PAD와
// 무관한 벡터로 분류를 수행해 silent 오류가 발생한다.

#[cfg(test)]
mod analyze_with_embedding_tests {
    use super::*;
    use crate::ports::{AnchorLoadError, PadAnchorSource, UtteranceAnalyzer};
    use std::sync::{Arc, Mutex};

    /// 호출 이력을 기록하는 mock embedder. outputs는 매 호출마다 그대로 반환.
    struct SpyEmbedder {
        outputs: Vec<Vec<f32>>,
        call_log: Arc<Mutex<Vec<Vec<String>>>>,
    }

    impl TextEmbedder for SpyEmbedder {
        fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
            self.call_log
                .lock()
                .unwrap()
                .push(texts.iter().map(|s| s.to_string()).collect());
            Ok(self.outputs.clone())
        }
    }

    /// 캐싱된 axis embedding을 즉시 반환하는 anchor source —
    /// PadAnalyzer::new가 init 단계에서 embedder를 호출하지 않게 한다.
    struct CachedAnchorSource {
        cached: CachedPadEmbeddings,
    }

    impl PadAnchorSource for CachedAnchorSource {
        fn load_anchors(&self) -> Result<PadAnchorSet, AnchorLoadError> {
            unreachable!("load_cached_embeddings가 Some 반환하므로 호출되지 않음")
        }
        fn load_cached_embeddings(
            &self,
        ) -> Result<Option<CachedPadEmbeddings>, AnchorLoadError> {
            Ok(Some(self.cached.clone()))
        }
        fn save_cached_embeddings(
            &self,
            _: &CachedPadEmbeddings,
        ) -> Result<(), AnchorLoadError> {
            Ok(())
        }
    }

    fn unit_axis_embeddings() -> CachedPadEmbeddings {
        // 3차원 축 단위 벡터 — to_pad는 cosine 차이 기반이므로 명확한 분리만 있으면 충분.
        CachedPadEmbeddings {
            model_id: "test".into(),
            dimension: 3,
            pleasure: CachedAxisEmbeddings {
                positive_mean: vec![1.0, 0.0, 0.0],
                negative_mean: vec![-1.0, 0.0, 0.0],
            },
            arousal: CachedAxisEmbeddings {
                positive_mean: vec![0.0, 1.0, 0.0],
                negative_mean: vec![0.0, -1.0, 0.0],
            },
            dominance: CachedAxisEmbeddings {
                positive_mean: vec![0.0, 0.0, 1.0],
                negative_mean: vec![0.0, 0.0, -1.0],
            },
        }
    }

    fn make_analyzer(
        spy_outputs: Vec<Vec<f32>>,
    ) -> (PadAnalyzer, Arc<Mutex<Vec<Vec<String>>>>) {
        let log = Arc::new(Mutex::new(Vec::new()));
        let embedder = SpyEmbedder {
            outputs: spy_outputs,
            call_log: log.clone(),
        };
        let source = CachedAnchorSource {
            cached: unit_axis_embeddings(),
        };
        let analyzer = PadAnalyzer::new(Box::new(embedder), &source).expect("init");
        (analyzer, log)
    }

    /// 반환 임베딩이 to_pad 입력과 동일하고, embedder는 발화 1회만 호출한다.
    #[test]
    fn returned_embedding_equals_input_for_to_pad() {
        let utterance_emb = vec![0.5, 0.3, -0.2];
        let (mut analyzer, log) = make_analyzer(vec![utterance_emb.clone()]);

        let (pad, returned) = analyzer.analyze_with_embedding("test utterance").unwrap();

        // embedder는 발화 텍스트로 정확히 1회 호출 (init은 cached로 우회)
        let calls = log.lock().unwrap();
        assert_eq!(calls.len(), 1, "embedder 1회 호출 (init 캐시 사용)");
        assert_eq!(calls[0], vec!["test utterance".to_string()]);

        // 반환 임베딩 == embedder 출력
        let returned = returned.expect("Some 임베딩 반환");
        assert_eq!(
            returned.as_slice(),
            utterance_emb.as_slice(),
            "반환 임베딩이 embedder 출력과 동일"
        );

        // PAD가 동일 임베딩으로 to_pad 호출한 것과 일치 (load-bearing invariant)
        // Deref<[f32]> 강제 변환으로 &returned을 &[f32]로 사용
        let recomputed = analyzer.to_pad(&returned);
        assert_eq!(
            pad, recomputed,
            "PAD = to_pad(반환된 임베딩) — Converter가 PadAnalyzer와 동일 벡터 공유 보장"
        );
    }

    /// 빈 임베딩 반환 시 (Pad::neutral, None) 페어를 반환한다.
    #[test]
    fn empty_embedding_returns_neutral_and_none() {
        let (mut analyzer, _) = make_analyzer(vec![]);

        let (pad, returned) = analyzer.analyze_with_embedding("test").unwrap();

        assert_eq!(pad, Pad::neutral(), "빈 임베딩 → neutral PAD");
        assert!(returned.is_none(), "빈 임베딩 → None (Converter 변환 skip 신호)");
    }

    /// trait의 `analyze`도 새 메서드와 일관된 결과를 낸다 (호환성 회귀 감시).
    #[test]
    fn analyze_matches_analyze_with_embedding_pad() {
        let utterance_emb = vec![0.7, -0.4, 0.2];
        let (mut analyzer, _) = make_analyzer(vec![utterance_emb.clone()]);

        let pad_only = analyzer.analyze("test").unwrap();

        // 새 인스턴스로 다시 만들어 analyze_with_embedding 호출
        let (mut analyzer2, _) = make_analyzer(vec![utterance_emb]);
        let (pad_with_emb, _) = analyzer2.analyze_with_embedding("test").unwrap();

        assert_eq!(pad_only, pad_with_emb, "두 메서드의 PAD가 동일");
    }
}
