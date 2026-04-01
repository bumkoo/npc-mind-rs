//! 인메모리 MindRepository 구현체
//!
//! Mind Studio에서 저장한 scenario.json을 로드하거나,
//! 프로그래밍 방식으로 NPC/관계를 등록하여 바로 사용할 수 있습니다.
//!
//! # 사용 예시
//!
//! ```rust,ignore
//! // Mind Studio JSON 로드
//! let repo = InMemoryRepository::from_file("data/scenario.json")?;
//! let mut service = FormattedMindService::new(repo, "ko")?;
//!
//! // 프로그래밍 방식
//! let mut repo = InMemoryRepository::new();
//! repo.add_npc(npc);
//! repo.add_relationship(rel);
//! ```

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::application::dto::SceneFocusInput;
use crate::domain::emotion::{EmotionState, Scene, SceneFocus};
use crate::domain::personality::{Npc, NpcBuilder, Score};
use crate::domain::relationship::{Relationship, RelationshipBuilder};
use crate::ports::{EmotionStore, NpcWorld, SceneStore};

// ---------------------------------------------------------------------------
// 에러 타입
// ---------------------------------------------------------------------------

/// Repository 로드 에러
#[derive(Debug, thiserror::Error)]
pub enum RepositoryLoadError {
    #[error("JSON 파싱 실패: {0}")]
    ParseError(String),
    #[error("파일 읽기 실패: {0}")]
    IoError(String),
    #[error("데이터 변환 실패: {0}")]
    ConversionError(String),
}

// ---------------------------------------------------------------------------
// JSON serde 구조체 (내부 전용)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ScenarioJson {
    #[serde(default)]
    npcs: HashMap<String, NpcJson>,
    #[serde(default)]
    relationships: HashMap<String, RelationshipJson>,
    #[serde(default)]
    objects: HashMap<String, ObjectJson>,
    #[serde(default)]
    scenario: ScenarioMeta,
    #[serde(default)]
    scene: Option<SceneJson>,
    #[serde(default)]
    turn_history: Vec<serde_json::Value>,
    // current_situation: 파싱만 하고 무시
    #[serde(default)]
    #[allow(dead_code)]
    current_situation: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct NpcJson {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    // HEXACO 24 facets
    #[serde(default)]
    sincerity: f32,
    #[serde(default)]
    fairness: f32,
    #[serde(default)]
    greed_avoidance: f32,
    #[serde(default)]
    modesty: f32,
    #[serde(default)]
    fearfulness: f32,
    #[serde(default)]
    anxiety: f32,
    #[serde(default)]
    dependence: f32,
    #[serde(default)]
    sentimentality: f32,
    #[serde(default)]
    social_self_esteem: f32,
    #[serde(default)]
    social_boldness: f32,
    #[serde(default)]
    sociability: f32,
    #[serde(default)]
    liveliness: f32,
    #[serde(default)]
    forgiveness: f32,
    #[serde(default)]
    gentleness: f32,
    #[serde(default)]
    flexibility: f32,
    #[serde(default)]
    patience: f32,
    #[serde(default)]
    organization: f32,
    #[serde(default)]
    diligence: f32,
    #[serde(default)]
    perfectionism: f32,
    #[serde(default)]
    prudence: f32,
    #[serde(default)]
    aesthetic_appreciation: f32,
    #[serde(default)]
    inquisitiveness: f32,
    #[serde(default)]
    creativity: f32,
    #[serde(default)]
    unconventionality: f32,
}

