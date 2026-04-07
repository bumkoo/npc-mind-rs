# MCP Server Architecture

> **대상 독자**: 엔진 개발자, MCP 서버 기여자.
> **소스**: `src/bin/mind-studio/mcp_server.rs`, `src/bin/mind-studio/main.rs`
>
> 이 문서는 MCP 서버가 `npc-mind-rs`의 DDD/헥사고날 아키텍처 안에서 어떤 위치를 차지하는지,
> SSE transport와 JSON-RPC 프로토콜이 어떻게 결합되어 있는지를 설명한다.

---

## 1. 아키텍처적 위치

MCP 서버는 도메인 엔진의 **adapter 계층**에 속한다. REST WebUI 백엔드와 나란히 배치되어
같은 `AppState`와 같은 `MindService`/`StudioService`를 공유한다.

```
┌─────────────────────────────────────────────────────────┐
│              Application Binary                         │
│              (npc-mind-studio)                          │
│                                                         │
│  ┌──────────────┐        ┌──────────────────┐          │
│  │  REST        │        │  MCP SSE         │          │
│  │  handlers/   │        │  mcp_server.rs   │  ← 현 문서│
│  │              │        │                  │          │
│  │ WebUI용      │        │ AI 에이전트용    │          │
│  └──────┬───────┘        └────────┬─────────┘          │
│         │                         │                    │
│         └──────────┬──────────────┘                    │
│                    ▼                                   │
│         ┌─────────────────────┐                        │
│         │  StudioService      │  ← Application Layer   │
│         │  (studio_service.rs)│    (adapter 공유)      │
│         └──────────┬──────────┘                        │
│                    │                                   │
│                    ▼                                   │
│         ┌─────────────────────┐                        │
│         │  MindService        │  ← Core Application    │
│         │  (도메인 orchestr.) │                        │
│         └──────────┬──────────┘                        │
│                    │                                   │
│                    ▼                                   │
│   ┌────────────────────────────────┐                   │
│   │  Domain Layer                  │                   │
│   │  (HEXACO, OCC, PAD, Scene,     │                   │
│   │   Relationship, Emotion, ...)  │                   │
│   └────────────────────────────────┘                   │
└─────────────────────────────────────────────────────────┘
```

**핵심 원칙**: MCP 서버는 얇은 변환 계층이다. JSON-RPC 요청을 받아 DTO로 역직렬화하고,
`StudioService`/`MindService`를 호출하고, 결과를 JSON-RPC 응답으로 직렬화할 뿐이다.
도메인 로직을 여기에 넣지 않는다.

---

## 2. SSE Transport 구조

MCP는 전통적인 단일 HTTP 엔드포인트가 아니라 **이원 엔드포인트(dual-endpoint) 패턴**을 사용한다.

| 엔드포인트 | 메서드 | 역할 |
|---|---|---|
| `/mcp/sse` | GET | 세션 생성 + **서버→클라이언트** 응답 스트림 (SSE) |
| `/mcp/message` | POST | **클라이언트→서버** JSON-RPC 요청 전송 |

### 연결 초기화 시퀀스

```
Client                    Server
  │                         │
  │─── GET /mcp/sse ───────▶│
  │                         │  ① 세션 생성 (UUID)
  │                         │  ② mpsc 채널 생성
  │                         │  ③ session_map에 저장
  │                         │
  │◀── event: endpoint ─────│
  │    data: /mcp/message?  │
  │         session_id=UUID │
  │                         │
  │    [SSE 스트림 유지]     │
  │                         │
  │─── POST /mcp/message ──▶│
  │    ?session_id=UUID     │  ④ JSON-RPC 파싱
  │    {jsonrpc, id, method,│  ⑤ 도구 dispatch
  │     params}             │  ⑥ 결과 래핑
  │                         │
  │◀── 200 {status:"sent"} ─│  ⑦ POST 즉시 ack (결과 아님)
  │                         │
  │◀── SSE data event ──────│  ⑧ 결과를 SSE 채널로 전송
  │    {jsonrpc, id, result}│
  │                         │
```

**핵심**: POST 응답 자체에 도구 결과가 담기지 않는다. POST는 "요청 접수 완료"만 알리고,
실제 결과(`jsonrpc` 응답)는 **SSE 스트림으로 비동기 전송**된다. 클라이언트는 POST의 `id`와
SSE로 도착한 `id`를 매칭시켜 결과를 짝 맞춘다.

