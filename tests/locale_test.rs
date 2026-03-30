//! 언어 설정 및 플러거블 포맷터 테스트
//!
//! FormattedMindService, LocaleBundle 머지, 빌트인 로케일 레지스트리,
//! GuideFormatter 커스텀 구현 등을 검증합니다.

mod common;

use npc_mind::application::dto::*;
use npc_mind::application::mind_service::MindService;
use npc_mind::application::formatted_service::FormattedMindService;
use npc_mind::domain::guide::ActingGuide;
use npc_mind::domain::relationship::Relationship;
use npc_mind::ports::GuideFormatter;
use npc_mind::presentation::formatter::LocaleFormatter;
use npc_mind::presentation::locale::LocaleBundle;
use npc_mind::presentation::builtin_toml;

use common::*;

// ===========================================================================
// 빌트인 로케일 레지스트리
// ===========================================================================

#[test]
fn builtin_toml_한국어_조회() {
    let toml = builtin_toml("ko");
    assert!(toml.is_some());
    assert!(toml.unwrap().contains("language = \"ko\""));
}

#[test]
fn builtin_toml_영어_조회() {
    let toml = builtin_toml("en");
    assert!(toml.is_some());
    assert!(toml.unwrap().contains("language = \"en\""));
}

#[test]
fn builtin_toml_미지원_언어는_none() {
    assert!(builtin_toml("zh").is_none());
    assert!(builtin_toml("ja").is_none());
    assert!(builtin_toml("").is_none());
}

// ===========================================================================
// LocaleBundle 머지 (deep merge)
// ===========================================================================

#[test]
fn locale_merge_감정_부분_덮어쓰기() {
    let base = builtin_toml("ko").unwrap();
    let overrides = r#"
[emotion]
Anger = "살기"
Joy = "환희"
"#;

    let bundle = LocaleBundle::from_toml_with_overrides(base, overrides).unwrap();

    // 덮어쓴 키
    assert_eq!(bundle.emotion.get("Anger").unwrap(), "살기");
    assert_eq!(bundle.emotion.get("Joy").unwrap(), "환희");

    // 덮어쓰지 않은 키 → 빌트인 유지
    assert_eq!(bundle.emotion.get("Fear").unwrap(), "두려움");
    assert_eq!(bundle.emotion.get("Distress").unwrap(), "고통");
}

#[test]
fn locale_merge_템플릿_부분_덮어쓰기() {
    let base = builtin_toml("ko").unwrap();
    let overrides = r#"
[template]
section_npc = "[인물: {name}]"
"#;

    let bundle = LocaleBundle::from_toml_with_overrides(base, overrides).unwrap();

    // 덮어쓴 키
    assert_eq!(bundle.template.section_npc, "[인물: {name}]");

    // 덮어쓰지 않은 키 → 빌트인 유지
    assert_eq!(bundle.template.section_personality, "[성격]");
    assert_eq!(bundle.template.section_emotion, "[현재 감정]");
}

#[test]
fn locale_merge_강도_라벨_덮어쓰기() {
    let base = builtin_toml("ko").unwrap();
    let overrides = r#"
[intensity]
extreme = "압도적"
"#;

    let bundle = LocaleBundle::from_toml_with_overrides(base, overrides).unwrap();

    assert_eq!(bundle.intensity.extreme, "압도적");
    assert_eq!(bundle.intensity.strong, "강한"); // 빌트인 유지
}

#[test]
fn locale_merge_빈_오버라이드는_기본값_유지() {
    let base = builtin_toml("ko").unwrap();
    let overrides = "";

    let bundle = LocaleBundle::from_toml_with_overrides(base, overrides).unwrap();
    let base_bundle = LocaleBundle::from_toml(base).unwrap();

    assert_eq!(bundle.emotion.get("Anger"), base_bundle.emotion.get("Anger"));
    assert_eq!(bundle.template.section_npc, base_bundle.template.section_npc);
}

// ===========================================================================
// FormattedMindService — 빌트인 언어
// ===========================================================================

fn make_formatted_service(lang: &str) -> FormattedMindService<MockRepository> {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));
    FormattedMindService::new(repo, lang).expect("서비스 생성 실패")
}

