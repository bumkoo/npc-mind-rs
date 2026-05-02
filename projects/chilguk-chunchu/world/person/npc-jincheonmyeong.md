---
id: npc-jincheonmyeong
kind: historical
name: 진천명(辰天命)
aliases:
  - 대진 태조
  - 건국 황제
  - 원년의 황제
status: dead
hexaco:
  honesty_humility: 0.5          # 건국자, 무림과 맹약 준수 — 신뢰도 pending
  emotionality: 0.0              # 단편 출처만, 추정 보통
  extraversion: 0.4              # 카리스마 (건국 + 270년 약속의 출발점)
  agreeableness: 0.4              # 정파·사파 통합·맹약 자세
  conscientiousness: 0.7         # 건국 체계 + 무림-조정 맹약 + 신질서 수립
  openness: 0.5                  # 혈교 격퇴 + 신질서 + 구파일방 결성 (전통 외)
temporal:
  birth_year: 미상 (원년 기준 50대 추정)
  death_year: 30년차 즈음 추정 (원년 즉위 후 30년 통치 가정)
  age_at_game_start: ~
  notes: |
    대진 태조. 별호 건국 황제. 원년(0년차) 창세전쟁(혈교 격퇴 + 대진 건국 + 구파일방
    결성)의 핵심 인물. 무림-조정 상호존중 맹약(구두) 270년 전 출발점. character-roster
    H01 + history.md §1.1 + history-characters.md §1·§13 단편 출처만. 단독 열전 부재 —
    매핑 신뢰도 pending. 본 Phase 등록은 event-empire-founding(원년 사건) FK 활성용.
affiliation: []                   # 대진 황실(group-daejin-court) Phase 1 등록되어 있으나 historical 시점이라 명목만
birthplace: ~
current_location: ~               # 사망 (대진 황실 추정)
summary: |
  대진 태조. 원년(0년차) 창세전쟁의 핵심 — 혈교 격퇴 + 대진 건국 + 구파일방 결성 +
  무림-조정 상호존중 맹약(구두). 270년간 모든 갈등의 출발점이자 모든 NPC의 "선조의
  영광" 인용 대상. character-roster H01 + history.md §1.1 + history-characters §1·§13의
  단편 출처만으로 매핑 신뢰도 pending. 본 Phase 등록은 event-empire-founding의 외래키
  활성을 위한 최소 형태이며, HEXACO·동기·관계 정밀 매핑은 Phase 6+ 단독 .md 작성 시
  재산출.
tags:
  - wuxia
  - person
  - historical
  - founding-emperor
  - dead
  - era-founding-anchor
extras:
  signature_skill: 건국 통수권 + 무림 통합 카리스마 (개인 무공 미상)
  biography_short: 대진 태조. 원년 창세전쟁 + 무림-조정 맹약. 270년 모든 갈등의 출발점.
  game_role: 대진 황실 정당성의 시조 + 270년 약속의 화자 (NPC 대사 "선조의 영광")
  priority: "★★★"
  combat_style: 미상 — 건국 황제로서의 통수권. 직접 무공은 단편 출처에 명시 없음.
  story_role: event-empire-founding의 핵심 + 혈교 멸망 판정의 화자(이후 빗나감) + 270년 누적 인과의 출발점
  pending_groups: []                  # 대진 황실 group은 Phase 1 등록되어 있으나 historical 시점
  big_five_legacy: {}                  # 열전 없음 — 단편 출처만
  values:
    chung: 0.7                         # 충 — 새 왕조에 대한 충성, 백성에 대한 책임
    eui: 0.6                           # 의 — 무림과의 맹약 의리
    hyo: 0.5                           # 효 — 시조 역할
    bok: 0.3                           # 복 — 혈교 격퇴 후 추가 복수 자제
    yah: 0.5                           # 야 — 신질서 수립 야망 (정복 야심은 아님)
  hexaco_facets: {}                     # heritage-pending 잠정 매핑
  heritage_doc_pending: true            # 단독 열전 .md 부재 (사양 §3.3)
  hexaco_confidence: pending            # history-characters 단편만 — 신뢰도 낮음, 추정값
  secret: |                             # `## 비밀` H2 미러 (사양 §3.8)
    1. 혈교 "멸망 판정"의 진짜 인지 — 270년 전 격퇴 시 본 인물이 혈교 잔당 가능성을
       알고도 "멸망"으로 공식화했는지, 진짜 모르고 판단했는지. 80년 후·240년 후 부활의
       원인이 본 인물의 판단 미스인지 시대적 한계인지.
    2. 무림-조정 맹약(구두)의 진짜 조건 — 270년 전 약속이 구두라는 점이 후대 갈등의
       출발점. 본 인물이 의도적으로 명문화하지 않았는지, 시대적 관행이었는지.
    3. 구파일방 결성의 권력 균형 — 본 인물이 구파일방을 정파 대표로 인정하면서
       사파(천마신교 등)의 형성 가능성을 어떻게 봤는지. 130년차 사파 형성의 인과
       추적 단서.
  player_relevance: 2                   # 메인 서사 직접 등장 미실현, NPC 인용 화자
