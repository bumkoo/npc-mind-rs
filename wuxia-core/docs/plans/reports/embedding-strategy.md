# 칠국춘추 — Sprint 3 Step 3.2: 임베딩 저장 및 조회 전략

> **문서 버전**: v1.2.0  
> **작성일**: 2026-02-21 05:10  
> **선정 모델**: bge-m3-Q4_K_M (후보 C)  
> **근거 문서**: `step3.1-embedding-benchmark-report.md` v2.0.0  
> **관련 문서**: `sprint3-progress.md`

---

## 1. 모델 선정 근거

벤치마크 10개 후보 중 bge-m3-Q4_K_M(C)를 적용 모델로 선정한다.

### 1.1 선정 이유

bge-m3-Q4_K_M는 동일 모델(bge-m3)의 상위 양자화(q8_0, 후보 B)와 β/α 비율이 KO 3.6x로 동률이면서, 실용 지표에서 전면 우위를 보인다. 속도 36ms로 B 대비 28% 빠르고, 파일 418MB로 31% 작으며, RAM 약 420MB로 30% 절약된다. 3개 언어 β 편차가 0.003으로 전 모델 중 가장 안정적이다. KO α=0.076은 B(0.082)보다 낮아 의역 인식도 미세하게 우수하다.

KO β 절대값은 B(0.299)보다 0.027 낮은 0.272이나, 둘 다 ✅ 기준(>0.20)을 충족하며 Top-K 검색에서 0.027 차이가 순위 역전을 일으킬 가능성은 낮다.

### 1.2 β/α 비율이 Top-K 정확도를 결정하는 이유

모델 선정의 핵심 기준이었던 β/α 비율은 본 전략의 기본 조회 방식인 Top-K 검색의 정확도를 직접 예측하는 지표다. 그 메커니즘을 설명한다.

NPC "남궁현"이 기억 100개를 보유하고, 플레이어가 "혈교 놈들이 또 마을을 습격했다며?"라고 물었다고 가정한다. Top-5 검색의 목표는 혈교 관련 기억 5개를 정확히 뽑아내는 것이다. 이때 벤치마크의 4단계 레벨(L1~L4)이 이 100개 기억의 코사인 유사도 분포를 예측한다.

```
  남궁현의 기억 100개와 쿼리 "혈교 습격"의 유사도 분포 (C 모델 기준):

  점수 0.70 ┤ ██  혈교 원문 기억 (L1 수준)     ← Top-5에 포함 ✓
            │
  ── α=0.076 간격 ──
            │
  점수 0.62 ┤ ████████  혈교 의역 기억 (L2 수준) ← Top-5에 포함 ✓
            │
            │
            │  ★ β=0.272 간격 (넓은 안전지대)
            │  이 간격 덕분에 다른 주제가 Top-5에 침투 불가
            │
  점수 0.35 ┤ ████████████████  소연/무공 기억 (L3 수준) ← Top-5 밖 ✓
            │
  점수 0.40 ┤ ████████████████████████  일상 기억 (L4 수준) ← Top-5 밖 ✓
```

β(=0.272)는 "관련 기억(L2)과 무관 기억(L3) 사이의 점수 간격"이다. 이 간격이 넓으면 Top-K에서 무관 기억이 관련 기억을 밀어내고 올라올 수 없다. α(=0.076)는 "의역 기억이 원문 기억에서 떨어진 거리"이다. 이 값이 작으면 의역 표현도 원문 근처에 위치하여 Top-K에 잘 포함된다. α가 너무 크면 의역 기억의 점수가 낮아져서, 관련 기억인데도 Top-K에서 밀리는 "의역 누락"이 발생한다.

