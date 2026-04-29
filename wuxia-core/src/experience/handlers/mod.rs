// wuxia-core/src/experience/handlers/mod.rs
//
// 경험 이벤트 핸들러 구현체 — Phase 2 MVP.
//
// 고정 실행 순서:
//   ① CharacterHandler (피로/부상)
//   ② GrowthHandler — Phase 3
//   ③ BondHandler (관계)
//   ④~⑥ — Phase 3+

pub mod bond_handler;
pub mod character_handler;
