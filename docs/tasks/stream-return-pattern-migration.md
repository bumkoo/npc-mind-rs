# Task — `send_message_stream` Stream 반환 패턴 적용 (tokio 누출 제거)

> **목적.** 현재 `ConversationPort::send_message_stream`은 `token_tx: tokio::sync::mpsc::Sender<String>`을 인자로 받아, **trait를 사용하는 모든 호출자가 tokio crate에 직접 의존하게 만드는 누출**이 존재한다. 본 태스크는 시그니처를 **Stream 반환** 형태로 뒤집어, 호출자가 채널을 만들 필요 없이 `Stream`을 폴링하기만 하면 토큰을 받을 수 있도록 변경한다. Bevy 게임 통합 시 게임 프로젝트의 Cargo.toml에서 tokio 직접 의존이 사라진다.
>
> **핵심 변경.** 호출자가 채널을 만들어 함수에 **넣어주는** 구조 → 함수가 Stream을 **반환하는** 구조로 역전.
>
> **범위.** `src/ports.rs`(시그니처), `src/adapter/rig_chat.rs`(어댑터 구현), `src/bin/mind-studio/handlers/chat.rs`(호출자), 관련 테스트.
>
> **소요 예상.** library core ~80 LoC + 어댑터 재작성 ~120 LoC + 호출자 재작성 ~40 LoC + 테스트 3~4개.

---

## 1. 배경 — 현재 구조와 누출의 정확한 위치

### 1.1 현재 시그니처 (`src/ports.rs`)

```rust
async fn send_message_stream(
    &self,
    session_id: &str,
    user_message: &str,
    token_tx: tokio::sync::mpsc::Sender<String>,    // ← 누출 지점
) -> Result<ChatResponse, ConversationError>;
```

### 1.2 현재 호출자 패턴 (`src/bin/mind-studio/handlers/chat.rs:36~38`)

```rust
let (token_tx, mut token_rx) = tokio::sync::mpsc::channel::<String>(64);
let llm_task = tokio::spawn(async move {
    chat_state_clone.send_message_stream(&session_id, &utterance, token_tx).await
});
while let Some(token) = token_rx.recv().await {
    yield Ok(/* SSE event */);
}
let chat_resp = llm_task.await?;
```

호출자가 (1) 채널 생성, (2) tokio task spawn, (3) 두 가지를 동시에 polling — 이 세 가지 모두를 직접 처리해야 한다.

### 1.3 누출의 본질

`token_tx`가 함수 인자로 등장하는 순간, 호출자 코드는:
- `tokio::sync::mpsc::channel(...)`을 호출해야 함 → tokio crate 의존
- `tokio::spawn`으로 task 분리 (혹은 join_set 사용)
- `Cargo.toml`에 `tokio = { ... features = ["sync", "rt"] }` 명시 필요

**Bevy 게임 통합 시**: Bevy는 자체 task pool(smol 기반)을 가지므로, tokio runtime을 별도로 관리해야 하는 부담이 발생한다.

### 1.4 본 태스크가 해결하는 것

호출자가 **자기 환경의 폴링 메커니즘**(tokio await, Bevy 매 프레임 try_next)으로 Stream을 소비하면 끝. 채널 생성·task spawn·동기화 모두 라이브러리 안으로 들어간다.

---

## 2. 목표

1. `ConversationPort::send_message_stream`의 시그니처를 **Stream 반환**으로 변경.
2. Stream item 타입을 `enum StreamItem { Token(String), Final(ChatResponse) }` 형태로 정의 — 토큰과 최종 응답이 같은 stream에서 흘러나옴.
3. `RigChatAdapter`의 구현을 새 시그니처에 맞게 재작성 — 기존 mpsc 패턴을 내부에 캡슐화.
4. `bin/mind-studio/handlers/chat.rs`의 SSE 핸들러를 새 시그니처에 맞게 단순화 — 외부 `tokio::spawn` 제거.
5. 누출 제거 검증: 호출자 코드에서 `tokio::sync::mpsc::Sender<String>` 등장이 0건이 되는지 grep으로 확인.

---

## 3. 완료 기준 (Definition of Done)

