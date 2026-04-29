// wuxia-core/src/time/mod.rs
//
// Time Domain — "지금은 언제인가?"
//
// The time domain is the world's heartbeat.
// It owns the GameClock, produces time-related events, and
// defines TimeEvent (the domain's own event enum).
//
// Important distinction:
//   shared/time.rs → GameTime, Season (data types, shared by all domains)
//   time/event.rs  → TimeEvent (도메인 이벤트)
//   time/clock.rs  → GameClock (the domain logic that advances time)

pub mod clock;
pub mod event;

pub use clock::GameClock;
pub use event::TimeEvent;