β/α 비율은 이 두 효과의 상대적 크기를 나타낸다. C의 KO β/α=3.6x는 "다른 주제를 밀어내는 간격(β)이 의역 누락 위험(α)의 3.6배"라는 의미다. 즉 Top-5에서 주제가 다른 기억이 침투할 가능성보다, 의역 기억이 Top-5에 포함될 가능성이 3.6배 높다. 비교로 D(e5-large)의 KO β/α=3.2x지만 β 절대값이 0.071로, L2(0.88)와 L3(0.81) 사이가 0.07밖에 안 되어 점수 분포가 겹치고 Top-5에 소연 기억이 섞여 들어온다.

### 1.3 폴백 계획

실전 통합 중 Q4 양자화의 품질 저하가 체감될 경우, 동일 아키텍처의 bge-m3-q8_0(후보 B)으로 교체한다. 코드 변경은 모델 파일 경로 한 줄뿐이며, 차원(1024)과 LanceDB 스키마가 동일하므로 재인덱싱만 필요하다.


---

## 2. 모델 운용 아키텍처

### 2.1 하드웨어 배치

```
  ┌──────────────────────────────────────────────────────┐
  │                  시스템 RAM (32GB)                     │
  │                                                       │
  │   bge-m3-Q4_K_M (~420MB)    ← CPU 임베딩 전용         │
  │   LanceDB 인덱스 + 메모리 캐시                         │
  │   Bevy ECS + 게임 로직                                │
  │                                                       │
  └──────────────────────────────────────────────────────┘
  ┌──────────────────────────────────────────────────────┐
  │              RTX 2070S VRAM (8GB)                      │
  │                                                       │
  │   gemma3:12b Q4_K_M (7.6GB)   ← NPC LLM 전용         │
  │                                                       │
  └──────────────────────────────────────────────────────┘
```

임베딩 모델은 CPU 전용(n_gpu_layers=0)으로 실행하여 GPU VRAM을 NPC LLM에 전부 할당한다. 36ms/건 속도는 LLM 응답(500~3000ms) 대기 시간 안에 완료되므로 체감 지연이 없다.

### 2.2 모델 로딩

LlamaCppEmbedding 어댑터의 기존 `new()` 메서드가 이미 n_gpu_layers=0을 사용하므로 변경 없이 적용 가능하다.

```rust
// 기존 코드 그대로 사용
let embedding = LlamaCppEmbedding::new(
    "models/bge-m3-Q4_K_M.gguf",
    "bge-m3-Q4"
)?;
// → 내부적으로 with_options(path, name, 0, 512) 호출
// → n_gpu_layers=0, n_ctx=512
```

모델은 게임 시작 시 1회 로딩(약 4초)하고 프로세스 수명 동안 유지한다. 스레드 안전(Mutex)이 보장되므로 여러 NPC가 동시에 임베딩을 요청해도 직렬화되어 처리된다.


---

## 3. LanceDB 저장 전략

### 3.1 테이블 스키마

NPC 기억을 저장하는 memories 테이블의 스키마다. bge-m3-Q4_K_M의 1024차원 벡터를 사용한다.

```
memories 테이블
  ┌──────────────────────┬──────────────────┬──────────────────────────────┐
  │ 컬럼                  │ 타입             │ 설명                          │
  ├──────────────────────┼──────────────────┼──────────────────────────────┤
  │ id                   │ String (UUID)    │ 기억 고유 ID                   │
  │ npc_id               │ String           │ 기억의 소유 NPC               │
  │ text                 │ String           │ 기억 원문 텍스트               │
  │ vector               │ FixedSizeList    │ 1024차원 f32 임베딩 벡터       │
  │                      │  <Float32>[1024] │                              │
  │ memory_type          │ String           │ Dialogue / Observation /      │
  │                      │                  │ Reflection / Action           │
  │ importance           │ Float32          │ 기억 중요도 (0.0 ~ 1.0)       │
  │ emotional_intensity  │ Float32          │ 감정 강도 (0.0 ~ 1.0)         │
  │ participants         │ String           │ 관련 인물 ID 목록 (쉼표 구분)  │
  │ location             │ String           │ 사건 발생 장소                 │
  │ game_time            │ String           │ 게임 내 시간 (ISO 8601)        │
  │ created_at           │ String           │ 실제 생성 시간 (ISO 8601)      │
  │ access_count         │ UInt32           │ 회상 횟수 (망각 계산용)         │
  │ last_accessed_at     │ String           │ 마지막 회상 시간               │
  └──────────────────────┴──────────────────┴──────────────────────────────┘
```