impl NpcJson {
    fn to_npc(&self) -> Npc {
        let s = Score::clamped;
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

#[derive(Deserialize)]
struct RelationshipJson {
    owner_id: String,
    target_id: String,
    #[serde(default)]
    closeness: f32,
    #[serde(default)]
    trust: f32,
    #[serde(default)]
    power: f32,
}

impl RelationshipJson {
    fn to_relationship(&self) -> Relationship {
        RelationshipBuilder::new(&self.owner_id, &self.target_id)
            .closeness(Score::clamped(self.closeness))
            .trust(Score::clamped(self.trust))
            .power(Score::clamped(self.power))
            .build()
    }
}

#[derive(Deserialize)]
struct ObjectJson {
    #[serde(default)]
    description: String,
    #[allow(dead_code)]
    #[serde(default)]
    category: Option<String>,
}

#[derive(Deserialize, Default)]
struct ScenarioMeta {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    #[allow(dead_code)]
    notes: Vec<String>,
}

#[derive(Deserialize)]
struct SceneJson {
    npc_id: String,
    partner_id: String,
    #[allow(dead_code)]
    #[serde(default)]
    description: String,
    #[serde(default)]
    focuses: Vec<SceneFocusInput>,
}

// ---------------------------------------------------------------------------
// InMemoryRepository
// ---------------------------------------------------------------------------

/// 인메모리 MindRepository 구현체
///
/// Mind Studio에서 저장한 scenario.json을 로드하거나,
/// 프로그래밍 방식으로 NPC/관계를 등록하여 사용합니다.
///
/// 3개의 포트 트레이트(`NpcWorld`, `EmotionStore`, `SceneStore`)를
/// 모두 구현하므로 `MindRepository`가 자동으로 파생됩니다.
pub struct InMemoryRepository {
    npcs: HashMap<String, Npc>,
    relationships: HashMap<String, Relationship>,
    objects: HashMap<String, String>,
    emotions: HashMap<String, EmotionState>,
    scene: Option<Scene>,
    // 메타데이터
    scenario_name: String,
    scenario_description: String,
    turn_history: Vec<serde_json::Value>,
}

impl InMemoryRepository {
    /// 빈 Repository를 생성합니다.
    pub fn new() -> Self {
        Self {
            npcs: HashMap::new(),
            relationships: HashMap::new(),
            objects: HashMap::new(),
            emotions: HashMap::new(),
            scene: None,
            scenario_name: String::new(),
            scenario_description: String::new(),
            turn_history: Vec::new(),
        }
    }

    /// JSON 문자열에서 로드합니다 (Mind Studio scenario.json 호환).
    pub fn from_json(json: &str) -> Result<Self, RepositoryLoadError> {
        let data: ScenarioJson = serde_json::from_str(json)
            .map_err(|e| RepositoryLoadError::ParseError(e.to_string()))?;

        let mut repo = Self::new();

        // 1. NPC 로드
        for npc_json in data.npcs.values() {
            repo.npcs.insert(npc_json.id.clone(), npc_json.to_npc());
        }

        // 2. 관계 로드
        for rel_json in data.relationships.values() {
            let rel = rel_json.to_relationship();
            let key = format!("{}:{}", rel_json.owner_id, rel_json.target_id);
            repo.relationships.insert(key, rel);
        }

        // 3. 오브젝트 로드
        for (id, obj) in &data.objects {
            repo.objects.insert(id.clone(), obj.description.clone());
        }

        // 4. 메타데이터
        repo.scenario_name = data.scenario.name;
        repo.scenario_description = data.scenario.description;
        repo.turn_history = data.turn_history;

        // 5. Scene 로드 (NPC/관계가 먼저 준비되어야 함)
        if let Some(scene_json) = data.scene {
            repo.load_scene_from_json(scene_json)?;
        }

        Ok(repo)
    }

    /// 파일에서 로드합니다 (Mind Studio scenario.json 호환).
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, RepositoryLoadError> {
        let json = std::fs::read_to_string(path.as_ref())
            .map_err(|e| RepositoryLoadError::IoError(e.to_string()))?;
        Self::from_json(&json)
    }

    /// Scene JSON을 도메인 객체로 변환하여 저장합니다.
    fn load_scene_from_json(&mut self, scene_json: SceneJson) -> Result<(), RepositoryLoadError> {
        let npc_id = &scene_json.npc_id;
        let partner_id = &scene_json.partner_id;

        let focuses: Vec<SceneFocus> = scene_json
            .focuses
            .iter()
            .map(|f| f.to_domain(self, npc_id, partner_id))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| RepositoryLoadError::ConversionError(e.to_string()))?;

        let mut scene = Scene::new(npc_id.clone(), partner_id.clone(), focuses);

        // Initial focus 설정
        if let Some(initial) = scene.initial_focus() {
            let id = initial.id.clone();
            scene.set_active_focus(id);
        }

        self.scene = Some(scene);
        Ok(())
    }

    // --- 편의 메서드 ---

    /// NPC를 등록합니다.
    pub fn add_npc(&mut self, npc: Npc) {
        self.npcs.insert(npc.id().to_string(), npc);
    }

    /// 관계를 등록합니다.
    pub fn add_relationship(&mut self, rel: Relationship) {
        let key = format!("{}:{}", rel.owner_id(), rel.target_id());
        self.relationships.insert(key, rel);
    }

    /// 오브젝트를 등록합니다.
    pub fn add_object(&mut self, id: impl Into<String>, description: impl Into<String>) {
        self.objects.insert(id.into(), description.into());
    }

    // --- 메타데이터 접근자 ---

    /// 시나리오 이름을 반환합니다.
    pub fn scenario_name(&self) -> &str {
        &self.scenario_name
    }

    /// 시나리오 설명을 반환합니다.
    pub fn scenario_description(&self) -> &str {
        &self.scenario_description
    }

    /// 턴 히스토리를 반환합니다.
    pub fn turn_history(&self) -> &[serde_json::Value] {
        &self.turn_history
    }
}

impl Default for InMemoryRepository {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 포트 구현
// ---------------------------------------------------------------------------

impl NpcWorld for InMemoryRepository {
    fn get_npc(&self, id: &str) -> Option<Npc> {
        self.npcs.get(id).cloned()
    }

