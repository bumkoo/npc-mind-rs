//! Phase 1 Mind Architecture — 실제 LLM calibration drift 검증 (디자이너 자동 검증용).
//!
//! Stage 4 narrative validation은 Mock LLM (ReflectionResult 직접 주입)으로 흐름만
//! 검증. 본 모듈은 *실제 LLM* (`RigChatAdapter` → llama-server)으로 호출해
//! `is_chitchat` / `summary` / `reasoning`을 캡처하고 calibration drift를 catch.
//!
//! ## 실행 방법
//!
//! ```powershell
//! # 1. llama-server 가동 (port 8081, gemma-4-E4B-it 또는 OpenAI-compatible 모델)
//! # 2. 환경변수 설정 후 실행:
//! $env:NPC_MIND_CHAT_URL = "http://127.0.0.1:8081/v1"
//! $env:__COMPAT_LAYER = "RunAsInvoker"
//! cargo test --features chat,embed,listener_perspective --test phase1_real_llm_test -- --ignored --nocapture
//! ```
//!
//! ## 검증 항목
//!
//! 3 narrative 시나리오 각각 실제 LLM 호출. **`is_chitchat`만 단언**한다:
//! - chitchat (잡담): is_chitchat=true 기대
//! - daily (가르침): is_chitchat=false 기대
//! - shanshenmiao (결단): is_chitchat=false 기대
//!
//! `significance_score`는 출력만 하고 단언하지 않는다 — LLM 산출값이 아니고,
//! 이 테스트 구성(analyzer 미부착)에서는 구조적으로 죽은 값이다. 근거는
//! `check_band` 주석 참조.
//!
//! 출력: stdout + `target/baseline/phase1-real-llm-results.json`
//!
//! **수동 검증 영역** (이 테스트가 catch 못 함): LLM `reasoning`의 *서사적 합리성*.
//! `target/baseline/phase1-real-llm-results.json`을 직접 읽고
//! `data/scenarios/phase1-validation/README.md` 체크리스트와 비교.

#![cfg(feature = "chat")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use npc_mind::adapter::memory_repository::InMemoryRepository;
use npc_mind::adapter::reflection_via_chat::ConversationBackedReflectionPort;
use npc_mind::adapter::rig_chat::RigChatAdapter;
use npc_mind::application::command::CommandDispatcher;
use npc_mind::application::dialogue_orchestrator::DialogueOrchestrator;
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::application::dto::{EventInput, SituationInput};
use npc_mind::application::reflection_service::{
    DefaultReflectionPromptBuilder, ReflectionRunner, ReflectionService,
};
use npc_mind::domain::personality::{HexacoProfile, NpcBuilder};
use npc_mind::domain::reflection::ReflectionResult;
use npc_mind::domain::relationship::Relationship;
use npc_mind::presentation::formatter::LocaleFormatter;
use npc_mind::presentation::locale::LocaleBundle;

const CHAT_URL_ENV: &str = "NPC_MIND_CHAT_URL";
const DEFAULT_CHAT_URL: &str = "http://127.0.0.1:8081/v1";

