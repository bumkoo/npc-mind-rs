//! Worldbuilding — 마크다운(SoT) → 도메인 변환 + `WorldRepository` 포트.
//!
//! Phase 1: Group 한정. 마크다운 frontmatter+H2 섹션 파서 + sync `WorldRepository`
//! trait. `SqliteWorldStore`는 `adapter/sqlite_world.rs` (embed feature).
//!
//! 장르 중립 — wuxia/판타지/SF 어휘 없음.

pub mod builder;
pub mod markdown;
pub mod mind_sync;
pub mod repository;

pub use repository::WorldRepository;
