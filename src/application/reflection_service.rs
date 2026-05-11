//! ReflectionService — `DialogueOrchestrator.end_session`이 호출.
//!
//! Phase 1 Mind Architecture (`relationships.md` v0.7 §6) — Outer Loop 진입 게이트.
//! LLM 분석(`ReflectionPort`) + 엔진 정량 계산(`compute_significance`)을 결합해
//! `ReflectionResult` 산출 → `Command::EndDialogue { reflection }` payload.
//!
//! ## OCP 준수 (Stage 0 Findings F2 #2 / spec §4.4 결정 3)
//!
//! `ReflectionService<P: ReflectionPort>` 제네릭. 본 모듈은 `ReflectionPort` trait에만
//! 의존 — 구체 어댑터(`ConversationBackedReflectionPort` 등) import 0건.
//! Phase 2.5+에서 다른 LLM 어댑터 추가 시 본 모듈 변경 0.
//!
//! ## Phase 1 prompt builder 정책 (Stage 0 Findings F6 보정)
//!
//! `relationships.md` v0.7 §6.2는 LLM 입력으로 `compass + taboo + life_question + 현재 PAD`
//! 4종을 요구. 그러나 Phase 1 A-min은 `Npc.inner_compass: Option<String>` (compass 한 줄)만
//! 도메인에 추가됨. 따라서 `DefaultReflectionPromptBuilder`는 **compass만** prompt에 포함하고
//! taboo/life_question은 *제외*. Phase 3c에서 `InnerCompass` struct로 승격되면 자연 활성화.

#[cfg(feature = "chat")]
use std::sync::Arc;

#[cfg(feature = "chat")]
use serde::Deserialize;

#[cfg(feature = "chat")]
use crate::domain::personality::Npc;
#[cfg(feature = "chat")]
use crate::domain::reflection::{
    DeclarativeEventPlaceholder, PartnershipEventPlaceholder, ReflectionResult, TurnSnapshot,
    compute_significance,
};
#[cfg(feature = "chat")]
use crate::ports::reflection::{ReflectionError, ReflectionPort, ReflectionPrompt};

// ===========================================================================
// PromptBuilder — 캐릭터별/장르별 customize 가능
// ===========================================================================

/// LLM에 보낼 prompt를 구성하는 추상화. Phase 2.5+에서 장르별/캐릭터별 prompt
/// 풍부화 시 본 trait를 다른 builder로 교체 가능. ReflectionService는
/// `Arc<dyn ReflectionPromptBuilder>`로만 보유.
#[cfg(feature = "chat")]
pub trait ReflectionPromptBuilder: Send + Sync {
    fn build(
        &self,
        npc: &Npc,
        partner: &Npc,
        turns: &[TurnSnapshot],
        session_hint: &str,
    ) -> Result<ReflectionPrompt, ReflectionError>;
}

/// 기본 wuxia 분석 prompt — Phase 1 default.
///
/// LLM 입력에 포함되는 NPC 컨텍스트는 **compass만** (Stage 0 Findings F6).
/// taboo / life_question은 Phase 3c `InnerCompass` 승격 시 활성화.
#[cfg(feature = "chat")]
pub struct DefaultReflectionPromptBuilder;

#[cfg(feature = "chat")]
impl ReflectionPromptBuilder for DefaultReflectionPromptBuilder {
    fn build(
        &self,
        npc: &Npc,
        partner: &Npc,
        turns: &[TurnSnapshot],
        session_hint: &str,
    ) -> Result<ReflectionPrompt, ReflectionError> {
        let npc_compass = npc.compass_short_label().unwrap_or("(가치 미설정)");
        let partner_name = partner.name();
        let npc_name = npc.name();

        let json_schema = r#"{
  "is_chitchat": bool,
  "summary": "1~2문장 한국어 요약",
  "declarative_events": [],
  "partnership_event": null,
  "reasoning": "이 판정의 근거 1~2문장"
}"#;
        let system_prompt = format!(
            "당신은 무협 서사 작가의 편집자입니다. \
             NPC '{npc_name}' (가치: {npc_compass})는 '{partner_name}'와의 대화를 막 마쳤습니다. \
             이 대화가 *서사적으로 의미 있는 사건*인지 아니면 *지나가는 잡담*인지 평가하세요.\n\
             \n반드시 다음 JSON 형식으로만 답하세요 (다른 텍스트 절대 금지):\n{json_schema}"
        );