---

## 개요
대진 태조. character-roster H01 + history.md §1.1 + history-characters §1·§13 단편
출처. 원년(0년차) 창세전쟁의 핵심 인물 — 혈교 격퇴 + 대진 건국 + 구파일방 결성 +
무림-조정 상호존중 맹약(구두). 270년간 모든 갈등의 출발점이자 NPC들의 "선조의 영광"
"270년 전 약속" 인용 화자. **단독 열전 .md 부재(`heritage_doc_pending: true`) + 단편
출처만이라 매핑 신뢰도 pending(`hexaco_confidence: pending`)** — Phase 6+ 정밀 패스
시 재산출 필수.

## 배경
출생 연도 미상 (원년 즉위 시점 50대 추정 → 원년 -50 = 마이너스). 청년기에 혈교가
대륙을 장악한 상황에서 각지의 무인들과 연합 결성 → 혈교 격퇴 전쟁 주도 → 대진 건국
+ 구파일방 결성 동시 달성. 본 인물 즉위 시점이 대진의 원년(0년차). 즉위 후 무림-조정
상호존중 맹약(구두)을 핵심 정책으로 수립 — 정파(구파일방)가 강호를 자치하고 황실은
지방 통치만 책임지는 분권 체제. 통치 기간은 약 30년 추정 (외부 명시 없음). 사망 후
직계 후계가 대진 2대 황제 등극. 270년이 지난 게임 시작 시점에는 모든 NPC의 인용
화자로만 존재.

## 동기
표층은 **신질서 수립** — 혈교의 폭정에서 대륙을 구하고 무림과 조정의 분권 체제를
정착. 심층은 **270년 후 문제의 씨앗** — 본 인물이 혈교 잔당 가능성을 인지했는지 미상.
80년 후 화산파의 잔당 발견·240년 후 부활·270년 후 화산파 멸문 모두 본 인물의 "멸망
판정"에 뿌리. 두려워했을 것은 (1) 혈교 부활 (2) 정파-사파 분열 (3) 무림-조정 균형
와해. 직접 두려워하지 않았던 것은 자기 사후의 후대 — 맹약(구두)이라는 점이 후대
유연성을 의도했을 가능성.

## 비밀
1. **혈교 "멸망 판정"의 진짜 인지**: 270년 전 격퇴 시 본 인물이 혈교 잔당 가능성을
   알고도 "멸망"으로 공식화했는지 미상. 80년 후·240년 후 부활의 원인이 본 인물의
   판단 미스인지 시대적 한계인지. 메인 서사 후반 진실 추적 변수.
2. **무림-조정 맹약(구두)의 진짜 조건**: 270년 전 약속이 구두라는 점이 후대 갈등
   (특히 240년 전 병권 회수 시도)의 출발점. 본 인물이 의도적으로 명문화하지 않았
   는지·시대적 관행이었는지 미상.
3. **구파일방 결성의 권력 균형**: 본 인물이 구파일방을 정파 대표로 인정하면서 사파
   (천마신교 등) 형성 가능성을 어떻게 봤는지. 130년차 사파 형성의 인과 추적 단서.

