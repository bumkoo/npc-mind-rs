# NPC Mind Studio — MCP Server

`npc-mind-rs`의 NPC 심리 시뮬레이션 엔진을 MCP(Model Context Protocol) 인터페이스로
노출하는 서버다. HEXACO 성격 모델 → OCC 감정 평가 → PAD 공간 매핑 파이프라인을
AI 에이전트가 도구 호출로 활용할 수 있게 해준다.

AI 에이전트(Claude Desktop, Claude Code 등)는 이 MCP를 통해 NPC를 만들고, 대화 Scene을
구성하고, 대사 입력에 대한 NPC의 감정 반응과 연기 가이드(Acting Directive)를 받을 수 있다.

## 주요 특징

- **네이티브 Rust SSE 구현** — Axum 기반, 외부 MCP 라이브러리 의존 없음
- **34개 도구 제공** — NPC/관계/오브젝트 CRUD, 감정 파이프라인, Scene 제어, 대화 세션
- **HEXACO 24 facets × OCC 22 emotions × PAD 3축** — 이론 기반 심리 모델링
- **도메인 주도 설계** — 헥사고날 아키텍처로 MCP는 adapter 계층에 위치

## 빠른 시작

### 서버 실행

```powershell
cargo run --release --features mind-studio,embed --bin npc-mind-studio
```

- HTTP: `http://127.0.0.1:3000`
- SSE 엔드포인트: `GET /mcp/sse` (endpoint 이벤트로 `/mcp/message?session_id=...` URL 전달)
- 메시지 엔드포인트: `POST /mcp/message?session_id=...`
- `embed` feature: `analyze_utterance` tool에서 BGE-M3 임베딩 사용 (필수)
- `chat` feature: `dialogue_*` tool에서 LLM 연기 응답 (선택)

### Claude Desktop 연결

`claude_desktop_config.json`에 다음 추가:

```json
{
  "mcpServers": {
    "npc-mind-studio": {
      "url": "http://127.0.0.1:3000/mcp/sse"
    }
  }
}
```

Claude Desktop을 완전히 종료 후 재시작하면 `npc-mind-studio:*` 도구들이 목록에 나타난다.

## 도구 카테고리 (34개)

`mcp_server.rs`의 카테고리 구분을 그대로 따른다.

### 세계 구축 CRUD (9개)

NPC, 관계, 오브젝트의 생성·조회·삭제. 시나리오 파일 로드 후 상태를 조정하거나
`create_full_scenario`로 한 번에 만드는 대신 개별 제어할 때 사용한다.

| 도구 | 용도 |
|---|---|
| `list_npcs` / `create_npc` / `delete_npc` | NPC HEXACO 프로필 CRUD |
| `list_relationships` / `create_relationship` / `delete_relationship` | closeness/trust/power CRUD |
| `list_objects` / `create_object` / `delete_object` | 씬 등장 사물·장소 CRUD |

### 감정 파이프라인 (5개)

엔진의 핵심. 상황 해석 → 감정 생성 → PAD 자극 적용 → 관계 반영의 전 과정.

| 도구 | 용도 |
|---|---|
| `appraise` | 상황 → OCC 감정 생성 + LLM 연기 프롬프트 |
| `apply_stimulus` | PAD 자극으로 감정 강도 갱신 + Beat 전환 체크 |
| `analyze_utterance` | 대사 → PAD 자동 분석 (embed feature) |
| `generate_guide` | 현재 감정 상태 기반 연기 가이드 재생성 |
| `after_dialogue` | 대화 종료 후 감정을 관계 변화에 반영 |

### 상태 관리 (5개)

| 도구 | 용도 |
|---|---|
| `get_history` | 턴별 히스토리 조회 (trace + input_pad 포함) |
| `get_situation` / `update_situation` | 상황 설정 패널 상태 (WebUI 동기화용) |
| `get_test_report` / `update_test_report` | 마크다운 테스트 레포트 메모리 관리 |

### 시나리오 관리 (4개)

| 도구 | 용도 |
|---|---|
| `list_scenarios` | 사용 가능한 시나리오 파일 목록 |
| `get_scenario_meta` | 현재 로드된 시나리오의 메타데이터 |
| `save_scenario` | `scenario` / `result` / `report` / `all` 모드로 저장 |
| `load_scenario` | 시나리오 파일 로드 (NPC/관계/Scene 복원) |

### 소스 텍스트 & 시나리오 생성 (3개)

원작 텍스트를 참조하여 시나리오를 생성하는 워크플로우.

| 도구 | 용도 |
|---|---|
| `list_source_texts` | `data/` 하위 `.txt` 파일 목록 |
| `read_source_text` | 챕터 단위로 원작 텍스트 읽기 |
| `create_full_scenario` | NPC + 관계 + 오브젝트 + Scene 일괄 생성 및 저장 |

### Scene 관리 (2개)

| 도구 | 용도 |
|---|---|
| `start_scene` | Focus 옵션 등록 + 초기 Focus 자동 appraise |
| `get_scene_info` | 활성 Focus, 대기 Focus, trigger 조건 조회 |

### 결과 관리 (2개)

| 도구 | 용도 |
|---|---|
| `get_save_dir` | 현재 시나리오의 결과 저장 디렉토리 자동 계산 |
| `load_result` | 턴 히스토리 포함 결과 파일 로드 (Scene 자동 복원) |

### LLM 대화 테스트 (3개, `chat` feature 필요)

| 도구 | 용도 |
|---|---|
| `dialogue_start` | appraise 프롬프트를 system prompt로 LLM 세션 시작 |
| `dialogue_turn` | 대사 전송 → NPC 연기 응답 + PAD 자동 적용 + Beat 갱신 |
| `dialogue_end` | 대화 종료 + (선택) `after_dialogue` 관계 갱신 |

### 기타 (1개)

| 도구 | 용도 |
|---|---|
| `get_npc_llm_config` | NPC 성격에 최적화된 LLM temperature/top_p 조회 |

## 문서

- **[`docs/agent-playbook.md`](docs/agent-playbook.md)** — ⭐ **AI 에이전트 필독**.
  도구를 어떤 순서·조합으로 호출해야 하는지, 도메인 개념(Beat/Trigger/Focus/PAD)을
  어떻게 이해해야 하는지를 담은 워크플로우 가이드. 이 MCP를 쓰는 AI는 이 문서부터 읽는다.
- **[`docs/rest-parity.md`](docs/rest-parity.md)** — REST API와 MCP tool의 대응 매핑표. 개발자용 내부 레퍼런스.
- **[`docs/tools-reference.md`](docs/tools-reference.md)** — 34개 도구의 입력/반환 스펙. 카테고리별 레퍼런스.
- **[`docs/architecture.md`](docs/architecture.md)** — SSE 이원 엔드포인트 구조, DDD adapter 위치, JSON-RPC 처리 흐름.
- **[`docs/troubleshooting.md`](docs/troubleshooting.md)** — 연결 실패, 빌드 함정, Windows 환경 특수성, 알려진 엔진 제약.
