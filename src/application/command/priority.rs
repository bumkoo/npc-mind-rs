//! EventHandler 실행 우선순위 상수
//!
//! B안 이행 Stage B0 선행 준비 모듈. 실제 핸들러가 등록되기 전까지는 상수만 존재.
//!
//! 핸들러는 `DeliveryMode` 내부에 `priority: i32`를 가지며, Dispatcher는 같은 모드의
//! 핸들러들을 **priority 오름차순**으로 실행한다. 작은 값이 먼저 실행된다.
//!
//! 상수값 간의 순서는 감정 평가 → 자극 → 가이드라는 파이프라인 논리에서 비롯되며,
//! 이 순서 뒤집힘이 회귀되지 않도록 `invariants` 테스트가 지킨다.

#![allow(dead_code)]

/// Transactional 모드 핸들러의 실행 순서
///
/// 작은 값 먼저 실행. 에러 시 커맨드 전체 중단.
pub mod transactional {
    /// Scene 시작 — 감정 평가보다 먼저 (초기 Focus의 appraise가 EmotionAppraised를 cascade).
    ///
    /// B4.1: 현재 `SceneAgent`만 이 priority를 쓰며, `SceneStartRequested` interest가
    /// 배타적이라 실행 순서에는 영향이 없다. 그러나 broadcast interest 도입 시나
    /// 같은 이벤트를 여럿이 소비하게 될 때 의미가 명확해지도록 상수로 고정.
    pub const SCENE_START: i32 = 5;

    /// 감정 평가 — Scene 시작 뒤, 자극/가이드 이전 (자극/가이드가 emotion_state 의존)
    pub const EMOTION_APPRAISAL: i32 = 10;

    /// 자극 적용 (PAD → 감정 변동, Beat 전환 follow-up 발행)
    pub const STIMULUS_APPLICATION: i32 = 15;

    /// 가이드 생성 — 감정 평가·자극 이후 (HandlerShared.emotion_state 의존)
    pub const GUIDE_GENERATION: i32 = 20;

    /// 관계 갱신 — Scene/Beat 종료 시
    pub const RELATIONSHIP_UPDATE: i32 = 30;

    /// 정보 전달 — 화자의 발화 이벤트를 청자당 1 `InformationTold` follow-up으로 팬아웃.
    /// 관계 갱신(30) 이후에 실행되어야 청자의 현재 trust 값을 반영할 수 있다 (§6.5, B6).
    pub const INFORMATION_TELLING: i32 = 35;

    /// 소문 확산 — TellInformation 이후(§6.5 B6). `RumorAgent`가
    /// `RumorSeeded`/`RumorSpread` follow-up을 발행하고 `RumorStore`에 저장한다.
    pub const RUMOR_SPREAD: i32 = 40;

    /// 감사 로그 — 가장 마지막
    pub const AUDIT: i32 = 90;
}

/// Inline 모드 핸들러(주로 Projection)의 실행 순서
///
/// 작은 값 먼저 실행. 에러는 로그만, 커맨드는 계속.
pub mod inline {
    /// 감정 프로젝션 (EmotionAppraised / StimulusApplied)
    pub const EMOTION_PROJECTION: i32 = 10;

    /// 관계 프로젝션 (RelationshipUpdated)
    pub const RELATIONSHIP_PROJECTION: i32 = 20;

    /// Scene 프로젝션 (SceneStarted / BeatTransitioned / SceneEnded)
    pub const SCENE_PROJECTION: i32 = 30;
}

#[cfg(test)]
mod invariants {
    use super::*;

    #[test]
    fn emotion_appraisal_runs_before_guide_generation() {
        assert!(transactional::EMOTION_APPRAISAL < transactional::GUIDE_GENERATION);
    }

    #[test]
    fn stimulus_application_runs_before_guide_generation() {
        assert!(transactional::STIMULUS_APPLICATION < transactional::GUIDE_GENERATION);
    }

    #[test]
    fn audit_runs_after_relationship_update() {
        assert!(transactional::AUDIT > transactional::RELATIONSHIP_UPDATE);
    }

    #[test]
    fn all_inline_priorities_are_positive() {
        assert!(inline::EMOTION_PROJECTION > 0);
        assert!(inline::RELATIONSHIP_PROJECTION > 0);
        assert!(inline::SCENE_PROJECTION > 0);
    }

    #[test]
    fn scene_start_runs_before_emotion_appraisal() {
        assert!(transactional::SCENE_START < transactional::EMOTION_APPRAISAL);
    }

    #[test]
    fn information_telling_runs_after_relationship_update() {
        assert!(transactional::INFORMATION_TELLING > transactional::RELATIONSHIP_UPDATE);
    }

    #[test]
    fn information_telling_runs_before_audit() {
        assert!(transactional::INFORMATION_TELLING < transactional::AUDIT);
    }

    #[test]
    fn rumor_spread_runs_after_information_telling() {
        assert!(transactional::RUMOR_SPREAD > transactional::INFORMATION_TELLING);
    }

    #[test]
    fn rumor_spread_runs_before_audit() {
        assert!(transactional::RUMOR_SPREAD < transactional::AUDIT);
    }
}
