# Step 3.8: 관계 영속 + 연대기 + 즉시 프롬프트 반영 (Iter 4)

**버전:** v2.0.0
**수정일:** 2026-03-02T01:30:00+09:00
**Status:** ✅ 전체 완료 (Iter 4-A~4-F, 2026-03-03 확인)  
**관련 문서:**
- `sprint3-progress.md` — Sprint 3 진행 현황 (Iter 1~3 완료 기록)
- `relationship-mechanic.md` — 관계 메카닉 설계 (2축 모델, 이벤트 소스 12개)
- `step4-embedding-sentiment-plan.md` — 감정 판정 파이프라인 (호감도 delta 산출)
- `dev-plan.md` — 전체 개발 로드맵
- `dependency-principles.md` — 의존성 원칙 (Port & Adapter, DI, Composition Root)
- `memory-domain-analysis.md` — 기억 도메인 분석 (MemoryRepository 패턴 참조)

---

## 1. 목표

soyeon_chat_v2.rs에서 대화 시 변경되는 호감도를 Player와의 관계(Relationship)로 반영하고, 변경된 관계 상태를 즉시 다음 턴 프롬프트에 반영하며, 두 종류의 영속 저장을 구현한다.

- **관계 현재 상태** — `relationships.json`에 모든 관계를 배열로 저장 (종료 시 1회)
- **관계 변화 이력** — `relationship_chronicles.jsonl`에 변화를 시간순 기록 (매 턴 append)

두 저장소 모두 Port & Adapter 패턴을 따르며, JSON/JSONL은 교체 가능한 어댑터 구현이다.

---

## 2. 현재 상태 진단

### 2.1 Iter 3까지 구현된 것

Iter 1~3에서 관계 시스템의 **산출** 부분은 완성되었다. 매 턴 감정 판정(극단 앵커 임베딩 + LLM 정기 판정)이 affinity_delta를 계산하고, ChatSession 내부에 cumulative_affinity_delta로 누적된다.

### 2.2 세 곳의 끊김

현재 호감도 변화가 계산되지만 **세 곳에서 끊겨 있다**.

```
  send() → affinity_delta 계산됨
       │
       ▼
  cumulative_affinity_delta += delta  ← ① 누적만 됨 (도메인 Relationship에 미반영)
       │
       ▼
  relationship_view: None             ← ② 관계 상태가 프롬프트에 없음
       │
       ▼
  end() → total_affinity_delta 출력   ← ③ 출력만 하고 사라짐 (영속 안됨)
       │
       ▼
  (프로그램 종료 → 다음 실행 시 affinity 0부터 다시 시작)
```

### 2.3 결과

소연이 플레이어와 3세션에 걸쳐 친해져도, 매번 Stranger 톤으로 대화가 시작된다. 플레이어가 쌓아온 관계가 소멸되는 것은 치명적인 게임플레이 문제다.

---

## 3. 영속 아키텍처 — Port & Adapter 패턴

### 3.1 핵심 원칙

JSON, JSONL, SQLite 등 저장 기술은 **어댑터(인프라)**이다. 도메인은 trait(Port)만 알고, 어떤 기술로 저장하는지 모른다. 이는 기억 도메인에서 MemoryRepository trait → InMemoryRepository / LanceDbRepository로 구현한 것과 동일한 패턴이다.

```
  기억 도메인 (이미 구현):
    trait MemoryRepository     → InMemoryRepository (테스트)
                               → LanceDbRepository (프로덕션)

  관계 도메인 (이번 Iter 4):
    trait RelationshipRepository → InMemoryRelRepo (테스트)
                                 → JsonFileRelRepo (프로덕션 MVP)
                                 → 향후 SQLite 등 교체 가능

    trait ChronicleRepository    → InMemoryChronicleRepo (테스트)
                                 → JsonlChronicleRepo (프로덕션 MVP)
                                 → 향후 SQLite 등 교체 가능
```

### 3.2 저장소 역할 분류 — wuxia-data vs wuxia-memory

프로젝트의 데이터는 세 종류이며, 각각 다른 crate가 담당한다.

```
  데이터 성격              담당 crate        기술            예시
  ──────────────────────────────────────────────────────────────────
  정적 (읽기 전용)         wuxia-data        TOML/JSON       관계 설명 텍스트
  개발자가 미리 작성                                          프롬프트 헤더
  게임 시작 시 1회 로딩                                       극단 앵커 문장
  게임 중 변경 안됨                                           캐릭터 초기 데이터

  동적 (벡터 검색)         wuxia-memory      LanceDB         NPC 기억
  게임 중 계속 읽기+쓰기                                      "의미가 비슷한 기억"

  동적 (구조화 조회)       wuxia-memory      JSON/JSONL      관계 현재 상태
  게임 중 계속 읽기+쓰기                     (→향후 SQLite)  관계 변화 이력
```

관계 JSON/JSONL은 파일 포맷이 wuxia-data의 TOML/JSON과 같더라도, **데이터 생명주기가 다르다**. wuxia-data는 "인쇄된 교과서" (한 번 읽고 끝)이고, 관계 저장소는 "일기장" (매번 쓰고 다시 읽음)이다. 따라서 관계 어댑터는 wuxia-memory에 넣는다.

```
  wuxia-memory/
    in_memory.rs                  ← impl MemoryRepository (테스트용, 기존)
    lancedb/                      ← impl MemoryRepository (프로덕션, 기존)
    chronicle/                    ← 신규
      mod.rs
      in_memory.rs                ← impl ChronicleRepository (테스트용)
      jsonl.rs                    ← impl ChronicleRepository (프로덕션 MVP)
    relationship_store/           ← 신규
      mod.rs
      in_memory.rs                ← impl RelationshipRepository (테스트용)
      json_file.rs                ← impl RelationshipRepository (프로덕션 MVP)
```

