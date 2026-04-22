//! StateInner ↔ shared CommandDispatcher 동기화 유틸리티 (B5.2 3/3)
//!
//! Mind Studio의 REST/MCP handler가 `AppState.shared_dispatcher`를 통해
//! v2 경로로 커맨드를 처리하는 고수준 wrapper 모음.
//!
//! ## 계약
//!
//! - **전제**: 호출 전에 공유 repo는 이미 fresh 상태. UI CRUD / scenario load
//!   경로가 `AppState::rebuild_repo_from_inner()`를 호출해 유지한다.
//! - **공유 dispatcher**: `state.shared_dispatcher.dispatch_v2(cmd).await`로
//!   dispatch. request 간 dispatcher·repo·EventStore·EventBus를 재사용하므로
//!   snapshot_to_repo/ephemeral dispatcher 없음.
//! - **사후 동기화**: dispatch 결과가 repo의 관계/감정/Scene을 변경했으므로
//!   `sync_from_repo(&repo, inner)`로 UI 뷰에 전파한다.
//!
//! ```text
//! (UI write) → rebuild_repo_from_inner → shared_repo 최신
//! (dispatch)  → state.shared_dispatcher.dispatch_v2 → shared_repo 변이
//!   → sync_from_repo(&shared_repo, inner) → UI 레이어 반영
//! ```

use crate::state::{AppState, RelationshipData, StateInner};
use npc_mind::ports::{EmotionStore, NpcWorld, SceneStore};
use npc_mind::InMemoryRepository;

use crate::handlers::AppError;
use npc_mind::application::command::dispatcher::{DispatchV2Error, DispatchV2Output};
use npc_mind::application::command::Command;
use npc_mind::application::dto::{
    build_appraise_result, build_emotion_fields, AfterDialogueResponse, AppraiseRequest,
    AppraiseResult, GuideRequest, GuideResult, PadOutput, RelationshipValues, SceneRequest,
    SceneResult, StimulusRequest, StimulusResult,
};
#[cfg(feature = "embed")]
use npc_mind::application::dto::{
    ApplyWorldEventRequest, SeedRumorRequest, SpreadRumorRequest, TellInformationRequest,
};
use npc_mind::domain::event::{EventKind, EventPayload};
use npc_mind::domain::guide::ActingGuide;
use npc_mind::domain::relationship::Relationship;

// ---------------------------------------------------------------------------
// sync_from_repo — dispatch 이후 repo → StateInner 반영
// ---------------------------------------------------------------------------

