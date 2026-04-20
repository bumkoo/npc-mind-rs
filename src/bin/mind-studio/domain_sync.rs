//! StateInner ↔ InMemoryRepository 양방향 동기화 유틸리티 (B5.2 2/3)
//!
//! Mind Studio의 v1 handler들이 v2 `dispatch_v2` 경로로 전환되면서,
//! `AppStateRepository<'a>` (lifetime-bound) 대신 owned `InMemoryRepository`가 필요해졌다.
//! 각 request마다 `StateInner` (UI 소스) → `InMemoryRepository` (도메인 작업용)로
//! snapshot을 만들고, dispatch 이후 역으로 동기화한다.
//!
//! ## 흐름
//! ```text
//! lock inner
//!   → snapshot_to_repo(&inner)    // read path
//!   → build CommandDispatcher + dispatch_v2
//!   → sync_from_repo(&repo, &mut inner)  // write path
//! unlock inner
//! ```
//!
//! ## 성능 메모
//! 매 request마다 npcs/relationships/emotions 전체를 clone. 일반 Mind Studio UI
//! 요청량(분당 수십 회) 기준 무시 가능. 병목이 되면 AppState 통합(B5.2 3/3)으로 해소.

use crate::state::{StateInner, RelationshipData};
use npc_mind::domain::emotion::Scene;
use npc_mind::ports::{EmotionStore, NpcWorld, SceneStore};
use npc_mind::InMemoryRepository;

/// `StateInner`를 읽어 도메인 작업용 `InMemoryRepository` snapshot을 만든다.
///
/// 포함: NPCs, Relationships, Emotions, Scene (scene_npc_id/partner_id/focuses/active).
/// 제외: turn_history, test_report, UI 메타 필드 등 도메인 로직이 쓰지 않는 필드.
pub fn snapshot_to_repo(inner: &StateInner) -> InMemoryRepository {
    let mut repo = InMemoryRepository::new();

    for profile in inner.npcs.values() {
        repo.add_npc(profile.to_npc());
    }
    for rel in inner.relationships.values() {
        repo.add_relationship(rel.to_relationship());
    }
    for (npc_id, state) in &inner.emotions {
        repo.save_emotion_state(npc_id, state.clone());
    }
    if let (Some(npc_id), Some(partner_id)) = (
        inner.scene_npc_id.as_ref(),
        inner.scene_partner_id.as_ref(),
    ) {
        let mut scene = Scene::new(
            npc_id.clone(),
            partner_id.clone(),
            inner.scene_focuses.clone(),
        );
        if let Some(ref id) = inner.active_focus_id {
            scene.set_active_focus(id.clone());
        }
        repo.save_scene(scene);
    }

    repo
}

/// `dispatch_v2` 이후 수정된 `InMemoryRepository` 상태를 `StateInner`로 역반영.
///
/// 동기화 대상: Relationships (수정된 값), Emotions, Scene (active focus).
/// NPC 프로필은 dispatch_v2 내부에서 변경되지 않으므로 sync 대상 아님.
pub fn sync_from_repo(repo: &InMemoryRepository, inner: &mut StateInner) {
    // Relationships — dispatcher가 save_relationship으로 갱신했을 수 있음.
    // 기존 inner.relationships의 key는 유지하면서 값을 도메인 기준으로 갱신.
    let existing_keys: Vec<(String, String, String)> = inner
        .relationships
        .iter()
        .map(|(k, r)| (k.clone(), r.owner_id.clone(), r.target_id.clone()))
        .collect();
    for (key, owner, target) in existing_keys {
        if let Some(rel) = repo.get_relationship(&owner, &target) {
            inner.relationships.insert(
                key,
                RelationshipData {
                    owner_id: owner,
                    target_id: target,
                    closeness: rel.closeness().value(),
                    trust: rel.trust().value(),
                    power: rel.power().value(),
                },
            );
        }
    }

    // Emotions — v2 write-back 결과를 그대로 반영.
    // repo.list_scene_ids와 달리 EmotionStore trait에는 list가 없으므로,
    // 기존 inner.emotions에 등록된 NPC + dispatch에서 새로 등장한 NPC 둘 다 커버.
    // → inner.emotions 전수 갱신 (존재하면 repo 값, 없으면 삭제 = 기존 맵 초기화 불필요).
    let existing_ids: Vec<String> = inner.emotions.keys().cloned().collect();
    for id in existing_ids {
        match repo.get_emotion_state(&id) {
            Some(state) => {
                inner.emotions.insert(id, state);
            }
            None => {
                inner.emotions.remove(&id);
            }
        }
    }
    // 신규 NPC 감정은 npcs 집합 전수 스캔
    let npc_ids: Vec<String> = inner.npcs.keys().cloned().collect();
    for id in npc_ids {
        if !inner.emotions.contains_key(&id) {
            if let Some(state) = repo.get_emotion_state(&id) {
                inner.emotions.insert(id, state);
            }
        }
    }

    // Scene — last_scene_id가 가리키는 현재 Scene 기준으로 UI 필드 동기화.
    match repo.get_scene() {
        Some(scene) => {
            inner.scene_npc_id = Some(scene.npc_id().to_string());
            inner.scene_partner_id = Some(scene.partner_id().to_string());
            inner.scene_focuses = scene.focuses().to_vec();
            inner.active_focus_id = scene.active_focus_id().map(|s| s.to_string());
        }
        None => {
            inner.scene_npc_id = None;
            inner.scene_partner_id = None;
            inner.scene_focuses.clear();
            inner.active_focus_id = None;
        }
    }
}