- [ ] `ConversationPort::send_message_stream`이 `Pin<Box<dyn Stream<Item = StreamItem> + Send + '_>>`을 반환한다.
- [ ] `StreamItem` enum이 `Token(String)`과 `Final(ChatResponse)` 두 variant를 가진다.
- [ ] `RigChatAdapter`가 새 시그니처를 구현하며, 내부적으로 mpsc + tokio::spawn을 사용해 토큰을 stream으로 흘려보낸다.
- [ ] `bin/mind-studio/handlers/chat.rs`의 `chat_message_stream` 핸들러가 외부 `tokio::sync::mpsc::channel`을 더 이상 호출하지 않는다.
- [ ] **검증 grep**: `findstr /S /I "tokio::sync::mpsc::Sender<String>" src\ports.rs` 결과가 0건.
- [ ] **검증 grep**: `findstr /S /I "tokio::sync::mpsc::channel" src\bin\mind-studio\handlers\chat.rs` 결과가 0건.
- [ ] `cargo test --workspace --all-features` 모두 통과.
- [ ] `cargo clippy --workspace --all-features -- -D warnings` 경고 없음.
- [ ] Mind Studio UI 수동 smoke test: 채팅 시작 → 토큰이 이전과 동일하게 SSE로 스트리밍됨.

---

## 4. 전제 및 주의사항

### 4.1 Library core 수정이 허용되는 범위
- `src/ports.rs` — `ConversationPort` trait 시그니처 + `StreamItem` enum (필수)
- `src/adapter/rig_chat.rs` — 어댑터 구현 (필수)
- `src/bin/mind-studio/handlers/chat.rs` — 호출자 단순화 (필수)
- `src/lib.rs` — `StreamItem` re-export 필요 시 추가
- 그 외 도메인·application·다른 어댑터는 수정 금지.

### 4.2 호환성 — Breaking change

**이건 명백한 breaking change다.** `ConversationPort` trait의 메서드 시그니처가 바뀌므로:
- 기존 어댑터 구현체(현재는 `RigChatAdapter` 1개)는 모두 재작성 필요
- 외부 사용자가 자체 어댑터를 만들었다면 깨짐 (현재 외부 사용자 0명이므로 영향 없음)
- 외부 호출자가 있다면 깨짐 (현재 외부 호출자 0명, 내부는 chat.rs 한 곳만)

**버전 정책**: GitHub 공개 전이므로 0.x 버전에서 자유롭게 변경 가능. 공개 후엔 SemVer minor bump 필요한 변경.

### 4.3 지켜야 할 원칙

- **호출자 측 tokio 의존 0**: 핵심 검증 기준. chat.rs 핸들러에서 `tokio::sync::mpsc::*`가 사라져야 함.
- **어댑터 내부에는 tokio 허용**: 어댑터(adapter/)는 외부 기술과 닿는 레이어이므로 tokio 사용 정당. 다만 그 사용이 trait 시그니처로 새어 나가지 않게 격리.
- **Stream item에 모든 정보 포함**: 토큰과 최종 응답을 같은 stream으로 흘려보내, 호출자가 별도 future를 await할 필요 없음.
- **Object-safety 유지**: `dyn ConversationPort`로 동적 디스패치 가능해야 함. `impl Stream` 반환 대신 `Pin<Box<dyn Stream>>` 사용.

### 4.4 사전 확인 사항

1. `src/ports.rs` — `ConversationPort` 전체 메서드 4개 시그니처와 `ChatResponse`, `LlamaTimings` 구조 확인.
2. `src/adapter/rig_chat.rs` — `send_message_stream` 현재 구현 (rig의 streaming API 사용) 확인. Streaming 응답을 어떻게 받아 mpsc로 전달하는지 흐름 파악.
3. `src/bin/mind-studio/handlers/chat.rs` — `chat_message_stream` SSE 핸들러의 yield 패턴 확인.
4. `Cargo.toml` — `futures` crate가 이미 의존성에 있는지 확인 (있음 — `futures = { ... features = ["std"] }`).

### 4.5 핵심 설계 결정 — Stream Item 형태

**채택안: 단일 Stream에 토큰과 최종 응답을 함께 흘려보냄**

```rust
pub enum StreamItem {
    /// 토큰 한 조각 (LLM이 생성하는 대로 즉시 전달)
    Token(String),
    /// 최종 응답 (timings 포함). Stream의 마지막 item으로 한 번만 발생.
    Final(ChatResponse),
}
```