### 3.3 두 Port의 역할

```
  RelationshipRepository        ChronicleRepository
  (현재 상태 = 통장 표지)        (변화 이력 = 거래 내역)
  ─────────────────────         ─────────────────────
  "지금 호감도가 몇이야?"        "언제 어디서 뭐가 바뀌었어?"
  save()  → 종료 시 1회          append()  → 매 턴 이벤트 발생 시
  find_by_pair() → 시작 시       find_by_pair() → 인연 일지 UI
  find_by_source() → 향후        find_by_session() → 세션 리뷰
                                 find_by_change_type() → 퀘스트 조건
```

---

## 4. 도메인 모델

### 4.1 RelationshipRepository Port (wuxia-core)

```rust
/// 관계 저장소 Port — "관계를 저장하고 꺼내줘"
///
/// 도메인이 아는 것: 이 trait의 메서드들
/// 도메인이 모르는 것: JSON인지, SQLite인지, 파일이 몇 개인지
pub trait RelationshipRepository: Send + Sync {
    /// 관계 한 건을 저장한다 (없으면 생성, 있으면 갱신).
    fn save(&mut self, rel: &Relationship) -> Result<(), String>;

    /// 특정 쌍의 관계를 조회한다.
    fn find_by_pair(
        &self,
        source: CharacterId,
        target: CharacterId,
    ) -> Result<Option<Relationship>, String>;

    /// 특정 캐릭터가 source인 모든 관계를 조회한다.
    /// "소연이 맺고 있는 모든 관계"
    fn find_by_source(
        &self,
        source: CharacterId,
    ) -> Result<Vec<Relationship>, String>;

    /// 특정 캐릭터가 target인 모든 관계를 조회한다.
    /// "플레이어를 향한 모든 NPC의 관계"
    fn find_by_target(
        &self,
        target: CharacterId,
    ) -> Result<Vec<Relationship>, String>;
}
```

### 4.2 RelationshipChronicle 도메인 모델 (wuxia-core)

관계 변화를 기록하는 구조체. "강호 인연록(因緣錄)" — 게임 세계의 객관적 기록이다.

기억(Observation)과의 차이: Observation은 NPC의 주관적 기억("그때 고마웠다"), Chronicle은 게임 세계의 객관적 사실("호감도 23→21로 변경됨").

```rust
/// 관계 변화 연대기 한 건.
///
/// 비유: 강호 인연록의 한 줄.
/// "1200년 3월 2일 저녁, 자유도시 주막에서
///  소연(5)이 플레이어(0)에 대한 호감이 23에서 21로 떨어졌다.
///  사유: 사부에 대해 무례하게 물음. 당무괴(4)가 곁에서 지켜보았다."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipChronicle {
    // ── 메타 (시스템 관리) ──
    pub seq: u64,                      // 전역 일련번호 (고유성 + 정렬)
    pub session_id: String,            // 대화 세션 묶음
    pub schema_ver: u32,               // 포맷 버전 (마이그레이션용, 초기값 1)

    // ── 누가 → 누구에게 ──
    pub source: CharacterId,           // 관계 주체 (소연 5)
    pub target: CharacterId,           // 관계 대상 (플레이어 0)

    // ── 언제 ──
    pub game_time: GameTime,           // { year: 1200, month: 3, day: 2 }
    pub game_watch: Option<String>,    // "Evening" (선택)

    // ── 어디서 ──
    pub location: Option<String>,      // "자유도시 주막" (선택)

    // ── 무엇이 변했나 ──
    pub change_type: ChangeType,       // Affinity / Trust / LevelChanged / ...

    // ── 왜 ──
    pub cause: String,                 // "사부에 대해 무례하게 물음"
    pub cause_source: CauseSource,     // Conversation / Action / Event / ...
    pub delta_source: Option<String>,  // "LlmTriggeredJudgment" (LLM 판정일 때만)

    // ── 연결 정보 ──
    pub event_group: Option<u64>,      // 같은 사건의 첫 seq 참조 (선택)
    pub witnesses: Vec<CharacterId>,   // 목격자 NPC (없으면 빈 배열)
}

/// 관계 변화의 종류.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Affinity { old: f32, new: f32 },
    Trust { old: f32, new: f32 },
    LevelChanged { old_level: String, new_level: String },
    TypeChanged { old_type: Option<String>, new_type: Option<String> },
    BondBroken { reason: String },
}

/// 변화의 원인 분류.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CauseSource {
    Conversation,   // 대화에서 발생
    Action,         // 행동에서 발생 (호위, 구출 등)
    Event,          // 세계 이벤트 (전쟁, 재해 등)
    TimePassage,    // 시간 경과에 의한 자연 변화
    ThirdParty,     // 제3자의 영향
}
```

### 4.3 ChronicleRepository Port (wuxia-core)

