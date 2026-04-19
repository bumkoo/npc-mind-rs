//! `build_converter_from_data_dir` 단위 테스트 (Phase 7 Step 5)
//!
//! main.rs의 `init_listener_perspective_converter`에서 분리된 순수 헬퍼를
//! tempfile 기반 minimal data dir로 검증한다.
//!
//! - valid 6개 파일이 있는 디렉토리 → `Some(EmbeddedConverter)`
//! - 파일 부재 디렉토리 → `None` (graceful warn, panic 없음)

use npc_mind::ports::{EmbedError, TextEmbedder};
use std::fs;
use std::path::Path;

use super::build_converter_from_data_dir;

// ---------------------------------------------------------------------------
// Mock embedder
// ---------------------------------------------------------------------------

/// 호출 인덱스 기반으로 살짝 다른 4차원 vector를 반환하는 mock.
///
/// 분류기 동점/zero-norm을 피하면서, 모델 의존성 없이 EmbeddedConverter::from_paths의
/// 내부 임베딩 호출을 통과시키기 위함.
struct ConstantEmbedder {
    counter: usize,
}

impl ConstantEmbedder {
    fn new() -> Self {
        Self { counter: 0 }
    }
}

impl TextEmbedder for ConstantEmbedder {
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        let result = texts
            .iter()
            .map(|_| {
                // 인덱스 기반 변동으로 분류기가 의미 있는 cosine 차이를 갖게 함
                let idx = self.counter;
                self.counter += 1;
                let mut v = vec![0.5; 4];
                v[idx % 4] = 1.0;
                v
            })
            .collect();
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// 데이터 파일 작성 헬퍼
// ---------------------------------------------------------------------------

const PREFILTER_TOML: &str = r#"
[meta]
version = "test"

[[category]]
name = "test_invert"
sign = "invert"
magnitude = "strong"
p_s_default = 0.5
description = "test"
patterns = ["^!INVERT!"]
"#;

const KEEP_TOML: &str = r#"
[meta]
version = "test"
group = "sign_keep"
[prototypes]
items = [
    { text = "K1", subtype = "x" },
    { text = "K2", subtype = "y" },
]
"#;

const INVERT_TOML: &str = r#"
[meta]
version = "test"
group = "sign_invert"
[prototypes]
items = [
    { text = "I1", subtype = "x" },
    { text = "I2", subtype = "y" },
]
"#;

const WEAK_TOML: &str = r#"
[meta]
version = "test"
group = "magnitude_weak"
[prototypes]
items = [
    { text = "W1", subtype = "x" },
    { text = "W2", subtype = "y" },
]
"#;

const NORMAL_TOML: &str = r#"
[meta]
version = "test"
group = "magnitude_normal"
[prototypes]
items = [
    { text = "N1", subtype = "x" },
    { text = "N2", subtype = "y" },
]
"#;

const STRONG_TOML: &str = r#"
[meta]
version = "test"
group = "magnitude_strong"
[prototypes]
items = [
    { text = "S1", subtype = "x" },
    { text = "S2", subtype = "y" },
]
"#;

fn write_minimal_data(root: &Path) {
    let prefilter_dir = root.join("prefilter");
    let proto_dir = root.join("prototypes");
    fs::create_dir_all(&prefilter_dir).unwrap();
    fs::create_dir_all(&proto_dir).unwrap();
    fs::write(prefilter_dir.join("patterns.toml"), PREFILTER_TOML).unwrap();
    fs::write(proto_dir.join("sign_keep.toml"), KEEP_TOML).unwrap();
    fs::write(proto_dir.join("sign_invert.toml"), INVERT_TOML).unwrap();
    fs::write(proto_dir.join("magnitude_weak.toml"), WEAK_TOML).unwrap();
    fs::write(proto_dir.join("magnitude_normal.toml"), NORMAL_TOML).unwrap();
    fs::write(proto_dir.join("magnitude_strong.toml"), STRONG_TOML).unwrap();
}

// ---------------------------------------------------------------------------
// 테스트
// ---------------------------------------------------------------------------

#[test]
fn build_returns_some_for_valid_data_dir() {
    let temp = tempfile::tempdir().unwrap();
    write_minimal_data(temp.path());

    let mut embedder = ConstantEmbedder::new();
    let converter = build_converter_from_data_dir(&mut embedder, temp.path());

    assert!(
        converter.is_some(),
        "유효한 data dir이면 EmbeddedConverter 반환"
    );
}

#[test]
fn build_returns_none_when_files_missing() {
    let temp = tempfile::tempdir().unwrap();
    // 디렉토리는 만들되 파일은 작성하지 않음

    let mut embedder = ConstantEmbedder::new();
    let converter = build_converter_from_data_dir(&mut embedder, temp.path());

    assert!(
        converter.is_none(),
        "패턴/프로토타입 파일 부재 시 None — graceful degradation"
    );
}

#[test]
fn build_returns_none_when_partially_missing() {
    let temp = tempfile::tempdir().unwrap();
    let prefilter_dir = temp.path().join("prefilter");
    fs::create_dir_all(&prefilter_dir).unwrap();
    fs::write(prefilter_dir.join("patterns.toml"), PREFILTER_TOML).unwrap();
    // prototypes 폴더 미작성

    let mut embedder = ConstantEmbedder::new();
    let converter = build_converter_from_data_dir(&mut embedder, temp.path());

    assert!(
        converter.is_none(),
        "프로토타입 일부만 누락되어도 None"
    );
}
