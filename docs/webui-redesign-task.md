# WebUI 레이아웃 전면 재설계 — 작업 인수인계

## 프로젝트 경로

```
C:\Users\bumko\projects\npc-mind-rs
```

## 대상 파일

```
src/bin/webui/static/index.html   ← 프론트엔드 (React + Babel, 단일 HTML)
src/bin/webui/main.rs             ← axum 서버 진입점
src/bin/webui/handlers.rs         ← API 핸들러
src/bin/webui/state.rs            ← 서버 상태 (NPC/관계/오브젝트 레지스트리)
src/bin/webui/trace_collector.rs  ← 엔진 trace 수집
```

## 빌드 & 실행

```powershell
cargo build --bin npc-webui --features webui
cargo run --bin npc-webui --features webui
# http://127.0.0.1:3000
```

## 백엔드 API (변경 없음 — 정상 동작 확인됨)

| 엔드포인트 | 메서드 | 기능 |
|---|---|---|
| `/api/npcs` | GET/POST | NPC CRUD |
| `/api/npcs/{id}` | DELETE | NPC 삭제 |
| `/api/relationships` | GET/POST | 관계 CRUD |
| `/api/relationships/{owner}/{target}` | DELETE | 관계 삭제 |
| `/api/objects` | GET/POST | 오브젝트 CRUD |
| `/api/objects/{id}` | DELETE | 오브젝트 삭제 |
| `/api/appraise` | POST | 감정 평가 (핵심 파이프라인) |
| `/api/stimulus` | POST | PAD 자극 적용 |
| `/api/guide` | POST | 가이드 재생성 |
| `/api/after-dialogue` | POST | 대화 종료 → 관계 갱신 |
| `/api/scenarios` | GET | data/ 폴더 시나리오 목록 |
| `/api/history` | GET | 턴별 기록 |
| `/api/save` | POST | JSON 파일 저장 |
| `/api/load` | POST | JSON 파일 로드 |

## 현재 상태: index.html 재작성 70% 완료

### 기존 레이아웃 (문제점)

```
┌─────────────────────────────────────────────┐
│ Header                                      │
├───────────────┬──────────────┬──────────────┤
│ NPC 프로필     │ 관계 관리     │ 오브젝트 등록  │  ← 상단 3컬럼
│ (인라인 편집)   │ (인라인 편집)  │ (인라인 편집)  │
│               │              │              │
│               │              │              │
├───────────────┴──────────────┴──────────────┤
│              거대한 빈 공간                    │  ← 문제!
├─────────────────┬───────────────────────────┤
│ 상황 설정        │ 결과                       │  ← 스크롤 필요
└─────────────────┴───────────────────────────┘
```

- 상단 3패널과 하단 패널 사이 빈 공간 과대
- 핵심 워크플로우(NPC 선택 → 상황 설정 → 결과 확인)가 한 화면에 안 들어옴
- NPC 프로필 선택과 상황 패널의 NPC 드롭다운이 연동 안 됨
- 턴 히스토리 API는 있으나 UI 없음

### 새 레이아웃 (목표)

```
┌─────────────────────────────────────────────────────┐
│ Header: 타이틀 | 상태 | 시나리오 로드/저장/새로고침      │
├────────┬──────────────┬────────────────────────────┤
│Sidebar │ Center       │ Right                      │
│(240px) │ (360px)      │ (나머지)                    │
│        │              │                            │
│▾ NPC   │ 상황 설정      │ [감정 상태|프롬프트|Trace|히스토리]│
│ 교룡   │ NPC↔대화상대   │                            │
│ 무백   │ 상황 설명      │ 지배 감정: Reproach (1.000)  │
│ + 추가  │              │ ████████████████████       │
│        │ ☑ Event      │                            │
│▾ 관계   │  사건 설명     │ Anger: 0.677               │
│ mu↔gyo │  자기영향 슬라  │ ██████████████             │
│ + 추가  │  □ 타인영향    │                            │
│        │              │ Distress: 0.353            │
│▾ 오브젝트│ □ Action     │ ████████                   │
│ + 추가  │ □ Object     │                            │
│        │              │ 분위기: -0.677              │
│        │ [감정 평가 실행]│                            │
│        │              │                            │
│        │ PAD 자극 적용  │                            │
│        │ P/A/D 슬라이더 │                            │
│        │ [자극 적용]    │                            │
│        │              │                            │
│        │ [대화 종료]    │                            │
└────────┴──────────────┴────────────────────────────┘
```

