//! API 핸들러 — CRUD + 파이프라인 실행

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use npc_mind::domain::emotion::*;
use npc_mind::domain::relationship::Relationship;
use npc_mind::presentation::korean::KoreanFormatter;
use npc_mind::ports::GuideFormatter;
use npc_mind::domain::guide::ActingGuide;

use crate::state::*;

// ---------------------------------------------------------------------------
// NPC CRUD
// ---------------------------------------------------------------------------

/// GET /api/npcs — 전체 NPC 목록
pub async fn list_npcs(State(state): State<AppState>) -> Json<Vec<NpcProfile>> {
    let inner = state.inner.read().await;
    let mut npcs: Vec<NpcProfile> = inner.npcs.values().cloned().collect();
    npcs.sort_by(|a, b| a.id.cmp(&b.id));
    Json(npcs)
}

/// POST /api/npcs — NPC 생성/업데이트
pub async fn upsert_npc(
    State(state): State<AppState>,
    Json(npc): Json<NpcProfile>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.npcs.insert(npc.id.clone(), npc);
    StatusCode::OK
}

/// DELETE /api/npcs/:id
pub async fn delete_npc(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.npcs.remove(&id);
    StatusCode::OK
}

// ---------------------------------------------------------------------------
// Relationship CRUD
// ---------------------------------------------------------------------------

/// GET /api/relationships
pub async fn list_relationships(State(state): State<AppState>) -> Json<Vec<RelationshipData>> {
    let inner = state.inner.read().await;
    let mut rels: Vec<RelationshipData> = inner.relationships.values().cloned().collect();
    rels.sort_by(|a, b| a.key().cmp(&b.key()));
    Json(rels)
}

/// POST /api/relationships
pub async fn upsert_relationship(
    State(state): State<AppState>,
    Json(rel): Json<RelationshipData>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    let key = rel.key();
    inner.relationships.insert(key, rel);
    StatusCode::OK
}

/// DELETE /api/relationships/:owner_id/:target_id
pub async fn delete_relationship(
    State(state): State<AppState>,
    axum::extract::Path((owner, target)): axum::extract::Path<(String, String)>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    let key = format!("{owner}:{target}");
    inner.relationships.remove(&key);
    StatusCode::OK
}

// ---------------------------------------------------------------------------
// Object CRUD
// ---------------------------------------------------------------------------

/// GET /api/objects
pub async fn list_objects(State(state): State<AppState>) -> Json<Vec<ObjectEntry>> {
    let inner = state.inner.read().await;
    let mut objs: Vec<ObjectEntry> = inner.objects.values().cloned().collect();
    objs.sort_by(|a, b| a.id.cmp(&b.id));
    Json(objs)
}

/// POST /api/objects
pub async fn upsert_object(
    State(state): State<AppState>,
    Json(obj): Json<ObjectEntry>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.objects.insert(obj.id.clone(), obj);
    StatusCode::OK
}

/// DELETE /api/objects/:id
pub async fn delete_object(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    let mut inner = state.inner.write().await;
    inner.objects.remove(&id);
    StatusCode::OK
}

// ---------------------------------------------------------------------------
// 파이프라인: 감정 평가
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct AppraiseRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub situation: SituationInput,
}

#[derive(Deserialize)]
pub struct SituationInput {
    pub description: String,
    pub event: Option<EventInput>,
    pub action: Option<ActionInput>,
    pub object: Option<ObjectInput>,
}

#[derive(Deserialize)]
pub struct EventInput {
    pub description: String,
    pub desirability_for_self: f32,
    pub other: Option<EventOtherInput>,
    pub prospect: Option<String>, // "anticipation", "hope_fulfilled", etc.
}

#[derive(Deserialize)]
pub struct EventOtherInput {
    pub target_id: String,
    pub desirability: f32,
}

#[derive(Deserialize)]
pub struct ActionInput {
    pub description: String,
    pub agent_id: Option<String>, // None=자기, Some=타인
    pub praiseworthiness: f32,
}

#[derive(Deserialize)]
pub struct ObjectInput {
    pub target_id: String,
    pub appealingness: f32,
}

#[derive(Serialize)]
pub struct AppraiseResponse {
    pub emotions: Vec<EmotionOutput>,
    pub dominant: Option<EmotionOutput>,
    pub mood: f32,
    pub prompt: String,
    pub trace: Vec<String>,
}

#[derive(Serialize)]
pub struct EmotionOutput {
    pub emotion_type: String,
    pub intensity: f32,
    pub context: Option<String>,
}

use crate::trace_collector::AppraisalCollector;