/// `dispatch_v2` 이후 수정된 공유 repo 상태를 `StateInner`로 반영.
///
/// 동기화 대상:
/// - Relationships (갱신된 closeness/trust/power)
/// - Emotions (전수 교체)
/// - Scene (active_focus 등)
///
/// NPC 프로필은 dispatch가 변경하지 않으므로 대상 아님.
pub fn sync_from_repo(repo: &InMemoryRepository, inner: &mut StateInner) {
    // Relationships — dispatcher가 save_relationship으로 갱신했을 수 있음.
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

    // Emotions — dispatch write-back 결과를 전수 반영.
    // 기존 inner.emotions 엔트리 + npcs 전수 스캔(신규 NPC 감정도 포함).
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
    let npc_ids: Vec<String> = inner.npcs.keys().cloned().collect();
    for id in npc_ids {
        if !inner.emotions.contains_key(&id) {
            if let Some(state) = repo.get_emotion_state(&id) {
                inner.emotions.insert(id, state);
            }
        }
    }

    // Scene — last_scene_id가 가리키는 현재 Scene을 UI 필드로 펼쳐 저장.
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

// ---------------------------------------------------------------------------
// Dispatch Helpers — Mind Studio handler 경로가 호출하는 고수준 wrapper
//
// 공통 패턴:
//   1. state.shared_dispatcher.dispatch_v2(cmd).await
//   2. HandlerShared + events에서 UI DTO 재구성
//   3. sync_from_repo (공유 repo → inner)
// ---------------------------------------------------------------------------

/// `Command::Appraise` dispatch.
pub async fn dispatch_appraise(
    state: &AppState,
    inner: &mut StateInner,
    req: AppraiseRequest,
) -> Result<AppraiseResult, AppError> {
    let cmd = Command::Appraise {
        npc_id: req.npc_id.clone(),
        partner_id: req.partner_id.clone(),
        situation: req.situation,
    };
    let output = state.shared_dispatcher.dispatch_v2(cmd).await?;

    let result = build_appraise_result_from_output(&output, &req.npc_id, &req.partner_id, state)?;

    {
        let guard = state.shared_dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }
    Ok(result)
}

/// `Command::ApplyStimulus` dispatch.
pub async fn dispatch_stimulus(
    state: &AppState,
    inner: &mut StateInner,
    req: StimulusRequest,
) -> Result<StimulusResult, AppError> {
    let cmd = Command::ApplyStimulus {
        npc_id: req.npc_id.clone(),
        partner_id: req.partner_id.clone(),
        pleasure: req.pleasure,
        arousal: req.arousal,
        dominance: req.dominance,
        situation_description: req.situation_description,
    };
    let output = state.shared_dispatcher.dispatch_v2(cmd).await?;

    let result =
        build_stimulus_result_from_output(&output, (req.pleasure, req.arousal, req.dominance), state)?;

    {
        let guard = state.shared_dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }
    Ok(result)
}

/// `Command::EndDialogue` dispatch — 관계 갱신 + 감정 clear + Scene clear.
pub async fn dispatch_end_dialogue(
    state: &AppState,
    inner: &mut StateInner,
    req: npc_mind::application::dto::AfterDialogueRequest,
) -> Result<AfterDialogueResponse, AppError> {
    let cmd = Command::EndDialogue {
        npc_id: req.npc_id.clone(),
        partner_id: req.partner_id.clone(),
        significance: req.significance,
    };
    let output = state.shared_dispatcher.dispatch_v2(cmd).await?;

    let response = build_after_dialogue_from_output(&output, &req.npc_id, &req.partner_id)?;

    {
        let guard = state.shared_dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }
    Ok(response)
}

/// `Command::GenerateGuide` dispatch.
pub async fn dispatch_generate_guide(
    state: &AppState,
    inner: &mut StateInner,
    req: GuideRequest,
) -> Result<GuideResult, AppError> {
    let cmd = Command::GenerateGuide {
        npc_id: req.npc_id.clone(),
        partner_id: req.partner_id.clone(),
        situation_description: req.situation_description,
    };
    let output = state.shared_dispatcher.dispatch_v2(cmd).await?;

    let guide = output.shared.guide.clone().ok_or_else(|| {
        AppError::V2Dispatch(DispatchV2Error::InvalidSituation(
            "GuideAgent 실행 결과 없음".into(),
        ))
    })?;

    {
        let guard = state.shared_dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }
    Ok(GuideResult { guide })
}

/// `Command::StartScene` dispatch.
pub async fn dispatch_start_scene(
    state: &AppState,
    inner: &mut StateInner,
    req: SceneRequest,
) -> Result<SceneResult, AppError> {
    let cmd = Command::StartScene {
        npc_id: req.npc_id.clone(),
        partner_id: req.partner_id.clone(),
        significance: req.significance,
        focuses: req.focuses.clone(),
    };
    let output = state.shared_dispatcher.dispatch_v2(cmd).await?;

    let (focus_count, active_focus_id) = {
        let guard = state.shared_dispatcher.repository_guard();
        let scene = guard.get_scene();
        (
            scene.as_ref().map(|s| s.focuses().len()).unwrap_or(0),
            scene.and_then(|s| s.active_focus_id().map(|id| id.to_string())),
        )
    };

    let initial_appraise = if output.shared.emotion_state.is_some() {
        Some(build_appraise_result_from_output(
            &output,
            &req.npc_id,
            &req.partner_id,
            state,
        )?)
    } else {
        None
    };

    {
        let guard = state.shared_dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }

    Ok(SceneResult {
        focus_count,
        initial_appraise,
        active_focus_id,
    })
}

// ---------------------------------------------------------------------------
// Step E1 — Memory / Rumor / World dispatch 헬퍼
//
// 모두 `DispatchV2Output` 원본을 그대로 반환한다. 이벤트 결과를 핸들러가 직접
// 검사해 SSE 방출/응답을 구성하도록 설계했다 (C2/C3 DTO response가 현재 미사용이라
// typed facade는 범위 외).
//
// 헬퍼 4종은 `embed` feature 활성 시에만 사용되는 REST 핸들러 전용이라 cfg-gate로
// dead-code 경고를 억제한다.
// ---------------------------------------------------------------------------

/// `Command::TellInformation` dispatch.
///
/// `RelationshipMemoryHandler`가 cause를 근거로 관계 갱신을 연쇄 발행할 수 있으므로
/// `sync_from_repo`로 UI 레이어를 재동기화한다.
#[cfg(feature = "embed")]
pub async fn dispatch_tell_information(
    state: &AppState,
    inner: &mut StateInner,
    req: TellInformationRequest,
) -> Result<DispatchV2Output, AppError> {
    let output = state
        .shared_dispatcher
        .dispatch_v2(Command::TellInformation(req))
        .await?;
    {
        let guard = state.shared_dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }
    Ok(output)
}

/// `Command::ApplyWorldEvent` dispatch.
#[cfg(feature = "embed")]
pub async fn dispatch_apply_world_event(
    state: &AppState,
    inner: &mut StateInner,
    req: ApplyWorldEventRequest,
) -> Result<DispatchV2Output, AppError> {
    let output = state
        .shared_dispatcher
        .dispatch_v2(Command::ApplyWorldEvent(req))
        .await?;
    {
        let guard = state.shared_dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }
    Ok(output)
}

/// `Command::SeedRumor` dispatch.
#[cfg(feature = "embed")]
pub async fn dispatch_seed_rumor(
    state: &AppState,
    inner: &mut StateInner,
    req: SeedRumorRequest,
) -> Result<DispatchV2Output, AppError> {
    let output = state
        .shared_dispatcher
        .dispatch_v2(Command::SeedRumor(req))
        .await?;
    {
        let guard = state.shared_dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }
    Ok(output)
}

/// `Command::SpreadRumor` dispatch.
#[cfg(feature = "embed")]
pub async fn dispatch_spread_rumor(
    state: &AppState,
    inner: &mut StateInner,
    req: SpreadRumorRequest,
) -> Result<DispatchV2Output, AppError> {
    let output = state
        .shared_dispatcher
        .dispatch_v2(Command::SpreadRumor(req))
        .await?;
    {
        let guard = state.shared_dispatcher.repository_guard();
        sync_from_repo(&*guard, inner);
    }
    Ok(output)
}

// ---------------------------------------------------------------------------
// 내부 헬퍼: DispatchV2Output → DTO 재구성
// ---------------------------------------------------------------------------

fn build_appraise_result_from_output(
    output: &DispatchV2Output,
    npc_id: &str,
    partner_id: &str,
    state: &AppState,
) -> Result<AppraiseResult, AppError> {
    let emotion_state = output.shared.emotion_state.as_ref().ok_or_else(|| {
        AppError::V2Dispatch(DispatchV2Error::InvalidSituation(
            "EmotionState 재구성 실패 (with_default_handlers 호출 여부 확인)".into(),
        ))
    })?;

    let guard = state.shared_dispatcher.repository_guard();
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
        emotion_state,
        situation_desc,
        effective_rel,
        &partner_name,
        vec![],
    ))
}

fn build_stimulus_result_from_output(
    output: &DispatchV2Output,
    input_pad: (f32, f32, f32),
    state: &AppState,
) -> Result<StimulusResult, AppError> {
    let emotion_state = output.shared.emotion_state.as_ref().ok_or_else(|| {
        AppError::V2Dispatch(DispatchV2Error::InvalidSituation(
            "EmotionState 재구성 실패".into(),
        ))
    })?;
    let guide: ActingGuide = output.shared.guide.as_ref().cloned().ok_or_else(|| {
        AppError::V2Dispatch(DispatchV2Error::InvalidSituation(
            "ActingGuide 재구성 실패 (GuideAgent 등록 확인)".into(),
        ))
    })?;

    let (emotions, dominant, mood) = build_emotion_fields(emotion_state);
    let beat_changed = output
        .events
        .iter()
        .any(|e| matches!(e.kind(), EventKind::BeatTransitioned));

    let active_focus_id = state
        .shared_dispatcher
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

/// EndDialogue 결과 events에서 본 요청에 해당하는 RelationshipUpdated를 선택.
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
                ..
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
