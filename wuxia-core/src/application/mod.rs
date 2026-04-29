// wuxia-core/src/application/mod.rs
//
// Application Layer — 도메인 간의 조율자 (Orchestrator)
//
// DDD에서 Application Service는 비즈니스 로직을 가지지 않는다.
// 대신, 도메인 객체들을 조합하여 유스케이스를 실현한다.
//
// 비유: 무림대회의 사회자 (主持人)
//   사회자는 직접 무공을 쓰지 않는다.
//   "자, 이제 시합 시작!" → 선수들이 싸운다.
//   "1년이 지났습니다!" → 모든 캐릭터가 나이를 먹는다.
//
// 현재 서비스:
//   TimeCharacterService — 시간 이벤트 → 캐릭터 변화 조율
//   TrainingService      — 수련 요청 → 성장 + 피로 + 부상 조율 [v2.3C]

pub mod time_character;
pub mod training;

pub use time_character::TimeCharacterService;
pub use training::{TrainingError, TrainingOutcome, TrainingService};