### 3.2 저장 용량 추산

1024차원 f32 벡터 1개 = 4,096 bytes (4KB). 메타데이터 약 500 bytes를 더하면 기억 1건 ≈ 4.5KB이다.

```
  NPC 1명 × 하루 평균 20건 기억 생성 (대화 10건 + 관찰 5건 + 성찰 5건)
  NPC 11명 × 게임 내 1년(365일) = 11 × 365 × 20 = 80,300건
  저장 용량 = 80,300 × 4.5KB ≈ 353MB

  → 게임 내 5년 운용 가정 시 약 1.7GB
  → 시스템 RAM과 디스크 모두 충분
```

### 3.3 인덱싱 전략

LanceDB는 자동으로 IVF-PQ(Inverted File Index + Product Quantization) 인덱스를 생성한다. 기억 수가 적은 초기(1만 건 미만)에는 인덱스 없이 브루트포스 검색이 충분히 빠르고, 기억이 축적되면 자동 인덱싱이 활성화된다.

```
  ~10,000건:  브루트포스 충분 (1024차원 × 10K = ~40MB, 1ms 미만)
  ~100,000건: IVF-PQ 인덱스 필요 (nprobe=20, 5ms 이하 목표)
  ~1,000,000건: 파티셔닝 고려 (npc_id별 분리 테이블 또는 필터)
```

### 3.4 파티셔닝 전략

초기에는 단일 memories 테이블로 운용하고, npc_id 필터로 특정 NPC의 기억만 검색한다. NPC 수가 늘거나 기억이 대규모로 축적되면, NPC별 또는 지역별 테이블 분리를 고려한다. 단, 현재 11명 NPC 규모에서는 단일 테이블 + 필터가 가장 단순하고 유지보수 비용이 낮다.


---

## 4. 조회 전략

### 4.1 기본 검색: Top-K

NPC 기억 검색의 기본 패턴은 Top-K다. 쿼리 텍스트를 임베딩한 후, 코사인 유사도가 가장 높은 K개 기억을 반환한다.

```
  플레이어 입력: "혈교 놈들이 또 마을을 습격했다며?"
                    │
                    ▼
  ① 임베딩:   embed("혈교 놈들이 또 마을을 습격했다며?")
              → [0.012, -0.045, 0.103, ...] (1024차원 벡터)
              → 소요: ~36ms (CPU)
                    │
                    ▼
  ② LanceDB 검색:  memories 테이블에서
                   WHERE npc_id = "남궁현"
                   ORDER BY cosine_similarity(query_vector, vector) DESC
                   LIMIT 5
              → 소요: ~1~5ms
                    │
                    ▼
  ③ 반환:    Top-5 기억 (텍스트 + 메타데이터)
```

이 Top-K 검색이 정확하려면 두 가지가 보장되어야 한다. 첫째, 관련 기억(혈교 의역 표현 포함)이 상위에 위치해야 한다. 둘째, 무관 기억(소연·무공·일상)이 상위에 침투하지 않아야 한다. 섹션 1.2에서 설명한 대로, 이것은 정확히 β와 α의 역할이다.

C 모델의 KO β=0.272는 의역 기억(L2≈0.62)과 다른 주제 기억(L3≈0.35) 사이에 0.27의 넓은 간격을 만든다. NPC가 기억 100개를 보유하더라도, 이 간격 덕분에 Top-5에는 혈교 관련 기억만 올라오고 소연의 시장 기억은 훨씬 아래에 위치한다. α=0.076은 의역 기억이 원문에서 크게 벗어나지 않아 Top-K에 잘 포함됨을 보장한다. β/α=3.6x라는 것은 이 "안전 간격"이 "의역 손실"의 3.6배이므로, 실전에서 주제 오염 없이 정확한 기억을 검색할 수 있다는 의미다.

