//! 마크다운(SoT) → 도메인 변환.
//!
//! - `frontmatter` — `---` 펜스로 둘러싸인 YAML 블록 추출.
//! - `group` — Group 마크다운 한 파일을 `Group` 애그리거트로 변환.
//!
//! H2 섹션 파싱은 line-based 단순 분할로 처리 (pulldown-cmark 미사용).

pub mod frontmatter;
pub mod group;
pub mod person;
pub mod place;

pub use frontmatter::{
    Frontmatter, FrontmatterError, parse_frontmatter, parse_h2_sections,
};
pub use group::{GroupMarkdownError, group_from_markdown};
pub use person::{PersonMarkdownError, person_from_markdown};
pub use place::{PlaceMarkdownError, place_from_markdown};
