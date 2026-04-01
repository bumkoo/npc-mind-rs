# NPC Mind Studio MCP Server

AI Agent(Claude Code)가 Mind Studio HTTP API를 통해 NPC 시나리오를 설계/테스트할 수 있는 MCP 서버입니다.

## 사전 준비

### 1. Mind Studio 서버 실행

```bash
# npc-mind-rs 프로젝트에서
cargo run --features mind-studio --bin npc-mind-studio
# http://127.0.0.1:3000 에서 실행됨
```

### 2. Python 의존성 설치

```bash
pip install -r mcp/requirements.txt
```

## 다른 프로젝트에서 사용하기

AI Agent가 작동할 프로젝트의 `.mcp.json`에 추가:

```json
{
  "mcpServers": {
    "mind-studio": {
      "command": "python",
      "args": ["/absolute/path/to/npc-mind-rs/mcp/mind_studio_server.py"],
      "env": {
        "MIND_STUDIO_URL": "http://localhost:3000"
      }
    }
  }
}
```

> `MIND_STUDIO_URL` 환경변수를 생략하면 기본값 `http://localhost:3000`을 사용합니다.

## 제공되는 도구 (16개)

### 세계 구축 (CRUD)

| 도구 | 설명 |
|------|------|
| `list_npcs` | NPC 목록 조회 |
| `create_npc` | NPC 생성/수정 (HEXACO 24 facets) |
| `delete_npc` | NPC 삭제 |
| `list_relationships` | 관계 목록 조회 |
| `create_relationship` | 관계 생성/수정 (closeness/trust/power) |
| `list_objects` | 오브젝트 목록 조회 |
| `create_object` | 오브젝트 생성/수정 |

### 감정 파이프라인

| 도구 | 설명 |
|------|------|
| `appraise` | 상황 평가 → OCC 감정 + LLM 프롬프트 생성 |
| `apply_stimulus` | PAD 자극 적용 → 감정 갱신 + Beat 전환 |
| `generate_guide` | 현재 감정으로 프롬프트 재생성 |
| `after_dialogue` | 대화 종료 → 관계 갱신 + 감정 초기화 |

### Scene 관리

| 도구 | 설명 |
|------|------|
| `start_scene` | Scene 시작 (Focus/Beat 등록 + 초기 감정 평가) |
| `get_scene_info` | 현재 Scene Focus 상태 조회 |

### 시나리오 관리

| 도구 | 설명 |
|------|------|
| `save_scenario` | 현재 상태를 JSON 파일로 저장 |
| `load_scenario` | 시나리오 JSON 로드 |
| `list_scenarios` | 사용 가능한 시나리오 목록 |

## AI Agent 워크플로우 예시

```
1. create_npc(id="jim", name="짐", description="온순하고 감성적인 인물",
              sincerity=0.7, fearfulness=0.6, sentimentality=0.8, patience=0.7)

2. create_npc(id="huck", name="헉", description="자유분방한 소년",
              sincerity=-0.3, fearfulness=-0.5, unconventionality=0.8)

3. create_relationship(owner_id="jim", target_id="huck",
                       closeness=0.55, trust=0.6, power=-0.3)

4. start_scene(npc_id="jim", partner_id="huck",
               description="안개 속 재회",
               focuses_json='[
                 {"id":"betrayal", "description":"거짓말 발각", "trigger":null,
                  "event":{"description":"헉이 거짓말로 속였다", "desirability_for_self":-0.8},
                  "action":{"description":"기만 행위", "agent_id":"huck", "praiseworthiness":-0.8}},
                 {"id":"apology", "description":"사과 수용", 
                  "trigger":[[{"emotion":"Anger","below":0.4},{"emotion":"Distress","below":0.3}]],
                  "event":{"description":"헉이 진심으로 사과", "desirability_for_self":0.7},
                  "action":{"description":"자존심을 꺾고 사과", "agent_id":"huck", "praiseworthiness":0.7}}
               ]')

5. # 프롬프트 확인 → 감정이 적절한지 검토

6. apply_stimulus(npc_id="jim", partner_id="huck",
                  pleasure=0.3, arousal=-0.2, dominance=0.1)
   # → beat_changed=true이면 사과 Beat로 전환됨

7. after_dialogue(npc_id="jim", partner_id="huck",
                  praiseworthiness=0.3, significance=0.7)

8. save_scenario(path="data/my_scenario/scenario.json")
```
