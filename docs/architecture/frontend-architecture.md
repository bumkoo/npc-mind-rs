# Mind Studio 프론트엔드 아키텍처

## 개요

Mind Studio 프론트엔드는 **Vite + React 18 + TypeScript + Zustand** 기반의 SPA입니다.
Axum REST API 백엔드와 분리된 별도 프로젝트(`mind-studio-ui/`)로 구성되며,
빌드 출력이 `src/bin/mind-studio/static/`에 배치되어 Axum `ServeDir`로 서빙됩니다.

```
mind-studio-ui/           ← 프론트엔드 프로젝트 루트
  src/
    App.tsx               ← 레이아웃 셸 + 스토어 연결
    api/client.ts         ← fetch wrapper
    stores/               ← Zustand 스토어 5개
    handlers/             ← 비즈니스 로직 (API 호출 + 스토어 갱신)
    hooks/                ← React 커스텀 훅
    components/           ← 프레젠테이션 컴포넌트
    types/index.ts        ← 공유 타입 정의
    __tests__/            ← Vitest 테스트
```

## 기술 스택

| 항목 | 기술 | 버전 | 용도 |
|------|------|------|------|
| UI 프레임워크 | React | 18.3 | 컴포넌트 렌더링 |
| 언어 | TypeScript | 5.6 | 타입 안전성 |
| 상태 관리 | Zustand | 5.0 | 5개 스토어로 전역 상태 관리 |
| 빌드 도구 | Vite | 6.0 | 번들링, HMR, 프록시 |
| 마크다운 | marked | 15.0 | 테스트 보고서 렌더링 |
| 테스트 | Vitest + Testing Library | 4.1 / 16.3 | 단위 테스트 |

## 아키텍처 다이어그램

### 계층 구조

```
┌─────────────────────────────────────────────────────────────┐
│                     Components (View)                        │
│  Sidebar · SituationPanel · ChatPanel · ResultPanel · Modals │
└──────────────────────────┬──────────────────────────────────┘
                           │ props (데이터 + 콜백)
┌──────────────────────────┴──────────────────────────────────┐
│                      App.tsx (Orchestrator)                   │
│  - 스토어 selector로 데이터 구독                               │
│  - 핸들러 함수에 스토어 액션 주입                               │
│  - 컴포넌트에 props 전달                                      │
└────────┬─────────────────────────────────┬──────────────────┘
         │                                 │
┌────────┴────────┐               ┌────────┴────────┐
│   Handlers      │               │   Hooks         │
│  appHandlers.ts │               │  useRefresh     │
│  loadHandlers.ts│               │  useChatPolling │
│                 │               │  useToast       │
│  (순수 async    │               │  useAutoSave    │
│   함수, 훅 없음)│               │                 │
└────────┬────────┘               └────────┬────────┘
         │                                 │
┌────────┴─────────────────────────────────┴──────────────────┐
│                   Zustand Stores (State)                      │
│  useEntityStore · useUIStore · useResultStore                │
│  useChatStore · useSceneStore                                │
└────────────────────────────┬────────────────────────────────┘
                             │
┌────────────────────────────┴────────────────────────────────┐
│                    api/client.ts (Transport)                  │
│  get<T> · post · put · del · postJson<T>                    │
│  (에러 검증: res.ok 체크, 4xx/5xx throw)                     │
└────────────────────────────┬────────────────────────────────┘
                             │ fetch (proxy: /api → Axum:3000)
┌────────────────────────────┴────────────────────────────────┐
│                  Axum REST API Backend                        │
│  /api/npcs · /api/appraise · /api/chat/message/stream (SSE) │
│  /api/events (SSE 실시간 상태 동기화)                          │
└─────────────────────────────────────────────────────────────┘
```

### 데이터 흐름

**단방향 데이터 흐름**: 사용자 입력 → Handler → API → Store → Component 리렌더

```
사용자 클릭 "감정 평가"
  → App.onAppraise()
    → handleAppraise(npcId, partnerId, situation, ...)
      → fetch POST /api/appraise
      → setResult(data)          // useResultStore
      → setTraceHistory([...])   // useResultStore
      → refresh()                // 8개 API 병렬 fetch
        → setNpcs, setRels, ...  // useEntityStore
  → ResultPanel 리렌더 (감정 바 차트 표시)
```

## Zustand 스토어 설계

### 스토어 분리 기준

