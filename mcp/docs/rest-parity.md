# REST API ↔ MCP Tool Parity

> **대상 독자**: 엔진 개발자. REST WebUI 백엔드와 MCP 서버가 같은 도메인 엔진을 두 개의 adapter로 노출하기 때문에, 두 인터페이스의 기능 대응을 추적하기 위한 문서.
>
> **소스**: `src/bin/mind-studio/main.rs` (REST routes), `src/bin/mind-studio/mcp_server.rs` (MCP tools)
>
> **확인 날짜**: 2026-04

---

## 개요

`npc-mind-studio` 바이너리는 동일한 `AppState`와 도메인 엔진을 두 가지 어댑터로 노출한다.

- **REST API** (`/api/*`) — WebUI(`src/bin/mind-studio/static/`) 백엔드 전용
- **MCP SSE** (`/mcp/sse`, `/mcp/message`) — AI 에이전트(Claude 등) 통합용

두 인터페이스는 `StudioService` 함수 계층에서 동일한 로직을 호출한다. 따라서 새 기능 추가 시 **두 쪽 모두 업데이트하는 것이 원칙**이다. 단, 각 인터페이스의 특성에 맞지 않는 예외는 아래에 명시한다.

---

## 완전 매핑표

### 세계 구축 CRUD

| REST | MCP tool | 비고 |
|---|---|---|
| `GET /api/npcs` | `list_npcs` | |
| `POST /api/npcs` | `create_npc` | REST는 upsert, MCP도 upsert |
| `DELETE /api/npcs/{id}` | `delete_npc` | |
| `GET /api/relationships` | `list_relationships` | |
| `POST /api/relationships` | `create_relationship` | |
| `DELETE /api/relationships/{owner_id}/{target_id}` | `delete_relationship` | |
| `GET /api/objects` | `list_objects` | |
| `POST /api/objects` | `create_object` | |
| `DELETE /api/objects/{id}` | `delete_object` | |

### 감정 파이프라인

| REST | MCP tool | 비고 |
|---|---|---|
| `POST /api/appraise` | `appraise` | |
| `POST /api/stimulus` | `apply_stimulus` | MCP 이름은 `apply_` 접두사 |
| `POST /api/analyze-utterance` | `analyze_utterance` | `embed` feature 필요 |
| `POST /api/guide` | `generate_guide` | |
| `POST /api/after-dialogue` | `after_dialogue` | |

### 상태 관리

| REST | MCP tool | 비고 |
|---|---|---|
| `GET /api/history` | `get_history` | |
| `GET /api/situation` | `get_situation` | |
| `PUT /api/situation` | `update_situation` | REST는 PUT, MCP는 단일 도구 |
| `GET /api/test-report` | `get_test_report` | |
| `PUT /api/test-report` | `update_test_report` | |

### 시나리오 관리

| REST | MCP tool | 비고 |
|---|---|---|
| `GET /api/scenarios` | `list_scenarios` | |
| `GET /api/scenario-meta` | `get_scenario_meta` | |
| `POST /api/save` | `save_scenario` | |
| `POST /api/load` | `load_scenario` | |

### Scene 관리

| REST | MCP tool | 비고 |
|---|---|---|
| `POST /api/scene` | `start_scene` | REST 이름은 `/scene`, MCP는 `start_scene` |
| `GET /api/scene-info` | `get_scene_info` | |

### 결과 관리

| REST | MCP tool | 비고 |
|---|---|---|
| `GET /api/save-dir` | `get_save_dir` | |
| `POST /api/load-result` | `load_result` | |

### LLM 대화 테스트 (`chat` feature)

| REST | MCP tool | 비고 |
|---|---|---|
| `POST /api/chat/start` | `dialogue_start` | |
| `POST /api/chat/message` | `dialogue_turn` | |
| `POST /api/chat/message/stream` | *(없음)* | SSE 스트리밍. MCP는 단일 응답만 |
| *(없음)* | `get_next_utterance` | test_script 커서 조회/전진. MCP 전용 |
| `POST /api/chat/end` | `dialogue_end` | |

---

## MCP 전용 도구 (REST에 없음)

다음 4개 도구는 MCP에만 있다. AI 에이전트의 자율 워크플로우 지원을 위한 고수준 기능들이라 WebUI 백엔드에는 불필요하다.

| MCP tool | 용도 | REST 미구현 사유 |
|---|---|---|
| `list_source_texts` | `data/` 하위 `.txt` 파일 목록 | WebUI는 파일 브라우저 미제공 |
| `read_source_text` | 원작 텍스트 챕터 단위 읽기 | WebUI는 외부 에디터로 파일 열람 |
| `create_full_scenario` | NPC+관계+오브젝트+Scene 일괄 생성 | WebUI는 단계별 패널로 구성 |
| `get_npc_llm_config` | NPC 성격 기반 LLM temperature/top_p | WebUI는 LLM 호출을 직접 안 함 |
| `get_next_utterance` | test_script 커서 조회/전진 | WebUI는 UI에서 직접 스크립트 전송 버튼 제공 |

### 추가 후보

MCP-only 도구는 계속 늘어날 수 있다. 기준:
- AI 에이전트가 자율 워크플로우에서 필요로 하는 조회/자동화
- WebUI가 다른 방식으로 제공하는 기능 (예: 파일 시스템 브라우징)

---

## REST 전용 엔드포인트 (MCP에 없음)

| REST | 사유 |
|---|---|
| `POST /api/chat/message/stream` | SSE 스트리밍 응답. MCP tool은 단일 JSON 응답 규약이므로 구조적으로 불가. 필요 시 MCP 도구 `dialogue_turn`이 완성된 응답을 반환 |

---

## 동기화 정책

### 신규 기능 추가 시

1. **도메인 로직은 `StudioService` 또는 `MindService`에 구현한다** — adapter 계층 중복 금지
2. **REST와 MCP 양쪽에 노출할지 결정한다**:
   - WebUI가 필요하면 REST 추가
   - AI 에이전트가 필요하면 MCP 추가
   - 둘 다 필요한 경우가 대부분 — 함께 추가
3. **이름 규약**:
   - REST: 동사 없는 리소스 명사 (`/api/scene`, `/api/history`)
   - MCP: 동사 포함 snake_case (`start_scene`, `get_history`)
4. **이 문서 업데이트** — 매핑표에 새 행 추가

### 이름 불일치 추적

현재 불일치 사례 (기능은 동일, 이름만 다름):

| REST | MCP | 정렬 방향 |
|---|---|---|
| `POST /api/stimulus` | `apply_stimulus` | MCP 쪽 동사 유지 |
| `POST /api/guide` | `generate_guide` | MCP 쪽 동사 유지 |
| `POST /api/scene` | `start_scene` | MCP 쪽 동사 유지 |
| `PUT /api/situation` | `update_situation` | MCP는 HTTP 메서드 구분 없으므로 동사 필수 |
| `PUT /api/test-report` | `update_test_report` | 동일 |

REST는 HTTP 메서드(GET/POST/PUT/DELETE)로 동사를 표현하고 MCP는 이름 자체에 동사를 포함한다. 이 차이는 각 프로토콜의 규약이므로 통일하지 않는다.

---

## 기능 카운트 스냅샷 (2026-04)

- REST endpoints: 27개 (base 23 + chat feature 4)
- MCP tools: 35개
- MCP-only: 5개 (`list_source_texts`, `read_source_text`, `create_full_scenario`, `get_npc_llm_config`, `get_next_utterance`)
- REST-only: 1개 (`/api/chat/message/stream`)

**현재 MCP는 REST의 상위 집합이다.** (streaming 제외)