호출자 패턴:

```rust
let mut stream = port.send_message_stream("session", "msg");
while let Some(item) = stream.next().await {
    match item {
        StreamItem::Token(t) => /* SSE 전송 또는 Bevy Events 발행 */,
        StreamItem::Final(resp) => /* 최종 처리 + break */,
    }
}
```

**왜 이 방식인가**:
- 호출자가 단일 stream만 폴링하면 끝 — 별도 task await 불필요
- `Result<ChatResponse>`를 stream 안의 variant로 끼워넣는 건 의미상 부담스러움. `Final` variant 하나면 충분
- 에러 처리는 `StreamItem::Token` / `StreamItem::Final` 외에 stream 자체가 종료되거나 별도 `StreamItem::Error(ConversationError)` 추가 가능 (아래 옵션)

**대안 검토 — 에러 처리**: 두 가지 방식 중 채택:
- (A) `Stream<Item = Result<StreamItem, ConversationError>>` — 매 item이 Result. 에러는 Stream 흐름 안에서 전파.
- (B) `Stream<Item = StreamItem>` + variant `StreamItem::Error(ConversationError)` 추가 — 에러도 일반 item.

**(A)를 채택**한다. 이유: Rust의 fallible stream 관행과 일치(`futures::TryStreamExt` 활용 가능), 호출자가 `?` 연산자로 에러를 자연스럽게 propagate 가능.

최종 시그니처:

```rust
pub enum StreamItem {
    Token(String),
    Final(ChatResponse),
}

#[async_trait::async_trait]
pub trait ConversationPort: Send + Sync {
    fn send_message_stream<'a>(
        &'a self,
        session_id: &'a str,
        user_message: &'a str,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamItem, ConversationError>> + Send + 'a>>;
    
    // 다른 메서드는 그대로 유지
}
```

`async fn`이 아니라 일반 `fn`으로 변경한다. Stream을 즉시 반환하므로 await이 함수 호출 시점에 필요 없다. 어댑터는 stream 내부에서 await을 자유롭게 사용 가능.

---

## 5. 작업 명세

### 5.1 작업 1 — `StreamItem` enum 정의 + trait 시그니처 변경

**파일:** `src/ports.rs`

**변경:**

```rust
use std::pin::Pin;
use futures::Stream;

/// 스트리밍 응답의 단일 항목.
///
/// LLM이 토큰을 생성하는 대로 `Token`이 흘러나오고,
/// 응답이 완료되면 마지막에 `Final`이 한 번 발행된다.
#[cfg(feature = "chat")]
#[derive(Debug, Clone)]
pub enum StreamItem {
    /// LLM이 막 생성한 토큰 한 조각
    Token(String),
    /// 최종 응답 — 누적된 전체 텍스트와 timings 포함.
    /// 정상 종료 시 stream의 마지막 item으로 정확히 한 번 발행된다.
    Final(ChatResponse),
}

#[cfg(feature = "chat")]
#[async_trait::async_trait]
pub trait ConversationPort: Send + Sync {
    // start_session, send_message, update_system_prompt, end_session — 변경 없음

    /// 상대의 대사를 전달하고 NPC(LLM) 응답을 토큰 단위로 스트리밍한다.
    ///
    /// 반환된 Stream은 `StreamItem::Token`을 여러 번 yield한 뒤,
    /// 마지막에 `StreamItem::Final`을 정확히 한 번 yield하고 종료된다.
    /// 에러 발생 시 `Err(ConversationError)`가 yield되고 stream이 종료된다.
    ///
    /// # 호출자 패턴
    ///
    /// ```rust,ignore
    /// let mut stream = port.send_message_stream("s1", "안녕");
    /// while let Some(item) = stream.next().await {
    ///     match item? {
    ///         StreamItem::Token(t) => print!("{}", t),
    ///         StreamItem::Final(resp) => return Ok(resp),
    ///     }
    /// }
    /// ```
    fn send_message_stream<'a>(
        &'a self,
        session_id: &'a str,
        user_message: &'a str,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamItem, ConversationError>> + Send + 'a>>;
}
```

**`async_trait` 영향**: 다른 메서드는 여전히 `async fn`이라 `#[async_trait]` 매크로가 필요하다. `send_message_stream`은 `async fn`이 아니므로 매크로 처리 시 무영향. 단, async_trait 매크로가 일반 `fn`도 받아주므로 그대로 두면 됨.

