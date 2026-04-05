# Troubleshooting

> MCP 서버를 실행·연결·사용하면서 자주 마주치는 문제와 해결 방법. 증상별로 찾아볼 수 있게
> 구성했다. 여기에 없는 문제를 해결하면 이 문서에 추가하자.

---

## 빠른 진단

| 증상 | 가장 먼저 확인할 것 |
|---|---|
| MCP 도구가 Claude에 안 보임 | 서버 실행 여부, Claude Desktop 완전 재시작 |
| `analyze_utterance` 에러 | `embed` feature로 빌드했는지 |
| `dialogue_*` 도구 없음 | `chat` feature로 빌드했는지 |
| `has_scene: false` | 시나리오 JSON에 `scene` 필드 존재 여부 |
| PAD D축이 항상 0.00 | 알려진 제약 — 수동 입력 필요 |
| `cargo check` 성공했는데 실제 실행 실패 | feature flag 누락 |

---

## 연결 문제

### MCP 도구가 Claude Desktop에 나타나지 않음

**원인 후보**:
1. 서버가 실행 중이지 않음
2. Claude Desktop이 재시작되지 않음
3. `claude_desktop_config.json` 설정 오류

**진단 순서**:

```powershell
# 1. 서버 응답 확인
curl http://127.0.0.1:3000/api/npcs
# → JSON 배열이 와야 함. 연결 거부면 서버 미실행.

# 2. MCP SSE 엔드포인트 직접 확인
curl -N http://127.0.0.1:3000/mcp/sse
# → "event: endpoint" 이벤트가 와야 함
```


**해결**:
- 서버 실행: `cargo run --release --features mind-studio,embed --bin npc-mind-studio`
- Claude Desktop **완전 종료** (트레이 아이콘까지) 후 재시작
- `claude_desktop_config.json` 형식 확인:
  ```json
  {
    "mcpServers": {
      "npc-mind-studio": {
        "url": "http://127.0.0.1:3000/mcp/sse"
      }
    }
  }
  ```

### 연결은 됐는데 도구 호출이 무응답

