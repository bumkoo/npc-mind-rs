// wuxia-core/src/shared/embedding.rs
//
// Embedding Port + 벡터 유틸리티 — Shared Kernel.
//
// 텍스트→벡터 변환 포트와 벡터 연산 유틸리티는 범용 인프라.
// memory(기억 검색), relationship(극단 앵커), LLM 어댑터(감정 판정) 등
// 여러 도메인과 어댑터에서 공통으로 사용된다.
//
// 의존성 방향:
//   wuxia-core (EmbeddingPort trait 정의) ← wuxia-memory (구현)
//   ↑ 이 방향은 절대 역전되지 않는다.
//
// 비유: 기억을 숫자로 바꾸는 서기관(書記官)
//   InMemory 시대에는 "혈교"라는 글자가 있는 기억만 찾았지만,
//   벡터 시대에는 "사파"로 검색해도 "혈교" 기억을 찾는다.
//   서기관이 글을 읽고 의미를 숫자로 기록해두기 때문이다.
//
// 구현체:
//   - MockEmbedding (wuxia-memory): 테스트용. 해시 기반 결정론적 벡터.
//   - FastEmbedAdapter (wuxia-memory): ONNX Runtime + multilingual-e5-small.
//   - LlamaCppEmbedding (wuxia-memory): llama.cpp + GGUF 모델 (bge-m3 등).
//
// 동기(sync) 인터페이스를 선택한 이유:
//   LlmPort와 동일 — wuxia-core는 async runtime을 모른다.
//   어댑터 내부에서 blocking 처리.

use super::port_error::PortError;

/// 텍스트 → 벡터 변환 포트 (헥사고날 아키텍처).
///
/// `Send + Sync` 바운드:
///   Bevy의 `Res<dyn EmbeddingPort>` 또는 `Arc<dyn EmbeddingPort>`로
///   여러 시스템에서 공유할 수 있도록.
///
/// # 구현체
/// - `MockEmbedding` (wuxia-memory): 테스트용. 해시 기반 결정론적 벡터.
/// - `FastEmbedAdapter` (wuxia-memory): ONNX Runtime + e5-small. feature "fastembed".
/// - `LlamaCppEmbedding` (wuxia-memory): llama.cpp + GGUF. feature "live-llm".
///
/// # Example (Mock 사용)
/// ```
/// use wuxia_core::shared::embedding::EmbeddingPort;
/// use wuxia_core::shared::PortError;
///
/// struct MockEmbed;
///
/// impl EmbeddingPort for MockEmbed {
///     fn embed(&self, text: &str) -> Result<Vec<f32>, PortError> {
///         Ok(vec![0.1, 0.2, 0.3])
///     }
///     fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, PortError> {
///         Ok(texts.iter().map(|_| vec![0.1, 0.2, 0.3]).collect())
///     }
///     fn dimension(&self) -> usize { 3 }
///     fn model_name(&self) -> &str { "mock" }
/// }
///
/// let embedder = MockEmbed;
/// let vec = embedder.embed("혈교는 사파다").unwrap();
/// assert_eq!(vec.len(), embedder.dimension());
/// ```
pub trait EmbeddingPort: Send + Sync {
    /// 텍스트 하나를 벡터로 변환한다.
    ///
    /// 반환되는 벡터의 길이는 항상 `dimension()`과 같다.
    /// 벡터는 L2 정규화(normalize)된 상태로 반환되어야 한다.
    /// (코사인 유사도 = 내적으로 계산 가능)
    ///
    /// # Arguments
    /// * `text` - 임베딩할 텍스트 (한국어, 영어, 중국어 모두 가능).
    ///
    /// # Returns
    /// * `Ok(Vec<f32>)` - 정규화된 임베딩 벡터.
    /// * `Err(PortError)` - 토큰화 실패, 모델 오류 등.
    fn embed(&self, text: &str) -> Result<Vec<f32>, PortError>;

    /// 문서(저장용) 텍스트를 벡터로 변환한다.
    ///
    /// 비대칭 임베딩 모델(EmbeddingGemma 등)에서는
    /// 검색 쿼리와 문서에 다른 접두어(prefix)를 사용한다:
    ///   - 검색: "task: search result | query: " (embed)
    ///   - 문서: "title: none | text: "          (embed_document)
    ///
    /// 기본 구현은 `embed()`로 위임한다 (대칭 모델 호환).
    fn embed_document(&self, text: &str) -> Result<Vec<f32>, PortError> {
        self.embed(text)
    }