K값은 NPC 대화 컨텍스트에 사용되므로, LLM 프롬프트에 넣을 수 있는 양으로 제한한다. 기본 K=5, 최대 K=10으로 설정한다. gemma3:12b의 128K 컨텍스트 윈도우를 고려하면 기억 10건(각 200자 내외 = ~100토큰)은 1,000토큰 이내로 충분히 여유 있다.

### 4.2 복합 검색: 벡터 + 메타데이터 필터

순수 벡터 검색만으로는 부족한 경우가 있다. 예를 들어 "최근 3일간의 기억"이나 "소연과 관련된 기억"처럼 시간·인물 조건이 붙을 때, LanceDB의 메타데이터 필터를 함께 사용한다.

```
  검색 시나리오별 필터 조합:

  ① "혈교에 대해 뭘 알고 있지?" (순수 의미 검색)
     → vector search only, K=5

  ② "어제 소연이 뭐라고 했지?" (시간 + 인물 필터)
     → WHERE participants LIKE '%소연%'
       AND game_time > '현재-1일'
       ORDER BY cosine_similarity DESC, LIMIT 5

  ③ NPC 자율 성찰 시 (감정 기반 회상)
     → WHERE npc_id = 'self'
       AND emotional_intensity > 0.7
       ORDER BY cosine_similarity DESC, LIMIT 3

  ④ NPC끼리 대화 시 (공유 기억 검색)
     → WHERE participants LIKE '%NPC_A%'
       AND participants LIKE '%NPC_B%'
       ORDER BY cosine_similarity DESC, LIMIT 5
```

### 4.3 검색 점수 계산: retrieval_score

벡터 유사도만으로 순위를 매기면, 오래된 기억과 최근 기억이 동등하게 취급된다. 실제 인간의 기억처럼 "최근 기억이 더 잘 떠오르고, 자주 떠올린 기억이 더 생생하며, 감정적으로 강렬한 기억이 더 오래 남는" 효과를 반영하기 위해 복합 점수를 사용한다.

Sprint 2에서 이미 구현한 retrieval_score 공식을 벡터 검색에 확장한다.

```
  retrieval_score = w1 × similarity     시맨틱 유사도 (벡터 코사인)
                 + w2 × recency         시간 감쇠 (최근일수록 높음)
                 + w3 × importance      기억 중요도 (NPC가 부여)
                 + w4 × emotional       감정 강도
                 + w5 × frequency       회상 빈도 (access_count 기반)

  가중치 초기값:
    w1 = 0.40  (의미적 관련성이 가장 중요)
    w2 = 0.25  (최근 기억 우선)
    w3 = 0.15  (중요한 사건 우선)
    w4 = 0.10  (감정적 기억 강화)
    w5 = 0.10  (자주 떠올린 기억 강화)
```

시간 감쇠 함수는 지수 감쇠를 사용한다.

```
  recency = exp(-decay_rate × days_elapsed)

  decay_rate = 0.05 (기본값)
  → 1일 전:   0.95
  → 7일 전:   0.70
  → 30일 전:  0.22
  → 90일 전:  0.01 (거의 잊힘)

  단, access_count가 높은 기억은 decay_rate를 낮춰 느리게 감쇠시킨다.
  adjusted_decay = decay_rate / (1 + 0.1 × access_count)
```

### 4.4 검색 파이프라인 전체 흐름