| 스토어 | 책임 | 주요 상태 |
|--------|------|----------|
| **useEntityStore** | 게임 엔티티 | `npcs`, `rels`, `objects`, `scenarios`, `history` |
| **useUIStore** | UI 탐색/선택 | `npcId`, `partnerId`, `modal`, `loading`, `connected`, `resultView*` |
| **useResultStore** | 분석 결과 | `result`, `traceHistory`, `resultTab`, `testReport`, `stimulusUtterance`, `llmModelInfo` |
| **useChatStore** | 대화 세션 | `chatMode`, `chatSessionId`, `chatMessages`, `chatScenarioTurns`, `chatEnded` |
| **useSceneStore** | 시나리오 컨텍스트 | `scenarioMeta`, `savedSituation`, `sceneInfo`, `scenarioSeeds` (E3.3) |
| **useMemoryStore** (E2/E3.1) | 기억·소문 조회 상태 | `entriesByNpc`, `rumors`, `selectedNpcId`, `layerFilter`, `mode`('npc'\|'topic'), `selectedTopic`, `topicEntries` |

### 스토어 구독 패턴

App.tsx에서 **개별 selector**를 사용하여 불필요한 리렌더링을 방지합니다:

```tsx
// (권장) 개별 selector — 해당 필드 변경 시에만 리렌더
const npcs = useEntityStore((s) => s.npcs)
const saveNpc = useEntityStore((s) => s.saveNpc)

// (지양) 전체 스토어 구독 — 모든 필드 변경 시 리렌더
const store = useEntityStore()  // ← 사용하지 않음
```

### 주요 패턴

**Functional updater** — 이전 상태 기반 변환:
```tsx
updateResult: (updater) => set((state) => ({ result: updater(state.result) }))
updateChatMessages: (updater) => set((state) => ({ chatMessages: updater(state.chatMessages) }))
updateSceneInfo: (updater) => set((state) => ({ sceneInfo: updater(state.sceneInfo) }))
```

**일괄 상태 설정** — 부분 업데이트 방지:
```tsx
setResultView: (opts) => set({
  resultViewMode: opts.mode,
  resultViewActive: opts.active,
  resultTurnHistory: opts.turnHistory,
  resultMessages: opts.messages,
  resultSelectedIdx: opts.selectedIdx,
})
```

**완전 초기화**:
```tsx
reset: () => set({
  chatMode: false, chatSessionId: null, chatMessages: [],
  chatLoading: false, chatScenarioTurns: [], chatScenarioIdx: 0,
  chatEnded: false, selectedMsgIdx: null,
})
```

## 핸들러 모듈

핸들러는 **순수 async 함수**로, React 훅을 사용하지 않습니다.
스토어 액션(setter)을 매개변수로 받아 API 호출 후 상태를 갱신합니다.

### appHandlers.ts (11개 함수)

| 함수 | API 엔드포인트 | 설명 |
|------|---------------|------|
| `handleAppraise` | `POST /api/appraise` | 초기 감정 평가 |
| `handleStimulus` | `POST /api/stimulus` | PAD 자극 적용 |
| `handleGuide` | `POST /api/guide` | 가이드 프롬프트 재생성 |
| `handleAfterDialogue` | `POST /api/after-dialogue` | 대화 후 관계 갱신 |
| `handleStartChat` | `POST /api/chat/start` | 대화 세션 시작 |
| `handleChatSend` | `POST /api/chat/message/stream` | SSE 스트리밍 대화 |
| `handleEndChat` | `POST /api/chat/end` | 대화 세션 종료 |
| `doSave` | `POST /api/save` | 파일 저장 헬퍼 |
| `saveScenario` | `GET /api/save-dir` → `POST /api/save` | 시나리오 저장 |
| `saveState` | 복합 | 상태 기반 저장 분기 |

### loadHandlers.ts (3개 함수)

| 함수 | API 엔드포인트 | 설명 |
|------|---------------|------|
| `loadScenario` | `POST /api/load` | 시나리오 파일 로드 |
| `loadResult` | `POST /api/load-result` | 테스트 결과 로드 + 대화 메시지 변환 |
| `updateTestReport` | `PUT /api/test-report` | 테스트 보고서 갱신 |

### SSE 스트리밍 (handleChatSend)

```
POST /api/chat/message/stream
  ↓ SSE 이벤트 스트림
  event: token → 토큰 누적 (실시간 타이핑 효과)
  event: done  → 최종 결과 파싱 (감정/PAD/Beat 전환)
  event: error → 에러 표시
```

- **AbortController**로 요청 취소 지원 (메모리 누수 방지)
- **클로저 캡처**로 메시지 인덱스 안정성 보장 (`capturedIdx`)

### 실시간 상태 동기화 (useStateSync)

```
백엔드 상태 변경 (MCP tool / REST handler)
  → state.emit(StateEvent::XxxChanged)
  → tokio::sync::broadcast 채널
  → GET /api/events (SSE 스트림)
  → EventSource (프론트엔드)
  → 이벤트 종류별 targeted re-fetch → Zustand 스토어 업데이트
```