### 5.2 작업 2 — `RigChatAdapter` 어댑터 구현

**파일:** `src/adapter/rig_chat.rs`

**변경:** 기존 mpsc 패턴은 어댑터 내부에 캡슐화되어 그대로 유지하되, 외부 인터페이스만 Stream 반환으로 감싼다.

```rust
fn send_message_stream<'a>(
    &'a self,
    session_id: &'a str,
    user_message: &'a str,
) -> Pin<Box<dyn Stream<Item = Result<StreamItem, ConversationError>> + Send + 'a>> {
    use async_stream::stream;
    
    Box::pin(stream! {
        // 1. 내부 채널 생성 (어댑터 내부 구현 디테일 — 외부에 노출 안 됨)
        let (token_tx, mut token_rx) = tokio::sync::mpsc::channel::<String>(64);
        
        // 2. 기존 streaming 로직을 task로 spawn
        //    (rig의 streaming API + ChatSession 갱신 로직)
        let session_id_owned = session_id.to_string();
        let user_message_owned = user_message.to_string();
        let adapter_ref = self;
        
        let llm_future = adapter_ref.run_streaming_internal(
            session_id_owned,
            user_message_owned,
            token_tx,
        );
        let mut llm_task = tokio::spawn(llm_future);
        
        // 3. 토큰을 흘려보내며 await을 점진적으로 진행
        loop {
            tokio::select! {
                Some(token) = token_rx.recv() => {
                    yield Ok(StreamItem::Token(token));
                }
                result = &mut llm_task => {
                    // task 완료 — 남은 토큰을 모두 비워주고 Final 발행
                    while let Ok(token) = token_rx.try_recv() {
                        yield Ok(StreamItem::Token(token));
                    }
                    match result {
                        Ok(Ok(chat_resp)) => yield Ok(StreamItem::Final(chat_resp)),
                        Ok(Err(e)) => yield Err(e),
                        Err(panic) => yield Err(ConversationError::InferenceError(
                            format!("스트리밍 task 패닉: {panic}")
                        )),
                    }
                    break;
                }
            }
        }
    })
}
```

**보조 메서드**: 기존 `send_message_stream`의 본체를 `run_streaming_internal`로 추출해 private 메서드로 만든다. 시그니처는 기존과 동일하게 `mpsc::Sender<String>`을 받음 — 외부에 노출되지 않으므로 누출 없음.

```rust
impl RigChatAdapter {
    /// 내부 streaming 실행 — mpsc 패턴은 어댑터 사적 구현 디테일.
    /// 외부에 노출되지 않으므로 tokio 의존이 안전하게 격리된다.
    async fn run_streaming_internal(
        &self,
        session_id: String,
        user_message: String,
        token_tx: tokio::sync::mpsc::Sender<String>,
    ) -> Result<ChatResponse, ConversationError> {
        // 기존 send_message_stream 본체를 여기로 이동
        // ... rig.streaming_chat() 호출 + 토큰 send + 누적 + ChatResponse 생성 ...
    }
}
```

### 5.3 작업 3 — `bin/mind-studio/handlers/chat.rs` 호출자 단순화

**파일:** `src/bin/mind-studio/handlers/chat.rs`

**변경 전 (현재):**

```rust
let (token_tx, mut token_rx) = tokio::sync::mpsc::channel::<String>(64);
let session_id = req.session_id.clone();
let utterance = req.utterance.clone();
let chat_state_clone = chat_state.clone();
let llm_task = tokio::spawn(async move {
    chat_state_clone.send_message_stream(&session_id, &utterance, token_tx).await
});
while let Some(token) = token_rx.recv().await {
    yield Ok(axum::response::sse::Event::default().event("token").data(token));
}
let chat_resp = match llm_task.await { Ok(Ok(resp)) => resp, Ok(Err(e)) => { ... } Err(e) => { ... } };
let npc_response = chat_resp.text;
let timings = chat_resp.timings;
// ... 후속 처리 ...
```

