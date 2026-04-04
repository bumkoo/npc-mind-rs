//! Appraisal trace 수집기
//!
//! 엔진의 tracing 이벤트를 수집하여 GUI에서 표시할 수 있는 문자열로 변환.
//! tracing::Layer를 구현하여 subscriber에 조합한다.

use std::sync::{Arc, Mutex};
use tracing::Subscriber;
use tracing::field::{Field, Visit};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// 엔진 trace 이벤트를 수집하는 Layer
#[derive(Clone)]
pub struct AppraisalCollector {
    entries: Arc<Mutex<Vec<String>>>,
}

impl AppraisalCollector {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 수집된 trace 항목을 가져오고 내부 버퍼 비움
    pub fn take_entries(&self) -> Vec<String> {
        // Mutex가 poisoned 되는 경우는 다른 스레드가 panic한 상황으로,
        // trace 수집이 중단되어도 서비스에 영향 없으므로 빈 벡터 반환.
        match self.entries.lock() {
            Ok(mut guard) => std::mem::take(&mut *guard),
            Err(_poisoned) => Vec::new(),
        }
    }
}

/// tracing 필드를 구조화된 (key, value) 쌍으로 수집
struct FieldVisitor {
    fields: Vec<(String, String)>,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields
            .push((field.name().to_string(), format!("{:?}", value)));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields
            .push((field.name().to_string(), format!("{:.3}", value)));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }
}

impl<S: Subscriber> Layer<S> for AppraisalCollector {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor { fields: Vec::new() };
        event.record(&mut visitor);

        // 감정 유형 추출
        let emotion = visitor
            .fields
            .iter()
            .find(|(k, _)| k == "emotion")
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "?".into());

        // 상황 맥락 추출
        let context = visitor
            .fields
            .iter()
            .find(|(k, _)| k == "context")
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "-".into());

        // 수치 데이터 추출 (emotion, context, message 제외)
        let parts: Vec<String> = visitor
            .fields
            .iter()
            .filter(|(k, _)| k != "emotion" && k != "context" && k != "message")
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let line = if parts.is_empty() {
            format!("→ {} [{}]", emotion, context)
        } else {
            format!("→ {}: {} [{}]", emotion, parts.join(", "), context)
        };

        // Mutex poisoned 시 trace 항목 유실 허용 (서비스 안정성 우선)
        if let Ok(mut guard) = self.entries.lock() {
            guard.push(line);
        }
    }
}