```rust
/// 관계 연대기 저장소 Port — 강호 인연록의 서기관
///
/// 관계 변화를 시간순으로 기록하고, 다양한 조건으로 조회한다.
/// 구현체는 JSONL, SQLite, InMemory 등 무엇이든 될 수 있다.
pub trait ChronicleRepository: Send + Sync {
    /// 연대기 한 건을 기록한다. 기록된 항목의 seq를 반환한다.
    fn append(&mut self, entry: RelationshipChronicle) -> Result<u64, String>;

    /// 특정 관계 쌍의 이력을 시간순으로 조회한다.
    /// 인연 일지 UI의 데이터 소스.
    fn find_by_pair(
        &self,
        source: CharacterId,
        target: CharacterId,
    ) -> Result<Vec<RelationshipChronicle>, String>;

    /// 특정 세션에서 발생한 모든 변화를 조회한다.
    /// "이번 만남에서 무슨 일이 있었나"
    fn find_by_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<RelationshipChronicle>, String>;

    /// 특정 change_type으로 필터한다.
    /// 퀘스트 조건: "적대 관계가 된 적 있는가?"
    fn find_by_change_type(
        &self,
        source: CharacterId,
        target: CharacterId,
        change_type: &str,
    ) -> Result<Vec<RelationshipChronicle>, String>;

    /// 전체 기록 수를 반환한다.
    fn count(&self) -> Result<u64, String>;
}
```

### 4.4 게임에서의 사용 시나리오

인연 일지 UI: `find_by_pair(소연, 플레이어)` → 소연과의 관계 이력을 시간순으로 표시.

NPC 회상: Chronicle의 구조화된 기록 + LanceDB Observation의 주관적 기억을 연결하여 "무엇이 바뀌었는지(이력서)" + "어떻게 느꼈는지(일기)"를 조합.

퀘스트 조건: `find_by_change_type(소연, 플레이어, "LevelChanged")` → "소연과 적대 관계가 된 적 있는가?" 판정.

NPC 간 관계 그래프: 전체 Chronicle을 스캔하여 관계 네트워크("강호 인맥도") 시각화.

밸런싱/디버깅: delta_source별 필터링으로 "극단 앵커가 너무 민감한가?" 분석.

---

## 5. 설계

### 5.1 목표 흐름

```
  ■ 시작 시
  relationships.json ──로딩──▶ Relationship (없으면 Stranger 생성)
  descriptions.toml  ──로딩──▶ RelationshipDescriptions
  ──조합──▶ build_relationship_view()
  ──주입──▶ ChatSession.relationship_view = Some(view)

  ■ 매 턴 (send 직후)
  reply.affinity_delta
  ──적용──▶ apply_conversation_effect(&mut relationship, &effect)
  ──변환──▶ build_relationship_view(&relationship, &descs, "ko")
  ──갱신──▶ session.update_relationship_view(Some(new_view))
  ──기록──▶ chronicle_repo.append(chronicle)  ← 신규
  ──효과──▶ 다음 send()의 <Relationship> 섹션이 최신 상태 반영

  ■ 종료 시 (/quit)
  relationship.record_interaction(game_time)
  ──저장──▶ rel_repo.save(&relationship)      ← Port 경유
  + 기억 저장 (기존 Iter 2~3 로직 유지)
```

### 5.2 전체 데이터 흐름도

```
  ┌──────────────────────────────────────────────────────────┐
  │  main() 시작                                              │
  │                                                           │
  │  ① rel_repo = JsonFileRelRepo("data/relationships.json") │
  │     chronicle_repo = JsonlChronicleRepo(                  │
  │         "data/relationship_chronicles.jsonl")              │
  │                                                           │
  │  ② rel_repo.find_by_pair(SOYEON, PLAYER)                 │
  │     ├─ 있음 → Relationship { affinity: 23.0, ... }       │
  │     └─ 없음 → Relationship::new(id, SOYEON, PLAYER)      │
  │                                                           │
  │  ③ load_descriptions("assets/data/.../descriptions.toml")│
  │     → RelationshipDescriptions (8 levels + 5 trust)      │
  │                                                           │
  │  ④ build_relationship_view(&rel, &descs, "ko")           │
  │     → RelationshipView {                                  │
  │          level_label: "아는 사이",                         │
  │          level_desc:  "이름 정도는 기억한다.",              │
  │          trust_desc:  "아직 신뢰하지 않는다.",              │
  │       }                                                   │
  │                                                           │
  │  ⑤ create_session(..., Some(initial_view))                │
  │     → ChatSession.relationship_view = Some(view)          │
  └──────────────────────────────────────────────────────────┘
          │
          ▼
  ┌──────────────────────────────────────────────────────────┐
  │  run_chat_loop() — 매 턴 반복                              │
  │                                                           │
  │  ⑥ session.send(input)                                    │
  │     내부: build_turn_context()                             │
  │       → format_relationship()                             │
  │       → self.relationship_view 읽기                       │
  │     내부: build_system_prompt()                            │
  │       → <Relationship>아는 사이. 이름 정도는...</Relationship>│
  │     → LLM이 이 톤에 맞춰 응답 생성                         │
  │     → 감정 판정 → affinity_delta: +2                       │
  │                                                           │
  │  ⑦ apply_conversation_effect(&mut rel, effect)            │
  │     → rel.affinity: 23.0 → 25.0                          │
  │     → events: [AffinityChanged]  (레벨 전환 시 +LevelChanged)│
  │                                                           │
  │  ⑧ events → RelationshipChronicle 변환                    │
  │     → chronicle_repo.append(chronicle)                    │
  │     → relationship_chronicles.jsonl에 1행 append           │
  │                                                           │
  │  ⑨ build_relationship_view(&rel, &descs, "ko")           │
  │     → 새 RelationshipView (레벨 변하면 내용도 변경)         │
  │                                                           │
  │  ⑩ session.update_relationship_view(Some(new_view))       │
  │     → 다음 send()가 갱신된 view 사용                       │
  │                                                           │
  │  ⑦~⑩ 반복 → affinity 50+ 넘으면 "친근" 톤으로 전환       │
  └──────────────────────────────────────────────────────────┘
          │  /quit 입력
          ▼
  ┌──────────────────────────────────────────────────────────┐
  │  finalize_session()                                       │
  │                                                           │
  │  ⑪ relationship.record_interaction(game_time)             │
  │     → interaction_count +1, last_interaction 갱신         │
  │                                                           │
  │  ⑫ rel_repo.save(&relationship)                          │
  │     → relationships.json 영속 저장                        │
  │                                                           │
  │  ⑬ 기존 기억 저장 (ObservationDraft → LanceDB)           │
  └──────────────────────────────────────────────────────────┘
          │
          ▼
  ┌──────────────────────────────────────────────────────────┐
  │  다음 실행 시                                              │
  │                                                           │
  │  rel_repo.find_by_pair(SOYEON, PLAYER)                    │
  │  → Relationship { affinity: 25.0, ... } 로딩              │
  │  → 첫 턴부터 Acquaintance 톤으로 대화 시작!               │
  └──────────────────────────────────────────────────────────┘
```

