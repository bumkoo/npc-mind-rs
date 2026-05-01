---
id: npc-NN                                  # 또는 H01 (역사 인물) · player (플레이어) · L01 (전설)
kind: active                                 # active | historical | legendary | player
name: 인물명(漢字)                            # 한자 병기 권장
aliases: []                                  # 별호·자(字)·호(號)·옛 이름
status: alive                                # alive | dead | missing | unknown
hexaco:                                      # 6 dim 일급 — 범위 -1.0 ~ +1.0 (Score VO 검증)
  honesty_humility: 0.0                      # 정직-겸손 (낮을수록 권모술수)
  emotionality: 0.0                          # 정서성 (높을수록 두려움·의존성)
  extraversion: 0.0                          # 외향성 (높을수록 사교적·활기)
  agreeableness: 0.0                         # 원만성 (낮을수록 비협조·복수심)
  conscientiousness: 0.0                     # 성실성 (높을수록 체계적·근면)
  openness: 0.0                              # 개방성 (높을수록 새로운 수단 수용)
temporal:
  birth_year: ~                              # "215년차 즈음" 자유 텍스트
  death_year: ~                              # 생존 시 ~ (null)
  age_at_game_start: ~                       # 게임 시작 시점 나이
  notes: ~
affiliation: []                              # Group ID 배열 (Phase 1 group과의 외래키)
birthplace: ~                                # Place ID 텍스트 (Phase 3 외래키 활성)
current_location: ~
summary: |
  1-3 문장 핵심 묘사. 게임 내 역할 + 결정적 특징.
tags: [wuxia, person]
extras:
  signature_skill: ~                         # 무공 / 대표 기술
  biography_short: ~                         # 한 줄 약력
  game_role: ~                               # 메인 적대자 · 거울형 동반자 등
  priority: ~                                # ★ ~ ★★★★★
  hexaco_facets:                             # 24 facet 정형 (선택, 빈칸이면 6 dim에서 spread)
    # H_sincerity, H_fairness, ... 등 24개 키 (HEXACO 학술 표기 그대로)
---

## 개요
산문 1-2 단락 — 인물의 핵심 인상·게임 첫 등장 인상.

## 배경
산문 — 출신·성장 환경·결정적 사건들.

## 동기
산문 — 무엇을 원하는가, 무엇을 두려워하는가.

## 비밀
산문 — 다른 사람은 모르는 것 (선택; active kind 권장).

## HEXACO 분석
산문 — 6 dim 결정 근거. 24 facet 보충 가능.

## 관계
- group-X 안에서: 역할
- npc-X 인물: 관계 유형

## 게임에서의 역할
산문 — 메인 퀘스트·서사 역할·플레이어 첫 조우 시점.
