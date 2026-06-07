# Phase 2.4.3 KICKOFF — Claude Code 핸드오프

> 정본 spec: `task-rel-phase2.4.3-relationship-modifiers.md` 🟢 FROZEN (2026-06-07)
> 본 문서 = Claude Code 착수 지시. 상세 명세는 spec 참조.

---

Phase 2.4.3 — RelationshipModifiers 통합 재설계를 구현해줘.

FROZEN spec (정본, 먼저 정독):
  docs/tasks/mind-architecture/task-rel-phase2.4.3-relationship-modifiers.md

착수 순서:
1. baseline 재확인 — `cargo test --lib` 실행, 554P/0F 확인 후 시작
2. §4.1 struct 재설계 (situation.rs) — 4필드 → magnitude/tilt_warm/tilt_cold 3필드
3. §4.2 modifiers() 재작성 (mod.rs)
4. §4.3 소비처 — action.rs(pw 부호로 warm/cold 선택), event.rs 공감=tilt_warm·적대=tilt_cold (add_valence 시그니처 불변)
5. §4.4 tuning.rs — const 6 신규 / 3 폐기 + AppraisalWeights trait·impl 동반 정리
6. §6 게이트 전부:
   - cargo test --lib 회귀 0
   - narrative S1~S4 재측정 → 박제값 갱신
   - gentleness 합산 통합테스트 신규 (온화 NPC × 친밀 상대 Reproach 과억제 확인)
   - grep 5종 (intensity_multiplier/empathy_modifier/hostility_modifier → 0건 등)

제약:
- (B) mapping.rs는 비범위 — 건드리지 마 (B-D3)
- PAD 벤치 기대값 변경 금지 (편차 시 보고)
- tilt 초기 weight 0.003/0.002/0.003 출발, narrative 결과로만 미세조정
- 의도한 파일만 stage 후 직접 커밋. push는 하지 마

완료 후 변경 요약 + cargo test 결과 + narrative 신/구 박제값 대비표 보고.
