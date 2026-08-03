//! Reflection 프롬프트 calibration — **고정 transcript 기반**
//!
//! `is_chitchat` 판정 품질만 잰다. NPC 대사를 생성하지 않고 손으로 쓴
//! `TurnSnapshot`을 `ReflectionRunner::reflect()`에 직접 넘긴다.
//!
//! ## 왜 대사를 생성하지 않는가
//!
//! `phase1_real_llm_test`는 매 실행마다 NPC 대사를 LLM으로 생성한 뒤 그 결과를
//! 다시 평가한다. 그래서 판정이 어긋났을 때 원인이
//!
//! - reflection 프롬프트가 부실한 것인지
//! - 그 실행에서 생성된 대사가 마침 밋밋했던 것인지
//!
//! 구분되지 않는다. transcript를 고정하면 변수가 **프롬프트 하나**로 줄어
//! 프롬프트를 고쳤을 때 효과를 신뢰할 수 있게 된다.
//!
//! 대사 생성을 포함한 전 구간 관통은 `phase1_real_llm_test`(E2E 스모크)가 담당한다.
//!
//! ## 실행
//!
//! ```powershell
//! # llama-server 가동 후
//! $env:NPC_MIND_CHAT_URL = "http://127.0.0.1:8081/v1"
//! cargo test --features chat --test reflection_calibration_test -- --ignored --nocapture
//! ```
//!
//! 배경: `docs/tasks/mind-architecture/reflection-test-restructure-handoff.md`

#![cfg(feature = "chat")]

use std::sync::Arc;
use std::time::Duration;

use npc_mind::adapter::reflection_via_chat::ConversationBackedReflectionPort;
use npc_mind::adapter::rig_chat::RigChatAdapter;
use npc_mind::application::reflection_service::{
    DefaultReflectionPromptBuilder, ReflectionPromptBuilder, ReflectionRunner, ReflectionService,
};
use npc_mind::domain::emotion::EmotionType;
use npc_mind::domain::pad::Pad;
use npc_mind::domain::personality::{Npc, NpcBuilder};
use npc_mind::domain::reflection::TurnSnapshot;

// ============================================================
// 고정 transcript
// ============================================================

struct Case {
    name: &'static str,
    npc_id: &'static str,
    npc_name: &'static str,
    npc_compass: &'static str,
    partner_id: &'static str,
    partner_name: &'static str,
    /// (user 발화, NPC 대사) — 둘 다 고정. reflection 프롬프트는 이 둘만 읽는다.
    turns: Vec<(&'static str, &'static str)>,
    /// 기대 판정. `None`이면 **어느 쪽이든 통과** — 판정이 흔들리는 것으로
    /// 실측 확인된 케이스다 (아래 `daily-training` 주석 참조).
    expect_chitchat: Option<bool>,
    /// 이 케이스가 무엇을 시험하는지 (실패 시 메시지에 실린다).
    intent: &'static str,
}

