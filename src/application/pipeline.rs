//! Pipeline — 순차 에이전트 체인 (v1, **deprecated**)
//!
//! **B5.1 (v0.2.0):** 이 모듈 전체는 `dispatch_v2()`의 transactional handler chain으로
//! 대체됨 (§5.1). v0.3.0에서 제거 예정. 마이그레이션 가이드: `CommandDispatcher::dispatch_v2`
//! + `with_default_handlers()` 조합 참조.
//!
//! ---
//!
//! 파이프라인 내부의 단계들은 순차 실행되며,
//! 앞 단계의 출력이 뒤 단계의 입력 컨텍스트에 반영됩니다.
//! 파이프라인 외부의 이벤트 소비자(Tier 2)는 비동기로 독립 실행됩니다.

#![allow(deprecated)]

use crate::domain::emotion::Scene;
use crate::domain::event::EventPayload;

use super::command::handler::{HandlerContext, HandlerOutput};
use super::command::types::CommandResult;
use super::mind_service::MindServiceError;

/// 파이프라인 단계: 한 번 실행되는 클로저
///
/// `PipelineState`를 읽고 수정하며, `HandlerOutput`을 반환합니다.
/// `FnOnce` — 각 단계는 커맨드별 데이터를 캡처하여 한 번만 실행.
#[deprecated(
    since = "0.2.0",
    note = "v1 Pipeline은 v2 dispatch_v2의 transactional handler chain으로 대체됨. v0.3.0에서 제거 예정."
)]
pub type PipelineStage =
    Box<dyn FnOnce(&mut PipelineState) -> Result<HandlerOutput, MindServiceError> + Send>;

/// 파이프라인 실행 중 상태 — 단계 간 전파
#[deprecated(
    since = "0.2.0",
    note = "v1 PipelineState는 v2 HandlerShared로 대체됨. v0.3.0에서 제거 예정."
)]
pub struct PipelineState {
    /// 현재 컨텍스트 (각 단계의 결과로 갱신됨)
    pub context: HandlerContext,
    /// 모든 단계에서 발생한 이벤트 축적
    pub accumulated_events: Vec<EventPayload>,
    /// 마지막 단계의 결과
    pub final_result: Option<CommandResult>,
    // --- 축적된 side-effects (last-write-wins) ---
    pub new_emotion_state: Option<(String, crate::domain::emotion::EmotionState)>,
    pub new_relationship: Option<(String, String, crate::domain::relationship::Relationship)>,
    pub clear_emotion: Option<String>,
    pub clear_scene: bool,
    pub save_scene: Option<Scene>,
}

impl PipelineState {
    /// 초기 상태 생성
    pub fn new(context: HandlerContext) -> Self {
        Self {
            context,
            accumulated_events: Vec::new(),
            final_result: None,
            new_emotion_state: None,
            new_relationship: None,
            clear_emotion: None,
            clear_scene: false,
            save_scene: None,
        }
    }

    /// HandlerOutput을 상태에 병합
    fn merge_output(&mut self, output: HandlerOutput) {
        // 이벤트 축적
        self.accumulated_events.extend(output.events);

        // 컨텍스트 갱신 — 다음 단계가 갱신된 상태를 볼 수 있도록
        if let Some((_, ref state)) = output.new_emotion_state {
            self.context.emotion_state = Some(state.clone());
        }
        if let Some((_, _, ref rel)) = output.new_relationship {
            self.context.relationship = Some(rel.clone());
        }
        if let Some(ref scene) = output.save_scene {
            self.context.scene = Some(scene.clone());
        }

        // Side-effects 축적 (last-write-wins)
        if output.new_emotion_state.is_some() {
            self.new_emotion_state = output.new_emotion_state;
        }
        if output.new_relationship.is_some() {
            self.new_relationship = output.new_relationship;
        }
        if output.clear_emotion.is_some() {
            self.clear_emotion = output.clear_emotion;
        }
        if output.clear_scene {
            self.clear_scene = true;
        }
        if output.save_scene.is_some() {
            self.save_scene = output.save_scene;
        }

        // 결과 갱신 (마지막 단계의 결과가 최종)
        self.final_result = Some(output.result);
    }
}

/// 순차 에이전트 파이프라인
///
/// ```rust,ignore
/// let pipeline = Pipeline::new()
///     .add_stage(Box::new(|state| emotion_agent.handle_appraise(..., &state.context)))
///     .add_stage(Box::new(|state| guide_agent.handle_generate(..., &state.context)));
///
/// let result = dispatcher.execute_pipeline(pipeline, &cmd)?;
/// ```
#[deprecated(
    since = "0.2.0",
    note = "v1 Pipeline은 dispatch_v2()의 transactional handler chain으로 대체됨. v0.3.0에서 제거 예정."
)]
pub struct Pipeline {
    stages: Vec<PipelineStage>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
        }
    }

    /// 파이프라인에 단계 추가 (빌더 패턴)
    pub fn add_stage(mut self, stage: PipelineStage) -> Self {
        self.stages.push(stage);
        self
    }

    /// 단계 수
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// 파이프라인 실행 — 모든 단계를 순차 실행
    ///
    /// 에러 발생 시 즉시 중단하고 에러를 반환합니다.
    /// 빈 파이프라인은 에러를 반환합니다.
    pub fn execute(self, mut state: PipelineState) -> Result<PipelineState, MindServiceError> {
        if self.stages.is_empty() {
            return Err(MindServiceError::InvalidSituation(
                "빈 파이프라인은 실행할 수 없습니다.".into(),
            ));
        }

        for stage in self.stages {
            let output = stage(&mut state)?;
            state.merge_output(output);
        }

        Ok(state)
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}