/// POST /api/appraise — 감정 평가 실행
pub async fn appraise(
    State(state): State<AppState>,
    Json(req): Json<AppraiseRequest>,
) -> Result<Json<AppraiseResponse>, (StatusCode, String)> {
    let inner = state.inner.read().await;

    // 1. NPC 조회
    let npc_profile = inner.npcs.get(&req.npc_id)
        .ok_or((StatusCode::NOT_FOUND, format!("NPC '{}' not found", req.npc_id)))?;
    let npc = npc_profile.to_npc();

    // 2. 대화 상대 관계 조회
    let rel_data = inner.find_relationship(&req.npc_id, &req.partner_id)
        .ok_or((StatusCode::NOT_FOUND,
            format!("Relationship '{}↔{}' not found", req.npc_id, req.partner_id)))?;
    let relationship = rel_data.to_relationship();

    // 3. Situation 구성
    let situation = build_situation(&req.situation, &inner, &req.npc_id, &req.partner_id)?;

    // 4. trace 수집 + 감정 평가
    let collector = state.collector.clone();
    collector.take_entries();
    let emotion_state = AppraisalEngine::appraise(npc.personality(), &situation, &relationship);
    let trace = collector.take_entries();

    // 5. 가이드 + 프롬프트 생성
    let guide = ActingGuide::build(&npc, &emotion_state, Some(situation.description.clone()), Some(&relationship));
    let formatter = KoreanFormatter::new();
    let prompt = formatter.format_prompt(&guide);

    // 6. 응답 구성 (emotion_state 이동 전에 모든 값 추출)
    let emotions: Vec<EmotionOutput> = emotion_state.emotions().iter()
        .map(|e| EmotionOutput {
            emotion_type: format!("{:?}", e.emotion_type()),
            intensity: e.intensity(),
            context: e.context().map(|s| s.to_string()),
        })
        .collect();

    let dominant = emotion_state.dominant().map(|e| EmotionOutput {
        emotion_type: format!("{:?}", e.emotion_type()),
        intensity: e.intensity(),
        context: e.context().map(|s| s.to_string()),
    });

    let mood = emotion_state.overall_valence();

    // 7. 감정 상태 저장
    drop(inner);
    {
        let mut inner = state.inner.write().await;
        inner.emotions.insert(req.npc_id.clone(), emotion_state);
    }

    Ok(Json(AppraiseResponse {
        emotions,
        dominant,
        mood,
        prompt,
        trace,
    }))
}

// ---------------------------------------------------------------------------
// Situation 빌드 헬퍼
// ---------------------------------------------------------------------------

fn build_situation(
    input: &SituationInput,
    state: &StateInner,
    npc_id: &str,
    partner_id: &str,
) -> Result<Situation, (StatusCode, String)> {
    // Event
    let event = if let Some(ref e) = input.event {
        let other = if let Some(ref o) = e.other {
            let rel_data = state.find_relationship(npc_id, &o.target_id)
                .ok_or((StatusCode::NOT_FOUND,
                    format!("Relationship '{}↔{}' not found", npc_id, o.target_id)))?;
            Some(DesirabilityForOther {
                target_id: o.target_id.clone(),
                desirability: o.desirability,
                relationship: rel_data.to_relationship(),
            })
        } else {
            None
        };

        let prospect = e.prospect.as_deref().and_then(|p| match p {
            "anticipation" => Some(Prospect::Anticipation),
            "hope_fulfilled" => Some(Prospect::Confirmation(ProspectResult::HopeFulfilled)),
            "hope_unfulfilled" => Some(Prospect::Confirmation(ProspectResult::HopeUnfulfilled)),
            "fear_unrealized" => Some(Prospect::Confirmation(ProspectResult::FearUnrealized)),
            "fear_confirmed" => Some(Prospect::Confirmation(ProspectResult::FearConfirmed)),
            _ => None,
        });

        Some(EventFocus {
            description: e.description.clone(),
            desirability_for_self: e.desirability_for_self,
            desirability_for_other: other,
            prospect,
        })
    } else {
        None
    };

    // Action — agent_id가 대화 상대면 None, 제3자면 관계 조회
    let action = if let Some(ref a) = input.action {
        let relationship = match &a.agent_id {
            Some(agent) if agent != partner_id => {
                // 제3자 → 관계 조회
                state.find_relationship(npc_id, agent)
                    .map(|r| r.to_relationship())
            }
            _ => None, // 자기 또는 대화 상대
        };
        Some(ActionFocus {
            description: a.description.clone(),
            agent_id: a.agent_id.clone(),
            praiseworthiness: a.praiseworthiness,
            relationship,
        })
    } else {
        None
    };

    // Object — 오브젝트 레지스트리에서 description 조회
    let object = if let Some(ref o) = input.object {
        let description = state.objects.get(&o.target_id)
            .map(|obj| obj.description.clone())
            .unwrap_or_else(|| o.target_id.clone());
        Some(ObjectFocus {
            target_id: o.target_id.clone(),
            target_description: description,
            appealingness: o.appealingness,
        })
    } else {
        None
    };

    Situation::new(input.description.clone(), event, action, object)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

// ---------------------------------------------------------------------------
// 저장/로드
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SaveRequest {
    pub path: String,
}

/// POST /api/save — JSON 파일로 저장
pub async fn save_state(
    State(state): State<AppState>,
    Json(req): Json<SaveRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let inner = state.inner.read().await;
    inner.save_to_file(std::path::Path::new(&req.path))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::OK)
}

/// POST /api/load — JSON 파일에서 로드
pub async fn load_state(
    State(state): State<AppState>,
    Json(req): Json<SaveRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let loaded = StateInner::load_from_file(std::path::Path::new(&req.path))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let mut inner = state.inner.write().await;
    *inner = loaded;
    Ok(StatusCode::OK)
}