- **스크롤 없이** 전체 워크플로우가 한 화면에 (`overflow:hidden; height:100vh`)
- NPC/관계/오브젝트 편집은 **모달 팝업**으로 분리
- 사이드바 NPC 클릭 → 중앙 패널 `npcId` 자동 연동
- 우측에 **히스토리 탭** 추가 (기존 API 활용)
- 감정 타입별 **고유 색상** 22개

## 작성 완료된 컴포넌트 (index.html 내부)

### CSS (완료)
- 전체 다크 테마 (CSS 변수 기반)
- 레이아웃: `.app`, `.header`, `.main`, `.sidebar`, `.center`, `.right`
- 사이드바: `.sidebar-section`, `.sidebar-header`, `.item-card`, `.add-btn`
- 폼: `label`, `input`, `select`, `textarea`, `.slider-row`
- 버튼: `.btn`, `.btn.primary`, `.btn.danger`, `.btn.small`, `.btn-full`
- 섹션 카드: `.section`, `.section-header`, `.section-body`
- 감정 바: `.emotion-row`, `.bar-bg`, `.bar-fill`
- 결과: `.prompt-box`, `.trace-box`, `.dominant-card`, `.mood-bar`
- 히스토리: `.history-item`
- 모달: `.modal-overlay`, `.modal`

### React 컴포넌트 (완료)
1. **`Slider`** — 공용 슬라이더 (`label`, `value`, `onChange`, `min/max/step`)
2. **`NpcModal`** — HEXACO 6차원 24facet 편집 모달
3. **`RelModal`** — 관계 편집 모달 (closeness/trust/power)
4. **`ObjModal`** — 오브젝트 편집 모달
5. **`Sidebar`** — NPC/관계/오브젝트 아코디언 목록 + 클릭 이벤트
6. **`SituationPanel`** — 상황 설정 (Event/Action/Object/PAD/대화종료)
7. **`ResultPanel`** — 결과 탭 컨테이너 (감정|프롬프트|Trace|히스토리)
8. **`EmotionView`** — 감정 바 시각화 + 지배 감정 카드 + 분위기
9. **`PromptView`** — 프롬프트 표시 + 클립보드 복사
10. **`TraceView`** — 엔진 trace 표시
11. **`HistoryView`** — 턴별 히스토리 (접이식 JSON)
12. **`emotionColor()`** — 감정 타입별 색상 헬퍼 (22개)

### App 컴포넌트 (70% 완료)
- state 선언 ✅
- `refresh()` (npcs, rels, objects, scenarios, history 일괄 로드) ✅
- CRUD 핸들러 (saveNpc, deleteNpc, saveRel, deleteRel, saveObj, deleteObj) ✅
- `handleAppraise` ✅
- `handleStimulus` ✅

### App 컴포넌트 — 남은 부분

아래 부분을 `index.html` 끝에 이어서 작성해야 합니다:

#### 1. `handleAfterDialogue` (후반부)

```javascript
const handleAfterDialogue = async () => {
  if(!npcId||!partnerId) return alert('NPC와 대화 상대를 선택하세요');
  const pwVal = prompt('도덕성 값 (-1.0~1.0, 없으면 빈칸):', '');
  const pw = pwVal ? parseFloat(pwVal) : null;
  try {
    const res = await fetch('/api/after-dialogue', {
      method:'POST', headers:{'Content-Type':'application/json'},
      body:JSON.stringify({npc_id:npcId, partner_id:partnerId, praiseworthiness: isNaN(pw) ? null : pw})
    });
    if(!res.ok) { alert('오류: '+await res.text()); return; }
    const data = await res.json();
    alert(`대화 종료\n친밀도: ${data.before.closeness.toFixed(3)} → ${data.after.closeness.toFixed(3)}\n신뢰도: ${data.before.trust.toFixed(3)} → ${data.after.trust.toFixed(3)}\n상하: ${data.before.power.toFixed(3)} → ${data.after.power.toFixed(3)}`);
    setResult(null);
    refresh();
  } catch(e) { alert('요청 실패: '+e); }
};
```

#### 2. `saveState` / `loadScenario`

