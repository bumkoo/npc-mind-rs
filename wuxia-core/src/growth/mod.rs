// wuxia-core/src/growth/mod.rs
//
// Growth Domain — "이 사람은 얼마나 강한가?"
//
// 캐릭터의 9가지 능력치를 관리한다.
// Character 도메인(누구인가)과 분리되어,
// CharacterId를 통해 연결된다.
//
// 현재 (Iteration 2.3 Step 4):
//   - GrowthProfile: 능력치 프로필 (Aggregate Root)
//   - StatBlock: 능력치 묶음 (Value Object, Serde 호환)
//   - StatType / StatCategory: 능력치 분류
//   - StatChange / ChangeSource: 능력치 변화 기록
//   - GrowthEvent: 단련/연마/노화 이벤트
//   - training: 성장 계수, 단련/연마/노화/실전위력 규칙
//   - MartialArt: 무공 정의 (Value Object)
//   - MartialArtType: 무공 유형 (5가지)
//   - MasteryLevel: 경지 (4단계)
//   - MartialArtProficiency: 무공 숙련도 (Value Object)
//   - GrowthError: 성장 도메인 에러
//
// 향후:
//   - Iteration 2.3A: 피로/부상 시스템
//   - Iteration 2.3B: 최대 강도 메카닉

pub mod error;
pub mod event;
pub mod martial_art;
pub mod model;
pub mod power;
pub mod stat;
pub mod training;

pub use error::GrowthError;
pub use event::{ChangeSource, GrowthEvent, StatChange};
pub use martial_art::{MartialArt, MartialArtProficiency, MartialArtType, MasteryLevel};
pub use model::GrowthProfile;
pub use power::{calculate_art_effective_power, calculate_best_art_power, calculate_combat_power};
pub use stat::{StatBlock, StatCategory, StatType, clamp_stat, STAT_MAX, STAT_MIN, STAT_DEFAULT};
pub use training::{
    ArtTrainingResult, calculate_art_training, calculate_base_max_intensity,
    calculate_effective_power, calculate_fatigue_from_training, calculate_injury_chance,
    calculate_max_intensity, calculate_stat_training, calculate_yearly_aging, growth_multiplier,
};