/// 1회용 v2 CommandDispatcher — snapshot된 repo + 기본 핸들러 등록.
///
/// 각 handler 호출마다 사용되며, 호출 끝나면 drop. EventStore/EventBus도 ephemeral.
/// UI는 handler의 반환값을 통해 결과를 받으므로 broadcast subscriber가 없어도 문제없음.
pub fn build_ephemeral_dispatcher(
    repo: InMemoryRepository,
) -> npc_mind::application::command::CommandDispatcher<InMemoryRepository> {
    use npc_mind::application::command::CommandDispatcher;
    use npc_mind::application::event_bus::EventBus;
    use npc_mind::application::event_store::{EventStore, InMemoryEventStore};
    use std::sync::Arc;

    let store: Arc<dyn EventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    CommandDispatcher::new(repo, store, bus).with_default_handlers()
}

// ---------------------------------------------------------------------------
// Dispatch Helpers — 각 Mind Studio handler 경로가 호출하는 고수준 wrapper
//
// 공통 패턴:
//   1. snapshot_to_repo (read)
//   2. build ephemeral dispatcher
//   3. dispatch_v2(cmd).await
//   4. HandlerShared + events에서 UI DTO 재구성
//   5. sync_from_repo (write)
// ---------------------------------------------------------------------------

use crate::handlers::AppError;
use npc_mind::application::command::dispatcher::{DispatchV2Error, DispatchV2Output};
use npc_mind::application::command::Command;
use npc_mind::application::dto::{
    build_appraise_result, build_emotion_fields, AfterDialogueResponse, AppraiseRequest,
    AppraiseResult, GuideRequest, GuideResult, PadOutput, RelationshipValues, SceneRequest,
    SceneResult, StimulusRequest, StimulusResult,
};
use npc_mind::domain::event::{EventKind, EventPayload};
use npc_mind::domain::guide::ActingGuide;
use npc_mind::domain::relationship::Relationship;

/// `Command::Appraise` dispatch — Mind Studio `perform_appraise` 등에서 사용.
///
/// 반환: 포맷팅 전 `AppraiseResult`. 호출자가 `result.format(formatter)`로 response 구성.
pub async fn dispatch_appraise(
    inner: &mut StateInner,
    req: AppraiseRequest,
) -> Result<AppraiseResult, AppError> {
    let repo = snapshot_to_repo(inner);
    let dispatcher = build_ephemeral_dispatcher(repo);

    let cmd = Command::Appraise {
        npc_id: req.npc_id.clone(),
        partner_id: req.partner_id.clone(),
        situation: req.situation,
    };
    let output = dispatcher.dispatch_v2(cmd).await?;

    let result = build_appraise_result_from_output(&output, &req.npc_id, &req.partner_id, &dispatcher)?;

    {
        let guard = dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }
    Ok(result)
}

