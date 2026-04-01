//! bge-m3-onnx-rust 기반 TextEmbedder 어댑터
//!
//! ort(ONNX Runtime)로 bge-m3 INT8 양자화 모델을 직접 실행하는 얇은 래퍼.
//! fastembed 대비 의존성 크레이트 수가 ~1/7 수준.
//! 도메인 로직(앵커 비교, PAD 계산)은 여기에 없다.

use std::path::Path;

use bge_m3_onnx_rust::BgeM3Embedder;

use crate::ports::{EmbedError, TextEmbedder};

/// ort(ONNX Runtime) 기반 임베딩 어댑터
pub struct OrtEmbedder {
    embedder: BgeM3Embedder,
}

impl OrtEmbedder {
    /// ONNX 모델과 토크나이저 파일 경로로 생성
    ///
    /// model_path: model_quantized.onnx (INT8, ~570MB)
    /// tokenizer_path: tokenizer.json
    pub fn new(
        model_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
    ) -> Result<Self, EmbedError> {
        bge_m3_onnx_rust::init_ort();
        let embedder = BgeM3Embedder::new(model_path, tokenizer_path)
            .map_err(|e| EmbedError::InitError(e.to_string()))?;
        Ok(Self { embedder })
    }
}

impl TextEmbedder for OrtEmbedder {
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            let dense = self
                .embedder
                .encode_dense(text)
                .map_err(|e| EmbedError::InferenceError(e.to_string()))?;
            results.push(dense);
        }
        Ok(results)
    }
}