fn appraise_req() -> AppraiseRequest {
    AppraiseRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation: SituationInput {
            description: "교룡이 백성을 도와줌".to_string(),
            event: None,
            action: Some(ActionInput {
                description: "백성을 도와줌".to_string(),
                agent_id: Some("gyo_ryong".to_string()),
                praiseworthiness: 0.7,
            }),
            object: None,
        },
    }
}

#[test]
fn formatted_service_한국어_프롬프트_생성() {
    let mut service = make_formatted_service("ko");
    let response = service.appraise(appraise_req(), || {}, || vec![]).unwrap();

    assert!(!response.prompt.is_empty());
    assert!(response.prompt.contains("[NPC:"));
    assert!(response.prompt.contains("[성격]"));
    assert!(response.prompt.contains("[현재 감정]"));
}

#[test]
fn formatted_service_영어_프롬프트_생성() {
    let mut service = make_formatted_service("en");
    let response = service.appraise(appraise_req(), || {}, || vec![]).unwrap();

    assert!(!response.prompt.is_empty());
    assert!(response.prompt.contains("[NPC:"));
    assert!(response.prompt.contains("[Personality]"));
    assert!(response.prompt.contains("[Current Emotion]"));
}

#[test]
fn formatted_service_미지원_언어_에러() {
    let repo = MockRepository::new();
    let result = FormattedMindService::new(repo, "zh");
    assert!(result.is_err());
}

#[test]
fn formatted_service_한국어와_영어_프롬프트가_다름() {
    let mut ko_service = make_formatted_service("ko");
    let mut en_service = make_formatted_service("en");

    let ko_response = ko_service.appraise(appraise_req(), || {}, || vec![]).unwrap();
    let en_response = en_service.appraise(appraise_req(), || {}, || vec![]).unwrap();

    // 동일한 감정이 생성되어야 함 (도메인 로직은 동일)
    assert_eq!(ko_response.emotions.len(), en_response.emotions.len());
    assert!((ko_response.mood - en_response.mood).abs() < 0.001);

    // 프롬프트 텍스트는 달라야 함
    assert_ne!(ko_response.prompt, en_response.prompt);
}

// ===========================================================================
// FormattedMindService — 오버라이드
// ===========================================================================

#[test]
fn formatted_service_커스텀_오버라이드_반영() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let overrides = r#"
[template]
section_npc = "[인물: {name}]"
"#;

    let mut service = FormattedMindService::with_overrides(repo, "ko", overrides).unwrap();
    let response = service.appraise(appraise_req(), || {}, || vec![]).unwrap();

    // 오버라이드된 섹션 헤더
    assert!(response.prompt.contains("[인물:"));
    // 오버라이드되지 않은 섹션 헤더는 기본값 유지
    assert!(response.prompt.contains("[성격]"));
}

#[test]
fn formatted_service_오버라이드_미지원_언어_에러() {
    let repo = MockRepository::new();
    let result = FormattedMindService::with_overrides(repo, "ja", "");
    assert!(result.is_err());
}

// ===========================================================================
// FormattedMindService — 완전 커스텀 TOML
// ===========================================================================

#[test]
fn formatted_service_커스텀_locale() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    // 빌트인 영어 TOML을 그대로 사용 (완전한 TOML 검증)
    let en_toml = builtin_toml("en").unwrap();
    let mut service = FormattedMindService::with_custom_locale(repo, en_toml).unwrap();

    let response = service.appraise(appraise_req(), || {}, || vec![]).unwrap();
    assert!(response.prompt.contains("[Personality]"));
}

#[test]
fn formatted_service_불완전_toml_에러() {
    let repo = MockRepository::new();
    let bad_toml = r#"
[meta]
language = "test"
name = "Test"
"#;
    let result = FormattedMindService::with_custom_locale(repo, bad_toml);
    assert!(result.is_err());
}

// ===========================================================================
// FormattedMindService — GuideFormatter 직접 구현
// ===========================================================================

struct TestFormatter;

impl GuideFormatter for TestFormatter {
    fn format_prompt(&self, guide: &ActingGuide) -> String {
        format!("TEST_PROMPT: {} feels {:?}", guide.npc_name, guide.emotion.dominant.as_ref().map(|e| e.emotion_type))
    }