### 5.3 data 폴더 구조

```
  data/
    soyeon_memory.lance/              ← NPC 기억 (LanceDB, 벡터 검색, 기존)
    relationships.json                ← 모든 관계 현재 상태 (종료 시 1회 덮어쓰기)
    relationship_chronicles.jsonl     ← 모든 관계 변화 이력 (매 턴 append)
```

### 5.4 세 저장소의 역할 비교

```
  LanceDB (soyeon_memory.lance)
    무엇: NPC의 주관적 기억 (자연어)
    예시: "그때 비녀를 찾아줬을 때 고마웠다"
    조회: 벡터 유사도 검색 ("비녀와 비슷한 기억 찾아줘")
    기술: LanceDB (wuxia-memory, 기존)

  JSON (relationships.json)
    무엇: 모든 관계의 현재 상태 스냅샷
    예시: { source:5, target:0, affinity:23.0, trust:0.0, ... }
    조회: 시작 시 1회 전체 로딩
    기술: serde_json (wuxia-memory, 신규)

  JSONL (relationship_chronicles.jsonl)
    무엇: 게임 세계의 객관적 관계 변화 기록
    예시: seq:42, Affinity 23→21, "사부 얘기에 불쾌"
    조회: 조건 검색 (쌍별, 시간별, 유형별)
    기술: serde_json line-by-line (wuxia-memory, 신규)
```

### 5.5 relationships.json 파일 포맷

모든 관계를 하나의 배열에 담는다. 소연 전용이 아니라 게임 세계의 모든 관계를 포함한다. MVP에서는 항목 1건(소연→플레이어)뿐이지만, NPC가 늘어나면 같은 파일에 항목만 추가된다.

```json
{
  "version": "1.0",
  "updated_at": "2026-03-01T22:30:00+09:00",
  "relationships": [
    {
      "id": 1,
      "source": 5,
      "target": 0,
      "relation_type": null,
      "affinity": 23.0,
      "trust": 0.0,
      "interaction_count": 3,
      "last_interaction": { "year": 1200, "month": 3, "day": 15 }
    }
  ]
}
```

Relationship 구조체는 현재 8필드이며, Serialize/Deserialize를 이미 derive하고 있으므로 필드가 추가되어도 직렬화는 자동으로 따라간다. 향후 심리 아키텍처(Phase 2.4)에서 감정 상태, wuxia 특화 속성(은혜/원한) 등이 추가될 수 있다.

### 5.6 relationship_chronicles.jsonl 파일 포맷

한 줄이 하나의 RelationshipChronicle JSON 객체이다. Append-only이므로 쓰기 도중 비정상 종료되어도 기존 줄은 보존된다.

```jsonl
{"seq":41,"session_id":"s_003","schema_ver":1,"source":5,"target":0,"game_time":{"year":1200,"month":3,"day":2},"game_watch":"Evening","location":"자유도시 주막","change_type":{"Affinity":{"old":23.0,"new":21.0}},"cause":"사부에 대해 무례하게 물음","cause_source":"Conversation","delta_source":"LlmTriggeredJudgment","event_group":null,"witnesses":[4]}
{"seq":42,"session_id":"s_003","schema_ver":1,"source":5,"target":0,"game_time":{"year":1200,"month":3,"day":2},"game_watch":"Evening","location":"자유도시 주막","change_type":{"LevelChanged":{"old_level":"Acquaintance","new_level":"Stranger"}},"cause":"사부에 대해 무례하게 물음","cause_source":"Conversation","delta_source":"LlmTriggeredJudgment","event_group":41,"witnesses":[4]}
```

seq:42의 event_group이 41을 참조하는 것은, 같은 사건("무례한 질문")에서 호감도 변화(seq:41)와 레벨 변화(seq:42)가 함께 발생했음을 나타낸다.

### 5.7 향후 어댑터 교체 시나리오

JSONL이 수천 건으로 커져서 조건 검색이 느려지면, ChronicleRepository 어댑터를 SQLite로 교체한다. 도메인 코드(trait, 도메인 모델)와 Application Service(soyeon_chat_v2.rs에서 trait 메서드 호출하는 부분)는 변경 없이, Composition Root(main)에서 `JsonlChronicleRepo::new(...)` 를 `SqliteChronicleRepo::new(...)` 로 바꾸기만 하면 된다. RelationshipRepository도 동일하게 교체 가능하다.

