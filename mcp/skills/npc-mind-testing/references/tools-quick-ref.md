# MCP Tools Quick Reference (테스트 세션 관련)

> 전체 34개 도구 중 테스트 세션에 주로 사용하는 도구만 발췌.

## LLM 대화 테스트 (chat feature)

| 도구 | 입력 | 반환 |
|---|---|---|
| `dialogue_start` | `{ session_id, appraise: { npc_id, partner_id, situation } }` | 초기 appraise + 세션 상태 |
| `dialogue_turn` | `{ session_id, npc_id, partner_id, utterance, pad?, situation_description? }` | LLM 응답 + 감정 갱신 + beat_changed |
| `dialogue_end` | `{ session_id, after_dialogue? }` | 대화 이력 + 관계 delta |

## 감정 파이프라인

| 도구 | 입력 | 반환 |
|---|---|---|
| `appraise` | `{ npc_id, partner_id, situation }` | OCC 감정 + 연기 프롬프트 |
| `apply_stimulus` | `{ req: { npc_id, pad: {P,A,D} } }` | 감정 갱신 + beat_changed |
| `analyze_utterance` | `{ utterance }` | PAD 3축 수치 |
| `generate_guide` | `{ req: { npc_id, partner_id } }` | 연기 가이드 재생성 |
| `after_dialogue` | `{ req: { npc_id, partner_id, significance } }` | before/after 관계 delta |

## 상태 관리

| 도구 | 입력 | 용도 |
|---|---|---|
| `get_history` | 없음 | 턴별 히스토리 (trace + PAD) |
| `get_test_report` | 없음 | 메모리 레포트 조회 |
| `update_test_report` | `{ content }` | 레포트 작성 (메모리) |

## 시나리오·Scene 관리

| 도구 | 입력 | 용도 |
|---|---|---|
| `load_scenario` | `{ path }` | 시나리오 로드 |
| `load_result` | `{ path }` | 결과 파일 로드 (Scene 복원) |
| `save_scenario` | `{ path, save_type }` | 저장 (result/all 권장) |
| `get_save_dir` | 없음 | 결과 디렉토리 경로 |
| `get_scene_info` | 없음 | Focus 상태 조회 |
| `get_scenario_meta` | 없음 | 시나리오 메타 |

## 기타

| 도구 | 입력 | 용도 |
|---|---|---|
| `get_npc_llm_config` | `{ npc_id }` | 성격 기반 temperature/top_p |