/// 테스트 시나리오 정의.
struct Scenario {
    name: &'static str,
    npc_id: &'static str,
    npc_name: &'static str,
    npc_compass: &'static str,
    partner_id: &'static str,
    partner_name: &'static str,
    /// 상황 설명 — appraise 단계에서 활성 Scene 미존재 시 필수
    situation: &'static str,
    /// 사건 desirability — 음수면 부정적 사건(잡담은 0, 결단은 음수)
    event_desirability: f32,
    /// (user_utterance, expected_npc_response_hint) — assistant 응답은 LLM이 생성.
    /// NPC 응답 hint는 prompt에 영향 없음, transcript 검토용 메타만.
    turns: Vec<&'static str>,
    /// 기대 밴드 — calibration 검증.
    band: ExpectedBand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 기대 밴드 — 판정은 `is_chitchat`만 본다 (`check_band` 주석 참조).
enum ExpectedBand {
    Chitchat,     // is_chitchat=true  기대 — 서사적으로 잉여인 대화
    Daily,        // is_chitchat=false 기대 — 일상이되 의미 있는 장면
    Shanshenmiao, // is_chitchat=false 기대 — 서사가 꺾이는 결단
}

fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "chitchat-passerby",
            npc_id: "lin_chong",
            npc_name: "임충",
            npc_compass: "협지대자 위국위민",
            partner_id: "passerby",
            partner_name: "행인",
            situation: "임충이 길에서 모르는 행인과 마주쳐 잠시 인사를 나눈다.",
            event_desirability: 0.05,
            turns: vec![
                "오늘 날씨가 좋군요.",
                "어디로 가시는 길이오?",
                "조심히 가시오.",
            ],
            band: ExpectedBand::Chitchat,
        },
        Scenario {
            name: "daily-training",
            npc_id: "shu_lien",
            npc_name: "수련",
            npc_compass: "공성명수신퇴 (功成名遂身退)",
            partner_id: "chunxueping",
            partner_name: "춘설병",
            situation: "수련이 제자 춘설병에게 검법 수련을 지도한다. 사제 간 따뜻한 가르침의 시간.",
            event_desirability: 0.5,
            // `turns`는 *상대(춘설병 = 제자)*의 발화만 담는다. NPC(수련 = 사부)의
            // 대사는 LLM이 매 턴 생성한다. 이전에는 2번이 사부 대사("잘 따라하는구나.
            // 이번엔 호흡에 집중해보아라.")로 잘못 들어가 있어 화자 역할이 뒤바뀌었고,
            // 그 결과 NPC가 제자처럼 응답해("네, 사부님 말씀대로 합니다") transcript가
            // 부정합해졌다. 그 부정합이 reflection 입력까지 오염시켰다.
            turns: vec![
                "사부님, 어제 가르쳐주신 검법을 다시 한번 보여주실 수 있나요?",
                "이렇게 하는 게 맞나요? 호흡이 자꾸 흐트러집니다.",
                "사부님, 감사합니다. 오늘 많이 배웠어요.",
            ],
            band: ExpectedBand::Daily,
        },
        Scenario {
            name: "lin-chong-shanshenmiao",
            npc_id: "lin_chong",
            npc_name: "임충",
            npc_compass: "협지대자 위국위민",
            partner_id: "lu_qian",
            partner_name: "육겸",
            situation: "임충이 폭설 속 산신묘에서 비를 피하다, 자신을 암살하려 했던 육겸 일당의 음모를 우연히 듣게 된다. 분노가 폭발해 처단을 결심한다.",
            event_desirability: -0.9,
            turns: vec![
                "폭풍이 분다. 산신묘에서 비를 피해야겠다.",
                "묘 안에 사람 목소리가 들린다... 누구지?",
                "육겸! 너희가 나를 죽이려 했단 말이냐!",
                "오늘로 끝이다. 체제도 너희도 모두 끝이다.",
            ],
            band: ExpectedBand::Shanshenmiao,
        },
    ]
}

/// 시나리오 결과 한 줄.
#[derive(Debug)]
struct ScenarioResult {
    name: String,
    band: ExpectedBand,
    is_chitchat: bool,
    significance_score: f32,
    summary: String,
    reasoning: Option<String>,
    turn_count: usize,
    end_session_elapsed: Duration,
    passes_band: bool,
}

#[tokio::test]
#[ignore = "requires running llama-server (set NPC_MIND_CHAT_URL); use --ignored to run"]
async fn phase1_real_llm_three_band_calibration() {
    let chat_url =
        std::env::var(CHAT_URL_ENV).unwrap_or_else(|_| DEFAULT_CHAT_URL.to_string());

    // Pre-flight: llama-server 가동 확인
    let health_url = chat_url.replace("/v1", "/health");
    let probe = reqwest::Client::new()
        .get(&health_url)
        .timeout(Duration::from_secs(3))
        .send()
        .await;
    if probe.is_err() {
        panic!(
            "llama-server unreachable at {health_url}. \
             Start llama-server + ensure NPC_MIND_CHAT_URL points to /v1 base URL."
        );
    }

    let mut results = Vec::new();

    for scenario in scenarios() {
        println!("\n=== 시나리오: {} ({} ↔ {}) ===", scenario.name, scenario.npc_name, scenario.partner_name);

        let result = run_scenario(&chat_url, &scenario).await;
        results.push(result);
    }

    // 결과 표 출력
    println!("\n\n=========================================");
    println!("Phase 1 Real LLM Calibration 결과");
    println!("=========================================");
    println!(
        "{:<28} {:<12} {:<7} {:<8} {:<10} {:<8}",
        "시나리오", "밴드", "chitchat", "score*", "elapsed", "pass"
    );
    println!("  * score = significance. 참고 출력일 뿐 판정에 쓰지 않는다 (check_band 주석 참조).");
    for r in &results {
        println!(
            "{:<28} {:<12} {:<7} {:<8.3} {:<10} {}",
            r.name,
            format!("{:?}", r.band),
            r.is_chitchat,
            r.significance_score,
            format!("{:.1}s", r.end_session_elapsed.as_secs_f64()),
            if r.passes_band { "✅" } else { "⚠️ DRIFT" }
        );
        if let Some(reasoning) = &r.reasoning {
            println!("    reasoning: {}", reasoning);
        }
        println!("    summary:   {}", r.summary);
    }

    // 결과를 JSON으로 박제 — README 체크리스트 비교용
    let json_path = "target/baseline/phase1-real-llm-results.json";
    std::fs::create_dir_all("target/baseline").ok();
    let json = serialize_results(&results, &chat_url);
    std::fs::write(json_path, &json).expect("write results JSON");
    println!("\n결과 박제: {json_path}");

    // 모든 밴드 통과해야 calibration OK.
    let drift_count = results.iter().filter(|r| !r.passes_band).count();
    if drift_count > 0 {
        eprintln!(
            "\n⚠️ Calibration drift {drift_count}/{}건 발견. \
             prompt 조정 또는 가중치 튜닝 검토 (spec §11.5 / docs/changes §4).",
            results.len()
        );
        // Hard fail은 안 함 — drift catch 자체가 본 테스트의 목적. 사용자가 결과 보고 결정.
    } else {
        println!("\n✅ 3 밴드 모두 calibration 통과.");
    }
}

