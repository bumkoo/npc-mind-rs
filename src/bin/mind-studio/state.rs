//! 웹 UI 서버 상태 — NPC, 관계, 오브젝트 레지스트리

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::trace_collector::AppraisalCollector;
use npc_mind::domain::emotion::EmotionState;
use npc_mind::domain::emotion::SceneFocus;
use npc_mind::domain::pad::PadAnalyzer;

/// 서버 공유 상태
#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<RwLock<StateInner>>,
    pub collector: AppraisalCollector,
    /// 대사 → PAD 분석기 (embed feature 활성 시에만 Some)
    pub analyzer: Option<Arc<Mutex<PadAnalyzer>>>,
    /// 연기 가이드 포맷터 (서버 시작 시 한 번 생성, 모든 핸들러에서 공유)
    pub formatter: Arc<dyn npc_mind::ports::GuideFormatter>,
}

impl AppState {
    pub fn new(collector: AppraisalCollector, analyzer: Option<PadAnalyzer>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(StateInner::default())),
            collector,
            analyzer: analyzer.map(|a| Arc::new(Mutex::new(a))),
            formatter: Arc::new(npc_mind::presentation::korean::KoreanFormatter::new()),
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
    /// 턴별 기록 (장면 설정 + 감정 평가 + 프롬프트)
    #[serde(default)]
    pub turn_history: Vec<TurnRecord>,
    /// 현재 상황 설정 패널 상태 (프론트엔드 폼 값 보존용)
    #[serde(default)]
    pub current_situation: Option<serde_json::Value>,
    /// Scene 정보 (시나리오 JSON에 저장됨)
    #[serde(default)]
    pub scene: Option<serde_json::Value>,
    /// Scene Focus 옵션 목록 (런타임 — 직렬화 제외)
    #[serde(skip)]
    pub scene_focuses: Vec<SceneFocus>,
    /// 현재 활성 Focus ID (런타임)
    #[serde(skip)]
    pub active_focus_id: Option<String>,
    /// 현재 Scene의 NPC ID (런타임)
    #[serde(skip)]
    pub scene_npc_id: Option<String>,
    /// 현재 Scene의 대화 상대 ID (런타임)
    #[serde(skip)]
    pub scene_partner_id: Option<String>,
}

/// 턴별 기록 — 장면 설정, 감정 결과, 프롬프트를 JSON으로 보존
#[derive(Clone, Serialize, Deserialize)]
pub struct TurnRecord {
    /// 턴 라벨 (예: "Turn 1: 유령 공포")
    pub label: String,
    /// 파이프라인 종류 ("appraise" | "stimulus" | "after_dialogue")
    pub action: String,
    /// 요청 파라미터 (SituationInput 등)
    pub request: serde_json::Value,
    /// 응답 결과 (감정, 프롬프트, trace 등)
    pub response: serde_json::Value,
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
    #[serde(default)]
    pub sincerity: f32,
    #[serde(default)]
    pub fairness: f32,
    #[serde(default)]
    pub greed_avoidance: f32,
    #[serde(default)]
    pub modesty: f32,
    // E: 정서성
    #[serde(default)]
    pub fearfulness: f32,
    #[serde(default)]
    pub anxiety: f32,
    #[serde(default)]
    pub dependence: f32,
    #[serde(default)]
    pub sentimentality: f32,
    // X: 외향성
    #[serde(default)]
    pub social_self_esteem: f32,
    #[serde(default)]
    pub social_boldness: f32,
    #[serde(default)]
    pub sociability: f32,
    #[serde(default)]
    pub liveliness: f32,
    // A: 원만성
    #[serde(default)]
    pub forgiveness: f32,
    #[serde(default)]
    pub gentleness: f32,
    #[serde(default)]
    pub flexibility: f32,
    #[serde(default)]
    pub patience: f32,
    // C: 성실성
    #[serde(default)]
    pub organization: f32,
    #[serde(default)]
    pub diligence: f32,
    #[serde(default)]
    pub perfectionism: f32,
    #[serde(default)]
    pub prudence: f32,
    // O: 경험개방성
    #[serde(default)]
    pub aesthetic_appreciation: f32,
    #[serde(default)]
    pub inquisitiveness: f32,
    #[serde(default)]
    pub creativity: f32,
    #[serde(default)]
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
        self.relationships
            .get(&key1)
            .or_else(|| self.relationships.get(&key2))
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
