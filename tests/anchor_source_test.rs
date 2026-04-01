//! PAD 앵커 소스 어댑터 테스트
//!
//! - TOML 파싱 정확성
//! - JSON 파싱 정확성
//! - 캐시 라운드트립
//! - 빌트인 레지스트리

use npc_mind::adapter::toml_anchor_source::TomlAnchorSource;
use npc_mind::adapter::json_anchor_source::JsonAnchorSource;
use npc_mind::domain::pad_anchors::builtin_anchor_toml;
use npc_mind::PadAnchorSource;

#[test]
fn toml_빌트인_앵커_파싱() {
    let toml = builtin_anchor_toml("ko").expect("ko 빌트인 앵커 없음");
    let source = TomlAnchorSource::from_content(toml);
    let anchors = source.load_anchors().expect("TOML 파싱 실패");

    assert_eq!(anchors.pleasure.positive.len(), 10);
    assert_eq!(anchors.pleasure.negative.len(), 10);
    assert_eq!(anchors.arousal.positive.len(), 10);
    assert_eq!(anchors.arousal.negative.len(), 10);
    assert_eq!(anchors.dominance.positive.len(), 10);
    assert_eq!(anchors.dominance.negative.len(), 6, "D- 역방향 제외로 6개");
}

#[test]
fn toml_앵커_텍스트_내용_확인() {
    let toml = builtin_anchor_toml("ko").unwrap();
    let source = TomlAnchorSource::from_content(toml);
    let anchors = source.load_anchors().unwrap();

    assert!(anchors.pleasure.positive[0].contains("기쁘"));
    assert!(anchors.dominance.positive[0].contains("주도"));
}

#[test]
fn toml_캐시_없으면_none() {
    let source = TomlAnchorSource::from_content(builtin_anchor_toml("ko").unwrap());
    let cached = source.load_cached_embeddings().expect("캐시 로드 실패");
    assert!(cached.is_none(), "캐시 경로 미설정 시 None");
}

#[test]
fn toml_캐시_라운드트립() {
    use npc_mind::domain::pad::{CachedPadEmbeddings, CachedAxisEmbeddings};

    let cache_path = std::env::temp_dir().join("npc_mind_test_cache.json");
    // 정리
    let _ = std::fs::remove_file(&cache_path);

    let source = TomlAnchorSource::from_content(builtin_anchor_toml("ko").unwrap())
        .with_cache_path(&cache_path);

    let dummy = CachedPadEmbeddings {
        model_id: "test-model".into(),
        dimension: 4,
        pleasure: CachedAxisEmbeddings {
            positive_mean: vec![0.1, 0.2, 0.3, 0.4],
            negative_mean: vec![-0.1, -0.2, -0.3, -0.4],
        },
        arousal: CachedAxisEmbeddings {
            positive_mean: vec![0.5, 0.6, 0.7, 0.8],
            negative_mean: vec![-0.5, -0.6, -0.7, -0.8],
        },
        dominance: CachedAxisEmbeddings {
            positive_mean: vec![0.9, 1.0, 1.1, 1.2],
            negative_mean: vec![-0.9, -1.0, -1.1, -1.2],
        },
    };

    source.save_cached_embeddings(&dummy).expect("캐시 저장 실패");
    let loaded = source.load_cached_embeddings().expect("캐시 로드 실패")
        .expect("캐시 있어야 함");

    assert_eq!(loaded.model_id, "test-model");
    assert_eq!(loaded.dimension, 4);
    assert_eq!(loaded.pleasure.positive_mean, vec![0.1, 0.2, 0.3, 0.4]);
    assert_eq!(loaded.arousal.negative_mean, vec![-0.5, -0.6, -0.7, -0.8]);

    // 정리
    let _ = std::fs::remove_file(&cache_path);
}

#[test]
fn json_앵커_파싱() {
    let json = r#"{
        "meta": { "language": "ko", "version": "1" },
        "pleasure": {
            "positive": ["기쁘다", "행복하다"],
            "negative": ["슬프다", "괴롭다"]
        },
        "arousal": {
            "positive": ["흥분된다"],
            "negative": ["차분하다"]
        },
        "dominance": {
            "positive": ["물러서라"],
            "negative": ["살려주세요"]
        }
    }"#;

    let source = JsonAnchorSource::from_content(json);
    let anchors = source.load_anchors().expect("JSON 파싱 실패");

    assert_eq!(anchors.pleasure.positive.len(), 2);
    assert_eq!(anchors.pleasure.negative.len(), 2);
    assert_eq!(anchors.arousal.positive.len(), 1);
    assert_eq!(anchors.dominance.negative[0], "살려주세요");
}

#[test]
fn 빌트인_레지스트리_ko_존재() {
    assert!(builtin_anchor_toml("ko").is_some());
}

#[test]
fn 빌트인_레지스트리_미지원_언어_none() {
    assert!(builtin_anchor_toml("fr").is_none());
}