    /// 여러 텍스트를 한 번에 벡터로 변환한다 (배치).
    ///
    /// 단건 `embed()`를 반복 호출하는 것보다 효율적.
    /// 기본 구현은 단건 호출을 반복하지만,
    /// FastEmbed 등은 내부적으로 배치 최적화를 수행한다.
    ///
    /// # Arguments
    /// * `texts` - 임베딩할 텍스트 슬라이스.
    ///
    /// # Returns
    /// * `Ok(Vec<Vec<f32>>)` - 각 텍스트의 정규화된 임베딩 벡터. 순서 보장.
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, PortError> {
        // 기본 구현: 단건 호출 반복 (구현체에서 오버라이드 가능)
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// 벡터 차원 수를 반환한다.
    ///
    /// 모델에 따라 다르다:
    ///   - multilingual-e5-small: 384
    ///   - bge-m3: 1024
    ///   - multilingual-e5-large: 1024
    fn dimension(&self) -> usize;

    /// 모델 이름을 반환한다 (벤치마크 보고서, 로깅용).
    fn model_name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// 벡터 유틸리티 함수
// ---------------------------------------------------------------------------

/// 두 벡터의 코사인 유사도를 계산한다.
///
/// 벡터가 이미 L2 정규화되어 있으면 내적(dot product)과 동일하다.
/// EmbeddingPort는 정규화된 벡터를 반환하므로, 이 함수는 사실상 내적이다.
/// 하지만 안전을 위해 정규화되지 않은 벡터도 처리한다.
///
/// # Returns
/// * `-1.0 ~ 1.0` — 1.0에 가까울수록 의미적으로 유사.
///
/// # Panics
/// * 두 벡터의 길이가 다르면 panic.
///
/// # Example
/// ```
/// use wuxia_core::shared::embedding::cosine_similarity;
///
/// let a = vec![1.0, 0.0, 0.0];
/// let b = vec![1.0, 0.0, 0.0];
/// assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
///
/// let c = vec![0.0, 1.0, 0.0];
/// assert!((cosine_similarity(&a, &c) - 0.0).abs() < 1e-6);
/// ```
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "벡터 차원이 다릅니다: {} vs {}", a.len(), b.len());

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }

    dot / (mag_a * mag_b)
}

/// 벡터를 L2 정규화한다 (크기를 1로 만든다).
///
/// 정규화된 벡터끼리의 코사인 유사도 = 내적.
/// EmbeddingPort 구현체에서 내부적으로 사용.
///
/// # Example
/// ```
/// use wuxia_core::shared::embedding::l2_normalize;
///
/// let v = vec![3.0, 4.0];
/// let n = l2_normalize(&v);
/// let magnitude: f32 = n.iter().map(|x| x * x).sum::<f32>().sqrt();
/// assert!((magnitude - 1.0).abs() < 1e-6);
/// ```
pub fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let magnitude: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude == 0.0 {
        return v.to_vec();
    }
    v.iter().map(|x| x / magnitude).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 최소 Mock — trait 구현 가능 여부 검증
    struct TestMockEmbed {
        dim: usize,
    }

    impl EmbeddingPort for TestMockEmbed {
        fn embed(&self, _text: &str) -> Result<Vec<f32>, PortError> {
            Ok(vec![0.0; self.dim])
        }
        fn dimension(&self) -> usize {
            self.dim
        }
        fn model_name(&self) -> &str {
            "test-mock"
        }
    }

    #[test]
    fn trait_object_works() {
        let embedder: Box<dyn EmbeddingPort> = Box::new(TestMockEmbed { dim: 384 });
        let vec = embedder.embed("테스트").unwrap();
        assert_eq!(vec.len(), 384);
        assert_eq!(embedder.dimension(), 384);
        assert_eq!(embedder.model_name(), "test-mock");
    }

    #[test]
    fn default_embed_batch_works() {
        let embedder = TestMockEmbed { dim: 3 };
        let result = embedder.embed_batch(&["가", "나", "다"]).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].len(), 3);
    }

    #[test]
    fn cosine_same_vector_is_one() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_is_zero() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn cosine_opposite_is_negative() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn cosine_zero_vector_returns_zero() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 2.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    #[should_panic(expected = "벡터 차원이 다릅니다")]
    fn cosine_different_dimensions_panics() {
        cosine_similarity(&[1.0, 2.0], &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn l2_normalize_unit_length() {
        let v = vec![3.0, 4.0];
        let n = l2_normalize(&v);
        let mag: f32 = n.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((mag - 1.0).abs() < 1e-6);
        assert!((n[0] - 0.6).abs() < 1e-6);
        assert!((n[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn l2_normalize_zero_vector() {
        let v = vec![0.0, 0.0, 0.0];
        let n = l2_normalize(&v);
        assert_eq!(n, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn l2_normalize_already_normalized() {
        let v = vec![1.0, 0.0, 0.0];
        let n = l2_normalize(&v);
        assert!((n[0] - 1.0).abs() < 1e-6);
    }
}