```javascript
const saveState = async () => {
  const sub = prompt('저장 경로 (data/ 기준):', '');
  if(!sub) return;
  const path = `C:\\Users\\bumko\\projects\\npc-mind-rs\\data\\${sub.replace(/\//g,'\\\\')}${sub.endsWith('.json')?'':'.json'}`;
  const res = await fetch('/api/save', {method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({path})});
  if(res.ok) { alert('저장 완료'); refresh(); }
  else alert('저장 실패: ' + await res.text());
};

const loadScenario = async (scenPath) => {
  if(!scenPath) return;
  const path = `C:\\Users\\bumko\\projects\\npc-mind-rs\\data\\${scenPath.replace(/\//g,'\\\\')}\\scenario.json`;
  const res = await fetch('/api/load', {method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({path})});
  if(res.ok) { refresh(); setResult(null); }
  else alert('로드 실패: ' + await res.text());
};
```

#### 3. App `return` JSX

```jsx
return (
  <div className="app">
    {/* Header */}
    <div className="header">
      <h1>NPC Mind Engine</h1>
      <span className={`status ${connected?'ok':''}`}>
        {connected?'연결됨':'연결 실패'} — NPC {npcs.length} · 관계 {rels.length} · 오브젝트 {objects.length}
      </span>
      <div className="actions">
        <select style={{background:'var(--bg3)',color:'var(--fg)',border:'1px solid var(--border)',borderRadius:'var(--radius)',padding:'3px 8px',fontSize:11,maxWidth:240}}
          onChange={e=>{if(e.target.value)loadScenario(e.target.value);e.target.value=''}}>
          <option value="">시나리오 로드...</option>
          {scenarios.map(s => <option key={s.path} value={s.path}>{s.label}</option>)}
        </select>
        <button className="btn small" onClick={saveState}>저장</button>
        <button className="btn small" onClick={refresh}>새로고침</button>
      </div>
    </div>

    {/* Main 3-column */}
    <div className="main">
      <Sidebar
        npcs={npcs} rels={rels} objects={objects}
        selectedNpcId={npcId}
        onSelectNpc={(id) => setNpcId(id)}
        onEditNpc={(n) => setModal({type:'npc',data:n})}
        onEditRel={(r) => setModal({type:'rel',data:r})}
        onEditObj={(o) => setModal({type:'obj',data:o})}
        onNewNpc={() => setModal({type:'npc',data:null})}
        onNewRel={() => setModal({type:'rel',data:null})}
        onNewObj={() => setModal({type:'obj',data:null})}
      />
      <SituationPanel
        npcs={npcs} objects={objects}
        npcId={npcId} setNpcId={setNpcId}
        partnerId={partnerId} setPartnerId={setPartnerId}
        onAppraise={handleAppraise}
        onStimulus={handleStimulus}
        onAfterDialogue={handleAfterDialogue}
        loading={loading}
      />
      <ResultPanel result={result} history={history} />
    </div>

    {/* Modals */}
    {modal?.type==='npc' && <NpcModal npc={modal.data} onSave={saveNpc} onDelete={deleteNpc} onClose={()=>setModal(null)} />}
    {modal?.type==='rel' && <RelModal rel={modal.data} npcIds={npcs.map(n=>n.id)} onSave={saveRel} onDelete={deleteRel} onClose={()=>setModal(null)} />}
    {modal?.type==='obj' && <ObjModal obj={modal.data} onSave={saveObj} onDelete={deleteObj} onClose={()=>setModal(null)} />}
  </div>
);
```

#### 4. 마운트

```javascript
}  // App 함수 닫기

ReactDOM.createRoot(document.getElementById('root')).render(<App />);
</script>
</body>
</html>
```

## API 요청/응답 형식 참고

### POST /api/appraise

```json
// Request
{
  "npc_id": "mu_baek",
  "partner_id": "gyo_ryong",
  "situation": {
    "description": "교룡이 무백의 의형제를 배신하고 독을 탔다",
    "event": {
      "description": "의형제가 독에 당했다",
      "desirability_for_self": -0.7,
      "other": null,
      "prospect": null
    },
    "action": {
      "description": "교룡의 배신과 독살 시도",
      "agent_id": "gyo_ryong",
      "praiseworthiness": -0.8
    },
    "object": null
  }
}

