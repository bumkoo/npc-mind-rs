// wuxia-core/src/shared/event_macros.rs
//
// Domain Event name() Macro — 도메인 이벤트 name() 패턴 제거용 매크로.
//
// 5개 도메인(Character, Growth, Time, Memory, Relationship)의 이벤트 enum이
// 동일한 `name() -> &'static str` 패턴을 반복한다.
// 이 매크로로 보일러플레이트를 제거한다.
//
// # Usage
// ```ignore
// impl_event_name!(CharacterEvent {
//     Aged => "CharacterAged",
//     LifeStageChanged => "CharacterLifeStageChanged",
// });
// ```

/// 도메인 이벤트 enum에 `pub fn name(&self) -> &'static str` 메서드를 구현한다.
///
/// 각 variant를 `{ .. }` 패턴으로 매칭하므로, struct-like variant에 적용 가능.
macro_rules! impl_event_name {
    ($enum_name:ident { $($variant:ident => $name:expr),+ $(,)? }) => {
        impl $enum_name {
            /// 로깅/디버깅용 이벤트 이름.
            pub fn name(&self) -> &'static str {
                match self {
                    $(Self::$variant { .. } => $name,)+
                }
            }
        }
    };
}

pub(crate) use impl_event_name;