---

## 6. 변경 범위 — 레이어별 정리

### 6.1 Layer A: wuxia-core (도메인) — Port 2개 + 도메인 모델 추가

| 요소 | 파일 | 상태 | 변경 |
|------|------|------|------|
| Relationship (Serialize/Deserialize) | relationship/types.rs | ✅ 기존 | 변경 없음 |
| RelationshipLevel, TrustLevel | relationship/level.rs, trust_level.rs | ✅ 기존 | 변경 없음 |
| RelationshipDescriptions | relationship/description.rs | ✅ 기존 | 변경 없음 |
| ConversationEffect + apply_conversation_effect() | relationship/effect.rs | ✅ 기존 | 변경 없음 |
| DomainEvent::Relationship(...) | relationship/event.rs | ✅ 기존 | 변경 없음 |
| **RelationshipRepository trait** | **relationship/repository.rs** | **🆕 신규** | **Port 정의** |
| **RelationshipChronicle struct** | **relationship/chronicle.rs** | **🆕 신규** | **도메인 모델** |
| **ChangeType, CauseSource enum** | **relationship/chronicle.rs** | **🆕 신규** | **값 객체** |
| **ChronicleRepository trait** | **relationship/chronicle.rs** | **🆕 신규** | **Port 정의** |

### 6.2 Layer B: wuxia-llm (session.rs) — 메서드 1개 추가

ChatSession에 `update_relationship_view()` setter를 추가한다. 현재 `relationship_view`는 생성 시 한 번만 설정되며, types.rs의 TODO가 이를 인지하고 있다.

```rust
impl<C: ContextProvider> ChatSession<C> {
    /// 관계 상태 뷰를 갱신한다. [Iter 4]
    ///
    /// 매 턴 affinity_delta 반영 후 호출하면,
    /// 다음 send()의 프롬프트 <Relationship> 섹션이 갱신된다.
    pub fn update_relationship_view(&mut self, view: Option<RelationshipView>) {
        self.relationship_view = view;
    }
}
```

테스트 2개 추가:

| # | 테스트 | 검증 내용 |
|---|--------|----------|
| 1 | update_view_changes_value | setter가 None → Some으로 변경되는지 |
| 2 | updated_view_affects_format | 변경 후 format_relationship()이 새 값을 반환하는지 |

### 6.3 Layer C: wuxia-memory — 어댑터 4개 추가

| 어댑터 | 구현하는 Port | 기술 의존성 | 용도 |
|--------|-------------|-----------|------|
| InMemoryRelRepo | RelationshipRepository | 없음 (Vec) | 단위 테스트 |
| JsonFileRelRepo | RelationshipRepository | serde_json, std::fs | 프로덕션 MVP |
| InMemoryChronicleRepo | ChronicleRepository | 없음 (Vec) | 단위 테스트 |
| JsonlChronicleRepo | ChronicleRepository | serde_json, std::fs | 프로덕션 MVP |

### 6.4 Layer D: soyeon_chat_v2.rs (Composition Root) — 핵심 변경

#### D-1. build_relationship_view() 헬퍼 함수

Relationship 수치와 descriptions.toml 데이터를 조합해서 RelationshipView를 생성하는 순수 함수다. 매 턴 호출된다.

```rust
fn build_relationship_view(
    rel: &Relationship,
    descs: &RelationshipDescriptions,
    locale: &str,
) -> RelationshipView {
    let level_key = rel.level().key();
    let trust_key = rel.trust_level().key();
    let (level_label, level_desc) = descs
        .lookup_relationship_level(level_key, locale)
        .unwrap_or(("???", ""));
    let (_, trust_desc) = descs
        .lookup_trust_level(trust_key, locale)
        .unwrap_or(("???", ""));
    RelationshipView {
        level_label: level_label.to_string(),
        level_desc: level_desc.to_string(),
        trust_desc: trust_desc.to_string(),
    }
}
```

#### D-2. main() — 저장소 생성 + 관계 로딩

```rust
// Composition Root — 어댑터 구현체를 선택하고 조립하는 유일한 장소
let mut rel_repo = JsonFileRelRepo::new("data/relationships.json");
let mut chronicle_repo = JsonlChronicleRepo::new("data/relationship_chronicles.jsonl");

let relationship = rel_repo
    .find_by_pair(SOYEON_ID, PLAYER_ID)?
    .unwrap_or_else(|| Relationship::new(next_id(), SOYEON_ID, PLAYER_ID));
```

#### D-3. run_chat_loop() — 매 턴 관계 반영 + 연대기 기록

```rust
// send() 직후 삽입되는 코드

if reply.affinity_delta != 0 {
    let effect = ConversationEffect::with_source(
        reply.affinity_delta,
        reply.sentiment_detail.as_ref()
            .map(|d| d.source)
            .unwrap_or(DeltaSource::LegacyTag),
    );
    let events = apply_conversation_effect(relationship, &effect);

    // 연대기 기록 (이벤트 → Chronicle 변환 → append)
    for event in &events {
        let chronicle = event_to_chronicle(event, &session_id, &game_time);
        chronicle_repo.append(chronicle)?;
    }

    // 레벨 전환 알림
    for event in &events {
        if let DomainEvent::Relationship(
            RelationshipEvent::LevelChanged { old_level, new_level, .. }
        ) = event {
            println!("  ⚡ 관계 변화: {:?} → {:?}", old_level, new_level);
        }
    }
}

// 매 턴 RelationshipView 갱신 (delta 0이어도 실행 — 비용 무시 가능)
let new_view = build_relationship_view(relationship, descriptions, "ko");
session.update_relationship_view(Some(new_view));
```