**원인**: MCP SSE transport의 이원 엔드포인트 패턴 때문. POST는 즉시 ack만 반환하고 실제
결과는 SSE 스트림으로 비동기 도착한다. 클라이언트가 SSE 스트림을 읽지 않으면 결과가 안 온다.
자세한 흐름은 [`architecture.md`](architecture.md#2-sse-transport-구조) 참조.

**해결**: 표준 MCP 클라이언트(Claude Desktop, Claude Code 등)를 사용하자. 직접 MCP 클라이언트를
구현하는 경우 SSE 스트림을 반드시 유지해야 한다.

---

## 서버 실행/빌드 문제

### `cargo check`는 통과하는데 서버 실행 시 에러

**원인**: `cargo check`가 기본 feature만 검사하기 때문. Mind Studio 바이너리는 `mind-studio` feature가 필요하다.

**해결**: 항상 feature flag를 명시하자.

```powershell
# ❌ 잘못된 확인 — mind-studio 바이너리 안 컴파일됨
cargo check

# ✅ 올바른 확인
cargo check --features mind-studio,embed

# chat 기능도 같이 쓰려면
cargo check --features mind-studio,embed,chat
```

### feature별 영향 범위

| Feature | 영향받는 도구 |
|---|---|
| `mind-studio` | Mind Studio 바이너리 전체 (모든 MCP 도구) |
| `embed` | `analyze_utterance` |
| `chat` | `dialogue_start`, `dialogue_turn`, `dialogue_end` |

### BGE-M3 모델 로드 실패

**증상**: `embed` feature로 빌드했지만 `analyze_utterance` 호출 시 "analyzer not available" 에러.

**원인 후보**:
1. 모델 파일 경로 오류
2. `tokenizer.json` 누락
3. ONNX 런타임 초기화 실패

**해결**:
```powershell
# 기본 경로 확인: ../models/bge-m3/
ls C:\Users\bumko\projects\models\bge-m3\

# 필요 파일:
# - model_quantized.onnx
# - tokenizer.json

# 환경변수로 경로 변경 가능
$env:NPC_MIND_MODEL_DIR = "C:\path\to\bge-m3"
```

모델 초기화는 수십 초 걸릴 수 있음. 서버 시작 로그에서 에러 메시지 확인.


### `npc-mind-studio.exe`가 `taskkill`로 종료되지 않음

**증상**: 서버 재빌드 전 프로세스 종료 시 `taskkill /F`가 "Access Denied" 반환.

**원인**: Windows 권한 제한. 관리자 권한 없이는 강제 종료 불가한 경우 있음.

**해결책**:
- **Ctrl+C** — 서버 터미널에서 직접 종료 (가장 권장)
- **관리자 PowerShell**로 `taskkill /F /IM npc-mind-studio.exe`
- 작업 관리자 → 세부 정보 → 프로세스 우클릭 → 작업 끝내기

재빌드할 때 기존 프로세스가 파일을 락하고 있으면 링크 실패함. 종료 확인 필수.

---

## MCP 도구 실행 문제

### "세션을 찾을 수 없습니다" 에러

**원인 후보**:
1. `dialogue_start` 없이 `dialogue_turn` 호출
2. `dialogue_end`로 이미 종료된 세션 재사용
3. 서버 재시작으로 메모리 세션 소실

**해결**: 항상 `dialogue_start`로 새 세션을 만들고, 각 호출에서 같은 `session_id` 사용.
서버 재시작 시 세션 상태는 완전 초기화되므로 `load_scenario`부터 다시 시작.

### `has_scene: false` 반환

**증상**: `start_scene` 또는 `load_scenario` 후 `get_scene_info` 호출 시 `has_scene: false`.

**원인 후보**:
1. 시나리오 JSON에 `scene` 필드 자체가 없음
2. `focuses` 구조 오류
3. `trigger` 형식 오류

**해결**: 시나리오 JSON의 `scene` 필드 형식 확인.

```json
{
  "scene": {
    "focuses": [
      {
        "id": "cornered",
        "description": "...",
        "event": { ... },
        "action": { ... },
        "trigger": [
          [{ "emotion": "Fear", "above": 0.7 }],
          [{ "emotion": "Distress", "above": 0.7 }]
        ]
      }
    ],
    "initial_focus_id": "cornered"
  }
}
```

- `focuses`는 **배열** (객체 아님)
- `trigger`는 **이중 중첩 배열**: 바깥 배열=OR, 안쪽 배열=AND
- 각 조건은 `{emotion, above/below}` 형식


### Beat가 매 턴 전환됨

**증상**: `dialogue_turn` 호출 시마다 `beat_changed: true`가 반환되어 Focus가 계속 바뀜.

**원인 후보**:
1. `check_trigger`가 활성 focus를 제외하지 않음 (state latching 버그)
2. 여러 Focus의 trigger 조건이 동시에 충족됨 (디자인 문제)

**해결**:
- 엔진 레벨: `src/domain/emotion/scene.rs`의 `check_trigger`가 활성 focus 제외하는지 확인
  (2026-04 수정 완료, `cargo test scene`으로 검증)
- 시나리오 레벨: trigger 조건이 서로 배타적인지 재설계. 자세한 내용은
  [`agent-playbook.md`](agent-playbook.md#5-beat-전환-관찰) 참조

### `update_test_report` 성공했는데 파일이 없음

**증상**: `update_test_report`가 성공 반환했는데 디스크에서 파일을 찾을 수 없음.

**원인**: `update_test_report`는 **메모리에만 저장**한다. 파일로 쓰려면 별도 호출 필요.

**해결**:
```
# 방법 1: report만 파일로
save_scenario(path="...", save_type="report")

# 방법 2: result JSON + report MD 동시 저장 (권장)
save_scenario(path="...", save_type="all")
```

### `save_type="all"` 사용 시 `.md` 파일만 안 생김

**원인**: `test_report` 내용이 비어있음. 구현상 result 저장은 성공하고 report는 조용히 skip됨.

**해결**: `save_scenario` 전에 `update_test_report`로 내용 먼저 채울 것.

---

## 알려진 엔진 제약

### PAD D축이 항상 0.00

**증상**: `analyze_utterance`가 반환하는 dominance 값이 거의 0에 가까움.

**원인**: D축 앵커의 구조적 한계. 지배/굴종 키워드가 상황별로 공유되기 때문에 discrimination 천장이 ~76%.

**해결**:
- 수동 PAD 입력으로 우회: `dialogue_turn(pad={dominance: 0.7, ...})` 명시적 전달
- 판단 기준표는 [`agent-playbook.md`의 PAD 입력 규칙](agent-playbook.md#4-pad-입력-규칙) 참조

### `analyze_utterance`가 청자의 반응을 반영하지 않음

**원인**: 현재는 **화자(speaker) 톤**만 분석. 청자(listener)의 PAD 반응은 관계/해석에 따라 달라짐.

**해결**: 청자 관점 변환 파이프라인은 미구현. 수동 PAD 입력으로 우회.
(향후 speech act classification, relationship-based transformation 등 도입 예정)

### 세션이 오래 쌓이면 메모리 증가

**원인**: `McpSessionManager`의 세션 cleanup 미구현. SSE 연결이 끊겨도 `HashMap` 엔트리가 남음.

**해결**: 개발 환경에서는 무시 가능. 장시간 운영 시 서버 재시작으로 리셋.
프로덕션 전 cleanup 로직 추가 필요 (알려진 과제).

### 대화 응답 스트리밍 불가

**증상**: `dialogue_turn`이 LLM 응답을 통째로 기다렸다가 한 번에 반환.

**원인**: MCP tool 응답 규약이 단일 `CallToolResult`임. 스트리밍은 구조적으로 불가.

**해결**: 스트리밍이 필요하면 REST `POST /api/chat/message/stream` 엔드포인트 직접 사용.


---

## Windows 환경 특수성

### `curl`로 JSON POST 보낼 때 인용부호 깨짐

**증상**: Windows CMD/PowerShell에서 `curl -d '{"key": "value"}'` 실행 시 JSON 파싱 에러.

**원인**: Windows shell의 quote stripping 동작이 불안정.

**해결**: Python `urllib.request` 사용 권장.

```python
# test_request.py
import urllib.request
import json

data = json.dumps({"npc_id": "jim", "partner_id": "israel"}).encode("utf-8")
req = urllib.request.Request(
    "http://127.0.0.1:3000/mcp/message?session_id=test",
    data=data,
    headers={"Content-Type": "application/json"}
)
response = urllib.request.urlopen(req)
print(response.read().decode("utf-8"))
```

실행:
```powershell
chcp 65001 > nul
python -X utf8 test_request.py
```

`chcp 65001`은 Windows 콘솔을 UTF-8 모드로 전환. 한글 JSON 깨짐 방지.

### PowerShell UTF-8 처리

**증상**: PowerShell에서 JSON 저장 시 BOM이 붙거나 인코딩이 CP949로 바뀜.

**원인**: `Set-Content`가 기본적으로 시스템 인코딩 사용.

**해결**: .NET 직접 호출.

```powershell
# 요청 body 인코딩
$json = '{"key": "value"}'
$bytes = [System.Text.Encoding]::UTF8.GetBytes($json)

# 파일 저장
[System.IO.File]::WriteAllText("path.json", $jsonContent, [System.Text.UTF8Encoding]::new($false))
# 두 번째 인자 $false는 BOM 없는 UTF-8
```

### 한글 로그 깨짐

**해결**: 서버 실행 전 콘솔 코드페이지 변경.

```powershell
chcp 65001
cargo run --release --features mind-studio,embed --bin npc-mind-studio
```

---

## 개발 도구 사용 시 주의

### Desktop Commander `edit_block` 조용한 실패

**증상**: `edit_block` 호출 후 "Successfully applied" 반환했지만 파일이 안 바뀜.

**원인**: `old_string`과 `new_string`이 완전히 동일할 때 도구가 match만 확인하고 조용히 성공 반환함.

**해결**:
- 수정 후 `read_file`로 실제 변경 확인
- old/new가 다른지 직접 검토
- 공백·줄바꿈까지 정확히 일치하는지 확인

### 모델 로드 시 테스트 타임아웃

**증상**: `cargo test` 실행 중 embed 관련 테스트가 timeout.

**해결**: 타임아웃을 충분히 길게 설정. BGE-M3 INT8 로드는 시간이 꽤 걸림.

```
# Desktop Commander의 start_process 사용 시
timeout_ms: 300000  # 5분
# 느린 머신에서는 600000  # 10분
```


---

## 데이터/파일 경로 문제

### `load_scenario` 경로 해석

**헷갈리는 점**: path 파라미터가 절대 경로인지, `data/` 하위 상대 경로인지.

**동작**:
- 절대 경로 (`C:\...`) → 그대로 사용
- `data/` 또는 `data\`로 시작 → 그대로 사용
- 그 외 → 자동으로 `data/` 접두사 추가

**예시**:
```
load_scenario(path="treasure_island/ch01/scenario.json")
# → data/treasure_island/ch01/scenario.json 로드

load_scenario(path="data/treasure_island/ch01/scenario.json")
# → data/treasure_island/ch01/scenario.json 로드 (중복 안 됨)
```

### `get_save_dir` 결과와 실제 디렉토리 구조 불일치

**증상**: `get_save_dir`가 반환한 경로에 시나리오와 관련 없는 파일이 섞여 있음.

**원인**: 서버가 디렉토리를 만들지 않음. 저장 시에만 자동 생성. 이전 테스트 잔재가 남아 있을 수 있음.

**해결**: 세션별 하위 디렉토리 사용 권장.
```
data/treasure_island/ch26/duel/
  session_001.json
  session_001.md
  session_002.json
  session_002.md
```

---

## 디버깅 팁

### 서버 로그 레벨 조절

```powershell
# 기본: npc_mind_studio=debug, npc_mind=trace
# MCP 관련만 보고 싶을 때
$env:RUST_LOG = "npc_mind_studio::mcp_server=debug,tower_http=info"
cargo run --release --features mind-studio,embed --bin npc-mind-studio
```

### MCP 요청 흐름 추적

서버는 모든 MCP 요청을 로그에 남긴다. 검색 키워드:

```
[MCP] SSE 연결          → 세션 생성
[MCP] 요청: method=...  → 클라이언트 요청 수신
[MCP] tools/call        → 도구 호출 시작
[MCP] SSE 응답 전송     → 결과를 SSE로 push
[MCP] SSE 에러 전송     → 실패한 요청
```

### 도구 단위 테스트

REST 엔드포인트로 MCP 도구와 같은 로직을 직접 테스트 가능. MCP 레이어를 제외한 순수 로직 검증에 유용.

```python
# appraise 직접 호출 (MCP 우회)
import urllib.request, json
data = json.dumps({
    "npc_id": "jim",
    "partner_id": "israel_hands",
    "situation": {...}
}).encode("utf-8")
req = urllib.request.Request(
    "http://127.0.0.1:3000/api/appraise",
    data=data,
    headers={"Content-Type": "application/json"}
)
print(urllib.request.urlopen(req).read().decode("utf-8"))
```

REST ↔ MCP 매핑은 [`rest-parity.md`](rest-parity.md) 참조.

---

## 그래도 해결 안 되면

1. 서버 로그 전체 캡처 (특히 MCP 요청 전후)
2. 재현 단계 기록 (어떤 도구를 어떤 순서로 호출했는지)
3. `cargo test --features mind-studio,embed,chat`으로 회귀 여부 확인
4. 해결되면 이 문서에 증상/원인/해결 추가

관련 문서:
- [`agent-playbook.md`](agent-playbook.md) — 도구 사용 규약
- [`architecture.md`](architecture.md) — 서버 내부 구조
- [`tools-reference.md`](tools-reference.md) — 도구 API 스펙
- [`rest-parity.md`](rest-parity.md) — REST↔MCP 매핑