// Response
{
  "emotions": [
    {"emotion_type": "Distress", "intensity": 0.353, "context": "의형제가 독에 당했다"},
    {"emotion_type": "Reproach", "intensity": 1.000, "context": "교룡의 배신과 독살 시도"},
    {"emotion_type": "Anger", "intensity": 0.677, "context": "교룡이 무백의 의형제를 배신하고 독을 탔다"}
  ],
  "dominant": {"emotion_type": "Reproach", "intensity": 1.000, "context": "교룡의 배신과 독살 시도"},
  "mood": -0.677,
  "prompt": "[NPC: 무백]\n정의로운 검객\n\n[성격]\n...\n[현재 감정]\n...\n[연기 지시]\n...",
  "trace": ["  → Distress: base_val=-0.700, ...", "  → Reproach: ...", "  → Anger: ..."]
}
```

### POST /api/stimulus

```json
// Request
{
  "npc_id": "mu_baek",
  "partner_id": "gyo_ryong",
  "situation_description": "교룡이 변명을 시작한다",
  "pleasure": -0.3,
  "arousal": 0.5,
  "dominance": -0.2
}
// Response: 동일한 AppraiseResponse 형식
```

### POST /api/after-dialogue

```json
// Request
{"npc_id": "mu_baek", "partner_id": "gyo_ryong", "praiseworthiness": -0.5}
// Response
{
  "before": {"closeness": 0.431, "trust": 0.240, "power": 0.550},
  "after": {"closeness": 0.397, "trust": 0.190, "power": 0.550}
}
```

## 작업 지시

1. `index.html`의 App 컴포넌트에 위 "남은 부분" 4개를 이어 붙여 파일을 완성한다
2. 서버를 재시작하고 (`cargo run --bin npc-webui --features webui`) 브라우저에서 동작 확인
3. 동작 확인 후 UX 미세 조정:
   - 사이드바에서 NPC 클릭 시 해당 NPC가 중앙 패널의 NPC 드롭다운에 반영되는지 확인
   - 모달 열고 닫기, CRUD 동작 확인
   - appraise → stimulus → after-dialogue 전체 워크플로우 테스트
   - 감정 바 색상, 지배 감정 카드, 프롬프트 복사 등 시각적 확인

---

## 개발 완료 내역 (2026-03-28)

### 브랜치

- `claude/flamboyant-kirch` → CSS + 모든 React 컴포넌트 + App 70% (커밋 `518a9cb`)
- `claude/flamboyant-kirch-Wg3zv` → App 나머지 30% 완성 (커밋 `c8fdfbb`)

### 1차: 프론트엔드 전면 재작성 (커밋 `518a9cb`)

| 카테고리 | 완료 항목 |
|---|---|
| **CSS** | 다크 테마(CSS 변수), 3-column 레이아웃, 사이드바, 폼, 버튼, 섹션 카드, 감정 바, 결과(프롬프트/trace), 히스토리, 모달 |
| **Slider** | 공용 슬라이더 컴포넌트 (`label`, `value`, `onChange`, `min/max/step`) |
| **NpcModal** | HEXACO 6차원 24facet 편집 모달 |
| **RelModal** | 관계 편집 모달 (closeness/trust/power) |
| **ObjModal** | 오브젝트 편집 모달 |
| **Sidebar** | NPC/관계/오브젝트 아코디언 목록 + 클릭→NPC 선택 연동 |
| **SituationPanel** | 상황 설정 (Event/Action/Object/PAD 자극/대화종료 전부 포함) |
| **ResultPanel** | 결과 탭 컨테이너 (감정│프롬프트│Trace│히스토리) |
| **EmotionView** | 감정 바 시각화 + 지배 감정 카드 + 분위기 |
| **PromptView** | 프롬프트 표시 + 클립보드 복사 |
| **TraceView** | 엔진 trace 표시 |
| **HistoryView** | 턴별 히스토리 (접이식 JSON) |
| **emotionColor()** | 감정 타입별 고유 색상 22개 |
| **App (70%)** | state 선언, `refresh()`, CRUD 핸들러 6종, `handleAppraise`, `handleStimulus` |

### 2차: App 컴포넌트 완성 (커밋 `c8fdfbb`)

| 항목 | 설명 |
|---|---|
| **`handleAfterDialogue`** | 대화 종료 → 도덕성 입력 → 관계 갱신 결과 표시 (친밀도/신뢰도/상하 before→after) |
| **`saveState`** | `data/` 기준 상대 경로로 시나리오 JSON 저장 |
| **`loadScenario`** | 시나리오 목록에서 선택 → JSON 로드 → 상태 초기화 |
| **App return JSX** | Header(타이틀/상태/시나리오 드롭다운/저장/새로고침) + 3-column(Sidebar/SituationPanel/ResultPanel) + Modals(NPC/관계/오브젝트) |
| **ReactDOM 마운트** | `createRoot` → `<App />` 렌더링 |

### 변경 사항 (원본 대비)

- `saveState`/`loadScenario`의 경로를 Windows 절대 경로(`C:\Users\bumko\...`)에서 **상대 경로**(`data/...`)로 변경
  - 서버 핸들러가 상대 경로를 올바르게 처리하는지 로컬 확인 필요

### 미확인 (로컬 브라우저 테스트 필요)

- [ ] 사이드바 NPC 클릭 → 중앙 패널 NPC 드롭다운 반영
- [ ] 모달 열기/닫기, NPC·관계·오브젝트 CRUD
- [ ] appraise → stimulus → after-dialogue 전체 워크플로우
- [ ] 감정 바 색상 22종, 지배 감정 카드, 프롬프트 복사
- [ ] 시나리오 저장/로드 경로 동작
- [ ] 스크롤 없이 전체 워크플로우 한 화면 표시 (`100vh`)