async fn run_scenario(chat_url: &str, scenario: &Scenario) -> ScenarioResult {
    // Repository + dispatcher
    let mut repo = InMemoryRepository::new();
    repo.add_npc(
        NpcBuilder::new(scenario.npc_id, scenario.npc_name)
            .with_inner_compass(scenario.npc_compass)
            .build(),
    );
    repo.add_npc(NpcBuilder::new(scenario.partner_id, scenario.partner_name).build());
    repo.add_relationship(Relationship::neutral(scenario.npc_id, scenario.partner_id));
    let repo_arc = Arc::new(std::sync::Mutex::new(repo));
    let event_store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(repo_arc, event_store, bus).with_default_handlers();

    // 2 RigChatAdapter 인스턴스 — RigChatAdapter는 Clone 없음. 같은 llama-server를
    // 가리키는 별도 어댑터 인스턴스 (각자 별도 session HashMap, 서버는 공유).
    // 이게 spec §4.4 결정 4의 "별도 KV slot" 구조와 정합.
    let dialogue_chat = RigChatAdapter::new(chat_url, "gemma-4-E4B-it-Q4_K_M.gguf")
        .with_timeout(Duration::from_secs(120));
    let reflection_chat = Arc::new(
        RigChatAdapter::new(chat_url, "gemma-4-E4B-it-Q4_K_M.gguf")
            .with_timeout(Duration::from_secs(120)),
    );

    // ReflectionService 부착 — 실제 LLM
    let reflection_port = Arc::new(ConversationBackedReflectionPort::new(reflection_chat));
    let prompt_builder: Arc<dyn npc_mind::application::reflection_service::ReflectionPromptBuilder> =
        Arc::new(DefaultReflectionPromptBuilder);
    let reflection_service: Arc<dyn ReflectionRunner> = Arc::new(ReflectionService::new(
        reflection_port,
        prompt_builder,
    ));

    let bundle = LocaleBundle::from_toml(npc_mind::presentation::builtin_toml("ko").expect("ko toml"))
        .expect("parse ko bundle");
    let formatter = Arc::new(LocaleFormatter::new(bundle));
    let mut orchestrator = DialogueOrchestrator::new(dispatcher, dialogue_chat, formatter)
        .with_reflection(reflection_service);

    // 세션 시작 + turn 진행
    let sid = format!("real-llm-{}", scenario.name);
    let start = Instant::now();

    let situation = SituationInput {
        description: scenario.situation.to_string(),
        event: Some(EventInput {
            description: scenario.situation.to_string(),
            desirability_for_self: scenario.event_desirability,
            other: None,
            prospect: None,
        }),
        action: None,
        object: None,
    };
    orchestrator
        .start_session(&sid, scenario.npc_id, scenario.partner_id, Some(situation))
        .await
        .expect("start_session OK");

    for utterance in &scenario.turns {
        match orchestrator.turn(&sid, utterance, None, None).await {
            Ok(outcome) => {
                println!("  user: {utterance}");
                println!("  npc:  {}", outcome.npc_response.lines().next().unwrap_or(""));
            }
            Err(e) => {
                eprintln!("  turn failed: {e}");
                break;
            }
        }
    }

    // end_session — reflection 호출 + Command::EndDialogue dispatch
    let outcome = orchestrator
        .end_session(&sid, Some(0.5))
        .await
        .expect("end_session OK");
    let elapsed = start.elapsed();

    let reflection = outcome
        .after_dialogue
        .as_ref()
        .and_then(|a| a.reflection.clone())
        .unwrap_or_else(|| {
            // fallback — reflection 결과 회수 실패
            ReflectionResult {
                is_chitchat: false,
                summary: "(no reflection returned)".into(),
                significance_score: 0.0,
                declarative_events: vec![],
                partnership_event: None,
                turn_count: scenario.turns.len(),
                llm_reasoning: Some("FALLBACK: end_session returned no reflection".into()),
            }
        });

    let passes_band = check_band(&reflection, scenario.band);

    ScenarioResult {
        name: scenario.name.to_string(),
        band: scenario.band,
        is_chitchat: reflection.is_chitchat,
        significance_score: reflection.significance_score,
        summary: reflection.summary,
        reasoning: reflection.llm_reasoning,
        turn_count: reflection.turn_count,
        end_session_elapsed: elapsed,
        passes_band,
    }
}

