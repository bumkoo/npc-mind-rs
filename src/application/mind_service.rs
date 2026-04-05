use crate::domain::emotion::{
    AppraisalEngine, EmotionState, Scene, SceneFocus, Situation, StimulusEngine,
};
use crate::domain::guide::ActingGuide;
use crate::domain::pad::Pad;
use crate::domain::personality::Npc;
use crate::domain::relationship::Relationship;
use crate::domain::tuning::{BEAT_DEFAULT_SIGNIFICANCE, BEAT_MERGE_THRESHOLD};
use crate::ports::{Appraiser, StimulusProcessor};

// 저장소 포트 재노출
pub use crate::ports::{EmotionStore, MindRepository, NpcWorld, SceneStore};

use super::relationship_service::RelationshipService;
use super::scene_service::SceneService;
use super::situation_service::SituationService;
use super::dto::*;

/// Mind 서비스 계층에서 발생하는 오류
///
/// 모든 공개 메서드는 이 타입을 반환하며, `thiserror` 기반으로
/// `Display`/`Error` 트레이트가 자동 구현됩니다.
#[derive(Debug, thiserror::Error)]
pub enum MindServiceError {
    /// 저장소에서 NPC를 찾을 수 없음
    #[error("NPC '{0}'를 찾을 수 없습니다.")]
    NpcNotFound(String),
    /// NPC↔Partner 관계가 등록되지 않음
    #[error("관계 '{0}↔{1}'를 찾을 수 없습니다.")]
    RelationshipNotFound(String, String),
    /// 상황 입력 데이터가 유효하지 않음 (잘못된 감정 유형, 누락 필드 등)
    #[error("상황 정보가 잘못되었습니다: {0}")]
    InvalidSituation(String),
    /// `appraise()`가 먼저 호출되지 않아 감정 상태가 없음
    #[error("현재 감정 상태를 찾을 수 없습니다. (먼저 평가를 실행하세요)")]
    EmotionStateNotFound,
    /// 로케일 TOML 파싱 또는 빌트인 언어 조회 실패 (`FormattedMindService` 전용)
    #[error("로케일 초기화 실패: {0}")]
    LocaleError(String),
}

/// NPC 심리 엔진의 핵심 진입점 (Application Service)
///
/// HEXACO 성격 모델 기반으로 OCC 감정을 생성하고,
/// LLM이 연기할 수 있는 [`ActingGuide`]를 포함한 결과를 반환합니다.
///
/// 제네릭 파라미터 `A`, `S`는 기본값이 있으므로 대부분의 경우
/// `MindService<R>` 형태로 사용합니다.
///
/// # 기본 사용 흐름
///
/// ```rust,ignore
/// let repo = InMemoryRepository::from_file("scenario.json")?;
/// let mut service = MindService::new(repo);
///
/// // 1. 상황 평가 → 감정 생성
/// let result = service.appraise(req, || {}, Vec::new)?;
///
/// // 2. 대화 중 상대 대사에 의한 감정 변동
/// let stim = service.apply_stimulus(stim_req, || {}, Vec::new)?;
///
/// // 3. 대화 종료 → 관계 갱신 + 감정 초기화
/// let after = service.after_dialogue(after_req)?;
/// ```
///
/// # 포맷팅이 필요한 경우
///
/// LLM에 바로 전달할 프롬프트 문자열이 필요하면 [`FormattedMindService`]를 사용하세요.
/// `MindService`는 도메인 결과만 반환합니다.
pub struct MindService<
    R: MindRepository,
    A: Appraiser = AppraisalEngine,
    S: StimulusProcessor = StimulusEngine,
> {
    repository: R,
    appraiser: A,
    stimulus_processor: S,
    relationship_service: RelationshipService,
    scene_service: SceneService,
    situation_service: SituationService,
}

impl<R: MindRepository> MindService<R> {
    /// 기본 엔진(`AppraisalEngine`, `StimulusEngine`)으로 서비스를 생성합니다.
    ///
    /// 커스텀 엔진이 필요하면 [`MindService::with_engines`]를 사용하세요.
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            appraiser: AppraisalEngine,
            stimulus_processor: StimulusEngine,
            relationship_service: RelationshipService::new(),
            scene_service: SceneService::new(),
            situation_service: SituationService::new(),
        }
    }
}