- **이벤트 종류**: `npc_changed`, `relationship_changed`, `object_changed`, `appraised`, `stimulus_applied`, `after_dialogue`, `scene_started`, `scene_info_changed`, `scenario_loaded`, `result_loaded`, `scenario_saved`, `situation_changed`, `test_report_changed`, `chat_started`, `chat_turn_completed`, `chat_ended`, `history_changed`, `memory_created`, `memory_superseded`, `memory_consolidated`, `rumor_seeded`, `rumor_spread` (Step E1)
- **디바운싱**: 100ms leading-edge — 빠른 연속 이벤트(예: `create_full_scenario`에서 NPC 다수 생성) 시 중복 fetch 방지
- **재연결**: 연결 끊김 시 exponential backoff (1s → 30s)
- **이벤트 누락**: `BroadcastStream` lagged 발생 시 `resync` → 전체 refresh
- **전체 상태 교체**: `scenario_loaded`, `result_loaded`, `resync` → `refresh()` 호출
- **시나리오 시드 fetch** (E3.3): `/api/scenario-seeds`는 시나리오 라이프사이클에서만 변하므로 `useRefresh`에 포함하지 않고, `useStateSync`가 최초 마운트 + `scenario_loaded`/`result_loaded`에서만 fetch해 `useSceneStore.scenarioSeeds`에 반영.

## 커스텀 훅

| 훅 | 역할 | 트리거 |
|----|------|--------|
| **useRefresh** | 8개 API 병렬 fetch → 5개 스토어 동기화 (E3.3 이후 `/api/scenario-seeds`는 제외 — `useStateSync`로 이관) | 마운트, CRUD 후, 대화 액션 후 |
| **useStateSync** | SSE `/api/events` 구독 → 이벤트별 targeted re-fetch | 마운트 시 연결, 자동 재연결 |
| **useChatPolling** | 2초 간격 히스토리 폴링 (SSE 보조) | `chatMode === true` |
| **useToast** | 알림 메시지 관리 (3초 자동 제거) | 핸들러에서 호출 |
| **useAutoSave** | 디바운스 자동 저장 (500ms) | SituationPanel 입력 변경 |

### useRefresh 최적화 로직

```tsx
// SceneInfo의 script_cursor에 대한 optimistic update
updateSceneInfo((prev) => {
  const next = si && si.has_scene ? si : null
  if (!prev || !next) return next
  // Beat 전환 시 서버 값 우선
  if (prev.active_focus_id !== next.active_focus_id) return next
  // 서버가 아직 반영 전이면 optimistic 값 유지
  if ((prev.script_cursor || 0) > (next.script_cursor || 0)) {
    return { ...next, script_cursor: prev.script_cursor }
  }
  return next
})
```

## 컴포넌트 트리

```
App
├── Header
│   ├── 시나리오 선택 드롭다운 (scenarios → optgroup 분리)
│   └── 저장 / 새로고침 버튼
├── Main (3단 레이아웃)
│   ├── Sidebar (좌측, 240px 고정)
│   │   ├── NPC 섹션 (접기/펼치기)
│   │   ├── 관계 섹션
│   │   └── 오브젝트 섹션
│   ├── Center (중앙, 360px 고정) — 조건부 렌더링
│   │   ├── ChatPanel (chatMode 시)
│   │   ├── ResultViewPanel (resultViewActive 시)
│   │   └── SituationPanel (기본)
│   │       └── FocusEditor (Scene Focus 편집)
│   └── ResultPanel (우측, flex)
│       ├── ScenePanel (감정 탭 상단)
│       ├── EmotionView / ContextView / StimulusView
│       ├── TraceView / ReportView / HistoryView
│       ├── MemoryView (E2: NPC 모드 / E3.1: Topic 모드 토글 → TopicHistoryView)
│       ├── RumorView (E2 표시 + E3.1: SeedForm / SpreadForm)
│       ├── ScenarioSeedsView (E3.3: 조회 전용 4 섹션 카드)
│       └── ModelInfoView
├── NpcModal / RelModal / ObjModal (조건부)
└── ToastContainer (고정 위치)
```

### 컴포넌트 설계 원칙

1. **프레젠테이션 컴포넌트**: 스토어에 직접 접근하지 않음. props로만 데이터/콜백 수신
2. **App.tsx만 스토어 구독**: 단일 연결점에서 데이터를 분배
3. **조건부 렌더링**: `chatMode`, `resultViewActive`, `chatEnded` 플래그로 UI 모드 전환

## 타입 시스템

### 핵심 타입 관계

