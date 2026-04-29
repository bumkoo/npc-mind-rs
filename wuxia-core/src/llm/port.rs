// wuxia-core/src/llm/port.rs
//
// LLM Port — 헥사고날 아키텍처의 출력 포트.
//
// wuxia-core가 "이런 모양의 문이 필요하다"고 선언하고 (trait),
// wuxia-llm이 "내가 그 문을 만들었다"고 구현한다 (impl).
//
// 의존성 방향:
//   wuxia-core (LlmPort trait 정의) ← wuxia-llm (LlamaCppAdapter가 구현)
//   ↑ 이 방향은 절대 역전되지 않는다.
//
// 비유: 강호의 전서구(傳書鴿) 규약
//   LlmPort = "편지를 이 양식으로 써서 보내라" (규약)
//   LlamaCppAdapter = "나는 비둘기를 키우는 사람이다" (구현)
//   MockLlm = "나는 비둘기 없이 연습용 답장을 쓰는 사람이다" (테스트)
//
// 동기(sync) 인터페이스를 선택한 이유:
//   - wuxia-core는 async runtime을 모른다 (순수 도메인).
//   - LLM 호출(500~3000ms)은 게임 루프와 별개 스레드에서 처리.
//   - 어댑터 내부에서 blocking 또는 tokio::block_on 사용.
//   - Bevy 통합 시 AsyncComputeTaskPool로 감싸면 됨.

use super::types::{LlmError, LlmRequest, LlmResponse};

/// LLM에서 생성된 토큰 하나를 전달하는 콜백 함수 타입.
/// 
/// 반환값: `true`면 계속 생성, `false`면 중단 요청.
pub type LlmTokenCallback = Box<dyn FnMut(&str) -> bool + Send>;

/// LLM과 통신하기 위한 포트 (헥사고날 아키텍처).
pub trait LlmPort: Send + Sync {
    /// 프롬프트를 보내고 응답 텍스트를 받는다.
    fn generate(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        self.generate_with_callback(request, None)
    }

    /// 스트리밍을 지원하는 생성 메서드.
    fn generate_with_callback(
        &self,
        request: &LlmRequest,
        callback: Option<LlmTokenCallback>,
    ) -> Result<LlmResponse, LlmError>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{CharacterSamplingProfile, Message, StopReason, SystemSamplingConfig};

    /// 테스트용 Mock — trait 구현 가능 여부 검증.
    struct TestMockLlm {
        fixed_response: String,
    }

    impl LlmPort for TestMockLlm {
        fn generate_with_callback(
            &self,
            _request: &LlmRequest,
            mut callback: Option<LlmTokenCallback>,
        ) -> Result<LlmResponse, LlmError> {
            if let Some(cb) = callback.as_mut() {
                let _ = cb(&self.fixed_response);
            }
            Ok(LlmResponse {
                text: self.fixed_response.clone(),
                tokens_generated: 10,
                prompt_tokens: 0,
                stop_reason: StopReason::EndOfText,
            })
        }
    }

    #[test]
    fn mock_implements_trait() {
        let mock = TestMockLlm {
            fixed_response: "그 검, 꽤 값나가겠네.".to_string(),
        };
        let request = LlmRequest {
            system_prompt: "너는 소연이다.".to_string(),
            messages: vec![Message::user("넌 누구야?")],
            character_profile: CharacterSamplingProfile::default(),
            system_config: SystemSamplingConfig::default(),
            system_reminder: None,
        };
        let response = mock.generate(&request).unwrap();
        assert_eq!(response.text, "그 검, 꽤 값나가겠네.");
    }

    #[test]
    fn trait_object_works() {
        let mock: Box<dyn LlmPort> = Box::new(TestMockLlm {
            fixed_response: "밥값 내면 알려줄게.".to_string(),
        });
        let request = LlmRequest {
            system_prompt: "너는 소연이다.".to_string(),
            messages: vec![Message::user("이름이 뭔데?")],
            character_profile: CharacterSamplingProfile::new(0.7, 150, 1.1),
            system_config: SystemSamplingConfig::default(),
            system_reminder: None,
        };
        let response = mock.generate(&request).unwrap();
        assert_eq!(response.text, "밥값 내면 알려줄게.");
    }
}
