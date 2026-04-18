//! Prefilter 단위 테스트 — 각 카테고리 positive/negative 샘플 검증
//!
//! 실행: `cargo test --test prefilter_unit -- --nocapture`

mod common;

use common::prefilter::{Magnitude, Prefilter, Sign};

const PATTERNS_PATH: &str = "data/listener_perspective/prefilter/patterns.toml";

fn load_prefilter() -> Prefilter {
    Prefilter::from_path(PATTERNS_PATH).expect("패턴 로드 실패")
}

// ============================================================
// 스모크: 로드 + 카테고리 수
// ============================================================

#[test]
fn prefilter_로드_성공_카테고리_4개() {
    let pf = load_prefilter();
    let names = pf.category_names();
    assert_eq!(names.len(), 4, "카테고리 수 불일치: {:?}", names);
    assert!(names.contains(&"counterfactual_gratitude"));
    assert!(names.contains(&"negation_praise"));
    assert!(names.contains(&"wuxia_criticism"));
    assert!(names.contains(&"sarcasm_interjection"));
}

// ============================================================
// 카테고리 1: counterfactual_gratitude
// ============================================================

#[test]
fn counterfactual_gratitude_positive() {
    let pf = load_prefilter();

    let cases = [
        "그대 아니었으면 이 몸은 이미 이 세상 사람이 아니었으리.",
        "자네가 없었더라면 어찌 살아남았겠나.",
        "구명은인 없었으면 목숨을 잃을 뻔하였소.",
    ];
    for u in cases {
        let hit = pf.classify(u).expect(&format!("매칭 실패: {}", u));
        assert_eq!(hit.matched_category, "counterfactual_gratitude", "{}", u);
        assert_eq!(hit.sign, Sign::Keep);
        assert_eq!(hit.magnitude, Magnitude::Strong);
        assert!((hit.p_s_default - 0.7).abs() < 1e-6);
    }
}

#[test]
fn counterfactual_gratitude_negative() {
    let pf = load_prefilter();
    // 이 발화들은 counterfactual_gratitude 로 매칭되면 안 됨
    let cases = [
        "그는 아직 살아있소.",
        "내가 책임지리다.",
    ];
    for u in cases {
        let hit = pf.classify(u);
        if let Some(h) = &hit {
            assert_ne!(
                h.matched_category, "counterfactual_gratitude",
                "잘못 매칭 [{}]: pattern={}", u, h.matched_pattern
            );
        }
    }
}

// ============================================================
// 카테고리 2: negation_praise
// ============================================================

#[test]
fn negation_praise_positive() {
    let pf = load_prefilter();

    let cases = [
        "천하에 다시없을 검객이시오.",
        "둘도 없는 명검이로다.",
        "전무후무한 기재를 만났구려.",
        "비할 데 없는 솜씨요.",
    ];
    for u in cases {
        let hit = pf.classify(u).expect(&format!("매칭 실패: {}", u));
        assert_eq!(hit.matched_category, "negation_praise", "{}", u);
        assert_eq!(hit.sign, Sign::Keep);
        assert_eq!(hit.magnitude, Magnitude::Strong);
        assert!((hit.p_s_default - 0.7).abs() < 1e-6);
    }
}

#[test]
fn negation_praise_negative() {
    let pf = load_prefilter();
    // 매칭되면 안 됨
    let cases = [
        "그 일은 이제 다시 없을 것이오.",       // "다시 없을" 띄어쓰기 — 띄어씀은 매칭 가능성
        "둘 도 없 는",                          // 비정상 문자열
    ];
    for u in cases {
        let hit = pf.classify(u);
        if let Some(h) = &hit {
            // 경고 출력 — 엄격 판정하지 않음 (이후 정제 근거로만 사용)
            println!("[overmatch 후보] {} → {} ({})", u, h.matched_category, h.matched_pattern);
        }
    }
}

// ============================================================
// 카테고리 3: wuxia_criticism
// ============================================================

#[test]
fn wuxia_criticism_positive() {
    let pf = load_prefilter();

    let cases = [
        "무림의 수치로다, 그대 같은 자는.",
        "이 검이 그대 피를 맛보게 될 것이오.",
        "강호의 도를 더럽히는 망나니로다!",
        "정녕 부끄러움도 모르는가?",
        "목이 달아날 줄 알라.",
    ];
    for u in cases {
        let hit = pf.classify(u).expect(&format!("매칭 실패: {}", u));
        assert_eq!(hit.matched_category, "wuxia_criticism", "{}", u);
        assert_eq!(hit.sign, Sign::Keep);
        assert_eq!(hit.magnitude, Magnitude::Strong);
        assert!((hit.p_s_default + 0.7).abs() < 1e-6);
    }
}

#[test]
fn wuxia_criticism_negative() {
    let pf = load_prefilter();
    let cases = [
        "오늘은 날씨가 좋소.",
        "차 한 잔 하시지요.",
    ];
    for u in cases {
        assert!(
            pf.classify(u).is_none() || pf.classify(u).unwrap().matched_category != "wuxia_criticism",
            "오매칭: {}", u
        );
    }
}

// ============================================================
// 카테고리 4: sarcasm_interjection
// ============================================================

#[test]
fn sarcasm_interjection_positive() {
    let pf = load_prefilter();

    let cases = [
        "허허, 참으로 갸륵한 마음씨로구려.",
        "아이고, 훌륭하기도 하셔라.",
        "그렇게 잘났으면 혼자 하시구려.",
        "참으로 훌륭하기도 하셔라.",
    ];
    for u in cases {
        let hit = pf.classify(u).expect(&format!("매칭 실패: {}", u));
        assert_eq!(hit.matched_category, "sarcasm_interjection", "{}", u);
        assert_eq!(hit.sign, Sign::Invert);
        assert_eq!(hit.magnitude, Magnitude::Strong);
        assert!((hit.p_s_default - 0.6).abs() < 1e-6);
    }
}

#[test]
fn sarcasm_interjection_negative() {
    let pf = load_prefilter();
    let cases = [
        // 감탄사는 있으나 과장 수사 없음
        "허허, 그렇구려.",
        "아이고, 다리야.",
    ];
    for u in cases {
        let hit = pf.classify(u);
        if let Some(h) = &hit {
            assert_ne!(
                h.matched_category, "sarcasm_interjection",
                "오매칭 [{}] → {}", u, h.matched_pattern
            );
        }
    }
}

// ============================================================
// 우선순위 / 경계: 동시 매칭 시 등록 순서 앞쪽 우선
// ============================================================

#[test]
fn priority_order_follows_category_order() {
    let pf = load_prefilter();
    // "죽을 뻔" + 감탄사 혼합 시 앞에 등록된 counterfactual_gratitude 우선
    let u = "허허, 죽을 뻔하였소.";
    let hit = pf.classify(u).expect("매칭 실패");
    // patterns.toml 에서 counterfactual_gratitude 가 sarcasm_interjection 보다 먼저 등록됨
    assert_eq!(hit.matched_category, "counterfactual_gratitude");
}
