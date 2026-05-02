use crate::domain::guide::ActingGuide;
use crate::ports::GuideFormatter;
use serde::{Deserialize, Serialize};

/// 가이드 생성 요청
#[derive(Serialize, Deserialize, Clone)]
pub struct GuideRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub situation_description: Option<String>,
}

/// Guide 도메인 결과
pub struct GuideResult {
    pub guide: ActingGuide,
}

/// 가이드 재생성 응답 (포맷팅 완료)
#[derive(Serialize, Deserialize, Clone)]
pub struct GuideResponse {
    pub prompt: String,
    pub json: String,
}

impl super::CanFormat for GuideResult {
    type Response = GuideResponse;
    fn format(self, formatter: &dyn GuideFormatter) -> Self::Response {
        let prompt = formatter.format_prompt(&self.guide);
        let json = formatter.format_json(&self.guide).unwrap_or_default();
        GuideResponse { prompt, json }
    }
}