```
AppraiseResult
├── emotions: Emotion[]          (감정 목록)
├── dominant: Emotion             (지배 감정)
├── mood: number                  (기분 점수)
├── prompt: string                (LLM 가이드 프롬프트)
├── relationship: Relationship    (관계 상태)
├── trace: string[]               (추론 로그)
├── input_pad: Pad                (입력 PAD 값)
├── beat_changed: boolean         (Beat 전환 여부)
└── llm_model: LlmModelInfo       (사용된 모델 정보)

ChatMessage
├── role: 'system' | 'user' | 'assistant'
├── content: string
├── emotions: Record<string, number> | null
├── snapshot: AppraiseResult | null   ← 이 시점의 전체 상태
├── activePrompt: string              ← 이 응답 생성에 사용된 프롬프트
├── pad: Pad | null                   ← 분석된 PAD 값
├── beat_changed / new_focus          ← Beat 전환 정보
└── streaming: boolean                ← SSE 스트리밍 중 여부

SceneInfo
├── has_scene: boolean
├── turns: ScenarioTurn[]             ← 시나리오 대사 목록
├── focuses: SceneFocus[]             ← Focus 옵션 (트리거 조건 포함)
├── script_cursor: number             ← 현재 테스트 스크립트 위치
└── significance: number              ← 장면 중요도
```

### 모달 상태 (Discriminated Union)

```tsx
type ModalState =
  | { type: 'npc'; data: Npc | null }       // null = 새 NPC 생성
  | { type: 'rel'; data: Relationship | null }
  | { type: 'obj'; data: GameObject | null }
```

## 빌드 파이프라인

### 개발 모드

```bash
cd mind-studio-ui && npm run dev  # Vite dev server (port 5173)
```

- **HMR** (Hot Module Replacement) 활성화
- **프록시**: `/api/*`, `/mcp/*` → `http://127.0.0.1:3000` (Axum 백엔드)
- Axum 서버를 별도로 실행해야 API 동작

### 프로덕션 빌드

```bash
cd mind-studio-ui && npm run build
# tsc -b (타입 체크) → vite build (번들링)
# 출력: ../src/bin/mind-studio/static/
#   ├── index.html       (0.4KB)
#   ├── assets/index-*.css (10.7KB, gzip 2.7KB)
#   └── assets/index-*.js  (246KB, gzip 77KB)
```

Axum 서버가 `ServeDir`로 `static/` 디렉토리를 서빙하므로 별도 배포 불필요.

### 프록시 설정 (`vite.config.ts`)

```ts
server: {
  port: 5173,
  proxy: {
    '/api': 'http://127.0.0.1:3000',
    '/mcp': 'http://127.0.0.1:3000',
  },
},
build: {
  outDir: '../src/bin/mind-studio/static',
  emptyOutDir: true,
},
```

## 테스트

### 구성

- **프레임워크**: Vitest 4.1 (Vite 네이티브, Jest 호환 문법)
- **DOM 환경**: jsdom
- **컴포넌트 테스트**: @testing-library/react
- **Setup**: `src/test/setup.ts` (jest-dom 매처 + cleanup)

### 테스트 파일

| 파일 | 테스트 수 | 대상 |
|------|----------|------|
| `stores.test.ts` | 30개 | 5개 Zustand 스토어 (초기값, setter, updater, reset) |
| `handlers.test.ts` | 14개 | appHandlers/loadHandlers (성공/에러/가드) |
| `api-client.test.ts` | 11개 | api.get/post/put/del/postJson + 에러 처리 |
| `hooks.test.ts` | 5개 | useToast (추가/자동제거/안정참조) |
| **합계** | **60개** | |

### 실행

```bash
cd mind-studio-ui
npm test           # 단일 실행
npm run test:watch # 감시 모드
```

## 설계 결정 사항

| 결정 | 이유 |
|------|------|
| Zustand (Context/Redux 아님) | 보일러플레이트 최소, 스토어 외부에서 `getState()` 접근 가능 |
| 핸들러를 순수 함수로 분리 | 스토어 액션을 매개변수로 받아 테스트 용이, 훅 규칙 제약 없음 |
| 컴포넌트가 스토어 직접 접근 안 함 | App.tsx를 단일 연결점으로 하여 데이터 흐름 추적 용이 |
| CSS Modules/Tailwind 미사용 | 글로벌 CSS 유지 (기존 디자인 토큰 활용, 마이그레이션 범위 최소화) |
| 라우터 미사용 | SPA 단일 뷰, URL 기반 탐색 불필요 |
| `emptyOutDir: true` | 빌드마다 이전 에셋 정리, Axum이 항상 최신 파일 서빙 |
