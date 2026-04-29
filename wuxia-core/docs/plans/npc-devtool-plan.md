# AI NPC 개발 도구 (Dev Tool) 계획서

> **버전**: v1.3.0 | **최종 수정**: 2026-03-04T18:00:00+09:00  
> **상태**: **Phase 2 진행 중 (핵심 기능 80% 완료)**  
> 📎 NPC 심리: [wuxia-npc-psychology-architecture.md](wuxia-npc-psychology-architecture.md)  
> 📎 관계 도메인: [wuxia-relationship-mechanic.md](wuxia-relationship-mechanic.md)  
> 📎 기억 도메인: [wuxia-memory-domain-analysis.md](wuxia-memory-domain-analysis.md)  

---

## 1. 개요 (업데이트됨)

### 1.1 목적
AI NPC 개발 및 디버깅을 위한 통합 GUI 환경 구축. 
**2026-03-04 진행 결과**: 기본적인 뼈대와 심리/관계/기억/프롬프트 핵심 편집 기능이 독립 크레이트(`npc-devtool`)로 구현됨.

### 1.2 기술 스택
- **GUI**: egui 0.31 + eframe 0.31 (Immediate Mode)
- **비동기**: tokio (런타임 제어) + crossbeam-channel (UI-LLM 통신)
- **도메인 연동**: wuxia-core (Character, Psychology, Relationship), wuxia-memory (InMemory)

---

## 3. 기능 목록 및 진행 상태

| ID | 카테고리 | 기능명 | 상태 | 비고 |
|---|---|---|---|---|
| **A. NPC 관리** | | | | |
| F01 | NPC 관리 | NPC 목록 표시 및 선택 | [x] | wuxia-core 프리셋 연동 |
| F02 | NPC 관리 | NPC 검색 | [x] | 실시간 이름 필터링 |
| F03 | NPC 관리 | 임시 NPC 생성 | [ ] | Phase 3 예정 |
| F04 | NPC 관리 | JSON 불러오기 / 내보내기 | [ ] | Phase 3 예정 |
| F05 | NPC 관리 | NPC 상태 전체 리셋 | [x] | 프리셋 복원 기능 구현 |
| F06 | NPC 관리 | 능력치 표시 및 오버라이드 | [x] | 인스펙터 내 표시 |
| **B. 대화 테스트** | | | | |
| F07 | 대화 | 메시지 입력 및 전송 | [x] | Ctrl+Enter 지원 |
| F08 | 대화 | NPC 응답 표시 | [x] | Mock 응답 (Async Bridge) |
| F09 | 대화 | 성능 메트릭 표시 | [ ] | TTFT 등 추후 추가 |
| F11 | 대화 | 대화 이력 초기화 | [x] | 🗑 리셋 버튼 |
| **C. 프롬프트 에디터** | | | | |
| F15 | 프롬프트 | 시스템 프롬프트 템플릿 편집 | [x] | multiline 에디터 |
| F16 | 프롬프트 | 프롬프트 변수 미리보기 | [x] | {name}, {hexaco} 등 |
| F19 | 프롬프트 | 조립된 최종 프롬프트 확인 | [x] | 실시간 치환 + 복사 기능 |
| **D. 심리 7층** | | | | |
| F20 | 심리 | HEXACO 6요인 표시 및 조정 | [x] | 0~100 슬라이더 |
| F21 | 심리 | 3축 가치관 + 신조 편집 | [x] | 슬라이더 + 텍스트 에디터 |
| F22 | 심리 | 5가치 슬라이더 | [x] | 충/의/효/복수/야망 |
| **E. 관계** | | | | |
| F29 | 관계 | 관계 목록 표시 | [x] | Grid 기반 테이블 |
| F30 | 관계 | 관계 수치 슬라이더 조정 | [x] | 호감/신뢰 실시간 수정 |
| F31 | 관계 | Level 자동 판정 표시 | [x] | 도메인 로직 기반 판정 |
| **F. 기억** | | | | |
| F35 | 기억 | 기억 검색 테스트 | [x] | InMemoryRepository 연동 |
| F36 | 기억 | 기억 수동 추가 | [x] | 중요도/내용 주입 |
| **G. 로그** | | | | |
| F40 | 로그 | 실시간 로그 스트림 | [x] | SYS/LLM/PSY/REL/MEM |
| F41 | 로그 | 로그 필터 | [ ] | 추후 보강 |
| F42 | 로그 | 로그 클리어 | [x] | Clear 버튼 |

---

## 4. 개발 계획 및 달성도

### 4.2 Phase 1 — 뼈대 (100% 완료)
- [x] 프로젝트 구조 생성 (`npc-devtool` 크레이트)
- [x] 4패널 레이아웃 및 한글 폰트(NotoSansKR) 적용
- [x] LlmBridge를 통한 비동기 메시징 구조 확립
- [x] 기본적인 대화 Mock 연동 및 로그 표시

### 4.3 Phase 2 — 핵심 기능 (80% 진행중)
- [x] **Iteration 2.1**: NPC 데이터 로드, 검색, 인스펙터 연동 완료
- [x] **Iteration 2.2**: 변수 치환 기반 프롬프트 에디터 완료
- [x] **Iteration 2.3**: 심리 7층(1~3층) 슬라이더 편집 기능 완료
- [x] **Iteration 2.4**: 관계 테이블 및 수치 조정 기능 완료
- [x] **Iteration 2.5**: 기억 검색 및 수동 추가 기능 완료
- [ ] **Iteration 2.6 ~ 2.11**: 컨텍스트 오버라이드, 파일 I/O, 고급 로그 필터 등 (잔여 작업)

---

## 6. 오늘(2026-03-04)의 주요 구현 사항

1.  **LlmBridge 아키텍처**: UI 스레드와 비동기 LLM 스레드를 `crossbeam-channel`로 분리하여 쾌적한 UX 보장.
2.  **도메인 객체 직접 조작**: `wuxia-core`의 복잡한 심리/관계 모델을 GUI 슬라이더로 직접 수정하고 결과를 즉시 확인할 수 있도록 맵핑.
3.  **UI/UX 최적화**: 
    - 입력창 포커스 탈취 방지 로직 (NPC 선택/전송 시에만 지능적 포커스).
    - 가변 레이아웃 적용으로 버튼 잘림 방지.
    - 전체 폰트 크기 상향을 통한 고해상도 가독성 확보.

---
*다음 작업 시 Iteration 2.6(컨텍스트 오버라이드) 또는 Phase 3(확장 기능)부터 재개 권장.*
