# MCP Tools Reference

> **소스**: `src/bin/mind-studio/mcp_server.rs`의 `list_tools()` 함수가 정식 소스다.
> 이 문서는 그 내용을 사람이 읽기 쉽게 정리한 레퍼런스이며, 실제 스키마와 불일치 시
> 소스 코드가 우선한다.
>
> **워크플로우 예시와 조합 사용법**은 [`agent-playbook.md`](agent-playbook.md) 참조.
> 이 문서는 각 도구의 API 스펙만 다룬다.

---

## 목차

1. [세계 구축 CRUD](#세계-구축-crud-9개) (9개)
2. [감정 파이프라인](#감정-파이프라인-5개) (5개)
3. [상태 관리](#상태-관리-5개) (5개)
4. [시나리오 관리](#시나리오-관리-4개) (4개)
5. [소스 텍스트 & 시나리오 생성](#소스-텍스트--시나리오-생성-3개) (3개)
6. [Scene 관리](#scene-관리-2개) (2개)
7. [결과 관리](#결과-관리-2개) (2개)
8. [LLM 대화 테스트](#llm-대화-테스트-4개-chat-feature) (4개)
9. [기타](#기타-1개) (1개)

총 **35개** 도구.

---

## 세계 구축 CRUD (9개)

NPC, 관계, 오브젝트의 생성·조회·삭제. 모든 CRUD 도구는 `scenario_modified = true`를
설정하므로 이후 `save_scenario`로 저장해야 영구 반영된다.

### `list_npcs`

등록된 모든 NPC 목록을 조회한다.

- **입력**: 없음
- **반환**: `NpcProfile[]` — HEXACO 24 facets 포함 전체 프로필 배열

### `create_npc`

NPC를 생성하거나 기존 ID를 덮어쓴다 (upsert).

- **입력**: `{ npc: NpcProfile }`
  - `npc.id` (필수), `npc.name`, `npc.description`
  - HEXACO 24 facets (sincerity, fairness, ..., unconventionality)
- **반환**: `{ status: "ok" }`

### `delete_npc`

NPC를 삭제한다.

- **입력**: `{ id: string }`
- **반환**: `{ status: "ok" }`

### `list_relationships`

모든 관계 목록을 조회한다. 키는 `owner_id:target_id` 형식.

- **입력**: 없음
- **반환**: `RelationshipData[]`

### `create_relationship`

관계를 생성하거나 덮어쓴다.

- **입력**: `{ rel: RelationshipData }`
  - `owner_id`, `target_id` (필수)
  - `closeness`, `trust`, `power` (-1.0 ~ 1.0)
- **반환**: `{ status: "ok" }`

### `delete_relationship`

관계를 삭제한다.

- **입력**: `{ owner_id: string, target_id: string }`
- **반환**: `{ status: "ok" }`

### `list_objects` / `create_object` / `delete_object`

씬에 등장하는 사물·장소. NPC와 동일한 CRUD 패턴.

- `list_objects` — 입력 없음, `ObjectEntry[]` 반환
- `create_object` — `{ obj: ObjectEntry }` 입력, `{ status: "ok" }` 반환
- `delete_object` — `{ id: string }` 입력, `{ status: "ok" }` 반환

---

## 감정 파이프라인 (5개)

엔진의 핵심. 상황 해석 → OCC 감정 생성 → PAD 자극 적용 → 관계 반영.

### `appraise`

상황을 평가하여 OCC 감정을 생성하고 LLM 연기 프롬프트를 반환한다.

- **입력**:
  ```json
  {
    "npc_id": "string",
    "partner_id": "string",
    "situation": { "event": {...}, "action": {...}, "object": {...} }
  }
  ```
- **반환**: `AppraiseResponse` — 생성된 OCC 감정 목록, dominant emotion, mood, PAD 위치,
  LLM 연기용 프롬프트 포함

### `apply_stimulus`

PAD 자극을 적용해 감정 강도를 갱신하고 Beat 전환 트리거를 체크한다.

- **입력**: `{ req: StimulusRequest }`
  - `req.npc_id`, `req.pad: { pleasure, arousal, dominance }`
- **반환**: 갱신된 감정 상태 + `beat_changed: bool` + `active_focus_id`

### `analyze_utterance`

대사 텍스트를 PAD 3축 수치로 분석한다. **`embed` feature 필요**.

- **입력**: `{ utterance: string }` — 순수 대사만 (지문 제외)
- **반환**: `{ pleasure: f32, arousal: f32, dominance: f32 }` (각 -1.0 ~ 1.0)
- **주의**: 화자 관점 분석. 청자 관점 PAD는 별도 변환 필요 (미구현)

### `generate_guide`

현재 감정 상태를 기반으로 연기 가이드를 재생성한다.

- **입력**: `{ req: GuideRequest }`
  - `req.npc_id`, `req.partner_id`, `req.situation_description` (선택 — 생략 시 현재 상황에서 자동 추출)
- **반환**: 포매팅된 연기 가이드 (prompt 필드 포함)

### `after_dialogue`

대화를 종료하고 누적된 감정 상태를 관계 수치 변화에 반영한다.

- **입력**: `{ req: AfterDialogueRequest }`
  - `req.npc_id`, `req.partner_id`, `req.significance` (0.0 ~ 1.0)
- **반환**: before/after 관계 수치 (closeness/trust/power delta)

---

## 상태 관리 (5개)

대화 턴 히스토리, 현재 상황, 테스트 레포트의 조회/수정.

### `get_history`

현재 세션의 턴별 히스토리를 조회한다.

- **입력**: 없음
- **반환**: `TurnRecord[]` — turn별 input_pad, trace, 감정 상태 스냅샷 포함

### `get_situation`

현재 상황 설정 패널의 상태를 조회한다 (WebUI 동기화용).

- **입력**: 없음
- **반환**: 현재 situation JSON 객체 또는 `null`

### `update_situation`

상황 설정 패널 상태를 업데이트한다. WebUI와 MCP 클라이언트 간 동기화 목적.

- **입력**: `{ body: object }` — 임의의 situation JSON
- **반환**: `{ status: "ok" }`

### `get_test_report`

메모리에 저장된 테스트 레포트(마크다운)를 조회한다.

- **입력**: 없음
- **반환**: `{ content: string }`

### `update_test_report`

테스트 레포트를 작성/덮어쓰기한다. 메모리에만 저장되므로 파일로 남기려면
`save_scenario(save_type="report" | "all")` 추가 호출 필요.

- **입력**: `{ content: string }` — 마크다운 형식
- **반환**: `{ status: "ok" }`

---

## 시나리오 관리 (4개)

시나리오 JSON 파일의 목록/로드/저장.

### `list_scenarios`

사용 가능한 시나리오 파일 목록을 조회한다.

- **입력**: 없음
- **반환**: 시나리오 파일 경로/이름 배열

### `get_scenario_meta`

현재 로드된 시나리오의 메타데이터(name, description, notes)를 조회한다.

- **입력**: 없음
- **반환**: scenario 메타 객체

### `save_scenario`

현재 상태를 지정된 경로에 저장한다. 4가지 모드 지원.

- **입력**:
  ```json
  {
    "path": "string",
    "save_type": "scenario" | "result" | "report" | "all"  // 선택
  }
  ```
- **save_type별 동작**:
  | 모드 | 저장 내용 | 확장자 |
  |---|---|---|
  | `"scenario"` | 시나리오 JSON (turn_history 제외) | `.json` |
  | `"result"` | 결과 JSON (turn_history 포함) — 기본값 | `.json` |
  | `"report"` | test_report 마크다운 | `.md` |
  | `"all"` | result JSON + report MD 동시 저장 | `.json` + `.md` |
- **반환**: `{ status: "ok", path, report_path?, saved: string[] }`

### `load_scenario`

지정된 경로의 시나리오 파일을 로드한다 (NPC/관계/오브젝트/Scene 복원).

- **입력**: `{ path: string }` — `data/` 하위 상대 경로 또는 절대 경로
- **반환**: `{ status: "ok", resolved_path: string }`

---

## 소스 텍스트 & 시나리오 생성 (3개)

원작 텍스트를 참조해서 시나리오를 자동 구성하는 AI 에이전트 전용 워크플로우.

### `list_source_texts`

`data/` 하위 `.txt` 파일 목록과 크기를 조회한다.

- **입력**: 없음
- **반환**: `{ files: [{ name, path, size_kb }] }`

### `read_source_text`

소스 텍스트 파일을 챕터 단위로 읽는다.

- **입력**:
  ```json
  {
    "path": "string",           // txt 파일 경로 (파일명만 줘도 됨)
    "chapter": 3                // 선택 — 생략 시 챕터 목록 반환
  }
  ```
- **반환**:
  - `chapter` 지정 시: `{ chapter, title, line_start, line_end, line_count, text }`
  - `chapter` 생략 시: `{ file, total_lines, chapter_count, chapters: [...] }`
- **챕터 감지 규칙**: 대문자로 시작하는 `CHAPTER `, `BOOK `, `PART ` 라인

### `create_full_scenario`

NPC + 관계 + 오브젝트 + Scene을 한 번에 생성하고 파일로 저장한 뒤 서버 상태에 로드한다.

- **입력**:
  ```json
  {
    "save_path": "string",   // data/ 하위 (예: "treasure_island/ch01/session_001/scenario.json")
    "scenario": {
      "scenario": { "name", "description", "notes" },
      "npcs": { "id": NpcProfile },
      "relationships": { "key": RelationshipData },
      "objects": { "id": ObjectEntry },
      "scene": SceneRequest
    }
  }
  ```
- **반환**: `{ status: "ok", path, npcs: N, relationships: M }`
- **주의**: relationships 키는 내부적으로 `owner_id:target_id` 형식으로 재생성됨

---

## Scene 관리 (2개)

Scene은 여러 Focus(심리 국면)와 Beat 전환 trigger를 담는 다단계 상황 컨테이너.

### `start_scene`

Scene을 시작한다. Focus 옵션 목록을 등록하고 초기 Focus를 자동으로 appraise한다.

- **입력**: `{ req: SceneRequest }`
  - `req.focuses`: Focus 배열 (각각 id, description, trigger, situation 포함)
  - `req.initial_focus_id`: 시작 Focus ID
- **반환**: 초기 Focus의 appraise 결과 (포맷된 연기 프롬프트 포함)

### `get_scene_info`

현재 Scene의 Focus 상태를 조회한다.

- **입력**: 없음
- **반환**:
  ```json
  {
    "has_scene": bool,
    "active_focus_id": "string",
    "focuses": [{ "id", "description", "trigger_display", "is_active" }]
  }
  ```
- **`trigger_display`**: 사람이 읽을 수 있는 trigger 조건 문자열 (예: `"(Fear > 0.7) OR (Distress > 0.7)"`)

---

## 결과 관리 (2개)

테스트 결과 파일의 경로 계산 및 로드.

### `get_save_dir`

현재 로드된 시나리오의 결과 저장 디렉토리 경로를 자동 계산한다.

- **입력**: 없음
- **반환**: 시나리오 파일명에서 `.json`을 제거한 디렉토리 경로
- **예**: 시나리오가 `data/treasure_island/ch26/duel.json`이면 → `data/treasure_island/ch26/duel/`

### `load_result`

테스트 결과 파일을 로드한다 (턴 히스토리 포함). Scene도 자동 복원.

- **입력**: `{ path: string }`
- **반환**: `{ status: "ok", resolved_path, turn_count: N }`

---

## LLM 대화 테스트 (4개, `chat` feature)

로컬 LLM(OpenAI 호환 API)과 연동한 다중 턴 대화 세션. `chat` feature로 빌드해야 사용 가능.

### `dialogue_start`

대화 세션을 시작한다. appraise 결과의 프롬프트를 system prompt로 LLM 세션을 생성한다.

- **입력**:
  ```json
  {
    "session_id": "string",          // 세션 고유 ID
    "appraise": {                    // AppraiseRequest
      "npc_id": "string",
      "partner_id": "string",
      "situation": { ... }
    }
  }
  ```
- **반환**: 초기 appraise 결과 + 세션 상태
- **부가 동작**: Scene의 initial Focus로 자동 reset (stale active_focus_id 제거)

### `dialogue_turn`

상대 대사를 LLM에 전송하고 NPC 역할로 응답을 받는다. PAD 자극 자동 적용 + Beat 전환 체크.

- **입력**:
  ```json
  {
    "session_id": "string",
    "npc_id": "string",
    "partner_id": "string",
    "utterance": "string",                    // 순수 대사만
    "pad": { "pleasure": 0.0, "arousal": 0.0, "dominance": 0.0 },  // 선택 — 생략 시 자동 분석
    "situation_description": "string"         // 선택
  }
  ```
- **반환**: LLM의 NPC 연기 응답 + 갱신된 감정 상태 + `beat_changed` + `active_focus_id`
- **Beat 전환 시 동작**: system prompt가 새 Focus의 프롬프트로 교체됨

### `get_next_utterance`

현재 Beat의 `test_script`에서 다음 대사를 조회하고 커서를 전진한다.

- **입력**:
  ```json
  {
    "advance": true   // 선택 — 기본값 true. false이면 peek만 (커서 전진 없음)
  }
  ```
- **반환**:
  ```json
  {
    "utterance": "대사 텍스트",
    "beat_id": "cornered",
    "index": 0,
    "remaining": 2,
    "total": 3,
    "exhausted": false
  }
  ```
- **`exhausted: true`**: 해당 Beat의 스크립트가 모두 소진됨. 즉흥 대사로 전환 필요
- **커서 리셋**: `dialogue_start` 및 Beat 전환 시 자동으로 0으로 초기화
- **`dialogue_turn`과의 연동**: 스크립트 대사를 `dialogue_turn(utterance=...)`에 그대로 전달하면 커서가 이중 전진하지 않음 (일치 확인 후 건너뜀)

### `dialogue_end`

대화 세션을 종료한다. `after_dialogue`를 함께 전달하면 관계 수치를 갱신한다.

- **입력**:
  ```json
  {
    "session_id": "string",
    "after_dialogue": {                   // 선택
      "npc_id": "string",
      "partner_id": "string",
      "significance": 1.0
    }
  }
  ```
- **반환**: 전체 대화 이력 + (선택적) 관계 변화 delta

---

## 기타 (1개)

### `get_npc_llm_config`

NPC의 HEXACO 성격에 최적화된 LLM 생성 파라미터를 조회한다.

- **입력**: `{ npc_id: string }`
- **반환**: `{ npc_id, temperature: f32, top_p: f32 }`
- **용도**: 외부 LLM 클라이언트가 NPC 연기 시 파라미터 참고용. `dialogue_*` tool은
  내부적으로 이 값을 자동 적용하므로 직접 호출 불필요.

---

## 자동 생성 고려

이 문서는 현재 수동 작성 상태다. 장기적으로는 `list_tools()` 함수의 JSON 출력에서
문서를 자동 생성하는 스크립트가 필요하다. 후보 구현:

- **방식 A**: 서버 실행 후 `GET /mcp/sse`로 endpoint 받고 `POST /mcp/message`에
  `{"method": "tools/list"}` 전송 → 응답 JSON을 마크다운으로 변환하는 Python 스크립트
- **방식 B**: `list_tools()`를 호출하는 별도 Rust bin (`bin/dump_tools_reference.rs`)
  추가, 빌드 시 마크다운 생성
- **방식 C**: `cargo xtask gen-docs` 패턴으로 프로젝트 전체 문서 생성 통합

현재 우선순위는 낮음. tool 스키마가 자주 바뀌기 시작하면 착수한다.

---

## 참고

- 각 tool의 워크플로우 조합 및 사용 예시: [`agent-playbook.md`](agent-playbook.md)
- REST API 대응 관계: [`rest-parity.md`](rest-parity.md)
- 실제 구현: `src/bin/mind-studio/mcp_server.rs` (정식 소스)
- DTO 타입 정의: `src/application/dto/` (크레이트 루트)