/// LLM 판정(`is_chitchat`)만 검사한다. `significance_score`는 의도적으로 보지 않는다.
///
/// # significance를 단언하지 않는 이유
///
/// `significance_score`는 LLM 응답이 아니라 `compute_significance(turns)`가 내는
/// **결정론적 엔진 계산값**이다. 그리고 이 테스트는 `with_analyzer`를 붙이지 않고
/// `pad_hint`도 `None`으로 넘기므로, `DialogueOrchestrator`가 매 턴 `Pad::neutral()`로
/// 폴백해 모든 `TurnSnapshot.pad_after`가 동일해진다. 그 결과:
///
/// - `pad_magnitude`(가중치 0.30) = 0 — 턴 사이 delta가 없다
/// - 자극이 적용되지 않아 감정이 안 흔들리므로 `peak_occ`(0.40)·`diversity`(0.15)도 낮다
/// - Beat 전환이 트리거되지 않아 `beat_signal`(0.15) = 0
///
/// 즉 **네 신호가 연쇄로 죽어 있어 밴드 단언은 처음부터 통과할 수 없다.**
/// 수식 자체의 회귀는 `phase1_bench_test`가 손수 구성한 `TurnSnapshot`으로 검증하고
/// (0.000 / 0.461 / 0.980), 그 값과 이 테스트의 출력값은 입력이 달라 **비교 대상이 아니다.**
///
/// 실제 대화 흐름에서 PAD·Beat 신호가 살아나는지(= 배선 검증)는 별도 통합 테스트의
/// 몫이다. 상세: `docs/tasks/mind-architecture/reflection-test-restructure-handoff.md`
fn check_band(reflection: &ReflectionResult, band: ExpectedBand) -> bool {
    let expected_chitchat = match band {
        ExpectedBand::Chitchat => true,
        ExpectedBand::Daily | ExpectedBand::Shanshenmiao => false,
    };
    reflection.is_chitchat == expected_chitchat
}

fn serialize_results(results: &[ScenarioResult], chat_url: &str) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"chat_url\": \"{}\",\n", chat_url));
    s.push_str(&format!(
        "  \"timestamp\": \"{}\",\n",
        chrono_now_or_epoch()
    ));
    s.push_str("  \"scenarios\": [\n");
    for (i, r) in results.iter().enumerate() {
        s.push_str("    {\n");
        s.push_str(&format!("      \"name\": \"{}\",\n", r.name));
        s.push_str(&format!("      \"band\": \"{:?}\",\n", r.band));
        s.push_str(&format!("      \"is_chitchat\": {},\n", r.is_chitchat));
        s.push_str(&format!(
            "      \"significance_score\": {:.4},\n",
            r.significance_score
        ));
        s.push_str(&format!("      \"summary\": {},\n", json_escape(&r.summary)));
        s.push_str(&format!(
            "      \"reasoning\": {},\n",
            r.reasoning
                .as_ref()
                .map(|s| json_escape(s))
                .unwrap_or_else(|| "null".into())
        ));
        s.push_str(&format!("      \"turn_count\": {},\n", r.turn_count));
        s.push_str(&format!(
            "      \"end_session_elapsed_secs\": {:.3},\n",
            r.end_session_elapsed.as_secs_f64()
        ));
        s.push_str(&format!("      \"passes_band\": {}\n", r.passes_band));
        s.push_str("    }");
        if i + 1 < results.len() {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("  ]\n");
    s.push_str("}\n");
    s
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn chrono_now_or_epoch() -> String {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_else(|_| "0".to_string())
}
