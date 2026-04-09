//! llama-server timings 캡처를 위한 HTTP 클라이언트 래퍼
//!
//! rig-core의 `HttpClientExt` 트레이트를 구현하는 래퍼로,
//! llama-server의 `/v1/chat/completions` 응답에 포함된 `timings` 객체를
//! 가로채서 저장한 뒤, 원본 응답은 rig에게 그대로 전달한다.
//!
//! rig 소스를 수정하지 않고 `ClientBuilder.http_client()`를 통해 주입한다.

use crate::ports::LlamaTimings;
use bytes::Bytes;
use futures::StreamExt;
use rig::http_client::{
    self, HttpClientExt, LazyBody, MultipartForm, Request, Response, StreamingResponse,
};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

type BoxedStream =
    Pin<Box<dyn rig::wasm_compat::WasmCompatSendStream<InnerItem = http_client::Result<Bytes>>>>;

/// llama-server 응답에서 `timings` 필드를 파싱하기 위한 envelope
#[derive(serde::Deserialize)]
struct TimingsEnvelope {
    timings: Option<LlamaTimings>,
}

/// `reqwest::Client`를 감싸며, HTTP 응답에서 `timings`를 캡처하는 클라이언트
///
/// `send()`에서는 응답 body 전체를 읽어 timings를 추출하고,
/// `send_streaming()`에서는 SSE 청크를 래핑하여 마지막 청크의 timings를 캡처한다.
#[derive(Clone, Debug)]
pub struct TimingsCapturingClient {
    inner: reqwest::Client,
    last_timings: Arc<RwLock<Option<LlamaTimings>>>,
}

impl TimingsCapturingClient {
    /// 새 캡처 클라이언트를 생성한다.
    ///
    /// `timings_store`는 `RigChatAdapter`와 공유하여 마지막 timings를 조회할 수 있다.
    pub fn new(timings_store: Arc<RwLock<Option<LlamaTimings>>>) -> Self {
        Self {
            inner: reqwest::Client::new(),
            last_timings: timings_store,
        }
    }
}

impl Default for TimingsCapturingClient {
    fn default() -> Self {
        Self {
            inner: reqwest::Client::new(),
            last_timings: Arc::new(RwLock::new(None)),
        }
    }
}

impl HttpClientExt for TimingsCapturingClient {
    fn send<T, U>(
        &self,
        req: Request<T>,
    ) -> impl std::future::Future<Output = http_client::Result<Response<LazyBody<U>>>>
           + rig::wasm_compat::WasmCompatSend
           + 'static
    where
        T: Into<Bytes> + Send,
        U: From<Bytes> + Send + 'static,
    {
        let (parts, body) = req.into_parts();
        let inner_req = self
            .inner
            .request(parts.method, parts.uri.to_string())
            .headers(parts.headers)
            .body(body.into());

        let timings_store = self.last_timings.clone();

        async move {
            let response = inner_req
                .send()
                .await
                .map_err(|e| http_client::Error::Instance(Box::new(e)))?;

            if !response.status().is_success() {
                return Err(http_client::Error::InvalidStatusCodeWithMessage(
                    response.status(),
                    response.text().await.unwrap_or_default(),
                ));
            }

            let mut res = Response::builder().status(response.status());
            if let Some(hs) = res.headers_mut() {
                *hs = response.headers().clone();
            }

            // body 전체를 읽어 timings 파싱 시도
            let bytes = response
                .bytes()
                .await
                .map_err(|e| http_client::Error::Instance(Box::new(e)))?;

            // timings 추출 (파싱 실패 시 무시 — llama-server가 아닌 경우)
            if let Ok(envelope) = serde_json::from_slice::<TimingsEnvelope>(&bytes) {
                *timings_store.write().await = envelope.timings;
            }

            let body: LazyBody<U> = Box::pin(async move { Ok(U::from(bytes)) });

            res.body(body).map_err(http_client::Error::Protocol)
        }
    }

    fn send_multipart<U>(
        &self,
        req: Request<MultipartForm>,
    ) -> impl std::future::Future<Output = http_client::Result<Response<LazyBody<U>>>>
           + rig::wasm_compat::WasmCompatSend
           + 'static
    where
        U: From<Bytes> + Send + 'static,
    {
        // multipart 요청에는 timings가 없으므로 inner에 위임
        let (parts, body) = req.into_parts();
        let form = reqwest::multipart::Form::from(body);

        let inner_req = self
            .inner
            .request(parts.method, parts.uri.to_string())
            .headers(parts.headers)
            .multipart(form);

        async move {
            let response = inner_req
                .send()
                .await
                .map_err(|e| http_client::Error::Instance(Box::new(e)))?;

            if !response.status().is_success() {
                return Err(http_client::Error::InvalidStatusCodeWithMessage(
                    response.status(),
                    response.text().await.unwrap_or_default(),
                ));
            }

            let mut res = Response::builder().status(response.status());
            if let Some(hs) = res.headers_mut() {
                *hs = response.headers().clone();
            }

            let body: LazyBody<U> = Box::pin(async {
                let bytes = response
                    .bytes()
                    .await
                    .map_err(|e| http_client::Error::Instance(Box::new(e)))?;
                Ok(U::from(bytes))
            });

            res.body(body).map_err(http_client::Error::Protocol)
        }
    }