/// `Command::ApplyStimulus` dispatch — Mind Studio `perform_stimulus`에서 사용.
pub async fn dispatch_stimulus(
    inner: &mut StateInner,
    req: StimulusRequest,
) -> Result<StimulusResult, AppError> {
    let repo = snapshot_to_repo(inner);
    let dispatcher = build_ephemeral_dispatcher(repo);

    let cmd = Command::ApplyStimulus {
        npc_id: req.npc_id.clone(),
        partner_id: req.partner_id.clone(),
        pleasure: req.pleasure,
        arousal: req.arousal,
        dominance: req.dominance,
        situation_description: req.situation_description,
    };
    let output = dispatcher.dispatch_v2(cmd).await?;

    let result = build_stimulus_result_from_output(
        &output,
        (req.pleasure, req.arousal, req.dominance),
        &dispatcher,
    )?;

    {
        let guard = dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }
    Ok(result)
}

/// `Command::EndDialogue` dispatch — `perform_after_dialogue`에서 사용.
///
/// v2의 EndDialogue는 관계 갱신 + 감정 clear + Scene clear를 모두 포함.
/// v1 `after_dialogue`와 동등한 side-effect 조합.
pub async fn dispatch_end_dialogue(
    inner: &mut StateInner,
    req: npc_mind::application::dto::AfterDialogueRequest,
) -> Result<AfterDialogueResponse, AppError> {
    let repo = snapshot_to_repo(inner);
    let dispatcher = build_ephemeral_dispatcher(repo);

    let cmd = Command::EndDialogue {
        npc_id: req.npc_id.clone(),
        partner_id: req.partner_id.clone(),
        significance: req.significance,
    };
    let output = dispatcher.dispatch_v2(cmd).await?;

    let response = build_after_dialogue_from_output(&output, &req.npc_id, &req.partner_id)?;

    {
        let guard = dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }
    Ok(response)
}

/// `Command::GenerateGuide` dispatch — `generate_guide` endpoint에서 사용.
pub async fn dispatch_generate_guide(
    inner: &mut StateInner,
    req: GuideRequest,
) -> Result<GuideResult, AppError> {
    let repo = snapshot_to_repo(inner);
    let dispatcher = build_ephemeral_dispatcher(repo);

    let cmd = Command::GenerateGuide {
        npc_id: req.npc_id.clone(),
        partner_id: req.partner_id.clone(),
        situation_description: req.situation_description,
    };
    let output = dispatcher.dispatch_v2(cmd).await?;

    let guide = output.shared.guide.clone().ok_or_else(|| {
        AppError::V2Dispatch(DispatchV2Error::InvalidSituation(
            "GuideAgent 실행 결과 없음".into(),
        ))
    })?;

    {
        let guard = dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }
    Ok(GuideResult { guide })
}

/// `Command::StartScene` dispatch — `perform_start_scene` 등에서 사용.
pub async fn dispatch_start_scene(
    inner: &mut StateInner,
    req: SceneRequest,
) -> Result<SceneResult, AppError> {
    let repo = snapshot_to_repo(inner);
    let dispatcher = build_ephemeral_dispatcher(repo);

    let cmd = Command::StartScene {
        npc_id: req.npc_id.clone(),
        partner_id: req.partner_id.clone(),
        significance: req.significance,
        focuses: req.focuses.clone(),
    };
    let output = dispatcher.dispatch_v2(cmd).await?;

    // active_focus_id / focus_count는 dispatch_v2 후 repo의 Scene에서 조회
    let (focus_count, active_focus_id) = {
        let guard = dispatcher.repository_guard();
        let scene = guard.get_scene();
        (
            scene.as_ref().map(|s| s.focuses().len()).unwrap_or(0),
            scene.and_then(|s| s.active_focus_id().map(|id| id.to_string())),
        )
    };

    // initial_appraise: output.events에 EmotionAppraised가 있으면 구성
    let initial_appraise = if output.shared.emotion_state.is_some() {
        Some(build_appraise_result_from_output(
            &output,
            &req.npc_id,
            &req.partner_id,
            &dispatcher,
        )?)
    } else {
        None
    };

    {
        let guard = dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }

    Ok(SceneResult {
        focus_count,
        initial_appraise,
        active_focus_id,
    })
}

// ---------------------------------------------------------------------------
// 내부 헬퍼: DispatchV2Output → DTO 재구성
// ---------------------------------------------------------------------------

