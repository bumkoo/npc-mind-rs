//! Phase 1 Mind Architecture — **실제 LLM E2E 스모크**.
//!
//! 스택 전 구간(dispatcher → orchestrator → `RigChatAdapter` → llama-server →
//! reflection → `EndDialogue`)이 실제 LLM과 왕복하는지 확인한다. mock 기반 회귀가
//! 닿지 못하는 구간이라, rig 버전업·API 전환 같은 어댑터 변경을 검증하는 유일한 수단이다.
//!
//! ## 판정 품질은 여기서 보지 않는다
//!
//! 이 테스트는 NPC 대사를 매 실행 새로 생성한다. 그래서 `is_chitchat`이 어긋나도
//! 원인이 프롬프트인지 그 실행에서 생성된 대사인지 분리되지 않는다.
//! 판정 calibration은 `tests/reflection_calibration_test.rs`가 **고정 transcript**로
//! 담당하며, 그쪽은 변수가 프롬프트 하나로 좁혀져 있다.
//!
//! 배경: `docs/tasks/mind-architecture/reflection-test-restructure-handoff.md`
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
//! 3 narrative 시나리오를 실제로 대화시키고 **스택 건강성만 단언**한다
//! (`check_stack_health`): TurnSnapshot 축적 · reflection 응답 파싱.
//! `is_chitchat`·`significance`는 참고 출력일 뿐이다.
//!
//! 출력: stdout + `target/baseline/phase1-real-llm-results.json`
//! (생성된 대사의 연기 품질은 자동 검증 불가 — 이 박제를 육안으로 검토)
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
use npc_mind::domain::personality::NpcBuilder;
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
    /// 시나리오 유형 라벨 (단언 미사용).
    band: ExpectedBand,
}

/// 시나리오 유형 라벨 — 출력 가독성용이며 단언에 쓰이지 않는다.
/// 실제 기대 판정은 `reflection_calibration_test`가 갖고 있다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedBand {
    Chitchat,     // 서사적으로 잉여인 대화
    Daily,        // 일상이되 의미 있는 장면
    Shanshenmiao, // 서사가 꺾이는 결단
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
}

#[tokio::test]
#[ignore = "requires running llama-server (set NPC_MIND_CHAT_URL); use --ignored to run"]
async fn phase1_real_llm_e2e_smoke() {
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
    println!("Phase 1 Real LLM E2E 스모크 결과");
    println!("=========================================");
    println!(
        "{:<28} {:<12} {:<7} {:<8} {:<10} {:<8}",
        "시나리오", "유형", "chitchat*", "score*", "elapsed", "stack"
    );
    println!(
        "  * chitchat·score는 참고 출력이며 단언하지 않는다.\n\
             판정 calibration은 reflection_calibration_test(고정 transcript)가 담당한다."
    );
    let mut failures = Vec::new();
    for r in &results {
        let health = check_stack_health(r);
        println!(
            "{:<28} {:<12} {:<7} {:<8.3} {:<10} {}",
            r.name,
            format!("{:?}", r.band),
            r.is_chitchat,
            r.significance_score,
            format!("{:.1}s", r.end_session_elapsed.as_secs_f64()),
            if health.is_ok() { "✅" } else { "❌" }
        );
        if let Some(reasoning) = &r.reasoning {
            println!("    reasoning: {}", reasoning);
        }
        println!("    summary:   {}", r.summary);
        if let Err(e) = health {
            println!("    ❌ {e}");
            failures.push(format!("{}: {e}", r.name));
        }
    }

    // 결과를 JSON으로 박제 — 대사 품질 육안 검토용
    let json_path = "target/baseline/phase1-real-llm-results.json";
    std::fs::create_dir_all("target/baseline").ok();
    let json = serialize_results(&results, &chat_url);
    std::fs::write(json_path, &json).expect("write results JSON");
    println!("\n결과 박제: {json_path}");

    assert!(
        failures.is_empty(),
        "E2E 스택 이상 {}건:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
    println!("\n✅ 3 시나리오 모두 스택 정상 (대사 생성 + reflection 왕복).");
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

    ScenarioResult {
        name: scenario.name.to_string(),
        band: scenario.band,
        is_chitchat: reflection.is_chitchat,
        significance_score: reflection.significance_score,
        summary: reflection.summary,
        reasoning: reflection.llm_reasoning,
        turn_count: reflection.turn_count,
        end_session_elapsed: elapsed,
    }
}

/// 스택 건강성 검사 — 대사가 생성됐고 reflection이 돌아왔는가.
///
/// **판정 품질은 여기서 보지 않는다.** `is_chitchat` calibration은
/// `tests/reflection_calibration_test.rs`가 고정 transcript로 담당한다.
/// 이 테스트는 대사를 매번 새로 생성하므로, 판정이 어긋나도 원인이 프롬프트인지
/// 그 실행에서 생성된 대사인지 분리되지 않는다.
///
/// `significance_score`도 보지 않는다 — LLM 산출값이 아니라
/// `compute_significance(turns)`의 결정론적 결과이고, 이 테스트는 `with_analyzer`를
/// 붙이지 않아 매 턴 `Pad::neutral()`로 폴백하므로 네 신호가 연쇄로 죽는다
/// (pad_magnitude=0 → 자극 미적용 → peak_occ·diversity 낮음 → beat_signal=0).
/// 수식 회귀는 `phase1_bench_test`, 배선 검증은 `turn_snapshot_pipeline_test`가 맡는다.
fn check_stack_health(r: &ScenarioResult) -> Result<(), String> {
    if r.turn_count == 0 {
        return Err("TurnSnapshot이 하나도 쌓이지 않았다".into());
    }
    if r.summary.trim().is_empty() {
        return Err("reflection summary가 비었다 (LLM 응답 파싱 실패 의심)".into());
    }
    if r.summary.contains("no reflection returned") {
        return Err("end_session이 reflection을 반환하지 않았다".into());
    }
    Ok(())
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
            "      \"end_session_elapsed_secs\": {:.3}\n",
            r.end_session_elapsed.as_secs_f64()
        ));
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
