//! 웹 UI 서버 상태 — NPC, 관계, 오브젝트 레지스트리

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use npc_mind::domain::emotion::EmotionState;
use crate::trace_collector::AppraisalCollector;

/// 서버 공유 상태
#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<RwLock<StateInner>>,
    pub collector: AppraisalCollector,
}

impl AppState {
    pub fn new(collector: AppraisalCollector) -> Self {
        Self {
            inner: Arc::new(RwLock::new(StateInner::default())),
            collector,
        }
    }
}

/// 내부 상태 (RwLock으로 보호)
#[derive(Default, Serialize, Deserialize)]
pub struct StateInner {
    /// NPC 프로필 레지스트리 (key: npc_id)
    pub npcs: HashMap<String, NpcProfile>,
    /// 관계 레지스트리 (key: "owner_id:target_id")
    pub relationships: HashMap<String, RelationshipData>,
    /// 오브젝트 레지스트리 (key: object_id)
    pub objects: HashMap<String, ObjectEntry>,
    /// 현재 감정 상태 (key: npc_id) — 직렬화 제외
    #[serde(skip)]
    pub emotions: HashMap<String, EmotionState>,
    /// 시나리오 메타데이터
    pub scenario: ScenarioMeta,
}

/// 시나리오 메타데이터
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct ScenarioMeta {
    pub name: String,
    pub description: String,
    /// 평가 노트 (Claude가 작성)
    pub notes: Vec<String>,
}

/// NPC 프로필 (HEXACO 24 facet + 메타)
#[derive(Clone, Serialize, Deserialize)]
pub struct NpcProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    // H: 정직-겸손성
    pub sincerity: f32,
    pub fairness: f32,
    pub greed_avoidance: f32,
    pub modesty: f32,
    // E: 정서성
    pub fearfulness: f32,
    pub anxiety: f32,
    pub dependence: f32,
    pub sentimentality: f32,
    // X: 외향성
    pub social_self_esteem: f32,
    pub social_boldness: f32,
    pub sociability: f32,
    pub liveliness: f32,
    // A: 원만성
    pub forgiveness: f32,
    pub gentleness: f32,
    pub flexibility: f32,
    pub patience: f32,
    // C: 성실성
    pub organization: f32,
    pub diligence: f32,
    pub perfectionism: f32,
    pub prudence: f32,
    // O: 경험개방성
    pub aesthetic_appreciation: f32,
    pub inquisitiveness: f32,
    pub creativity: f32,
    pub unconventionality: f32,
}

/// 관계 데이터
#[derive(Clone, Serialize, Deserialize)]
pub struct RelationshipData {
    pub owner_id: String,
    pub target_id: String,
    pub closeness: f32,
    pub trust: f32,
    pub power: f32,
}

impl RelationshipData {
    /// 레지스트리 키 생성 ("owner:target")
    pub fn key(&self) -> String {
        format!("{}:{}", self.owner_id, self.target_id)
    }
}

/// 오브젝트 등록 정보
#[derive(Clone, Serialize, Deserialize)]
pub struct ObjectEntry {
    pub id: String,
    pub description: String,
    /// 카테고리 (사물/장소/NPC특성 등 — 선택적)
    pub category: Option<String>,
}

// ---------------------------------------------------------------------------
// 도메인 변환
// ---------------------------------------------------------------------------

use npc_mind::domain::personality::{Npc, NpcBuilder, Score};
use npc_mind::domain::relationship::{Relationship, RelationshipBuilder};

impl NpcProfile {
    /// NPC 도메인 객체로 변환
    pub fn to_npc(&self) -> Npc {
        let s = |v: f32| Score::clamped(v);
        NpcBuilder::new(&self.id, &self.name)
            .description(&self.description)
            .honesty_humility(|h| {
                h.sincerity = s(self.sincerity);
                h.fairness = s(self.fairness);
                h.greed_avoidance = s(self.greed_avoidance);
                h.modesty = s(self.modesty);
            })
            .emotionality(|e| {
                e.fearfulness = s(self.fearfulness);
                e.anxiety = s(self.anxiety);
                e.dependence = s(self.dependence);
                e.sentimentality = s(self.sentimentality);
            })
            .extraversion(|x| {
                x.social_self_esteem = s(self.social_self_esteem);
                x.social_boldness = s(self.social_boldness);
                x.sociability = s(self.sociability);
                x.liveliness = s(self.liveliness);
            })
            .agreeableness(|a| {
                a.forgiveness = s(self.forgiveness);
                a.gentleness = s(self.gentleness);
                a.flexibility = s(self.flexibility);
                a.patience = s(self.patience);
            })
            .conscientiousness(|c| {
                c.organization = s(self.organization);
                c.diligence = s(self.diligence);
                c.perfectionism = s(self.perfectionism);
                c.prudence = s(self.prudence);
            })
            .openness(|o| {
                o.aesthetic_appreciation = s(self.aesthetic_appreciation);
                o.inquisitiveness = s(self.inquisitiveness);
                o.creativity = s(self.creativity);
                o.unconventionality = s(self.unconventionality);
            })
            .build()
    }
}

impl RelationshipData {
    /// Relationship 도메인 객체로 변환
    pub fn to_relationship(&self) -> Relationship {
        let s = |v: f32| Score::clamped(v);
        RelationshipBuilder::new(&self.owner_id, &self.target_id)
            .closeness(s(self.closeness))
            .trust(s(self.trust))
            .power(s(self.power))
            .build()
    }
}

impl StateInner {
    /// 관계 조회 (양방향 — owner:target 또는 target:owner)
    pub fn find_relationship(&self, id_a: &str, id_b: &str) -> Option<&RelationshipData> {
        let key1 = format!("{id_a}:{id_b}");
        let key2 = format!("{id_b}:{id_a}");
        self.relationships.get(&key1).or_else(|| self.relationships.get(&key2))
    }

    /// JSON 파일로 저장
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    /// JSON 파일에서 로드
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, String> {
        let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&json).map_err(|e| e.to_string())
    }
}