```
  플레이어 입력
       │
       ▼
  ┌─────────────┐
  │  ① 임베딩    │  embed(query) → 1024d 벡터, ~36ms
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │  ② 벡터검색  │  LanceDB Top-K×2 (여유분 확보), ~1-5ms
  │  + 필터      │  WHERE npc_id = '...' AND 시간/인물 조건
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │  ③ 리랭킹    │  retrieval_score 복합 점수로 재정렬
  │             │  similarity × 0.4 + recency × 0.25 + ...
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │  ④ Top-K    │  상위 K개 선택 (기본 K=5)
  │  최종 선택   │
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │  ⑤ 프롬프트  │  NPC 성격 + 상황 + 선택된 기억 → LLM 프롬프트
  │  구성        │
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │  ⑥ LLM 응답 │  gemma3:12b → NPC 대사 생성, ~500-3000ms
  └─────────────┘

  전체 소요 시간:
    임베딩(36ms) + 검색(5ms) + 리랭킹(1ms) + LLM(500~3000ms)
    ≈ 542 ~ 3042ms
    → 병목은 LLM이며, 임베딩+검색은 전체의 1~8% 수준
```


---

## 5. 저장 파이프라인

### 5.1 기억 생성 흐름

NPC 기억은 세 가지 경로로 생성된다.

```
  경로 ①: 대화 기억 (Dialogue)
    플레이어 ↔ NPC 대화 후 자동 저장
    → text: 대화 요약 (LLM이 생성)
    → importance: LLM이 판단 (0.0~1.0)
    → emotional_intensity: LLM이 판단

  경로 ②: 관찰 기억 (Observation)
    NPC가 주변 사건을 목격할 때
    → text: "[남궁현]이 [소연]과 시장에서 대화하는 것을 보았다"
    → importance: 이벤트 시스템이 부여
    → emotional_intensity: NPC 성격 + 이벤트 유형으로 계산

  경로 ③: 성찰 기억 (Reflection)
    NPC가 축적된 기억을 바탕으로 자율 성찰할 때
    → 기존 기억 Top-K를 LLM에 넣고 "이 경험들에서 무엇을 깨달았는가?" 질문
    → text: LLM이 생성한 성찰 내용
    → importance: 보통 높음 (0.7~1.0)
```

### 5.2 저장 시 임베딩

기억 텍스트를 임베딩하여 벡터와 함께 LanceDB에 저장한다.

```rust
// 의사 코드
async fn store_memory(
    lance: &LanceDbRepository,
    embedding: &LlamaCppEmbedding,
    memory: NpcMemory,
) -> Result<(), Error> {
    // ① 텍스트 임베딩 (~36ms)
    let vector = embedding.embed(&memory.text)?;

    // ② LanceDB에 저장
    lance.insert("memories", MemoryRecord {
        id: memory.id,
        npc_id: memory.npc_id,
        text: memory.text,
        vector,               // 1024차원
        memory_type: memory.memory_type,
        importance: memory.importance,
        emotional_intensity: memory.emotional_intensity,
        participants: memory.participants.join(","),
        location: memory.location,
        game_time: memory.game_time,
        created_at: now_iso8601(),
        access_count: 0,
        last_accessed_at: now_iso8601(),
    }).await?;

    Ok(())
}
```

### 5.3 망각 메커니즘

인간의 기억처럼 NPC 기억도 시간이 지나면 희미해진다. 물리적 삭제가 아닌 논리적 망각으로 구현한다.

```
  망각 판정 (게임 내 1일 단위 실행):

  for each memory:
    adjusted_decay = decay_rate / (1 + 0.1 × access_count)
    retention = exp(-adjusted_decay × days_elapsed)

    if retention < 0.01:
      memory.status = "forgotten"  (검색 제외, 삭제는 안 함)
    elif retention < 0.10:
      memory.status = "fading"     (검색 시 가중치 추가 감소)
    else:
      memory.status = "active"     (정상 검색)
```

중요도(importance)가 높은 기억은 decay_rate를 추가로 낮춘다. 트라우마급 사건(importance=1.0)은 거의 잊히지 않고, 일상 관찰(importance=0.2)은 한 달 안에 희미해진다.