#### D-4. finalize_session() — 관계 저장

```rust
relationship.record_interaction(game_time);
rel_repo.save(relationship)?;
// 기존 기억 저장 유지
```

### 6.5 Cargo.toml 의존성

```toml
# wuxia-app/Cargo.toml에 추가
[dependencies]
serde_json = "1"        # 관계 JSON/JSONL 영속화
chrono = "0.4"          # updated_at 타임스탬프 (선택)
```

---

## 7. RelationshipView 구조 확인

코드 확인 결과, RelationshipView는 필드 3개로 구성된다.

```rust
// wuxia-llm/src/prompt/types.rs
pub struct RelationshipView {
    pub level_label: String,   // "친근"
    pub level_desc: String,    // "상대에게 호감이 있다."
    pub trust_desc: String,    // "어느 정도 신뢰하지만, 비밀은 아직."
}
```

format_relationship_for_prompt()가 이 세 필드를 조합하여 프롬프트 문자열을 만든다. build_system_prompt()가 `<Relationship>` XML 태그 안에 삽입한다.

descriptions.toml은 `assets/data/relationship/descriptions.toml` 경로에 존재하며, `wuxia-data::relationship_desc::load_descriptions()` 함수가 이미 구현되어 있다. Iter 4에서 새로 만들 코드 없이 그대로 호출한다.

---

## 8. 프롬프트 즉시 반영 메커니즘 상세

```
  session.update_relationship_view(Some(new_view))
  → self.relationship_view = Some(new_view)       // ① setter

  다음 턴: session.send(user_input)
  → self.build_turn_context(user_input)            // ② 매 턴 호출
    → self.format_relationship()                   // ③ 관계 포맷팅
      → self.relationship_view.as_ref().map(...)   // ④ 최신 view 읽기
      → format_relationship_for_prompt(view, ...)  // ⑤ 자연어 변환
    → PromptContext { relationship_summary: Some("관계: 친근\n호감이 있다.") }

  → build_system_prompt(&prompt_data, ..., &context)
    → <Relationship>관계: 친근. 호감이 있다.</Relationship>  // ⑥ 프롬프트 삽입
```

비용: HashMap lookup 2회 (level + trust) + String 할당 3회. LLM 추론 500~3000ms 대비 무시 가능한 수준이다.

---

## 9. 레벨 전환 시나리오 (예상 플레이)

```
  세션 1 (첫 만남):
    시작: Stranger (affinity 0, trust 0)
    프롬프트: "아직 모르는 사이. 경계를 늦추지 않는다."
    소연 대사 톤: "누구야? 뭘 원하는 건데."
    10턴 대화, 평균 delta +2/턴
    종료: affinity 20 → ⚡ Acquaintance 전환!
    relationships.json 저장: { affinity: 20.0, trust: 0.0, count: 1 }
    chronicles.jsonl: 10여 건의 Affinity 변화 + 1건의 LevelChanged 기록

  세션 2 (재회):
    시작: Acquaintance 로딩 (affinity 20)
    프롬프트: "이름 정도는 기억한다. 경계는 풀었다."
    소연 대사 톤: "어, 또 왔어? 뭐 필요한 거 있어?"
    15턴 대화
    종료: affinity 42 → 아직 Acquaintance
    relationships.json 저장: { affinity: 42.0, trust: 0.0, count: 2 }

  세션 3 (친해지는 중):
    시작: Acquaintance (affinity 42)
    10턴째에 affinity 50 도달
    → trust가 30 미만이므로 아직 Friendly 아님
    → 대화로는 trust가 오르지 않음 (relationship-mechanic §4.1)
    → Friendly 전환은 '행동' 이벤트가 필요
    종료: affinity 60 → 아직 Acquaintance (trust 부족)

  세션 4 이후: 호위/구출 이벤트로 trust 획득 → Friendly 전환
```

---

## 10. 구현 순서 (6단계 Iterative)

각 단계마다 빌드/테스트 전에 사용자 확인을 요청한다.

### 10.1 Iter 4-A: session.rs setter 추가 ✅ 완료

| 항목 | 내용 |
|------|------|
| **목표** | ChatSession에 update_relationship_view() 추가 |
| **변경 파일** | `wuxia-llm/src/conversation/session.rs` |
| **추가 코드** | pub fn update_relationship_view() — 4줄 |
| **테스트** | +2개 (setter 동작, format_relationship 반영) |
| **검증** | `cargo test -p wuxia-llm` 통과 |

### 10.2 Iter 4-B: wuxia-core Port + 도메인 모델 ✅ 완료

| 항목 | 내용 |
|------|------|
| **목표** | RelationshipRepository trait, RelationshipChronicle struct, ChronicleRepository trait |
| **신규 파일** | `wuxia-core/src/relationship/repository.rs`, `wuxia-core/src/relationship/chronicle.rs` |
| **변경 파일** | `wuxia-core/src/relationship/mod.rs` (re-export 추가) |
| **테스트** | Chronicle 생성 + ChangeType 직렬화 라운드트립 |
| **검증** | `cargo test -p wuxia-core` 통과 |

### 10.3 Iter 4-C: InMemory 어댑터 + 테스트 ✅ 완료

