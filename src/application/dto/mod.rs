use crate::ports::GuideFormatter;

pub mod emotion;
pub mod guide;
pub mod information;
pub mod relationship;
pub mod rumor;
pub mod scene;
pub mod world;

pub use emotion::*;
pub use guide::*;
pub use information::*;
pub use relationship::*;
pub use rumor::*;
pub use scene::*;
pub use world::*;

/// 포맷팅 가능한 도메인 결과 트레이트
pub trait CanFormat {
    /// 해당 결과의 포맷팅된 응답 타입
    type Response;
    /// GuideFormatter를 적용하여 Response로 변환
    fn format(self, formatter: &dyn GuideFormatter) -> Self::Response;
}

impl CanFormat for emotion::AppraiseResult {
    type Response = emotion::AppraiseResponse;
    fn format(self, formatter: &dyn GuideFormatter) -> Self::Response {
        emotion::AppraiseResponse {
            emotions: self.emotions,
            dominant: self.dominant,
            mood: self.mood,
            prompt: formatter.format_prompt(&self.guide),
            trace: self.trace,
        }
    }
}

impl CanFormat for emotion::StimulusResult {
    type Response = emotion::StimulusResponse;
    fn format(self, formatter: &dyn GuideFormatter) -> Self::Response {
        emotion::StimulusResponse {
            emotions: self.emotions,
            dominant: self.dominant,
            mood: self.mood,
            prompt: formatter.format_prompt(&self.guide),
            trace: self.trace,
            beat_changed: self.beat_changed,
            active_focus_id: self.active_focus_id,
            input_pad: self.input_pad,
        }
    }
}