    fn send_streaming<T>(
        &self,
        req: Request<T>,
    ) -> impl std::future::Future<Output = http_client::Result<StreamingResponse>>
           + rig::wasm_compat::WasmCompatSend
    where
        T: Into<Bytes>,
    {
        let (parts, body) = req.into_parts();

        let inner_req = self
            .inner
            .request(parts.method, parts.uri.to_string())
            .headers(parts.headers)
            .body::<Bytes>(body.into())
            .build()
            .map_err(|x| http_client::Error::Instance(x.into()))
            .unwrap();

        let client = self.inner.clone();
        let timings_store = self.last_timings.clone();

        async move {
            let response: reqwest::Response = client
                .execute(inner_req)
                .await
                .map_err(|e| http_client::Error::Instance(Box::new(e)))?;

            if !response.status().is_success() {
                return Err(http_client::Error::InvalidStatusCodeWithMessage(
                    response.status(),
                    response.text().await.unwrap_or_default(),
                ));
            }

            let mut res = Response::builder()
                .status(response.status())
                .version(response.version());

            if let Some(hs) = res.headers_mut() {
                *hs = response.headers().clone();
            }

            // SSE 스트림을 래핑하여 각 청크에서 timings를 캡처
            let inner_stream = response
                .bytes_stream()
                .map(|chunk| chunk.map_err(|e| http_client::Error::Instance(Box::new(e))));

            let mapped_stream: BoxedStream = Box::pin(TimingsCapturingStream {
                inner: Box::pin(inner_stream),
                timings_store,
                buffer: String::new(),
            });

            res.body(mapped_stream).map_err(http_client::Error::Protocol)
        }
    }
}

/// SSE 스트림을 래핑하여 `data:` 라인에서 timings를 캡처하는 스트림
///
/// llama-server는 스트리밍 모드에서 마지막 `data:` 청크(finish_reason: "stop")에
/// `timings` 객체를 포함한다. 이 스트림은 각 청크의 원본 bytes를 그대로 전달하면서
/// 백그라운드에서 timings를 파싱한다.
struct TimingsCapturingStream {
    inner: Pin<Box<dyn futures::Stream<Item = http_client::Result<Bytes>> + Send>>,
    timings_store: Arc<RwLock<Option<LlamaTimings>>>,
    /// 청크 경계를 넘는 SSE data를 축적하는 버퍼
    buffer: String,
}

/// SSE `data:` 라인에서 timings를 추출하는 헬퍼
#[derive(serde::Deserialize)]
struct StreamingTimingsEnvelope {
    timings: Option<LlamaTimings>,
}

impl futures::Stream for TimingsCapturingStream {
    type Item = http_client::Result<Bytes>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let result = Pin::new(&mut this.inner).poll_next(cx);

        if let std::task::Poll::Ready(Some(Ok(bytes))) = &result {
            // 청크의 텍스트에서 timings 포함 여부 확인
            if let Ok(text) = std::str::from_utf8(bytes) {
                this.buffer.push_str(text);

                // SSE data 라인 처리
                for line in this.buffer.lines() {
                    let line = line.trim();
                    if let Some(data) = line.strip_prefix("data:") {
                        let data = data.trim();
                        if data == "[DONE]" || data.is_empty() {
                            continue;
                        }
                        // timings 필드가 포함된 JSON인지 빠르게 확인 후 파싱
                        if data.contains("\"timings\"") {
                            if let Ok(envelope) =
                                serde_json::from_str::<StreamingTimingsEnvelope>(data)
                            {
                                if envelope.timings.is_some() {
                                    let store = this.timings_store.clone();
                                    let timings = envelope.timings;
                                    // 비동기 write를 위해 spawn — poll에서 .await 불가
                                    tokio::spawn(async move {
                                        *store.write().await = timings;
                                    });
                                }
                            }
                        }
                    }
                }

                // 완전한 라인만 처리했으므로, 마지막 불완전 라인만 보존
                if let Some(last_newline) = this.buffer.rfind('\n') {
                    this.buffer = this.buffer[last_newline + 1..].to_string();
                }
            }
        }

        result
    }
}