**변경 후 (목표):**

```rust
use futures::StreamExt;
use npc_mind::StreamItem;   // re-export 필요

let mut stream = chat_state.send_message_stream(&req.session_id, &req.utterance);
let mut chat_resp: Option<npc_mind::ChatResponse> = None;
while let Some(item) = stream.next().await {
    match item {
        Ok(StreamItem::Token(t)) => {
            yield Ok(axum::response::sse::Event::default().event("token").data(t));
        }
        Ok(StreamItem::Final(resp)) => {
            chat_resp = Some(resp);
            break;
        }
        Err(e) => {
            yield Ok(axum::response::sse::Event::default().event("error").data(e.to_string()));
            return;
        }
    }
}
let chat_resp = match chat_resp {
    Some(r) => r,
    None => {
        yield Ok(axum::response::sse::Event::default().event("error").data("스트림이 Final 없이 종료됨"));
        return;
    }
};
let npc_response = chat_resp.text;
let timings = chat_resp.timings;
// ... 후속 처리는 그대로 ...
```

**핵심 변화**:
- `tokio::sync::mpsc::channel` 호출 제거
- `tokio::spawn` 제거
- 단일 stream loop만 남음
- 코드량 감소 + 가독성 향상

### 5.4 작업 4 — `lib.rs`에 `StreamItem` re-export

**파일:** `src/lib.rs`

```rust
#[cfg(feature = "chat")]
pub use ports::{
    ChatResponse, ConversationError, ConversationPort, DialogueRole, DialogueTurn,
    LlamaTimings, StreamItem,    // ← 추가
};
```

Mind Studio bin과 외부 라이브러리 사용자가 `npc_mind::StreamItem`으로 접근 가능해진다.

---

## 6. 테스트 요구사항

### 6.1 Stream item 발행 순서 검증 (필수)

```rust
#[tokio::test]
async fn stream_yields_tokens_then_final() {
    let port = build_mock_chat_port_with_response("안녕하시오.");
    port.start_session("s1", "system prompt", None).await.unwrap();
    
    let mut stream = port.send_message_stream("s1", "오랜만이군");
    let mut tokens = Vec::new();
    let mut final_resp = None;
    
    while let Some(item) = stream.next().await {
        match item.unwrap() {
            StreamItem::Token(t) => tokens.push(t),
            StreamItem::Final(resp) => {
                assert!(final_resp.is_none(), "Final must yield exactly once");
                final_resp = Some(resp);
            }
        }
    }
    
    assert!(!tokens.is_empty(), "expected at least one token");
    let final_resp = final_resp.expect("Final must be yielded");
    let concatenated: String = tokens.into_iter().collect();
    assert_eq!(final_resp.text, concatenated, "Final.text must match concatenated tokens");
}
```

### 6.2 Stream 종료 검증 (필수)

```rust
#[tokio::test]
async fn stream_terminates_after_final() {
    let port = build_mock_chat_port_with_response("hi");
    port.start_session("s2", "prompt", None).await.unwrap();
    
    let mut stream = port.send_message_stream("s2", "msg");
    let mut saw_final = false;
    while let Some(item) = stream.next().await {
        if let Ok(StreamItem::Final(_)) = item {
            saw_final = true;
        }
    }
    
    assert!(saw_final, "stream must yield Final before terminating");
}
```

### 6.3 에러 전파 검증 (필수)

```rust
#[tokio::test]
async fn stream_propagates_errors() {
    let port = build_mock_chat_port_that_fails();
    port.start_session("s3", "prompt", None).await.unwrap();
    
    let mut stream = port.send_message_stream("s3", "msg");
    let mut saw_error = false;
    while let Some(item) = stream.next().await {
        if item.is_err() {
            saw_error = true;
            break;
        }
    }
    
    assert!(saw_error, "stream must propagate errors as Err item");
}
```

### 6.4 누출 제거 검증 (필수 — grep 기반)

자동화된 단위 테스트가 아니라 빌드 후 검증 단계로 추가:

