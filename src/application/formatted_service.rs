//! 포맷팅 포함 서비스 — MindService + GuideFormatter 조합
//!
//! `MindService`의 도메인 결과에 자동으로 포맷팅을 적용하여
//! `prompt: String`이 포함된 응답을 반환합니다.
//!
//! # 사용 예시
//!
//! ```rust,ignore
//! // 빌트인 한국어 (기본 엔진)
//! let service = FormattedMindService::new(repo, "ko")?;
//!
//! // 빌트인 + 커스텀 오버라이드
//! let service = FormattedMindService::with_overrides(repo, "ko", custom_toml)?;
//!
//! // 완전 커스텀 TOML
//! let service = FormattedMindService::with_custom_locale(repo, my_toml)?;
//!
//! // GuideFormatter 트레이트 직접 구현
//! let service = FormattedMindService::with_formatter(repo, my_formatter);
//! ```

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

// 기본 엔진 사용 (기존 코드 호환)
impl<R: MindRepository> FormattedMindService<R> {
    /// 빌트인 언어로 생성합니다.
    ///
    /// 지원 언어: `"ko"` (한국어), `"en"` (영어)
    pub fn new(repository: R, lang: &str) -> Result<Self, MindServiceError> {
        let toml = builtin_toml(lang).ok_or_else(|| {
            MindServiceError::LocaleError(format!(
                "지원하지 않는 빌트인 언어: '{}' (지원: ko, en)",
                lang
            ))
        })?;
        let formatter = LocaleFormatter::from_toml(toml)
            .map_err(|e| MindServiceError::LocaleError(e.to_string()))?;
        Ok(Self {
            service: MindService::new(repository),
            formatter: Box::new(formatter),
        })
    }

    /// 빌트인 언어 위에 커스텀 TOML을 부분 덮어쓰기하여 생성합니다.
    pub fn with_overrides(
        repository: R,
        lang: &str,
        overrides: &str,
    ) -> Result<Self, MindServiceError> {
        let base = builtin_toml(lang).ok_or_else(|| {
            MindServiceError::LocaleError(format!(
                "지원하지 않는 빌트인 언어: '{}' (지원: ko, en)",
                lang
            ))
        })?;
        let bundle = LocaleBundle::from_toml_with_overrides(base, overrides)
            .map_err(|e| MindServiceError::LocaleError(e.to_string()))?;
        let formatter = LocaleFormatter::new(bundle);
        Ok(Self {
            service: MindService::new(repository),
            formatter: Box::new(formatter),
        })
    }

    /// 완전한 커스텀 TOML로 생성합니다 (빌트인 없이).
    pub fn with_custom_locale(repository: R, toml_content: &str) -> Result<Self, MindServiceError> {
        let formatter = LocaleFormatter::from_toml(toml_content)
            .map_err(|e| MindServiceError::LocaleError(e.to_string()))?;
        Ok(Self {
            service: MindService::new(repository),
            formatter: Box::new(formatter),
        })
    }

    /// GuideFormatter 트레이트 구현체를 직접 주입하여 생성합니다.
    pub fn with_formatter(repository: R, formatter: impl GuideFormatter + 'static) -> Self {
        Self {
            service: MindService::new(repository),
            formatter: Box::new(formatter),
        }
    }
}

impl<R: MindRepository, A: Appraiser, S: StimulusProcessor> FormattedMindService<R, A, S> {
    /// 내부 MindService의 저장소에 대한 가변 참조
    pub fn repository_mut(&mut self) -> &mut R {
        self.service.repository_mut()
    }

    /// 내부 MindService의 저장소에 대한 불변 참조
    pub fn repository(&self) -> &R {
        self.service.repository()
    }

    /// 내부 포맷터에 대한 참조
    pub fn formatter(&self) -> &dyn GuideFormatter {
        &*self.formatter
    }

    /// 상황을 평가하여 감정을 생성하고 포맷팅된 가이드를 반환합니다.
    pub fn appraise(
        &mut self,
        req: AppraiseRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<AppraiseResponse, MindServiceError> {
        let result = self.service.appraise(req, before_eval, after_eval)?;
        Ok(result.format(&*self.formatter))
    }

    /// 대화 턴 중 발생한 PAD 자극을 적용하여 감정을 갱신합니다.
    pub fn apply_stimulus(
        &mut self,
        req: StimulusRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<StimulusResponse, MindServiceError> {
        let result = self.service.apply_stimulus(req, before_eval, after_eval)?;
        Ok(result.format(&*self.formatter))
    }

    /// Scene 시작: Focus 목록 등록 + 초기 Focus 자동 appraise
    pub fn start_scene(
        &mut self,
        req: SceneRequest,
        before_eval: impl FnMut(),
        after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<SceneResponse, MindServiceError> {
        let result = self.service.start_scene(req, before_eval, after_eval)?;
        Ok(result.format(&*self.formatter))
    }

    /// 현재 Scene Focus 상태를 조회합니다.
    pub fn scene_info(&self) -> SceneInfoResult {
        self.service.scene_info()
    }

    /// Scene Focus를 직접 로드합니다.
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

    /// 현재 감정 상태 기반으로 포맷팅된 가이드를 생성합니다.
    pub fn generate_guide(&self, req: GuideRequest) -> Result<GuideResponse, MindServiceError> {
        let result = self.service.generate_guide(req)?;
        Ok(result.format(&*self.formatter))
    }

    /// 대화 종료 후 관계를 갱신합니다.
    pub fn after_dialogue(
        &mut self,
        req: AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, MindServiceError> {
        self.service.after_dialogue(req)
    }

    /// Beat 종료 시 관계를 갱신합니다.
    pub fn after_beat(
        &mut self,
        req: AfterDialogueRequest,
    ) -> Result<AfterDialogueResponse, MindServiceError> {
        self.service.after_beat(req)
    }
}
