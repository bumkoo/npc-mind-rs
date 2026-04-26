use axum::Json;
use axum::extract::State;
use futures::StreamExt;
use npc_mind::StreamItem;
use npc_mind::application::dialogue_test_service::*;
use crate::state::*;
use crate::studio_service::StudioService;
use super::AppError;

/// POST /api/chat/start — 대화 세션 시작
pub async fn chat_start(
    State(state): State<AppState>,
    Json(req): Json<ChatStartRequest>,
) -> Result<Json<ChatStartResponse>, AppError> {
    let response = StudioService::perform_chat_start(&state, req).await?;
    Ok(Json(response))
}

/// POST /api/chat/message — 대사 전송
pub async fn chat_message(
    State(state): State<AppState>,
    Json(req): Json<ChatTurnRequest>,
) -> Result<Json<ChatTurnResponse>, AppError> {
    let response = StudioService::perform_chat_message(&state, req).await?;
    Ok(Json(response))
}

/// POST /api/chat/message/stream — 응답 스트리밍
pub async fn chat_message_stream(
    State(state): State<AppState>,
    Json(req): Json<ChatTurnRequest>,
) -> axum::response::Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    let stream = async_stream::stream! {
        let chat_state = match state.chat.as_ref() { Some(c) => c, None => { yield Ok(axum::response::sse::Event::default().event("error").data("chat feature가 비활성입니다.")); return; } };
        let mut token_stream = chat_state.send_message_stream(&req.session_id, &req.utterance);
        let mut chat_resp: Option<npc_mind::ChatResponse> = None;
        while let Some(item) = token_stream.next().await {
            match item {
                Ok(StreamItem::Token(t)) => yield Ok(axum::response::sse::Event::default().event("token").data(t)),
                Ok(StreamItem::Final(resp)) => { chat_resp = Some(resp); break; }
                Err(e) => { yield Ok(axum::response::sse::Event::default().event("error").data(e.to_string())); return; }
            }
        }
        let chat_resp = match chat_resp { Some(r) => r, None => { yield Ok(axum::response::sse::Event::default().event("error").data("스트림이 Final 없이 종료됨")); return; } };
        let npc_response = chat_resp.text;
        let timings = chat_resp.timings;

        let (stimulus, beat_changed) = match StudioService::process_chat_turn_result(&state, &req, npc_response.clone()).await {
            Ok(res) => res,
            Err(e) => { yield Ok(axum::response::sse::Event::default().event("error").data(e.to_string())); return; }
        };

        let final_response = ChatTurnResponse { npc_response, stimulus, beat_changed, timings };
        yield Ok(axum::response::sse::Event::default().event("done").data(serde_json::to_string(&final_response).unwrap_or_default()));
    };
    axum::response::Sse::new(stream)
}

/// POST /api/chat/end
pub async fn chat_end(
    State(state): State<AppState>,
    Json(req): Json<ChatEndRequest>,
) -> Result<Json<ChatEndResponse>, AppError> {
    let response = StudioService::perform_chat_end(&state, req).await?;
    Ok(Json(response))
}