/// `TurnSnapshot`으로 변환.
///
/// `occ_emotions`/`pad_*`/`beat_changed`는 **reflection 프롬프트가 읽지 않는다**
/// (`format_transcript`가 `user_utterance`/`npc_response`만 뽑는다). 그래도 도메인
/// 타입을 온전히 채우기 위해 장면에 어울리는 값을 넣어 둔다 — 이 테스트의 판정에는
/// 영향이 없다.
fn to_snapshots(turns: &[(&str, &str)], occ: &[(EmotionType, f32)], pad: Pad) -> Vec<TurnSnapshot> {
    turns
        .iter()
        .enumerate()
        .map(|(i, (user, npc))| TurnSnapshot {
            user_utterance: (*user).to_string(),
            npc_response: (*npc).to_string(),
            occ_emotions: occ.to_vec(),
            pad_before: pad,
            pad_after: pad,
            beat_changed: false,
            turn_index: (i + 1) as u32,
        })
        .collect()
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "chitchat-passerby",
            npc_id: "lin_chong",
            npc_name: "임충",
            npc_compass: "협지대자 위국위민",
            partner_id: "passerby",
            partner_name: "행인",
            intent: "스쳐 지나가는 인사 — 관계에 흔적을 남기지 않는다 (4/4 안정)",
            turns: vec![
                (
                    "오늘 날씨가 좋군요.",
                    "그렇구려. 비 갠 뒤라 바람이 제법 맑소.",
                ),
                (
                    "어디로 가시는 길이오?",
                    "그저 발길 닿는 대로 가는 중이오. 특별히 정한 곳은 없소.",
                ),
                ("조심히 가시오.", "고맙소. 그대도 무탈하시오."),
            ],
            expect_chitchat: Some(true),
        },
        Case {
            name: "daily-training",
            npc_id: "shu_lien",
            npc_name: "수련",
            npc_compass: "공성명수신퇴 (功成名遂身退)",
            partner_id: "chunxueping",
            partner_name: "춘설병",
            // ⚠️ 판정이 흔들리는 케이스 — 어느 쪽이든 통과시킨다.
            //
            // 동일한 이 고정 transcript로 4회 실행한 결과 `is_chitchat`이 2:2로 갈렸다
            // (false / true / true / false). chitchat·shanshenmiao는 같은 조건에서
            // 4/4 안정이었으므로, 입력 변동이 아니라 **이 장면이 모델의 결정 경계에
            // 걸쳐 있다**는 뜻이다. 사건성은 없고 관계만 쌓이는 중간 층위라 프롬프트의
            // 이분법("서사적 사건 vs 지나가는 잡담")으로는 어느 쪽으로도 읽힌다.
            //
            // 현재 reflection 호출은 generation_config = None이라 서버 기본
            // temperature(창작용)로 돈다 — `ConversationBackedReflectionPort::analyze`
            // 주석의 "향후 reflection 전용 temperature 인하 가능" 참조.
            //
            // TODO 차후 결정 필요 (택일 또는 조합):
            //   (a) reflection 호출을 temperature=0으로 결정론화 — 분류 작업이므로
            //       같은 입력에 같은 답이 나와야 한다. 게임 로직상으로도 같은 대화를
            //       두 번 끝냈는데 관계 갱신 여부가 갈리는 건 곤란하다.
            //   (b) 프롬프트 개선 — 판정 기준을 게이트 의미("관계에 흔적을 남기는가")에
            //       맞추고, 관계 맥락(사제 관계·Relationship 4축)을 주입한다.
            //   (c) 중간 층위를 판정에서 배제 — daily 밴드는 significance 축에만
            //       맡기고 is_chitchat은 양극단만 판정하게 한다.
            // 상세: docs/tasks/mind-architecture/reflection-test-restructure-handoff.md
            intent: "사제 간 수련 — 중간 밴드. 판정 불안정(4회 중 2:2)으로 어느 쪽이든 통과",
            turns: vec![
                (
                    "사부님, 어제 가르쳐주신 검법을 다시 한번 보여주실 수 있나요?",
                    "네, 춘설병. 어제 본 동작이 아직 몸에 새겨지지 않았나 보구나. \
                     검을 잡는 손의 위치부터 다시 차분히 따라해 보아라.",
                ),
                (
                    "이렇게 하는 게 맞나요? 호흡이 자꾸 흐트러집니다.",
                    "손가락이 검날에 너무 매몰되지 말아라. \
                     숨은 검이 아니라 너의 마음에서 나오게 하라.",
                ),
                (
                    "사부님, 감사합니다. 오늘 많이 배웠어요.",
                    "마음이 열려 있으니 배움도 잘 들어오겠지. \
                     오늘처럼 꾸준히 따라오면 곧 검이 스스로 움직이는 법을 알게 될 것이다.",
                ),
            ],
            expect_chitchat: None, // ← 흔들림 확인됨. 위 주석 참조.
        },
        Case {
            name: "lin-chong-shanshenmiao",
            npc_id: "lin_chong",
            npc_name: "임충",
            npc_compass: "협지대자 위국위민",
            partner_id: "lu_qian",
            partner_name: "육겸",
            intent: "적대 확정 + 결단 — 서사가 꺾인다",
            turns: vec![
                (
                    "폭풍이 분다. 산신묘에서 비를 피해야겠다.",
                    "눈보라가 심상치 않구나. 이 묘에서 잠시 몸을 녹이는 수밖에.",
                ),
                (
                    "묘 안에 사람 목소리가 들린다... 누구지?",
                    "저 목소리는… 육겸이다. 벽 너머로 저들이 나를 불태워 죽이려 했다는 말이 들린다.",
                ),
                (
                    "육겸! 너희가 나를 죽이려 했단 말이냐!",
                    "이제 와 변명은 필요 없다. 내 가족을 짓밟고 나를 여기까지 몰아넣은 것이 네놈들이었구나.",
                ),
                (
                    "오늘로 끝이다. 체제도 너희도 모두 끝이다.",
                    "그래. 오늘 이 자리에서 끝을 보자. \
                     더는 참지 않겠다 — 내 손으로 네놈들을 베고 이 길을 떠나겠다.",
                ),
            ],
            expect_chitchat: Some(false),
        },
    ]
}