impl<R: MindRepository, A: Appraiser, S: StimulusProcessor> MindService<R, A, S> {
    /// 커스텀 평가/자극 엔진을 주입하여 서비스를 생성합니다.
    ///
    /// 테스트에서 Mock 엔진을 사용하거나,
    /// 도메인 로직을 확장할 때 활용합니다.
    pub fn with_engines(repository: R, appraiser: A, stimulus_processor: S) -> Self {
        Self {
            repository,
            appraiser,
            stimulus_processor,
            relationship_service: RelationshipService::new(),
            scene_service: SceneService::new(),
            situation_service: SituationService::new(),
        }
    }

    /// 내부 저장소에 대한 가변 참조를 반환합니다.
    ///
    /// NPC/관계/오브젝트를 직접 추가·수정할 때 사용합니다.
    pub fn repository_mut(&mut self) -> &mut R {
        &mut self.repository
    }

    /// 내부 저장소에 대한 불변 참조를 반환합니다.
    pub fn repository(&self) -> &R {
        &self.repository
    }

    // ---------------------------------------------------------------------------
    // 도메인 변환 헬퍼 (ARCH-2: DTO에서 이동됨)
    // ---------------------------------------------------------------------------

    // ---------------------------------------------------------------------------
    // 공통 내부 헬퍼 (중복 제거)
    // ---------------------------------------------------------------------------

    /// NPC와 관계 정보를 한 번에 조회합니다.
    fn prepare_context(
        &self,
        npc_id: &str,
        partner_id: &str,
    ) -> Result<(Npc, Relationship), MindServiceError> {
        let npc = self
            .repository
            .get_npc(npc_id)
            .ok_or_else(|| MindServiceError::NpcNotFound(npc_id.to_string()))?;

        let relationship = self
            .repository
            .get_relationship(npc_id, partner_id)
            .ok_or_else(|| {
                MindServiceError::RelationshipNotFound(npc_id.to_string(), partner_id.to_string())
            })?;

        Ok((npc, relationship))
    }

    /// 평가 엔진을 실행하고 결과를 저장한 뒤 AppraiseResult를 반환합니다.
    fn execute_appraise_workflow(
        &mut self,
        npc: &Npc,
        relationship: &Relationship,
        situation: &Situation,
        mut before_eval: impl FnMut(),
        mut after_eval: impl FnMut() -> Vec<String>,
    ) -> AppraiseResult {
        before_eval();
        let emotion_state =
            self.appraiser
                .appraise(npc.personality(), situation, &relationship.modifiers());
        let trace = after_eval();

        let result = build_appraise_result(
            npc,
            &emotion_state,
            Some(situation.description.clone()),
            Some(relationship),
            trace,
        );

        self.repository.save_emotion_state(npc.id(), emotion_state);
        result
    }

    // ---------------------------------------------------------------------------
    // 공개 API 메서드
    // ---------------------------------------------------------------------------

