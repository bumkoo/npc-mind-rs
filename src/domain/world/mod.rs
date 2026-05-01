//! Worldbuilding 도메인 — 9 인스턴스 도메인 + Atlas 관계 도메인.
//!
//! Phase 1에서 **Group**만 구현. 나머지 8개 (Person·Place·Item·Skill·Knowledge·Lore·
//! Event·Era)는 후속 Phase에서 채워지는 빈 자리. Atlas는 Phase 4 별도 모듈.
//!
//! 장르 중립 원칙 — 이 모듈에는 wuxia/판타지/SF 어떤 장르 어휘도 들어가지 않는다.
//! 장르 특화 어휘(황실·문파·결사 등)는 `genres/<name>/`·`projects/<name>/`에만.

pub mod era;
pub mod event;
pub mod group;
pub mod item;
pub mod knowledge;
pub mod lore;
pub mod person;
pub mod place;
pub mod skill;

pub use group::{
    Group, GroupFilter, GroupId, GroupStatus, MemberRef, Temporal, WorldError,
    detect_parent_group_cycle,
};
pub use person::{HexacoSix, Person, PersonFilter, PersonId, PersonStatus, PersonTemporal};