// ============================================================
// 실행
// ============================================================

fn npc(id: &str, name: &str, compass: Option<&str>) -> Npc {
    let b = NpcBuilder::new(id, name);
    match compass {
        Some(c) => b.with_inner_compass(c).build(),
        None => b.build(),
    }
}

async fn build_runner(chat_url: &str) -> Arc<dyn ReflectionRunner> {
    // 모델명은 서버에서 자동 감지. 실패하면 임의 이름으로 폴백
    // (llama-server는 model 필드를 무시한다).
    let adapter = match RigChatAdapter::connect(chat_url).await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("모델 자동 감지 실패({e}) — 'local-model'로 폴백");
            RigChatAdapter::new(chat_url, "local-model")
        }
    };
    let chat = Arc::new(adapter.with_timeout(Duration::from_secs(120)));
    let port = Arc::new(ConversationBackedReflectionPort::new(chat));
    let builder: Arc<dyn ReflectionPromptBuilder> = Arc::new(DefaultReflectionPromptBuilder);
    Arc::new(ReflectionService::new(port, builder))
}

#[tokio::test]
#[ignore = "requires running llama-server (set NPC_MIND_CHAT_URL); use --ignored to run"]
async fn reflection_prompt_판정_calibration() {
    let chat_url = std::env::var("NPC_MIND_CHAT_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8081/v1".to_string());
    let runner = build_runner(&chat_url).await;

    let occ = [(EmotionType::Joy, 0.3)];
    let pad = Pad::new(0.1, 0.1, 0.0);

    let mut failures = Vec::new();

    println!("\n=== Reflection 프롬프트 calibration (고정 transcript) ===");
    println!("url: {chat_url}\n");

    for case in cases() {
        let turns = to_snapshots(&case.turns, &occ, pad);
        let speaker = npc(case.npc_id, case.npc_name, Some(case.npc_compass));
        let partner = npc(case.partner_id, case.partner_name, None);

        let result = runner
            .reflect(&format!("calib-{}", case.name), &turns, &speaker, &partner)
            .await;

        // `None`이면 어느 쪽이든 통과 — 판정이 흔들리는 것으로 실측 확인된 케이스.
        let ok = case
            .expect_chitchat
            .is_none_or(|expected| result.is_chitchat == expected);
        let expect_label = match case.expect_chitchat {
            Some(b) => b.to_string(),
            None => "any".to_string(),
        };
        println!(
            "{:<26} is_chitchat={:<5} (기대 {:<5}) {}",
            case.name,
            result.is_chitchat,
            expect_label,
            if ok { "✅" } else { "❌" }
        );
        println!("    의도:      {}", case.intent);
        println!("    summary:   {}", result.summary);
        if let Some(r) = &result.llm_reasoning {
            println!("    reasoning: {r}");
        }
        println!();

        if !ok {
            failures.push(format!(
                "{} — is_chitchat={} (기대 {expect_label}). 의도: {}",
                case.name, result.is_chitchat, case.intent
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "reflection 프롬프트 판정 불일치 {}건:\n  {}\n\n\
         transcript가 고정이므로 원인은 프롬프트(또는 모델)에 한정된다. \
         대사 생성 변동성은 배제된 상태다.",
        failures.len(),
        failures.join("\n  ")
    );
}