| 항목 | 내용 |
|------|------|
| **목표** | InMemoryRelRepo, InMemoryChronicleRepo (테스트용 어댑터) |
| **신규 파일** | `wuxia-memory/src/relationship_store/in_memory.rs`, `wuxia-memory/src/chronicle/in_memory.rs` |
| **테스트** | save/find_by_pair/find_by_source 라운드트립, append/find_by_pair/find_by_session 라운드트립 |
| **검증** | `cargo test -p wuxia-memory` 통과 |

### 10.4 Iter 4-D: JSON/JSONL 어댑터 ✅ 완료

| 항목 | 내용 |
|------|------|
| **목표** | JsonFileRelRepo, JsonlChronicleRepo (프로덕션 MVP 어댑터) |
| **신규 파일** | `wuxia-memory/src/relationship_store/json_file.rs`, `wuxia-memory/src/chronicle/jsonl.rs` |
| **테스트** | 파일 I/O 라운드트립 (임시 디렉토리), append 후 재로딩, JSONL 부분 파싱 |
| **검증** | `cargo test -p wuxia-memory` 통과 |
| **의존성** | serde_json (wuxia-memory Cargo.toml 추가) |

### 10.5 Iter 4-E: soyeon_chat_v2 전체 연결 ✅ 완료

| 항목 | 내용 |
|------|------|
| **목표** | 전체 파이프라인 연결 (저장소 생성→로딩→매턴반영→연대기기록→저장) |
| **변경 파일** | `wuxia-app/examples/soyeon_chat_v2.rs`, `wuxia-app/Cargo.toml` |
| **변경 함수** | main(), run_chat_loop(), finalize_session() |
| **핵심 변경** | Composition Root에서 어댑터 생성, create_session에 initial_view 주입 |
| **검증** | `cargo check -p wuxia-app --features live-demo` 통과 |

### 10.6 Iter 4-F: 플레이테스트 + 디버그 보강 ✅ 완료

| 항목 | 내용 |
|------|------|
| **목표** | 실제 동작 검증 + UX 개선 |
| **추가 기능** | /info에 관계 상태, 레벨 전환 ⚡ 메시지, 디버그 관계+연대기 출력 |
| **검증** | 실제 실행 + 대화 + /quit + 재실행 확인 |

검증 체크리스트:

| # | 검증 항목 | 확인 방법 |
|---|----------|----------|
| 1 | 첫 실행 시 Stranger로 시작 | 출력 확인 |
| 2 | 대화 중 affinity 변화 | 디버그 모드 관계 출력 |
| 3 | /quit 시 relationships.json 저장 | data/relationships.json 파일 확인 |
| 4 | /quit 시 chronicles.jsonl에 이력 기록 | data/relationship_chronicles.jsonl 확인 |
| 5 | 재실행 시 이전 관계 로딩 | 출력에서 저장된 affinity 값 확인 |
| 6 | 프롬프트에 관계 반영 | 디버그 모드 프롬프트 출력 확인 |
| 7 | 레벨 전환 시 메시지 | ⚡ 메시지 출력 확인 |
| 8 | /info에 관계 표시 | /info 명령어 확인 |

---

## 11. 파일 변경 매트릭스

| 파일 | 4-A | 4-B | 4-C | 4-D | 4-E | 4-F |
|------|:---:|:---:|:---:|:---:|:---:|:---:|
| wuxia-llm/.../session.rs | **수정** | | | | | |
| wuxia-core/.../repository.rs (신규) | | **생성** | | | | |
| wuxia-core/.../chronicle.rs (신규) | | **생성** | | | | |
| wuxia-core/.../mod.rs | | **수정** | | | | |
| wuxia-memory/.../relationship_store/ (신규) | | | **생성** | **생성** | | |
| wuxia-memory/.../chronicle/ (신규) | | | **생성** | **생성** | | |
| wuxia-memory/Cargo.toml | | | | **수정** | | |
| wuxia-app/Cargo.toml | | | | | **수정** | |
| wuxia-app/examples/soyeon_chat_v2.rs | | | | | **수정** | **수정** |
| data/relationships.json | | | | | | **생성**(런타임) |
| data/relationship_chronicles.jsonl | | | | | | **생성**(런타임) |

### 11.1 soyeon_chat_v2.rs 주석 헤더 갱신

```
  ✅ Iter 1: ChatSession + NullContextProvider
  ✅ Iter 2: LiveContextProvider + LanceDB
  ✅ Iter 3: SentimentPipeline
  ✅ Iter 4: Relationship + Chronicle (관계 영속 + 연대기)  ← 갱신
  ⬜ Iter 5: CLI 완성 + 디버그 모드
```

---

## 12. 설계 결정 사항

### 12.1 JSON/JSONL은 어댑터이며 교체 가능하다

영속 기술(JSON, JSONL, SQLite 등)은 도메인의 관심사가 아니라 어댑터의 구현 세부사항이다. 도메인은 RelationshipRepository와 ChronicleRepository trait만 알고, 그 뒤에 어떤 기술이 있는지 모른다. 이는 기억 도메인에서 MemoryRepository → InMemoryRepository / LanceDbRepository로 구현한 것과 동일한 패턴이다 (dependency-principles.md 원칙 1: Port & Adapter).

MVP에서 JSON/JSONL을 선택한 이유는 세 가지이다. 첫째, NPC 1명(소연)의 관계 1건을 다루는 수준에서 DB는 오버엔지니어링이다. 둘째, JSON/JSONL은 사람이 직접 읽고 수정할 수 있어 디버깅에 유리하다. 셋째, JSONL의 append-only 특성은 비정상 종료 시에도 기존 기록을 보존한다.

