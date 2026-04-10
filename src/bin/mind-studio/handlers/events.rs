//! SSE 엔드포인트 — 실시간 상태 변경 이벤트 스트림

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::state::AppState;

/// `GET /api/events` — 상태 변경 이벤트를 SSE로 스트리밍
pub async fn sse_events(
    State(state): State<AppState>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.event_tx.subscribe();

    let stream = async_stream::stream! {
        // 연결 성공 이벤트
        yield Ok(Event::default().event("connected").data("ok"));

        let mut broadcast = BroadcastStream::new(rx);
        while let Some(result) = broadcast.next().await {
            match result {
                Ok(event) => {
                    yield Ok(Event::default().event(event.name()).data("ok"));
                }
                Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(_)) => {
                    // 이벤트 누락 — 프론트엔드에 전체 동기화 요청
                    yield Ok(Event::default().event("resync").data("ok"));
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