```bash
# 라이브러리 표면에 tokio mpsc Sender<String>이 없어야 함
findstr /S /I /C:"tokio::sync::mpsc::Sender<String>" src\ports.rs
# 결과: 0건이어야 함

# 호출자에서 외부 채널 생성이 없어야 함
findstr /S /I /C:"tokio::sync::mpsc::channel" src\bin\mind-studio\handlers\chat.rs
# 결과: 0건이어야 함
```

이 검증을 PR description에 결과 캡처로 첨부.

### 6.5 Mind Studio UI smoke test (필수)

- Mind Studio 기동 → NPC 생성 → 시나리오 로드 → 채팅 시작
- 메시지 전송 후 SSE 토큰이 이전과 **동일한 페이스로** 흘러나오는지 시각적 확인
- 최종 응답 (`done` 이벤트)이 정상적으로 도착하는지 확인
- 에러 케이스(LLM 서버 다운 등)에서 `error` 이벤트가 전송되는지 확인

### 6.6 회귀 확인 (필수)

- `cargo test --workspace --all-features` 전체 통과
- 기존 `RigChatAdapter`를 직접 사용하는 다른 코드가 있다면 모두 갱신
- read-side-activation, correlation-id-activation, parent-event-id-activation의 모든 테스트가 통과 (회귀 baseline)

---

## 7. 점진적 도입 순서 (권장)

본 태스크는 breaking change라 단계 분리가 약간 다르다. 안전한 순서는:

### 1단계 — 시그니처 + 어댑터 (실제 변경)
- 작업 1 (StreamItem + trait 시그니처)
- 작업 2 (RigChatAdapter 재구현)
- 작업 4 (lib.rs re-export)
- **이 시점에 빌드는 깨진다** — chat.rs 핸들러가 아직 옛 시그니처로 호출하므로

### 2단계 — 호출자 갱신 (빌드 복구)
- 작업 3 (chat.rs 핸들러 단순화)
- 빌드 복구. 컴파일 통과.

### 3단계 — 테스트 추가
- 6.1, 6.2, 6.3 단위 테스트 추가 + 통과 확인
- 6.4 grep 검증

### 4단계 — 수동 smoke test
- 6.5 Mind Studio UI 검증
- 토큰 페이스, 최종 응답, 에러 케이스 모두 확인

### 5단계 — 문서화
- README의 chat feature 사용 예제 코드 갱신 (이전 mpsc 패턴 → 새 Stream 패턴)
- `docs/architecture/system-overview.md` §6 트레이드오프 항목에 "tokio 누출 제거 — Bevy 통합 시 게임 프로젝트가 tokio 직접 의존 안 해도 됨" 추가
- `CLAUDE.md` 또는 changelog에 breaking change 명시

---

## 8. 체크리스트 (PR 올리기 전)

### Library core
- [ ] `src/ports.rs`에 `StreamItem` enum 추가
- [ ] `ConversationPort::send_message_stream` 시그니처를 Stream 반환으로 변경
- [ ] `Pin<Box<dyn Stream<...>>>` import 정리
- [ ] `src/lib.rs`에 `StreamItem` re-export 추가

### 어댑터
- [ ] `RigChatAdapter::send_message_stream` 새 시그니처로 재구현
- [ ] 기존 mpsc 로직을 `run_streaming_internal`로 private 추출
- [ ] `tokio::select!` 또는 동등 패턴으로 토큰/최종 응답 합류

### 호출자
- [ ] `chat.rs::chat_message_stream`에서 외부 mpsc + spawn 제거
- [ ] 단일 stream loop로 단순화
- [ ] 기존 SSE 이벤트(`token`, `error`, `done`) 동일하게 발행

### 테스트
- [ ] `stream_yields_tokens_then_final` 통과
- [ ] `stream_terminates_after_final` 통과
- [ ] `stream_propagates_errors` 통과
- [ ] grep 검증 통과 (`ports.rs`에 `mpsc::Sender<String>` 0건, chat.rs에 `mpsc::channel` 0건)
- [ ] `cargo test --workspace --all-features` 전체 통과
- [ ] `cargo clippy --workspace --all-features -- -D warnings` 경고 없음

### 수동 smoke test
- [ ] Mind Studio UI에서 채팅 시작 → 토큰 스트리밍 페이스 이전과 동일
- [ ] `done` 이벤트 정상 수신
- [ ] LLM 서버 에러 시 `error` 이벤트 정상 수신