### 왜 이 패턴인가

- **HTTP 단방향 제약 극복**: HTTP는 서버가 클라이언트에 능동적으로 푸시할 수 없다. SSE로
  "서버→클라이언트" 채널을 먼저 열어두고, POST는 그 채널을 통해 응답이 나오게 한다.
- **MCP SSE transport 표준**: Anthropic의 MCP 표준 SSE transport 규약을 따른다. Claude
  Desktop, Claude Code 등 표준 MCP 클라이언트가 이 패턴을 기대한다.

---

## 3. 세션 관리

### `McpSessionManager`

```rust
pub struct McpSessionManager {
    sessions: RwLock<HashMap<String, mpsc::Sender<String>>>,
}
```

- **Key**: UUID v4 문자열
- **Value**: 해당 세션의 SSE 스트림으로 메시지를 보낼 `mpsc::Sender`
- **동시성**: `RwLock`으로 보호. 세션 생성/삭제는 write lock, 메시지 전송은 read lock

### 세션 생명주기

1. **생성**: `GET /mcp/sse` → `create_session()` → UUID + channel + insert
2. **사용**: `POST /mcp/message?session_id=X` → `send_to_session(id, msg)` → channel에 전송
3. **종료**: 현재 **미구현** (아래 "알려진 제약사항" 참조)

### `AppState`와의 관계

`MindMcpService`는 `AppState`를 소유한다. `AppState`는 `Arc`로 래핑되어 REST
핸들러와 MCP 핸들러가 같은 도메인 상태(NPC, 관계, Scene, history 등)를 공유한다.

```rust
pub struct MindMcpService {
    state: AppState,                     // 공유 도메인 상태
    pub session_manager: McpSessionManager, // MCP 전용 세션
}
```

이 설계로 다음이 자동으로 성립한다:
- REST WebUI에서 NPC를 추가하면 MCP 클라이언트의 `list_npcs`에도 즉시 반영된다
- MCP로 `load_scenario` 하면 WebUI의 대시보드도 새 시나리오를 본다
- 둘은 같은 `turn_history`를 기록하고 읽는다

---

## 4. JSON-RPC 메시지 처리

### 지원 메서드

| Method | 역할 | MCP 표준 |
|---|---|---|
| `initialize` | 프로토콜 버전 + 서버 정보 교환 | ✓ |
| `ping` | 연결 확인 | ✓ |
| `tools/list` | 도구 목록 조회 | ✓ |
| `tools/call` | 도구 실행 (35개 도구) | ✓ |
| `notifications/*` | 알림 (응답 불필요) | ✓ |

서버 정보:
- `protocolVersion`: `"2024-11-05"`
- `name`: `"npc-mind-studio"`
- `version`: `"0.1.0"`
- `capabilities`: `{"tools": {}}`

### `tools/call` 결과 래핑

MCP 표준 `CallToolResult` 형식으로 결과를 감싼다.

**성공 시**:
```json
{
  "content": [
    { "type": "text", "text": "<도구 결과 JSON 문자열>" }
  ]
}
```

**에러 시**:
```json
{
  "content": [
    { "type": "text", "text": "<에러 메시지>" }
  ],
  "isError": true
}
```

도구가 반환한 JSON 값은 `val.to_string()`으로 문자열화되어 `text` 필드에 담긴다.
클라이언트는 이 문자열을 다시 파싱해야 한다. 이는 MCP 표준의 제약이다.

---

## 5. Tool Dispatch 흐름

`call_tool(name, arguments)`에서 거대한 `match name` 블록으로 각 도구를 분기한다.
각 분기는 다음 패턴을 따른다:

```rust
"appraise" => {
    // 1. 인자를 DTO로 역직렬화
    let args: AppraiseRequest = serde_json::from_value(arguments.clone())?;
    
    // 2. Application Service 호출
    let resp = StudioService::perform_appraise(&self.state, args).await?;
    
    // 3. 응답을 Value로 직렬화
    Ok(serde_json::to_value(resp)?)
}
```

### 호출 경로 예시 (`appraise`)

```
MCP Client (Claude)
    │
    │ tools/call { name: "appraise", arguments: {...} }
    ▼
mcp_message_handler()              [HTTP layer]
    │
    ▼
MindMcpService::call_tool("appraise", ...)   [MCP adapter]
    │
    ▼
StudioService::perform_appraise(state, req)  [Application Service]
    │
    ▼
MindService::appraise(...)                    [Core Application]
    │
    ▼
AppraisalEngine / SituationService ...       [Domain]
```