```
  effective_decay = base_decay × (1.2 - importance)

  importance=1.0: effective_decay = 0.05 × 0.2 = 0.01  → 느린 감쇠
  importance=0.5: effective_decay = 0.05 × 0.7 = 0.035 → 보통 감쇠
  importance=0.2: effective_decay = 0.05 × 1.0 = 0.05  → 빠른 감쇠
```


---

## 6. 구현 계획

### 6.1 단계별 구현

Iterative 방식으로 작동하는 작은 부분부터 구현하고 검증한다.

```
  Step 3.2.1: LanceDB 연결 + 스키마 생성
    → LanceDbRepository 구조체
    → memories 테이블 생성
    → 단위 테스트: 테이블 존재 확인
    → 예상 작업량: 2~3시간

  Step 3.2.2: 기억 저장 (Write)
    → store_memory() 구현
    → 임베딩 + LanceDB insert
    → 단위 테스트: 저장 후 count 확인
    → 예상 작업량: 2~3시간

  Step 3.2.3: 기억 검색 (Read — 순수 벡터)
    → search_similar() 구현
    → Top-K 벡터 검색
    → 단위 테스트: 유사 쿼리 → 관련 기억 반환 확인
    → 예상 작업량: 2~3시간

  Step 3.2.4: 메타데이터 필터 추가
    → npc_id, game_time, participants 필터
    → 복합 검색 테스트
    → 예상 작업량: 1~2시간

  Step 3.2.5: retrieval_score 리랭킹
    → 벡터 유사도 + 시간감쇠 + 중요도 + 감정 + 빈도
    → 기존 Sprint 2 retrieval_score와 통합
    → 단위 테스트: 리랭킹 순서 검증
    → 예상 작업량: 2~3시간

  Step 3.2.6: 망각 메커니즘
    → 일일 배치 처리
    → status 필드 업데이트
    → 단위 테스트: 시간 경과에 따른 상태 변화
    → 예상 작업량: 1~2시간

  Step 3.2.7: ChatSession 통합
    → 기존 InMemoryRepository 대체
    → 통합 테스트: 대화 → 저장 → 검색 → LLM 프롬프트
    → 예상 작업량: 3~4시간
```

### 6.2 파일 구조

```
  wuxia-memory/src/
    embedding/
      llamacpp_adapter.rs   ← 기존 (변경 없음)
      mod.rs
    lancedb/                ← 새로 생성
      mod.rs                  LanceDbRepository 구조체
      schema.rs               테이블 스키마 정의
      query.rs                검색 + 리랭킹 로직
    in_memory.rs            ← 기존 (폴백용 유지)
    lib.rs
```

### 6.3 의존성

```toml
# wuxia-memory/Cargo.toml 에 추가
[dependencies]
lancedb = { version = "0.x", optional = true }  # 정확한 버전은 구현 시 확인
arrow = { version = "53", optional = true }

[features]
lancedb-store = ["lancedb", "arrow"]
```

lancedb feature는 선택적으로 활성화하여, InMemoryRepository를 사용하는 기존 테스트에 영향을 주지 않는다.

---

## 7. 리스크와 대응

### 7.1 Q4 양자화 실전 품질

벤치마크는 56개 고정 문장으로 측정한 것이다. 실제 플레이어의 자유 입력은 어휘와 문체가 다양하므로 Q4의 양자화 손실이 더 체감될 수 있다. 대응 방안으로, Step 3.2.3 완료 후 자유 입력 문장 20~30개로 수동 품질 테스트를 수행한다. 문제가 발견되면 B(q8_0)로 교체한다. 교체 비용은 모델 파일 경로 한 줄 + 재인덱싱이므로 낮다.

### 7.2 LanceDB Rust 생태계 성숙도