fn build_appraise_result_from_output(
    output: &DispatchV2Output,
    npc_id: &str,
    partner_id: &str,
    dispatcher: &npc_mind::application::command::CommandDispatcher<InMemoryRepository>,
) -> Result<AppraiseResult, AppError> {
    let state = output.shared.emotion_state.as_ref().ok_or_else(|| {
        AppError::V2Dispatch(DispatchV2Error::InvalidSituation(
            "EmotionState 재구성 실패 (with_default_handlers 호출 여부 확인)".into(),
        ))
    })?;

    let guard = dispatcher.repository_guard();
    let npc = guard.get_npc(npc_id).ok_or_else(|| {
        AppError::V2Dispatch(DispatchV2Error::InvalidSituation(format!(
            "NPC {} not found",
            npc_id
        )))
    })?;
    let partner_name = guard
        .get_npc(partner_id)
        .map(|p| p.name().to_string())
        .unwrap_or_else(|| partner_id.to_string());
    let rel: Option<Relationship> = guard
        .get_relationship(npc_id, partner_id)
        .or_else(|| guard.get_relationship(partner_id, npc_id));
    drop(guard);

    let situation_desc = output.events.iter().find_map(|e| match &e.payload {
        EventPayload::EmotionAppraised {
            situation_description,
            ..
        } => situation_description.clone(),
        _ => None,
    });
    let effective_rel = output.shared.relationship.as_ref().or(rel.as_ref());

    Ok(build_appraise_result(
        &npc,
        state,
        situation_desc,
        effective_rel,
        &partner_name,
        vec![],
    ))
}

fn build_stimulus_result_from_output(
    output: &DispatchV2Output,
    input_pad: (f32, f32, f32),
    dispatcher: &npc_mind::application::command::CommandDispatcher<InMemoryRepository>,
) -> Result<StimulusResult, AppError> {
    let state = output.shared.emotion_state.as_ref().ok_or_else(|| {
        AppError::V2Dispatch(DispatchV2Error::InvalidSituation(
            "EmotionState 재구성 실패".into(),
        ))
    })?;
    let guide: ActingGuide = output.shared.guide.as_ref().cloned().ok_or_else(|| {
        AppError::V2Dispatch(DispatchV2Error::InvalidSituation(
            "ActingGuide 재구성 실패 (GuideAgent 등록 확인)".into(),
        ))
    })?;

    let (emotions, dominant, mood) = build_emotion_fields(state);
    let beat_changed = output
        .events
        .iter()
        .any(|e| matches!(e.kind(), EventKind::BeatTransitioned));

    let active_focus_id = dispatcher
        .repository_guard()
        .get_scene()
        .and_then(|s| s.active_focus_id().map(|id| id.to_string()));

    Ok(StimulusResult {
        emotions,
        dominant,
        mood,
        guide,
        trace: vec![],
        beat_changed,
        active_focus_id,
        input_pad: Some(PadOutput {
            pleasure: input_pad.0,
            arousal: input_pad.1,
            dominance: input_pad.2,
        }),
    })
}

/// EndDialogue 결과 events에서 **본 요청에 해당하는** RelationshipUpdated를 선택해 response 구성.
///
/// RelationshipAgent가 여러 관계를 업데이트하는 경우(현재 spec에선 1건이지만 확장 가능)
/// 본 요청의 (npc_id, partner_id)와 payload의 owner/target 쌍이 일치하는 이벤트만 선택.
/// 양방향 일치 허용 — Relationship 저장 방향은 구현 세부이므로 owner↔target 순서를 교체해도 매치.
fn build_after_dialogue_from_output(
    output: &DispatchV2Output,
    npc_id: &str,
    partner_id: &str,
) -> Result<AfterDialogueResponse, AppError> {
    output
        .events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::RelationshipUpdated {
                owner_id,
                target_id,
                before_closeness,
                before_trust,
                before_power,
                after_closeness,
                after_trust,
                after_power,
            } if (owner_id == npc_id && target_id == partner_id)
                || (owner_id == partner_id && target_id == npc_id) =>
            {
                Some(AfterDialogueResponse {
                    before: RelationshipValues {
                        closeness: *before_closeness,
                        trust: *before_trust,
                        power: *before_power,
                    },
                    after: RelationshipValues {
                        closeness: *after_closeness,
                        trust: *after_trust,
                        power: *after_power,
                    },
                })
            }
            _ => None,
        })
        .ok_or_else(|| {
            AppError::V2Dispatch(DispatchV2Error::InvalidSituation(format!(
                "RelationshipUpdated 이벤트 부재 ({}↔{})",
                npc_id, partner_id
            )))
        })
}