도메인 로직은 `src/domain/` 하위에서만 일어난다. MCP 계층은 단지 직렬화/역직렬화와 호출 라우팅만 담당.

---

## 6. 네이티브 Rust 구현 선택 이유

MCP Rust SDK 대신 Axum 기반의 네이티브 구현을 선택한 이유:

### 의존성 최소화

GitHub 배포 프로젝트에서 외부 MCP 라이브러리 의존성은 리스크다:
- 해당 라이브러리가 MCP 스펙 변경에 뒤처지면 업그레이드 블록
- 다른 프로젝트와 버전 충돌 가능성
- `cargo audit` 경보 surface 증가

현재 구현은 `axum` + `tokio` + `serde_json`만 사용한다. 모두 이미 WebUI 백엔드에서 쓰는 크레이트들.

### REST와 라우터 공유

```rust
let router = Router::new()
    .route("/api/npcs", ...)           // REST
    ...
    .merge(mcp_server::mcp_router());   // MCP

router.with_state(state)
```

`Router::merge()`로 하나의 Axum 앱에 두 인터페이스를 얹는다. 포트도 하나(3000), 프로세스도 하나.

### 상태 공유의 단순함

`AppState` 하나를 REST와 MCP가 같이 쓴다. 외부 MCP 서버 프로세스를 띄우고 IPC/DB로 동기화하는 복잡함이 없다.

### 비용

반대급부로 감수해야 하는 것들:
- MCP 프로토콜 변경 시 직접 따라가야 함 (지금은 MCP 스펙이 비교적 안정적이라 감당 가능)
- 고수준 추상화 부재 (raw JSON-RPC 처리)
- MCP 표준 클라이언트 라이브러리의 추가 기능(progress notification, cancellation 등) 미구현

---

## 7. 알려진 제약사항

### 세션 정리 미구현

`McpSessionManager::remove_session()`은 `#[allow(dead_code)]` 상태. SSE 연결이
끊어져도 세션 엔트리가 `HashMap`에 남는다. 장시간 운영 시 메모리 누수 가능.

**현재 영향**: 개발/테스트 환경에서는 무시 가능. 프로덕션 전 해결 필요.

**해결 방향 후보**:
- SSE stream drop 감지 시 `remove_session` 호출
- 주기적 timeout 기반 정리 (N분간 트래픽 없으면 제거)

### Streaming 응답 미지원

`dialogue_turn`은 MCP에서 단일 JSON 응답만 반환한다. REST의 `POST /api/chat/message/stream`에
해당하는 SSE 스트리밍 버전은 MCP tool로 노출되지 않는다. MCP tool 응답 규약이
단일 `CallToolResult`이기 때문.

장문 응답이 필요한 경우 REST 엔드포인트를 직접 사용해야 한다.

### Progress Notification 미구현

장시간 도구(예: embed 초기 로드, LLM 호출)의 진행률을 MCP `notifications/progress`로
보내지 않는다. 클라이언트는 완료될 때까지 대기만 한다.

### Cancellation 미구현

MCP `notifications/cancelled`를 받아도 in-flight 도구 호출을 중단하지 않는다.
Rust async cancellation을 제대로 전파하려면 `CancellationToken`을 모든 도구 호출 경로에 주입해야 함.

---

## 8. 관련 파일

| 파일 | 역할 |
|---|---|
| `src/bin/mind-studio/main.rs` | 서버 부트스트랩, 라우터 빌드 |
| `src/bin/mind-studio/mcp_server.rs` | **MCP 서버 전체 구현** (이 문서의 대상) |
| `src/bin/mind-studio/state.rs` | `AppState`, 세션 상태 구조체 (`script_cursor` 포함) |
| `src/bin/mind-studio/studio_service.rs` | Application Service 계층 |
| `src/bin/mind-studio/handlers/` | REST 핸들러 (비교 대상) |
| `src/application/mind_service.rs` | Core Application 계층 |
| `src/application/dto/` | 요청/응답 DTO 타입 |

---

## 참고

- **REST↔MCP 기능 대응**: [`rest-parity.md`](rest-parity.md)
- **도구 API 스펙**: [`tools-reference.md`](tools-reference.md)
- **사용 워크플로우**: [`agent-playbook.md`](agent-playbook.md)
- **MCP 공식 스펙**: https://spec.modelcontextprotocol.io/