    fn get_relationship(&self, owner_id: &str, target_id: &str) -> Option<Relationship> {
        let key = format!("{owner_id}:{target_id}");
        self.relationships.get(&key).cloned().or_else(|| {
            let rev = format!("{target_id}:{owner_id}");
            self.relationships.get(&rev).cloned()
        })
    }

    fn get_object_description(&self, object_id: &str) -> Option<String> {
        self.objects.get(object_id).cloned()
    }

    fn save_relationship(&mut self, owner_id: &str, target_id: &str, rel: Relationship) {
        let key = format!("{owner_id}:{target_id}");
        // 기존 키 유지 (양방향 탐색)
        let existing_key = if self.relationships.contains_key(&key) {
            key
        } else {
            let rev = format!("{target_id}:{owner_id}");
            if self.relationships.contains_key(&rev) {
                rev
            } else {
                key
            }
        };
        self.relationships.insert(existing_key, rel);
    }
}

impl EmotionStore for InMemoryRepository {
    fn get_emotion_state(&self, npc_id: &str) -> Option<EmotionState> {
        self.emotions.get(npc_id).cloned()
    }

    fn save_emotion_state(&mut self, npc_id: &str, state: EmotionState) {
        self.emotions.insert(npc_id.to_string(), state);
    }

    fn clear_emotion_state(&mut self, npc_id: &str) {
        self.emotions.remove(npc_id);
    }
}

impl SceneStore for InMemoryRepository {
    fn get_scene(&self) -> Option<Scene> {
        self.scene.clone()
    }

    fn save_scene(&mut self, scene: Scene) {
        self.scene = Some(scene);
    }

    fn clear_scene(&mut self) {
        self.scene = None;
    }
}

// ---------------------------------------------------------------------------
// &mut InMemoryRepository 포트 구현 (MindService<&mut InMemoryRepository> 지원)
// ---------------------------------------------------------------------------

impl NpcWorld for &mut InMemoryRepository {
    fn get_npc(&self, id: &str) -> Option<Npc> {
        (**self).get_npc(id)
    }
    fn get_relationship(&self, owner_id: &str, target_id: &str) -> Option<Relationship> {
        (**self).get_relationship(owner_id, target_id)
    }
    fn get_object_description(&self, object_id: &str) -> Option<String> {
        (**self).get_object_description(object_id)
    }
    fn save_relationship(&mut self, owner_id: &str, target_id: &str, rel: Relationship) {
        (**self).save_relationship(owner_id, target_id, rel)
    }
}

impl EmotionStore for &mut InMemoryRepository {
    fn get_emotion_state(&self, npc_id: &str) -> Option<EmotionState> {
        (**self).get_emotion_state(npc_id)
    }
    fn save_emotion_state(&mut self, npc_id: &str, state: EmotionState) {
        (**self).save_emotion_state(npc_id, state)
    }
    fn clear_emotion_state(&mut self, npc_id: &str) {
        (**self).clear_emotion_state(npc_id)
    }
}

impl SceneStore for &mut InMemoryRepository {
    fn get_scene(&self) -> Option<Scene> {
        (**self).get_scene()
    }
    fn save_scene(&mut self, scene: Scene) {
        (**self).save_scene(scene)
    }
    fn clear_scene(&mut self) {
        (**self).clear_scene()
    }
}
