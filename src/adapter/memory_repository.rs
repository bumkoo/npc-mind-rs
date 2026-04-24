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
use crate::domain::scene_id::SceneId;
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
    /// Mind Studio UI 상태 보존용 — 로드 시 파싱하지만 엔진에서는 미사용
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
    /// Mind Studio JSON 스키마 호환용 — 향후 오브젝트 분류에 활용 예정
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
    /// 시나리오 메모 — JSON 저장/로드 호환용, 엔진에서는 미사용
    #[serde(default)]
    #[allow(dead_code)]
    notes: Vec<String>,
}

#[derive(Deserialize)]
struct SceneJson {
    npc_id: String,
    partner_id: String,
    /// Scene 설명 — JSON 저장/로드 호환용, 엔진에서는 미사용
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
    /// B4 Session 2: 다중 Scene 지원을 위해 `Option<Scene>` → `HashMap<SceneId, Scene>` 전환.
    /// `SceneStore::get_scene()` (단수)는 `last_scene_id`가 가리키는 Scene을 반환하여
    /// 단일 Scene 기존 테스트를 보존.
    scenes: HashMap<SceneId, Scene>,
    /// 마지막으로 `save_scene`/`save_scene_for` 호출된 Scene의 id.
    /// `SceneStore::get_scene()` 호출 시 반환할 "현재 Scene"을 결정.
    last_scene_id: Option<SceneId>,
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
            scenes: HashMap::new(),
            last_scene_id: None,
            scenario_name: String::new(),
            scenario_description: String::new(),
            turn_history: Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // B4 Session 2: 다중 Scene 접근 (inherent 메서드 — SceneStore trait은 미변경)
    // -----------------------------------------------------------------------

    /// SceneId로 특정 Scene 조회
    pub fn get_scene_by_id(&self, scene_id: &SceneId) -> Option<Scene> {
        self.scenes.get(scene_id).cloned()
    }

    /// 현재 활성 Scene 목록 (등록된 모든 Scene의 id)
    pub fn list_scene_ids(&self) -> Vec<SceneId> {
        self.scenes.keys().cloned().collect()
    }

    /// 특정 Scene 제거. `last_scene_id`가 이 Scene이었다면 다른 Scene으로 이동 또는 None.
    pub fn clear_scene_by_id(&mut self, scene_id: &SceneId) {
        self.scenes.remove(scene_id);
        if self.last_scene_id.as_ref() == Some(scene_id) {
            self.last_scene_id = self.scenes.keys().next().cloned();
        }
    }

    /// 주어진 Scene을 명시적으로 저장 (동일한 Scene id면 교체).
    /// `SceneStore::save_scene`와 동일 동작 — 명시적 호출 편의용.
    pub fn save_scene_for(&mut self, scene: Scene) {
        self.save_scene(scene);
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
            .map(|f| {
                let event_other_modifiers = f
                    .event
                    .as_ref()
                    .and_then(|e| e.other.as_ref())
                    .and_then(|o| self.get_relationship(npc_id, &o.target_id).map(|r| r.modifiers()));

                let action_agent_modifiers = f
                    .action
                    .as_ref()
                    .and_then(|a| a.agent_id.as_ref())
                    .filter(|&agent| agent != partner_id && agent != npc_id)
                    .and_then(|agent| self.get_relationship(npc_id, agent).map(|r| r.modifiers()));

                let object_description = f
                    .object
                    .as_ref()
                    .and_then(|o| self.get_object_description(&o.target_id));

                f.to_domain(
                    event_other_modifiers,
                    action_agent_modifiers,
                    object_description,
                    npc_id,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| RepositoryLoadError::ConversionError(e.to_string()))?;

        let mut scene = Scene::new(npc_id.clone(), partner_id.clone(), focuses);

        // Initial focus 설정
        if let Some(initial) = scene.initial_focus() {
            let id = initial.id.clone();
            scene.set_active_focus(id);
        }

        self.save_scene(scene);
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
    /// 마지막으로 저장된 Scene 반환 (단일 Scene 테스트 호환용).
    /// 다중 Scene 환경에서 특정 Scene이 필요하면 `get_scene_by_id` 사용.
    fn get_scene(&self) -> Option<Scene> {
        self.last_scene_id
            .as_ref()
            .and_then(|id| self.scenes.get(id).cloned())
    }

    /// Scene을 `(npc_id, partner_id)` 키로 저장. 동일 키 Scene이 있으면 교체.
    /// `last_scene_id`를 이 Scene으로 갱신 — `get_scene()` 단수 조회의 기본 대상.
    fn save_scene(&mut self, scene: Scene) {
        let id = SceneId::from(&scene);
        self.scenes.insert(id.clone(), scene);
        self.last_scene_id = Some(id);
    }

    /// 모든 Scene을 제거 — 단일 Scene 의미론 유지 (`get_scene()` 이후 None).
    /// 특정 Scene만 제거하려면 `clear_scene_by_id` 사용.
    fn clear_scene(&mut self) {
        self.scenes.clear();
        self.last_scene_id = None;
    }

    /// B4 Session 3: 다중 Scene 직접 조회 — `last_scene_id`를 거치지 않고 HashMap에서
    /// scene_id 키로 바로 lookup. `StimulusPolicy`/`RelationshipPolicy`가 올바른 Scene을
    /// 식별하는 데 필수.
    fn get_scene_by_id(&self, scene_id: &SceneId) -> Option<Scene> {
        self.scenes.get(scene_id).cloned()
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