        let transcript = format_transcript(turns);
        let user_message = format!(
            "[대화 transcript]\n{transcript}\n\n\
             [지시]\n위 대화를 평가하세요. JSON으로만 답하세요."
        );

        Ok(ReflectionPrompt {
            system_prompt,
            user_message,
            session_hint: Some(session_hint.to_string()),
        })
    }
}

#[cfg(feature = "chat")]
fn format_transcript(turns: &[TurnSnapshot]) -> String {
    if turns.is_empty() {
        return "(대화 없음)".to_string();
    }
    turns
        .iter()
        .map(|t| {
            format!(
                "[turn {}]\n  user: {}\n  npc:  {}",
                t.turn_index, t.user_utterance, t.npc_response
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

// ===========================================================================
// ReflectionRunner — DialogueOrchestrator가 보유하는 trait object
// ===========================================================================

/// dyn-compatible 추상화. `DialogueOrchestrator`가 generic `<R, C, P>` 추가 없이
/// `Option<Arc<dyn ReflectionRunner>>`로 보유 가능 (Stage 0 Findings F2 #2 보정).
///
/// `ReflectionService<P: ReflectionPort>`가 본 trait를 자동 구현 — 호출자는
/// `ReflectionService::new(...)` 생성 후 `Arc::new(svc) as Arc<dyn ReflectionRunner>`로 주입.
#[cfg(feature = "chat")]
#[async_trait::async_trait]
pub trait ReflectionRunner: Send + Sync {
    async fn reflect(
        &self,
        sid: &str,
        turns: &[TurnSnapshot],
        npc: &Npc,
        partner: &Npc,
    ) -> ReflectionResult;
}

#[cfg(feature = "chat")]
#[async_trait::async_trait]
impl<P: ReflectionPort + 'static> ReflectionRunner for ReflectionService<P> {
    async fn reflect(
        &self,
        sid: &str,
        turns: &[TurnSnapshot],
        npc: &Npc,
        partner: &Npc,
    ) -> ReflectionResult {
        ReflectionService::reflect(self, sid, turns, npc, partner).await
    }
}

// ===========================================================================
// ReflectionService — 본체
// ===========================================================================

/// `DialogueOrchestrator.end_session`에서 호출되는 메인 서비스.
///
/// 흐름:
/// 1. `compute_significance(turns)` — 결정론, <1ms (engine 부분)
/// 2. `prompt_builder.build(...)` — system + user prompt 구성
/// 3. `port.analyze(prompt).await` — LLM 호출 (Phase 1: ~2~5s)
/// 4. JSON 파싱 → `ReflectionResult` 합성
/// 5. 실패 (LLM 에러 / JSON 깨짐 / 타임아웃) 시 `fallback_result`로 보수적 결과
///    (`is_chitchat = false` → outer loop 진입 보장 → 게임 진행 막힘 0)
#[cfg(feature = "chat")]
pub struct ReflectionService<P: ReflectionPort> {
    port: Arc<P>,
    prompt_builder: Arc<dyn ReflectionPromptBuilder>,
}

#[cfg(feature = "chat")]
impl<P: ReflectionPort + 'static> ReflectionService<P> {
    pub fn new(port: Arc<P>, prompt_builder: Arc<dyn ReflectionPromptBuilder>) -> Self {
        Self {
            port,
            prompt_builder,
        }
    }

    /// 누적 turn snapshot + NPC 컨텍스트 → `ReflectionResult`.
    pub async fn reflect(
        &self,
        sid: &str,
        turns: &[TurnSnapshot],
        npc: &Npc,
        partner: &Npc,
    ) -> ReflectionResult {
        // (1) Engine 정량 — 결정론.
        let significance = compute_significance(turns);

        // (2) Prompt 구성 — 실패 시 fallback.
        let prompt = match self.prompt_builder.build(npc, partner, turns, sid) {
            Ok(p) => p,
            Err(e) => {
                return Self::fallback_result(
                    turns,
                    significance,
                    format!("prompt build error: {e}"),
                );
            }
        };

        // (3) LLM 호출 — port 추상화에만 의존.
        let raw_text = match self.port.analyze(prompt).await {
            Ok(t) => t,
            Err(e) => {
                let reason = match &e {
                    ReflectionError::Timeout(_) => format!("Timeout: {e}"),
                    other => format!("LLM error: {other}"),
                };
                return Self::fallback_result(turns, significance, reason);
            }
        };

        // (4) JSON 파싱 — LLM이 markdown fence (```json...```) 또는
        // prose prefix를 추가하는 경우가 잦으므로 robust 파싱: fence/prefix 제거 후 시도.
        // spec §11.1 위험에 대한 mitigation.
        let cleaned = strip_json_envelope(&raw_text);
        let parsed: ReflectionLlmOutput = match serde_json::from_str(cleaned) {
            Ok(p) => p,
            Err(e) => {
                let preview: String = raw_text.chars().take(120).collect();
                return Self::fallback_result(
                    turns,
                    significance,
                    format!("parse error: {e} | raw[..120]={preview}"),
                );
            }
        };

        // (5) 합성. Phase 1 결정 9 — declarative_events / partnership_event는
        // *항상 빈/None*. LLM 출력은 무시 (Phase 2에서 활성화).
        ReflectionResult {
            is_chitchat: parsed.is_chitchat,
            summary: parsed.summary,
            significance_score: significance,
            declarative_events: vec![],
            partnership_event: None,
            turn_count: turns.len(),
            llm_reasoning: parsed.reasoning,
        }
    }

    /// LLM 실패 / JSON 깨짐 / 타임아웃 시 보수적 fallback.
    /// `is_chitchat = false` → outer loop 진입 보장 → 게임 진행 막힘 0.
    /// `llm_reasoning`에 "FALLBACK: <reason>" 박제 (calibration audit 가능).
    fn fallback_result(turns: &[TurnSnapshot], significance: f32, reason: String) -> ReflectionResult {
        ReflectionResult {
            is_chitchat: false,
            summary: "(reflection failed — fallback)".into(),
            significance_score: significance,
            declarative_events: vec![],
            partnership_event: None,
            turn_count: turns.len(),
            llm_reasoning: Some(format!("FALLBACK: {reason}")),
        }
    }
}

/// LLM 응답에서 raw JSON 부분만 추출 (markdown fence 또는 prose prefix 제거).
/// 흔한 LLM 행동 패턴 대응 — spec §11.1 robust 파싱.
///
/// 처리:
/// - ` ```json ... ``` ` 또는 ` ``` ... ``` ` fence 제거
/// - 첫 `{` ~ 마지막 `}` 추출 (prose prefix/suffix 무시)
/// - 둘 다 실패 시 원본 trim 반환 (serde_json이 정확한 에러 반환 가능)
#[cfg(feature = "chat")]
fn strip_json_envelope(raw: &str) -> &str {
    let trimmed = raw.trim();

    // 1. fenced code block 처리: ```json\n{...}\n``` 또는 ```\n{...}\n```
    if let Some(rest) = trimmed.strip_prefix("```json").or_else(|| trimmed.strip_prefix("```")) {
        if let Some(end) = rest.rfind("```") {
            let inner = rest[..end].trim();
            // 안에서 다시 객체 추출 시도
            if let (Some(start), Some(close)) = (inner.find('{'), inner.rfind('}')) {
                if start <= close {
                    return &inner[start..=close];
                }
            }
            return inner;
        }
    }

    // 2. fence 없는 경우: 첫 { ~ 마지막 } 추출
    if let (Some(start), Some(close)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if start <= close {
            return &trimmed[start..=close];
        }
    }

    // 3. fallback: trim만
    trimmed
}

/// LLM 응답 JSON 스키마.
///
/// Phase 1 결정 9 — declarative_events / partnership_event는 *항상* 빈/None이어야
/// 한다 (Phase 2 Channel 1 활성 시 schema 정의). 따라서 LLM이 무엇을 반환하든
/// `serde(default)`로 *무시* — placeholder 필드를 `serde_json::Value`로 받아
/// 후처리에서 버린다 (LLM이 string array, 객체 등 임의 형태 반환해도 parse 실패 안 함).
/// Phase 2에서 정식 schema로 교체.
#[cfg(feature = "chat")]
#[derive(Debug, Deserialize)]
struct ReflectionLlmOutput {
    is_chitchat: bool,
    summary: String,
    /// Phase 1: 무시. LLM이 어떤 형태로 보내든 parse 실패 막음.
    #[serde(default)]
    #[allow(dead_code)]
    declarative_events: Option<serde_json::Value>,
    /// Phase 1: 무시.
    #[serde(default)]
    #[allow(dead_code)]
    partnership_event: Option<serde_json::Value>,
    #[serde(default)]
    reasoning: Option<String>,
}

// ===========================================================================
// 단위 테스트 — Mock ReflectionPort + 4 분기
// ===========================================================================

#[cfg(all(test, feature = "chat"))]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::Duration;

    use async_trait::async_trait;

    use crate::domain::pad::Pad;
    use crate::domain::personality::{HexacoProfile, Npc};

    /// 카운트 가능한 Mock — 호출 시 canned 응답 또는 에러 반환.
    struct MockReflectionPort {
        canned: Mutex<Result<String, ReflectionError>>,
        call_count: std::sync::atomic::AtomicU32,
    }

    impl MockReflectionPort {
        fn ok(text: impl Into<String>) -> Self {
            Self {
                canned: Mutex::new(Ok(text.into())),
                call_count: std::sync::atomic::AtomicU32::new(0),
            }
        }
        fn err(e: ReflectionError) -> Self {
            Self {
                canned: Mutex::new(Err(e)),
                call_count: std::sync::atomic::AtomicU32::new(0),
            }
        }
    }

    #[async_trait]
    impl ReflectionPort for MockReflectionPort {
        async fn analyze(&self, _: ReflectionPrompt) -> Result<String, ReflectionError> {
            self.call_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.canned.lock().unwrap().clone()
        }
    }

    fn npc(id: &str, name: &str) -> Npc {
        Npc::new(id, name, "test desc", HexacoProfile::neutral())
    }

    fn dummy_turns() -> Vec<TurnSnapshot> {
        vec![TurnSnapshot {
            user_utterance: "안녕하세요".into(),
            npc_response: "안녕".into(),
            occ_emotions: vec![],
            pad_before: Pad::neutral(),
            pad_after: Pad::neutral(),
            beat_changed: false,
            turn_index: 1,
        }]
    }

    #[tokio::test]
    async fn reflect_returns_chitchat_when_llm_says_so() {
        let port = Arc::new(MockReflectionPort::ok(
            r#"{"is_chitchat": true, "summary": "지나가는 인사", "reasoning": "의례적"}"#,
        ));
        let svc = ReflectionService::new(
            port.clone(),
            Arc::new(DefaultReflectionPromptBuilder) as Arc<dyn ReflectionPromptBuilder>,
        );
        let result = svc
            .reflect("sid-1", &dummy_turns(), &npc("a", "Alice"), &npc("b", "Bob"))
            .await;
        assert!(result.is_chitchat);
        assert_eq!(result.summary, "지나가는 인사");
        assert_eq!(result.turn_count, 1);
        assert_eq!(
            port.call_count.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }

    #[tokio::test]
    async fn reflect_returns_significant_event_when_llm_says_so() {
        let port = Arc::new(MockReflectionPort::ok(
            r#"{"is_chitchat": false, "summary": "결단 사건", "reasoning": "OCC 0.9+"}"#,
        ));
        let svc = ReflectionService::new(
            port,
            Arc::new(DefaultReflectionPromptBuilder) as Arc<dyn ReflectionPromptBuilder>,
        );
        let result = svc
            .reflect("sid-2", &dummy_turns(), &npc("a", "Alice"), &npc("b", "Bob"))
            .await;
        assert!(!result.is_chitchat);
        assert_eq!(result.summary, "결단 사건");
    }

    #[tokio::test]
    async fn reflect_invalid_json_returns_fallback_with_reason() {
        let port = Arc::new(MockReflectionPort::ok("not valid json at all"));
        let svc = ReflectionService::new(
            port,
            Arc::new(DefaultReflectionPromptBuilder) as Arc<dyn ReflectionPromptBuilder>,
        );
        let result = svc
            .reflect("sid-3", &dummy_turns(), &npc("a", "Alice"), &npc("b", "Bob"))
            .await;
        // fallback: is_chitchat=false (보수적), reasoning에 "FALLBACK" 표기
        assert!(!result.is_chitchat);
        assert!(result.summary.contains("fallback"));
        assert!(
            result
                .llm_reasoning
                .as_ref()
                .map(|s| s.contains("FALLBACK"))
                .unwrap_or(false)
        );
    }

    #[tokio::test]
    async fn reflect_timeout_returns_fallback_with_timeout_reason() {
        let port = Arc::new(MockReflectionPort::err(ReflectionError::Timeout(
            Duration::from_secs(10),
        )));
        let svc = ReflectionService::new(
            port,
            Arc::new(DefaultReflectionPromptBuilder) as Arc<dyn ReflectionPromptBuilder>,
        );
        let result = svc
            .reflect("sid-4", &dummy_turns(), &npc("a", "Alice"), &npc("b", "Bob"))
            .await;
        assert!(!result.is_chitchat);
        assert!(
            result
                .llm_reasoning
                .as_ref()
                .map(|s| s.contains("Timeout"))
                .unwrap_or(false)
        );
    }

    // ---- strip_json_envelope unit tests (spec §11.1 robust 파싱) ----

    #[test]
    fn strip_json_envelope_plain_json_passthrough() {
        let raw = r#"{"is_chitchat": true, "summary": "test"}"#;
        assert_eq!(strip_json_envelope(raw), raw);
    }

    #[test]
    fn strip_json_envelope_markdown_fence_json() {
        let raw = "```json\n{\"is_chitchat\": false, \"summary\": \"중요\"}\n```";
        let cleaned = strip_json_envelope(raw);
        assert!(cleaned.starts_with('{'));
        assert!(cleaned.ends_with('}'));
        let parsed: serde_json::Value = serde_json::from_str(cleaned).unwrap();
        assert_eq!(parsed["is_chitchat"], false);
    }

    #[test]
    fn strip_json_envelope_markdown_fence_bare() {
        let raw = "```\n{\"is_chitchat\": true}\n```";
        let cleaned = strip_json_envelope(raw);
        let parsed: serde_json::Value = serde_json::from_str(cleaned).unwrap();
        assert_eq!(parsed["is_chitchat"], true);
    }

    #[test]
    fn strip_json_envelope_prose_prefix() {
        let raw = "좋습니다, 다음과 같이 분석합니다: {\"is_chitchat\": true, \"summary\": \"인사\"} 끝.";
        let cleaned = strip_json_envelope(raw);
        let parsed: serde_json::Value = serde_json::from_str(cleaned).unwrap();
        assert_eq!(parsed["is_chitchat"], true);
    }

    #[test]
    fn strip_json_envelope_no_json_returns_trimmed() {
        let raw = "  totally not json  ";
        assert_eq!(strip_json_envelope(raw), "totally not json");
    }

    #[tokio::test]
    async fn reflect_handles_markdown_fence_response() {
        // LLM이 ```json fence로 응답한 경우 — robust 파싱이 정상 동작.
        let port = Arc::new(MockReflectionPort::ok(
            "```json\n{\"is_chitchat\": true, \"summary\": \"잡담\"}\n```",
        ));
        let svc = ReflectionService::new(
            port,
            Arc::new(DefaultReflectionPromptBuilder) as Arc<dyn ReflectionPromptBuilder>,
        );
        let result = svc
            .reflect("sid-fence", &dummy_turns(), &npc("a", "Alice"), &npc("b", "Bob"))
            .await;
        assert!(result.is_chitchat);
        assert_eq!(result.summary, "잡담");
    }

    #[tokio::test]
    async fn reflect_significance_score_uses_engine_value_not_llm() {
        // LLM이 어떤 답을 하든 significance_score는 compute_significance 결과.
        let port = Arc::new(MockReflectionPort::ok(
            r#"{"is_chitchat": false, "summary": "test"}"#,
        ));
        let svc = ReflectionService::new(
            port,
            Arc::new(DefaultReflectionPromptBuilder) as Arc<dyn ReflectionPromptBuilder>,
        );
        let result = svc
            .reflect("sid-5", &[], &npc("a", "Alice"), &npc("b", "Bob"))
            .await;
        // 빈 turns → compute_significance = 0.0
        assert_eq!(result.significance_score, 0.0);
    }
}