LanceDB의 Rust SDK는 Python 대비 덜 성숙할 수 있다. API가 불안정하거나 문서가 부족할 경우, 구현 시 Context7에서 최신 문서를 확인하고, 필요하면 Python SDK로 프로토타이핑 후 Rust로 포팅하는 전략을 취한다.

### 7.3 동시 접근

Bevy ECS에서 여러 NPC 시스템이 동시에 임베딩/검색을 요청할 수 있다. LlamaCppEmbedding은 Mutex로 직렬화되어 있으므로 안전하지만, 병목이 될 수 있다. NPC 수가 11명이고 매 턴 동시에 기억을 검색할 확률은 낮으므로 초기에는 문제 없으나, 향후 확장 시 임베딩 요청을 큐잉하는 구조를 고려한다.

---

## 8. 향후 확장: 언어별 모델 분리

### 8.1 배경

벤치마크에서 KO와 EN의 최적 모델이 다르게 나타났다. C(bge-m3-Q4)는 KO에 최적화하여 선정한 모델이며, EN에서는 G(gemma-qat-q4)가 β=0.299, β/α=5.2x, 속도 13ms, 크기 ~200MB로 더 우수하다. 현재는 KO 단일 언어 출시를 우선하므로 C 단일 모델로 운용하지만, EN 로컬라이제이션 시 언어별 모델 분리를 고려할 수 있다.

### 8.2 EN 후보

| 모델 | EN β | EN α | β/α | 속도 | 차원 | 크기 |
|------|:----:|:----:|:---:|:----:|:----:|:----:|
| G: gemma-qat-q4 | 0.299 | 0.058 ⚠️ | 5.2x | 13ms | 768 | ~200MB |

C(bge-m3-Q4)의 EN 성능(β=0.276, β/α=4.7x, 36ms)과 비교하면 G는 β가 동등 이상이면서 속도가 3배 빠르고 크기가 절반이다. 단, 벡터 차원이 768로 C(1024)와 다르므로 LanceDB 테이블을 공유할 수 없다.

### 8.3 인터페이스 설계

현재 `EmbeddingPort` 트레이트가 이미 존재하므로, 나중에 언어별 라우팅 레이어만 추가하면 된다. 핵심은 현재 코드를 변경하지 않고, 새 구조체로 감싸는 방식이다.

```rust
// 현재 (변경 없음)
trait EmbeddingPort {
    fn embed(&self, text: &str) -> Result<Vec<f32>, String>;
    fn dimension(&self) -> usize;
}

// 나중에 EN 추가 시
struct MultiLangEmbedding {
    ko: Box<dyn EmbeddingPort>,  // bge-m3-Q4 (1024d)
    en: Box<dyn EmbeddingPort>,  // gemma-qat-q4 (768d)
}
```

LanceDB 측은 언어별 벡터 컬럼을 추가하거나 테이블을 분리하는 두 가지 방법이 있으며, EN 구현 시점에 결정한다.

### 8.4 현재 단계의 행동 항목

지금 할 것은 없다. `EmbeddingPort` 트레이트가 이미 추상화 역할을 하고 있으므로, C 단일 모델로 구현을 진행한다. EN 모델 교체가 필요한 시점에 이 섹션을 참조하여 설계한다.

---

## 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|------|---------|----------|
| v1.0.0 | 2026-02-21 04:10 | 최초 작성. bge-m3-Q4_K_M(C) 기준 저장/조회 전략 수립 |
| v1.1.0 | 2026-02-21 04:30 | 섹션 1.2 추가: β/α 비율과 Top-K 정확도의 관계 설명. 섹션 4.1에 β 간격이 검색 정확도를 보장하는 메커니즘 설명 추가 |
| v1.2.0 | 2026-02-21 05:10 | 섹션 8 추가: 향후 EN 모델 분리 전략. EN 후보로 G(gemma-qat-q4) 기록. EmbeddingPort 트레이트 기반 인터페이스 분리 방안 |
