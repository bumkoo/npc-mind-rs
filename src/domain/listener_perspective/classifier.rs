//! k-NN top-k 분류 공통 수학
//!
//! Sign(2-way), Magnitude(3-way) 분류기가 공유하는 순수 수학 함수.
//! 임베딩·프로토타입 구조에 의존하지 않음.
//!
//! 설계: `docs/emotion/sign-classifier-design.md` §3.2

/// 코사인 유사도
///
/// 두 벡터의 방향 유사성 (-1 ~ +1). 크기가 0인 벡터는 0.0 반환.
pub fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(
        a.len(),
        b.len(),
        "cosine_sim: 벡터 차원 불일치 {} vs {}",
        a.len(),
        b.len()
    );
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

/// 내림차순으로 정렬된 유사도 슬라이스에서 상위 `k` 개 평균을 계산
///
/// - 입력이 `k` 보다 짧으면 전체 평균 반환
/// - 입력이 비어있으면 0.0 반환
///
/// **주의**: 입력은 이미 **내림차순 정렬** 되어 있어야 한다.
/// 분류기에서는 그룹별로 필터링된 유사도 배열을 정렬해서 넘긴다.
pub fn top_k_mean_sorted(sorted_desc: &[f32], k: usize) -> f32 {
    if sorted_desc.is_empty() {
        return 0.0;
    }
    let n = sorted_desc.len().min(k);
    sorted_desc[..n].iter().sum::<f32>() / n as f32
}

/// 내림차순으로 정렬된 유사도 벡터를 반환 (전체 정렬)
pub fn sorted_similarities_desc<'a, T>(
    query: &[f32],
    prototype_embeddings: &'a [Vec<f32>],
    prototypes: &'a [T],
) -> Vec<(&'a T, f32)> {
    let mut pairs: Vec<(&'a T, f32)> = prototypes
        .iter()
        .zip(prototype_embeddings.iter())
        .map(|(proto, emb)| (proto, cosine_sim(query, emb)))
        .collect();
    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_sim_identical() {
        let v = vec![1.0, 2.0, 3.0];
        assert!((cosine_sim(&v, &v) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_sim_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!(cosine_sim(&a, &b).abs() < 1e-5);
    }

    #[test]
    fn cosine_sim_opposite() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        assert!((cosine_sim(&a, &b) + 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_sim_zero_vector() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(cosine_sim(&a, &b), 0.0);
    }

    #[test]
    fn top_k_mean_normal() {
        let sorted = vec![0.9, 0.8, 0.7, 0.5, 0.3];
        let mean3 = top_k_mean_sorted(&sorted, 3);
        assert!((mean3 - 0.8).abs() < 1e-5);
    }

    #[test]
    fn top_k_mean_k_exceeds_len() {
        let sorted = vec![0.9, 0.8];
        let mean = top_k_mean_sorted(&sorted, 3);
        assert!((mean - 0.85).abs() < 1e-5);
    }

    #[test]
    fn top_k_mean_empty() {
        let empty: Vec<f32> = vec![];
        assert_eq!(top_k_mean_sorted(&empty, 3), 0.0);
    }

    #[test]
    fn sorted_similarities_desc_sorts_correctly() {
        let query = vec![1.0, 0.0];
        let protos = vec!["a", "b", "c"];
        let embeds = vec![
            vec![0.5, 0.5],   // cos ≈ 0.707
            vec![1.0, 0.0],   // cos = 1.0
            vec![0.0, 1.0],   // cos = 0.0
        ];
        let sorted = sorted_similarities_desc(&query, &embeds, &protos);
        assert_eq!(*sorted[0].0, "b");
        assert!((sorted[0].1 - 1.0).abs() < 1e-5);
        assert_eq!(*sorted[1].0, "a");
        assert_eq!(*sorted[2].0, "c");
    }
}
