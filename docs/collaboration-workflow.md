# NPC Mind Engine — 협업 워크플로우

## 개요

이 문서는 Bekay와 Claude가 NPC 심리 엔진을 **반복적으로 개선**하기 위한 협업 철학과 역할 분담을 정의한다.

> **MCP 도구 실행 가이드**: 구체적인 MCP 도구 호출 순서·PAD 입력 규칙·저장 경로 등은 [`mcp-testing-guide.md`](../mcp-testing-guide.md) 참조.

---

## 용어 정의

> 용어(Scene, Beat, Utterance)의 정의와 엔진 호출 매핑은 [CLAUDE.md 용어 정의](../CLAUDE.md#용어-정의) 참조.

**구조 관계:**
```
도서 (허클베리 핀)
 └── 챕터 (Ch.15)
      └── Scene (안개/Trash)    ← 하나의 대화 단위
           ├── Beat 1          ← 감정 전환 비트, appraise 1회
           │    ├── Utterance   ← 대사, stimulus 입력
           │    └── Utterance
           ├── Beat 2          ← 자동 전환 시 Trace 로그 생성
           │    └── Utterance
           └── Scene 종료      ← after_dialogue (관계 갱신)
```

---

## 역할 분담

| 활동 | 주체 | 비고 |
|---|---|---|
| 장면 선택·시나리오 기획 | Bekay | 원작 문학 기반 |
| 인물 프로파일 초안 | Claude | HEXACO 24 facets |
| 인물 프로파일 검토·조정 | Bekay | 원작 충실도 판단 |
| 엔진 실행·감정 평가 | Claude (MCP 도구) | 자율 수행 |
| 실시간 상태 확인 | Bekay (WebUI) | http://127.0.0.1:3000 |
| 테스트 레포트 작성 | Claude | 마크다운 |
| 개선점 식별 | Bekay + Claude | 소크라테스 대화식 |
| 엔진 코드 수정 | Claude 제안 → Bekay 승인 | DDD 원칙 준수 |
| Git 커밋 | Bekay | Claude는 커밋 메시지 제안만 |

---

## 개선 루프 (Improvement Loop)

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  ① 장면 선택          원작 문학에서 대화 장면 추출           │
│       │                                                     │
│       ▼                                                     │
│  ② 인물 프로파일 생성   HEXACO + 관계 초깃값                 │
│       │                                                     │
│       ▼                                                     │
│  ③ 감정 평가 실행       시나리오 로드 → 대화 진행            │
│       │                                                     │
│       ▼                                                     │
│  ④ 결과 검증            감정 아크 + Beat 전환 + 관계 변화    │
│       │                                                     │
│       ▼                                                     │
│  ⑤ 개선점 식별          무엇을 고쳐야 하는가?                │
│       │                                                     │
│       ├── 프로파일 문제  → ② 로 복귀                         │
│       ├── 가중치 문제    → 엔진 코드 수정 → ③ 재실행         │
│       ├── PAD 앵커 문제  → 앵커 개선 → ③ 재실행              │
│       ├── 가이드 문제    → directive 로직 수정 → ③ 재실행    │
│       └── 만족          → ⑥ 저장                            │
│                                                             │
│       ▼                                                     │
│  ⑥ 저장 + 다음 장면     결과 저장 → 다음 장면으로 이동       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**안심 리팩토링 원칙**:
- 도메인 로직 수정 시 `cargo test --features mind-studio,embed`로 회귀 테스트
- 통합 테스트(`handler_tests.rs`)로 파이프라인 무결성 검증
- DTO가 저장소에 의존하지 않도록 `SituationService` 경유 확인
- DDD 네이밍 규약 준수 (Domain `~Engine`/`~Analyzer`, Application `~Service`)

---

## 프로젝트 발전 로드맵

### Phase 1 & 2: 기초 및 정밀도 (완료)
- HEXACO-OCC 매핑 및 Scene/Beat 자동 전환 시스템 구축
- **[2026-04]** 애플리케이션 계층 분리 (Mind/Situation/Relationship/Scene) 완료

### Phase 3: 데이터 무결성 및 가시성 (완료)
- 대화 중 PAD 분석 결과 및 상세 Trace 로그의 완벽한 보존 및 UI 연동
- 통합 테스트(`handler_tests.rs`) 강화를 통한 리팩토링 안정성 확보
- **[2026-04-04]** Rust 네이티브 MCP 서버 프로토콜 — `initialize`/`notifications`/`ping` 핸드셰이크 지원
- **[2026-04-04]** Claude Desktop MCP 연동 검증 완료 (`mcp-remote` 브릿지 경유)
- **[2026-04-05]** Beat state latching — 활성 Focus 재전환 중복 이벤트 방지
- **[2026-04-05]** `save_type="all"` — result JSON + report MD 통합 저장

### Phase 4: 가이드 품질 및 청자 변환 (진행 예정)
- 청자 관점 PAD 자동 변환 알고리즘 설계 (프로덕션 게임용 필수)
- Beat trigger 임계값 완화 (Anger<0.7, Distress<0.6)
- stimulus 상수 튜닝 (MIN_INERTIA 0.30→0.20~0.25, IMPACT_RATE 0.5→0.6)
- 관계 delta 스케일 튜닝 (pivotal scene 이후 너무 작음)
- 2단계 비선형 PAD 스케일링 (출력 범위 ±0.2 → ±1.0)
- LLM 프롬프트 세밀화 및 간결성 지시

### Phase 5: 품질 평가 (계획)
- DeepEval 방법론 참고한 NPC 감정 아크 평가 기준 수립
- 고전 문학 자동 시나리오 생성 파이프라인
- 다중 턴 감정 누적 (현재는 stateless 설계)
