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

    /// 세계 오버레이 — Guide 직후, Relationship 이전 (§6.5 B6, C11 잠정).
    /// `ApplyWorldEventRequested`를 `WorldEventOccurred`로 변환하는 순수 팬아웃.
    pub const WORLD_OVERLAY: i32 = 25;

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

    /// Memory 저장 계열 Inline 핸들러 — `TellingIngestionHandler` (Step C2),
    /// `RumorDistributionHandler` (Step C3) 공용. Projection(30) 이후 실행되어
    /// 쿼리 일관성 뷰가 먼저 업데이트된 뒤 기억 인덱싱이 일어난다.
    pub const MEMORY_INGESTION: i32 = 40;

    /// World 오버레이 MemoryEntry 생성 — `WorldOverlayHandler` (Step D).
    /// Canonical 생성 + 기존 Topic Canonical supersede가 이 단계에서 일어난다.
    pub const WORLD_OVERLAY_INGESTION: i32 = 45;

    /// 관계 갱신 기록 — `RelationshipMemoryHandler` (Step D). `RelationshipUpdated`를
    /// cause variant에 따라 MemoryEntry로 기록. Projection/Memory 인덱싱 뒤에서 돈다.
    pub const RELATIONSHIP_MEMORY: i32 = 50;

    /// Scene 통합 (Layer A→B) — `SceneConsolidationHandler` (Step D). 모든 기억
    /// 인덱싱 뒤에서 실행되어 Scene 턴별 엔트리를 Layer B로 흡수할 수 있다.
    pub const SCENE_CONSOLIDATION: i32 = 60;
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

    #[test]
    fn memory_ingestion_runs_after_scene_projection() {
        // Projection 3종보다 뒤에서 실행되어야 쿼리 일관성 보장.
        assert!(inline::MEMORY_INGESTION > inline::SCENE_PROJECTION);
    }

    #[test]
    fn world_overlay_runs_after_guide_before_relationship() {
        // §6.5 B6: Guide 직후, Relationship 이전.
        assert!(transactional::WORLD_OVERLAY > transactional::GUIDE_GENERATION);
        assert!(transactional::WORLD_OVERLAY < transactional::RELATIONSHIP_UPDATE);
    }

    #[test]
    fn world_overlay_ingestion_runs_after_memory_ingestion() {
        // MemoryEntry 생성 → supersede 순서 보장.
        assert!(inline::WORLD_OVERLAY_INGESTION > inline::MEMORY_INGESTION);
    }

    #[test]
    fn relationship_memory_runs_after_world_overlay_ingestion() {
        // World 오버레이 Canonical이 먼저 생성된 뒤 관계 기억에 반영될 수 있게.
        assert!(inline::RELATIONSHIP_MEMORY > inline::WORLD_OVERLAY_INGESTION);
    }

    #[test]
    fn scene_consolidation_runs_last() {
        // Layer A→B 흡수는 모든 Layer A 엔트리 인덱싱 이후에 돌아야 놓치는 것이 없다.
        assert!(inline::SCENE_CONSOLIDATION > inline::RELATIONSHIP_MEMORY);
    }
}
