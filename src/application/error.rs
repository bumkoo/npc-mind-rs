//! Application layer 공용 에러 타입

/// 애플리케이션 서비스 레이어 공용 에러
///
/// DTO 검증, situation 해석, 저장소 lookup 실패 등에서 반환됩니다.
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
    /// 로케일 TOML 파싱 또는 빌트인 언어 조회 실패
    #[error("로케일 초기화 실패: {0}")]
    LocaleError(String),
}