    /// 상황을 평가하여 OCC 감정을 생성하고, [`ActingGuide`]를 포함한 결과를 반환합니다.
    ///
    /// Beat의 시작점입니다. NPC의 HEXACO 성격이 상황(Event/Action/Object)을
    /// 해석하여 감정 상태를 결정하고, 이를 기반으로 연기 가이드를 생성합니다.
    ///
    /// # 콜백 파라미터
    /// - `before_eval`: 평가 직전 호출 (tracing 시작 등)
    /// - `after_eval`: 평가 직후 호출, trace 문자열 반환 (tracing 종료 등)
    ///
    /// # 에러
    /// - [`MindServiceError::NpcNotFound`] — NPC ID가 저장소에 없을 때
    /// - [`MindServiceError::RelationshipNotFound`] — NPC↔Partner 관계가 없을 때
    /// - [`MindServiceError::InvalidSituation`] — 상황 데이터가 잘못되었을 때
    pub fn appraise(
        &mut self,
        req: AppraiseRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<AppraiseResult, MindServiceError> {
        let (npc, relationship) = self.prepare_context(&req.npc_id, &req.partner_id)?;
        let situation = self.situation_service.to_situation(&self.repository, &req.situation, &req.npc_id, &req.partner_id)?;

        Ok(
            self.execute_appraise_workflow(
                &npc,
                &relationship,
                &situation,
                before_eval,
                after_eval,
            ),
        )
    }

    /// 대화 턴 중 상대 대사에 의한 PAD 자극을 적용하여 감정을 갱신합니다.
    ///
    /// 현재 감정의 관성을 고려하여 자극 수용도를 조절합니다:
    /// 감정이 강할수록 자극에 덜 흔들리고, 약할수록 쉽게 변합니다.
    ///
    /// Scene이 활성 상태이면 Focus trigger 조건을 자동 체크하여
    /// 조건 충족 시 Beat 전환(`transition_beat`)을 수행합니다.
    /// Beat 전환 시 결과의 `beat_changed`가 `true`로 설정됩니다.
    ///
    /// # 에러
    /// - [`MindServiceError::EmotionStateNotFound`] — `appraise()`가 먼저 호출되지 않았을 때
    pub fn apply_stimulus(
        &mut self,
        req: StimulusRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<StimulusResult, MindServiceError> {
        let (npc, relationship) = self.prepare_context(&req.npc_id, &req.partner_id)?;

        let current = self
            .repository
            .get_emotion_state(&req.npc_id)
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        // 1. PAD 자극 적용 (관성)
        let pad = Pad {
            pleasure: req.pleasure,
            arousal: req.arousal,
            dominance: req.dominance,
        };
        let stimulated_state =
            self.stimulus_processor
                .apply_stimulus(npc.personality(), &current, &pad);
        self.repository
            .save_emotion_state(&req.npc_id, stimulated_state.clone());

        // 2. Scene Focus trigger 체크 → Beat 전환
        if let Some((mut scene, focus)) = self
            .repository
            .get_scene()
            .and_then(|s| self.scene_service.check_trigger(&s, &stimulated_state).map(|f| (s, f)))
        {
            return self.transition_beat(
                &req.npc_id,
                &npc,
                &relationship,
                &mut scene,
                focus,
                pad,
                before_eval,
                after_eval,
            );
        }

        // 3. Beat 전환 없음 (하지만 현재 Scene이 있으면 active_focus_id는 유지)
        let (emotions, dominant, mood) = build_emotion_fields(&stimulated_state);
        let guide = ActingGuide::build(
            &npc,
            &stimulated_state,
            req.situation_description.clone(),
            Some(&relationship),
        );

        // Scene이 활성 상태면 현재 active_focus_id를 반환 (Beat 전환이 없어도 UI가 상태를 알 수 있도록)
        let active_focus_id = self
            .repository
            .get_scene()
            .and_then(|s| s.active_focus_id().map(|id| id.to_string()));

        Ok(StimulusResult {
            emotions,
            dominant,
            mood,
            guide,
            trace: vec![],
            beat_changed: false,
            active_focus_id,
            input_pad: Some(PadOutput {
                pleasure: req.pleasure,
                arousal: req.arousal,
                dominance: req.dominance,
            }),
        })
    }

    /// Beat 전환: 관계 갱신 → 새 Focus appraise → 감정 병합
    fn transition_beat(
        &mut self,
        npc_id: &str,
        npc: &Npc,
        relationship: &Relationship,
        scene: &mut Scene,
        focus: SceneFocus,
        input_pad: Pad,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<StimulusResult, MindServiceError> {
        self.update_beat_relationship(scene);

        let situation = focus
            .to_situation()
            .map_err(|e| MindServiceError::InvalidSituation(e.to_string()))?;

        let previous = self
            .repository
            .get_emotion_state(npc_id)
            .unwrap_or_default();

        // 통합 워크플로우 호출 (결과는 internal_appraise가 저장함)
        let app_result =
            self.execute_appraise_workflow(npc, relationship, &situation, before_eval, after_eval);

        // 병합 처리 (이전 감정 + 새 감정)
        let new_state = self
            .repository
            .get_emotion_state(npc_id)
            .unwrap_or_default();
        let merged = EmotionState::merge_from_beat(&previous, &new_state, BEAT_MERGE_THRESHOLD);
        self.repository.save_emotion_state(npc_id, merged.clone());

        // 병합된 상태로 필드 재구성
        let (emotions, dominant, mood) = build_emotion_fields(&merged);
        let guide = ActingGuide::build(
            npc,
            &merged,
            Some(focus.description.clone()),
            Some(relationship),
        );

        let focus_id = focus.id.clone();
        scene.set_active_focus(focus_id.clone());
        self.repository.save_scene(scene.clone());

        Ok(StimulusResult {
            emotions,
            dominant,
            mood,
            guide,
            trace: app_result.trace,
            beat_changed: true,
            active_focus_id: Some(focus_id),
            input_pad: Some(PadOutput {
                pleasure: input_pad.pleasure,
                arousal: input_pad.arousal,
                dominance: input_pad.dominance,
            }),
        })
    }

    fn update_beat_relationship(&mut self, scene: &Scene) {
        let beat_req = AfterDialogueRequest {
            npc_id: scene.npc_id().to_string(),
            partner_id: scene.partner_id().to_string(),
            significance: Some(BEAT_DEFAULT_SIGNIFICANCE),
        };
        let _ = self.update_relationship(&beat_req);
    }

    /// Scene을 시작합니다: Focus 옵션 목록 등록 + Initial Focus 자동 평가.
    ///
    /// 게임이 대화 시작 시 가능한 감정 전환점(Focus)들을 등록하면,
    /// 엔진이 `Initial` 트리거를 가진 Focus를 자동으로 찾아 `appraise`합니다.
    /// 이후 `apply_stimulus` 호출 시 다른 Focus의 조건이 충족되면
    /// 자동으로 Beat 전환이 발생합니다.
    ///
    /// # 반환값
    /// - `SceneResult.focus_count`: 등록된 Focus 수
    /// - `SceneResult.initial_appraise`: Initial Focus 평가 결과 (있으면)
    /// - `SceneResult.active_focus_id`: 활성화된 Focus ID (있으면)
    pub fn start_scene(
        &mut self,
        req: SceneRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<SceneResult, MindServiceError> {
        let focuses: Vec<SceneFocus> = req
            .focuses
            .iter()
            .map(|f| self.situation_service.to_scene_focus(&self.repository, f, &req.npc_id, &req.partner_id))
            .collect::<Result<Vec<_>, _>>()?;

        let focus_count = focuses.len();
        let significance = req.significance.unwrap_or(0.5);
        let mut scene =
            Scene::with_significance(req.npc_id.clone(), req.partner_id.clone(), focuses, significance);

        let (initial_appraise, active_focus_id) =
            self.appraise_initial_focus(&mut scene, before_eval, after_eval)?;

        self.repository.save_scene(scene);
        Ok(SceneResult {
            focus_count,
            initial_appraise,
            active_focus_id,
        })
    }

    /// 현재 Scene의 Focus 상태를 조회합니다.
    ///
    /// 활성/대기 Focus 목록과 각각의 트리거 조건을 반환합니다.
    /// Scene이 없으면 `has_scene: false`인 결과를 반환합니다.
    pub fn scene_info(&self) -> SceneInfoResult {
        let Some(scene) = self.repository.get_scene() else {
            return SceneInfoResult {
                has_scene: false,
                npc_id: None,
                partner_id: None,
                active_focus_id: None,
                significance: None,
                focuses: vec![],
            };
        };

        self.scene_service.build_scene_info(&scene)
    }

    /// 시나리오 로드 시 Scene Focus를 직접 복원합니다.
    ///
    /// `start_scene`과 동일하게 Scene을 생성하고 Initial Focus를 평가하지만,
    /// 트레이싱 콜백 없이 동작합니다. `InMemoryRepository::from_file()` 후
    /// 저장된 Scene 데이터를 복원할 때 사용합니다.
    pub fn load_scene_focuses(
        &mut self,
        focuses: Vec<SceneFocus>,
        npc_id: String,
        partner_id: String,
        significance: f32,
    ) -> Result<Option<AppraiseResult>, MindServiceError> {
        let mut scene = Scene::with_significance(npc_id, partner_id, focuses, significance);
        let (result, _) = self.appraise_initial_focus(&mut scene, || {}, Vec::new)?;
        self.repository.save_scene(scene);
        Ok(result)
    }

    fn appraise_initial_focus(
        &mut self,
        scene: &mut Scene,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<(Option<AppraiseResult>, Option<String>), MindServiceError> {
        let npc_id = scene.npc_id();
        let partner_id = scene.partner_id();

        let Some(focus) = scene.initial_focus().cloned() else {
            return Ok((None, None));
        };

        let (npc, relationship) = self.prepare_context(npc_id, partner_id)?;
        let situation = focus
            .to_situation()
            .map_err(|e| MindServiceError::InvalidSituation(e.to_string()))?;

        let result = self.execute_appraise_workflow(
            &npc,
            &relationship,
            &situation,
            before_eval,
            after_eval,
        );
        scene.set_active_focus(focus.id.clone());

        Ok((Some(result), Some(focus.id)))
    }

    /// 현재 감정 상태에서 [`ActingGuide`]를 재생성합니다.
    ///
    /// 감정 상태를 변경하지 않고, 기존 상태를 기반으로
    /// Tone, Attitude, BehavioralTendency, Restriction을 다시 결정합니다.
    /// UI에서 가이드만 다시 확인하거나, 상황 설명만 바꿔서
    /// 새 가이드를 받고 싶을 때 사용합니다.
    ///
    /// # 에러
    /// - [`MindServiceError::EmotionStateNotFound`] — `appraise()`가 먼저 호출되지 않았을 때
    pub fn generate_guide(&self, req: GuideRequest) -> Result<GuideResult, MindServiceError> {
        let (npc, relationship) = self.prepare_context(&req.npc_id, &req.partner_id)?;
        let emotion_state = self
            .repository
            .get_emotion_state(&req.npc_id)
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let guide = ActingGuide::build(
            &npc,
            &emotion_state,
            req.situation_description.clone(),
            Some(&relationship),
        );
        Ok(GuideResult { guide })
    }

    /// 대화(Scene) 종료 후 관계를 갱신하고 감정을 초기화합니다.
    ///
    /// 대화 중 누적된 감정(Joy, Anger 등)을 바탕으로
    /// closeness를 조정합니다. `significance` (0.0~1.0)가
    /// 높을수록 관계 변동 폭이 커집니다.
    ///
    /// 호출 후 해당 NPC의 감정 상태와 Scene이 모두 초기화됩니다.
    ///
    /// # 반환값
    /// - `AfterDialogueResponse.before` / `.after`: 관계 변동 전후 값
    pub fn after_dialogue(
        &mut self,
        req: AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, MindServiceError> {
        let response = self.update_relationship(&req)?;
        self.repository.clear_emotion_state(&req.npc_id);
        self.repository.clear_scene();
        Ok(response)
    }

    /// Beat 종료 시 관계를 갱신합니다 (감정은 유지).
    ///
    /// `after_dialogue`와 달리 감정 상태와 Scene을 초기화하지 않습니다.
    /// 같은 Scene 안에서 Beat가 전환될 때 중간 관계 정산에 사용합니다.
    pub fn after_beat(
        &mut self,
        req: AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, MindServiceError> {
        self.update_relationship(&req)
    }

    fn update_relationship(
        &mut self,
        req: &AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, MindServiceError> {
        let relationship = self
            .repository
            .get_relationship(&req.npc_id, &req.partner_id)
            .or_else(|| {
                self.repository
                    .get_relationship(&req.partner_id, &req.npc_id)
            })
            .ok_or_else(|| {
                MindServiceError::RelationshipNotFound(req.npc_id.clone(), req.partner_id.clone())
            })?;

        let emotion_state = self
            .repository
            .get_emotion_state(&req.npc_id)
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let (new_rel, response) = self
            .relationship_service
            .update_relationship(&relationship, &emotion_state, req);

        self.repository
            .save_relationship(&req.npc_id, &req.partner_id, new_rel);
        Ok(response)
    }
}
