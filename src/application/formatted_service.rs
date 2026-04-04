//! 포맷팅 포함 서비스 — MindService + GuideFormatter 조합
//!
//! `MindService`의 도메인 결과에 자동으로 포맷팅을 적용하여
//! `prompt: String`이 포함된 응답을 반환합니다.

use crate::domain::emotion::{AppraisalEngine, SceneFocus, StimulusEngine};
use crate::ports::{Appraiser, GuideFormatter, StimulusProcessor};
use crate::presentation::builtin_toml;
use crate::presentation::formatter::LocaleFormatter;
use crate::presentation::locale::LocaleBundle;

use super::dto::*;
use super::mind_service::{MindRepository, MindService, MindServiceError};

/// 포맷팅 포함 Mind 서비스
///
/// `MindService` 위에 `GuideFormatter`를 조합하여
/// 모든 응답에 포맷팅된 `prompt` 문자열을 포함시킵니다.
pub struct FormattedMindService<
    R: MindRepository,
    A: Appraiser = AppraisalEngine,
    S: StimulusProcessor = StimulusEngine,
> {
    service: MindService<R, A, S>,
    formatter: Box<dyn GuideFormatter>,
}

// 생성자 로직
impl<R: MindRepository> FormattedMindService<R> {
    pub fn new(repository: R, lang: &str) -> Result<Self, MindServiceError> {
        let toml = builtin_toml(lang).ok_or_else(|| {
            MindServiceError::LocaleError(format!("지원하지 않는 빌트인 언어: '{}'", lang))
        })?;
        let formatter = LocaleFormatter::from_toml(toml)
            .map_err(|e| MindServiceError::LocaleError(e.to_string()))?;
        Ok(Self {
            service: MindService::new(repository),
            formatter: Box::new(formatter),
        })
    }

    pub fn with_overrides(
        repository: R,
        lang: &str,
        overrides: &str,
    ) -> Result<Self, MindServiceError> {
        let base = builtin_toml(lang).ok_or_else(|| {
            MindServiceError::LocaleError(format!("지원하지 않는 빌트인 언어: '{}'", lang))
        })?;
        let bundle = LocaleBundle::from_toml_with_overrides(base, overrides)
            .map_err(|e| MindServiceError::LocaleError(e.to_string()))?;
        let formatter = LocaleFormatter::new(bundle);
        Ok(Self {
            service: MindService::new(repository),
            formatter: Box::new(formatter),
        })
    }

    pub fn with_custom_locale(repository: R, toml_content: &str) -> Result<Self, MindServiceError> {
        let formatter = LocaleFormatter::from_toml(toml_content)
            .map_err(|e| MindServiceError::LocaleError(e.to_string()))?;
        Ok(Self {
            service: MindService::new(repository),
            formatter: Box::new(formatter),
        })
    }

    pub fn with_formatter(repository: R, formatter: impl GuideFormatter + 'static) -> Self {
        Self {
            service: MindService::new(repository),
            formatter: Box::new(formatter),
        }
    }
}

impl<R: MindRepository, A: Appraiser, S: StimulusProcessor> FormattedMindService<R, A, S> {
    // ---------------------------------------------------------------------------
    // 내부 헬퍼
    // ---------------------------------------------------------------------------

    fn format<T: CanFormat>(&self, result: T) -> T::Response {
        result.format(&*self.formatter)
    }

    // ---------------------------------------------------------------------------
    // 위임 메서드 (Delegation)
    // ---------------------------------------------------------------------------

    pub fn repository_mut(&mut self) -> &mut R {
        self.service.repository_mut()
    }

    pub fn repository(&self) -> &R {
        self.service.repository()
    }

    pub fn formatter(&self) -> &dyn GuideFormatter {
        &*self.formatter
    }

    // ---------------------------------------------------------------------------
    // 포맷팅 API
    // ---------------------------------------------------------------------------

    pub fn appraise(
        &mut self,
        req: AppraiseRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<AppraiseResponse, MindServiceError> {
        self.service
            .appraise(req, before_eval, after_eval)
            .map(|r| self.format(r))
    }

    pub fn apply_stimulus(
        &mut self,
        req: StimulusRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<StimulusResponse, MindServiceError> {
        self.service
            .apply_stimulus(req, before_eval, after_eval)
            .map(|r| self.format(r))
    }

    pub fn start_scene(
        &mut self,
        req: SceneRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<SceneResponse, MindServiceError> {
        self.service
            .start_scene(req, before_eval, after_eval)
            .map(|r| self.format(r))
    }

    pub fn generate_guide(&self, req: GuideRequest) -> Result<GuideResponse, MindServiceError> {
        self.service.generate_guide(req).map(|r| self.format(r))
    }

    // 포맷팅이 필요 없는 단순 전달 메서드들
    pub fn scene_info(&self) -> SceneInfoResult {
        self.service.scene_info()
    }

    pub fn load_scene_focuses(
        &mut self,
        focuses: Vec<SceneFocus>,
        npc_id: String,
        partner_id: String,
        significance: f32,
    ) -> Result<Option<AppraiseResult>, MindServiceError> {
        self.service
            .load_scene_focuses(focuses, npc_id, partner_id, significance)
    }

    pub fn after_dialogue(
        &mut self,
        req: AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, MindServiceError> {
        self.service.after_dialogue(req)
    }

    pub fn after_beat(
        &mut self,
        req: AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, MindServiceError> {
        self.service.after_beat(req)
    }
}