## HEXACO 분석
character-roster H01 + history.md §1.1 + history-characters §1·§13 단편 출처에서
추론한 6 dim. **사양 §3.3 직교 플래그 적용** — 단독 열전 .md 부재(`heritage_doc_pending:
true`) + 단편 출처만이라 매핑 신뢰도 pending(`hexaco_confidence: pending`). Phase 6+
단독 .md 작성 시 재검토 필수.

- **H +0.5 (정직-겸손, 중상, 잠정)**: 건국자 + 무림과의 맹약 준수 + "선조의 영광"
  화자. modesty 양수(분권 체제 채택), greed_avoidance 양수(맹약 우선). 신뢰도 pending.
- **E 0.0 (정서성, 중립, 미상)**: 단편 출처만 — 미상. 중립 가정.
- **X +0.4 (외향성, 중상, 잠정)**: 카리스마(건국 + 270년 약속의 출발점) + 무림 통합.
  social_self_esteem 양수, social_boldness 양수.
- **A +0.4 (원만성, 중상, 잠정)**: 정파·사파 통합·맹약 자세 + 분권 체제. forgiveness
  양수, gentleness 보통, patience 양수(맹약 협상).
- **C +0.7 (성실성, 높음, 잠정)**: 건국 체계 + 무림-조정 맹약 + 신질서 수립. organization
  매우 양수, diligence 양수, prudence 양수.
- **O +0.5 (개방성, 중상, 잠정)**: 혈교 격퇴 + 신질서 + 구파일방 결성 (전통 외 새 체제).
  unconventionality 양수, inquisitiveness 보통.

매핑 신뢰도 pending — 본 인물 단독 .md 작성 시 재검토 필수.

## 관계
- **affiliation 빈** — 대진 황실(group-daejin-court)은 Phase 1 등록되어 있으나
  historical 시점이라 명목만. Phase 6+ historical group 카테고리 추가 시 affiliation
  승격 또는 group의 founding_member 필드로 이관.
- **현무진인(H02, 미등록)·원혜대사(H03, 미등록)·자양진인(H04, 미등록)·천리안(H06,
  미등록)**: 원년 동지. 구파일방 결성 동지. 본 인물의 통합 노력의 직접 협력자.
- **적마존(H05, 미등록)**: 270년 전 패퇴된 혈교 교주. 본 인물의 직접 적대자.
  "멸망 판정"의 대상.
- **태광제(H07, 미등록)**: 3대 황제. 본 인물의 후예. 240년 전 병권 회수 시도 ("맹약을
  깨려 한") 인물 — 본 인물의 분권 체제와 충돌.
- **단운(npc-danun)**: 17대 황제 (태무제). 본 인물의 270년 후 직계 후예. 본 인물의
  맹약을 결정적으로 깬 인물.
- **현재 NPC 8명(npc-01~07·11)**: 모두 본 인물의 270년 후예. "선조의 영광" 인용 시
  본 인물 참조.

## 게임에서의 역할
event-empire-founding(원년 창세전쟁)의 핵심 + 270년 모든 갈등의 출발점. 게임 시작
시점 사망 — 모든 등장은 NPC 인용·문서 단편으로만. 메인 퀘스트 단계:

1. **초반** — 대진 황실 NPC들의 "선조의 영광" 인용에서 언급.
2. **중반** — 무림-조정 맹약의 출처 추적 시 본 인물의 의도 분석. 정파-사파 분쟁
   배경에서 본 인물의 분권 체제가 단서.
3. **후반** — 혈교 부활 추적 시 본 인물의 "멸망 판정"이 시대적 한계인지 의도적 무시
   인지가 결정적 단서. 270년 누적 인과의 시조.

본 인물은 `kind=historical` mind 시스템 등록 대상 아님. 직접 대화 없음 — NPC 인용·
문서 단편으로만 정체 드러남. **Phase 6+ Memory 시스템 통합 시 historical 인물 회상
체계의 시조이자 단독 열전 .md 작성 시 hexaco_confidence: pending → precise 승급
후보**.