    fn format_json(&self, guide: &ActingGuide) -> Result<String, serde_json::Error> {
        serde_json::to_string(&guide.npc_name)
    }
}

#[test]
fn formatted_service_커스텀_formatter_주입() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = FormattedMindService::with_formatter(repo, TestFormatter);
    let response = service.appraise(appraise_req(), || {}, || vec![]).unwrap();

    assert!(response.prompt.starts_with("TEST_PROMPT:"));
    assert!(response.prompt.contains("무백"));
}

// ===========================================================================
// MindService — 도메인 결과 + 나중에 포맷팅
// ===========================================================================

#[test]
fn mind_service_결과에서_나중에_포맷팅() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::new(repo);
    let result = service.appraise(appraise_req(), || {}, || vec![]).unwrap();

    // ActingGuide에 직접 접근
    assert_eq!(result.guide.npc_name, "무백");
    assert!(result.guide.emotion.dominant.is_some());

    // 한국어로 포맷팅
    let ko_toml = builtin_toml("ko").unwrap();
    let ko_formatter = LocaleFormatter::from_toml(ko_toml).unwrap();
    let ko_response = result.format(&ko_formatter);
    assert!(ko_response.prompt.contains("[성격]"));
}

#[test]
fn mind_service_stimulus_결과_포맷팅() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::new(repo);

    // 먼저 감정 생성
    service.appraise(appraise_req(), || {}, || vec![]).unwrap();

    // stimulus 적용
    let stim_req = StimulusRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation_description: Some("교룡의 겸손한 태도".to_string()),
        pleasure: 0.5,
        arousal: 0.2,
        dominance: 0.0,
    };
    let result = service.apply_stimulus(stim_req).unwrap();

    // StimulusResult 필드 확인
    assert!(!result.beat_changed);
    assert!(result.active_focus_id.is_none());

    // 포맷팅
    let en_toml = builtin_toml("en").unwrap();
    let en_formatter = LocaleFormatter::from_toml(en_toml).unwrap();
    let response = result.format(&en_formatter);
    assert!(response.prompt.contains("[Personality]"));
    assert!(!response.beat_changed);
}

#[test]
fn mind_service_guide_결과_포맷팅() {
    let mut repo = MockRepository::new();
    repo.add_npc(make_무백());
    repo.add_npc(make_교룡());
    repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

    let mut service = MindService::new(repo);
    service.appraise(appraise_req(), || {}, || vec![]).unwrap();

    let guide_req = GuideRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation_description: None,
    };
    let result = service.generate_guide(guide_req).unwrap();

    // GuideResult → GuideResponse
    let ko_formatter = LocaleFormatter::from_toml(builtin_toml("ko").unwrap()).unwrap();
    let response = result.format(&ko_formatter);
    assert!(!response.prompt.is_empty());
    assert!(!response.json.is_empty());
}

// ===========================================================================
// FormattedMindService — 전체 흐름 (appraise → stimulus → after_dialogue)
// ===========================================================================

#[test]
fn formatted_service_전체_흐름() {
    let mut service = make_formatted_service("ko");

    // 1. Appraise
    let res1 = service.appraise(appraise_req(), || {}, || vec![]).unwrap();
    assert!(!res1.emotions.is_empty());
    assert!(res1.prompt.contains("[NPC:"));

    // 2. Stimulus
    let stim_req = StimulusRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation_description: None,
        pleasure: 0.5,
        arousal: 0.2,
        dominance: 0.0,
    };
    let res2 = service.apply_stimulus(stim_req).unwrap();
    assert!(!res2.prompt.is_empty());

    // 3. Generate Guide
    let guide_req = GuideRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        situation_description: Some("테스트".to_string()),
    };
    let res3 = service.generate_guide(guide_req).unwrap();
    assert!(!res3.prompt.is_empty());
    assert!(!res3.json.is_empty());

    // 4. After Dialogue
    let after_req = AfterDialogueRequest {
        npc_id: "mu_baek".to_string(),
        partner_id: "gyo_ryong".to_string(),
        praiseworthiness: Some(0.5),
        significance: None,
    };
    let res4 = service.after_dialogue(after_req).unwrap();
    assert!(res4.after.closeness > res4.before.closeness);
}
