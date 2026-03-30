//! 포맷팅 포함 서비스 — MindService + GuideFormatter 조합
//!
//! `MindService`의 도메인 결과에 자동으로 포맷팅을 적용하여
//! `prompt: String`이 포함된 응답을 반환합니다.
//!
//! # 사용 예시
//!
//! ```rust,ignore
//! // 빌트인 한국어
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

use crate::ports::GuideFormatter;
use crate::presentation::formatter::LocaleFormatter;
use crate::presentation::locale::LocaleBundle;
use crate::presentation::builtin_toml;

use super::mind_service::{MindService, MindRepository, MindServiceError};
use super::dto::*;

/// 포맷팅 포함 Mind 서비스
///
/// `MindService` 위에 `GuideFormatter`를 조합하여
/// 모든 응답에 포맷팅된 `prompt` 문자열을 포함시킵니다.
pub struct FormattedMindService<R: MindRepository> {
    service: MindService<R>,
    formatter: Box<dyn GuideFormatter>,
}

impl<R: MindRepository> FormattedMindService<R> {
    /// 빌트인 언어로 생성합니다.
    ///
    /// 지원 언어: `"ko"` (한국어), `"en"` (영어)
    pub fn new(repository: R, lang: &str) -> Result<Self, MindServiceError> {
        let toml = builtin_toml(lang)
            .ok_or_else(|| MindServiceError::LocaleError(
                format!("지원하지 않는 빌트인 언어: '{}' (지원: ko, en)", lang)
            ))?;
        let formatter = LocaleFormatter::from_toml(toml)
            .map_err(|e| MindServiceError::LocaleError(e.to_string()))?;
        Ok(Self {
            service: MindService::new(repository),
            formatter: Box::new(formatter),
        })
    }

    /// 빌트인 언어 위에 커스텀 TOML을 부분 덮어쓰기하여 생성합니다.
    ///
    /// override TOML에 정의된 키만 교체되고, 나머지는 빌트인 값이 유지됩니다.
    pub fn with_overrides(repository: R, lang: &str, overrides: &str) -> Result<Self, MindServiceError> {
        let base = builtin_toml(lang)
            .ok_or_else(|| MindServiceError::LocaleError(
                format!("지원하지 않는 빌트인 언어: '{}' (지원: ko, en)", lang)
            ))?;
        let bundle = LocaleBundle::from_toml_with_overrides(base, overrides)
            .map_err(|e| MindServiceError::LocaleError(e.to_string()))?;
        let formatter = LocaleFormatter::new(bundle);
        Ok(Self {
            service: MindService::new(repository),
            formatter: Box::new(formatter),
        })
    }

    /// 완전한 커스텀 TOML로 생성합니다 (빌트인 없이).
    ///
    /// 모든 필수 키가 TOML에 포함되어 있어야 합니다.
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
    pub fn apply_stimulus(&mut self, req: StimulusRequest) -> Result<StimulusResponse, MindServiceError> {
        let result = self.service.apply_stimulus(req)?;
        Ok(result.format(&*self.formatter))
    }

    /// 현재 감정 상태 기반으로 포맷팅된 가이드를 생성합니다.
    pub fn generate_guide(&self, req: GuideRequest) -> Result<GuideResponse, MindServiceError> {
        let result = self.service.generate_guide(req)?;
        Ok(result.format(&*self.formatter))
    }

    /// 대화 종료 후 관계를 갱신합니다.
    pub fn after_dialogue(&mut self, req: AfterDialogueRequest) -> Result<AfterDialogueResponse, MindServiceError> {
        self.service.after_dialogue(req)
    }

    /// Beat 종료 시 관계를 갱신합니다.
    pub fn after_beat(&mut self, req: AfterDialogueRequest) -> Result<AfterDialogueResponse, MindServiceError> {
        self.service.after_beat(req)
    }
}