### 문서
- [ ] README의 chat 예제 갱신
- [ ] system-overview.md 트레이드오프 항목 추가
- [ ] CHANGELOG에 breaking change 명시

---

## 9. 관련 파일 (작업 시 참조 경로)

| 역할 | 경로 | 변경 여부 |
|---|---|---|
| ConversationPort trait + StreamItem | `src/ports.rs` | 수정 (시그니처 + enum 추가) |
| RigChatAdapter 어댑터 | `src/adapter/rig_chat.rs` | 수정 (재구현) |
| Mind Studio chat 핸들러 | `src/bin/mind-studio/handlers/chat.rs` | 수정 (단순화) |
| 라이브러리 공개 export | `src/lib.rs` | 수정 (StreamItem export) |
| Cargo.toml | `Cargo.toml` | 변경 없음 — `futures`는 이미 의존성 |

---

## 10. Out of Scope / 후속 작업

본 태스크에서 **하지 않는다**:

- **다른 ConversationPort 메서드의 변경.** `start_session`, `send_message`, `update_system_prompt`, `end_session`은 그대로. 이들은 mpsc를 안 쓰므로 누출이 없다.
- **Bevy 통합 예제.** 본 태스크는 누출 제거까지. Bevy plugin 패턴이나 ECS 매핑은 별도.
- **Streaming의 backpressure 정책.** Stream item이 빠르게 생성될 때 호출자가 느리게 소비하면 buffering이 어떻게 일어나는지 — 현재 mpsc(64)와 동일한 buffer를 쓰므로 동작은 같다. 명시적 backpressure 정책 변경은 별도 태스크.
- **다른 LLM 어댑터.** `RigChatAdapter` 외에 추가 어댑터(예: 직접 reqwest, mock 어댑터)가 있다면 별도 갱신 작업이 필요하지만, 현재 구현체는 1개뿐이다.
- **Error type 확장.** `ConversationError`에 streaming 전용 variant 추가 등은 별도 태스크.

---

## 11. 위험 요소

### 11.1 `async_stream::stream!` 매크로 사용
`async_stream` crate가 이미 의존성에 있는지 확인 필요. Cargo.toml의 `chat = ["dep:async-stream", ...]`에 포함되어 있다 — OK. 매크로 사용 시 lifetime이 까다로울 수 있음 — `'a` lifetime 명시 + `move` capture 주의.

### 11.2 `tokio::select!` 사용
어댑터 내부에서 `tokio::select!`로 토큰 수신 + task 완료를 합류한다. 이는 어댑터 내부이므로 tokio 사용 정당하다. 다만 select가 cancel-safe해야 함 — `mpsc::Receiver::recv`는 cancel-safe, `JoinHandle`은 cancel-safe. 안전.

### 11.3 토큰 손실 가능성
`tokio::select!`에서 task 완료 분기가 먼저 선택되면 채널에 남은 토큰이 손실될 수 있음. 이를 방지하기 위해 본 명세는 task 완료 후에도 `try_recv`로 남은 토큰을 모두 비워준다 (작업 2의 코드 참조). **이 부분은 반드시 구현되어야 한다.**

### 11.4 SSE 이벤트 페이스 변화
새 구현이 기존과 동일한 토큰 페이스를 유지해야 한다. mpsc(64) 버퍼는 동일하게 유지. 다만 stream 내부에 한 단계 더 wrapping이 들어가 미세한 latency가 추가될 수 있다 — 수동 smoke test에서 확인.

### 11.5 Object-safety
`fn ... -> Pin<Box<dyn Stream<...> + Send + 'a>>`은 object-safe하다. `dyn ConversationPort` 객체에서 호출 가능. 단, 일반화된 lifetime 처리에 주의. `'a`는 `&'a self`와 `&'a str` 인자에 묶이므로 stream이 그들의 lifetime 안에서만 살 수 있다 — 이는 정상 동작.

### 11.6 외부 사용자 영향 — 현재는 0
외부 사용자가 `RigChatAdapter` 외 자체 어댑터를 만들었거나, `send_message_stream`을 직접 호출하는 코드가 있다면 깨진다. 현재 외부 사용자 0명이므로 영향 없음. 향후 GitHub 공개 후엔 SemVer minor bump로 표시.