향후 JSONL 검색이 느려지면 (수천 건 이상) SQLite로 교체한다. Composition Root(main)에서 어댑터 구현체를 바꾸기만 하면 되고, 도메인 코드는 한 줄도 변경하지 않는다.

### 12.2 relationships.json은 모든 관계를 담는다

파일명이 `soyeon_relationship.json`이 아니라 `relationships.json`인 이유는, 이 파일이 소연 전용이 아니라 게임 세계의 모든 관계를 배열로 담기 때문이다. NPC가 10명이 되어도 파일 하나에 항목만 늘어난다. "파일을 관계별로 나눌까, 하나로 합칠까?"는 어댑터가 결정할 문제이며, JsonFileRelRepo는 단일 파일 + 배열 방식을 선택했다.

### 12.3 왜 매 턴 build_relationship_view인가

delta가 0인 턴에도 view를 갱신하는 이유는 두 가지다. 첫째, HashMap lookup 2회 + String 3회 할당의 비용이 LLM 추론(500~3000ms) 대비 무시 가능하다. 둘째, "delta가 0이면 스킵"하는 조건 분기를 넣으면 향후 trust 변경 등 다른 축이 추가될 때 분기 조건이 복잡해진다.

### 12.4 왜 콜백이 아닌 setter인가

setter 방식은 Application Service(soyeon_chat_v2.rs)가 관계 갱신 책임을 명시적으로 가지므로 아키텍처가 명확하다. ChatSession은 "프롬프트 빌드 + LLM 호출"만 담당하고, "관계 갱신 판단"은 상위 계층이 담당하는 관심사 분리가 유지된다.

### 12.5 cumulative_affinity_delta와의 관계

기존 ChatSession 내부의 cumulative_affinity_delta는 유지한다. Application Service가 이 값과 실제 Relationship.affinity() 변화량이 일치하는지 교차 검증할 수 있다. 이중 추적이지만 디버깅 가치가 있으므로 당분간 유지한다.

### 12.6 영속 시점 정리

| 저장소 | 시점 | 방식 | 비정상 종료 시 |
|--------|------|------|---------------|
| relationships.json | /quit 시 1회 | 전체 덮어쓰기 | 이전 세션 상태 유지 |
| relationship_chronicles.jsonl | 매 턴 이벤트 발생 시 | 1행 append | 마지막 줄만 손실 가능 |
| soyeon_memory.lance (기존) | /quit 시 | LanceDB add | 이전 세션 기억 유지 |

---

## 13. 위험 요소 및 대응

| 위험 | 영향 | 대응 |
|------|------|------|
| Relationship Serialize 포맷 변경 시 기존 JSON 로딩 실패 | 이전 세션 관계 소멸 | version 필드로 마이그레이션 감지, 실패 시 새로 생성 |
| descriptions.toml에 key 누락 시 lookup 실패 | 프롬프트에 "???" 표시 | unwrap_or 방어 코딩 + 시작 시 key 개수 검증 |
| JSONL이 수천 건으로 커지면 find_by_pair 느려짐 | 조회 지연 | SQLite 어댑터로 교체 (Port 변경 없음) |
| 대화 중 비정상 종료 시 relationships.json 미저장 | 관계 소멸 | Iter 5에서 주기적 자동 저장 검토 (10턴마다 등) |
| chronicles.jsonl 마지막 줄 깨짐 | 마지막 1건 손실 | 로딩 시 마지막 줄 파싱 실패하면 건너뛰기 |

---

## 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|------|----------|-----------|
| v1.0.0 | 2026-03-01T22:00:00+09:00 | 초기 작성. 현재 상태 진단(3곳 끊김), 목표 흐름, 전체 데이터 흐름도, 변경 범위 4레이어(core 변경없음/llm setter 1개/app 5건/cargo.toml), RelationshipView 구조 확인(3필드), 프롬프트 즉시 반영 메커니즘 코드 경로 추적, 레벨 전환 시나리오 4세션, 구현 순서 4단계(A~D), 파일 변경 매트릭스, 설계 결정 4건(JSON 선택/매턴 갱신/setter vs 콜백/이중 추적), 위험 요소 4건. |
| v2.0.0 | 2026-03-02T01:30:00+09:00 | Port & Adapter 영속 아키텍처 전면 도입. §3 신설: 영속 아키텍처(RelationshipRepository + ChronicleRepository trait), 저장소 역할 분류(wuxia-data vs wuxia-memory), wuxia-memory 모듈 구조. §4 신설: 도메인 모델(RelationshipChronicle 13필드, ChangeType/CauseSource enum), Port 2개(RelationshipRepository 4메서드, ChronicleRepository 5메서드), 게임 사용 시나리오 5건. §5 대폭 수정: 파일명 soyeon_relationship.json→relationships.json(모든 관계 배열), Relationship 전체 8필드 JSON 예시, relationship_chronicles.jsonl 포맷+예시, 데이터 흐름도에 chronicle_repo.append 단계 추가, 향후 어댑터 교체 시나리오. §6 수정: Layer A "변경 없음"→"Port 2개+도메인모델 추가", Layer C 신설(wuxia-memory 어댑터 4개). §10 수정: 구현 순서 4단계→6단계(B: Port+모델, C: InMemory어댑터, D: JSON/JSONL어댑터 분리). §11 파일 매트릭스 6단계로 확장. §12 설계 결정: 10.1→12.1 "JSON/JSONL은 교체 가능 어댑터" 관점 전환, 12.2 "모든 관계를 한 파일에" 추가, 12.6 영속 시점 정리표 추가. §13 위험: JSONL 성능+깨짐 대응 추가. |
