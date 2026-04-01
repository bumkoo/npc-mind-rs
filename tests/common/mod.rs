//! 테스트 공통 유틸리티
//!
//! 무협 4인 캐릭터 빌더 + Score 헬퍼 + 관계 팩토리 + 테스트 컨텍스트

#![allow(dead_code)]

use npc_mind::domain::personality::*;
use npc_mind::domain::relationship::Relationship;
use npc_mind::domain::emotion::{EmotionState, EmotionType, Situation, EventFocus, ActionFocus, RelationshipModifiers};
use npc_mind::InMemoryRepository;
use npc_mind::application::mind_service::MindService;

/// MockRepository는 InMemoryRepository의 별칭입니다 (기존 테스트 호환).
pub type MockRepository = InMemoryRepository;

pub fn score(v: f32) -> Score {
    Score::new(v, "").unwrap()
}

// ---------------------------------------------------------------------------
// 감정 헬퍼
// ---------------------------------------------------------------------------

pub fn find_emotion(state: &EmotionState, etype: EmotionType) -> Option<f32> {
    state.emotions().iter()
        .find(|e| e.emotion_type() == etype)
        .map(|e| e.intensity())
}

pub fn has_emotion(state: &EmotionState, etype: EmotionType) -> bool {
    find_emotion(state, etype).is_some()
}

// ---------------------------------------------------------------------------
// 시나리오 헬퍼
// ---------------------------------------------------------------------------

/// 배신 상황 (desirability: -0.6, praiseworthiness: -0.7)
pub fn 배신_상황() -> Situation {
    배신_상황_with_desc("배신")
}

pub fn 배신_상황_with_desc(description: &str) -> Situation {
    Situation::new(
        description,
        Some(EventFocus {
            description: "".into(),
            desirability_for_self: -0.6,
            desirability_for_other: None,
            prospect: None,
        }),
        Some(ActionFocus {
            description: "".into(),
            agent_id: Some("partner".into()),
            modifiers: None,
            praiseworthiness: -0.7,
        }),
        None,
    ).unwrap()
}

// ---------------------------------------------------------------------------
// 관계 / 저장소 헬퍼
// ---------------------------------------------------------------------------

/// 테스트용 중립 관계 (관계 갱신 테스트 등에서 사용)
pub fn neutral_rel() -> Relationship {
    Relationship::neutral("npc", "test")
}

/// 테스트용 중립 modifier (감정 평가 시 기본값)
pub fn neutral_mods() -> RelationshipModifiers {
    RelationshipModifiers::neutral()
}

/// 표준 테스트 컨텍스트
/// 
/// 무백, 교룡이 미리 로드되어 있고 중립 관계가 설정된 상태로 시작합니다.
pub struct TestContext {
    pub repo: InMemoryRepository,
    pub mu_baek: Npc,
    pub gyo_ryong: Npc,
}

impl TestContext {
    pub fn new() -> Self {
        let mut repo = InMemoryRepository::new();
        let mu_baek = make_무백();
        let gyo_ryong = make_교룡();

        repo.add_npc(mu_baek.clone());
        repo.add_npc(gyo_ryong.clone());
        repo.add_relationship(Relationship::neutral("mu_baek", "gyo_ryong"));

        Self { repo, mu_baek, gyo_ryong }
    }

    pub fn service(&mut self) -> MindService<&mut InMemoryRepository> {
        MindService::new(&mut self.repo)
    }
}

/// 무백 — 정의로운 검객. 의리와 절제를 중시한다.
pub fn make_무백() -> Npc {
    let s = score;
    NpcBuilder::new("mu_baek", "무백")
        .description("정의로운 검객. 의리와 절제를 중시한다.")
        .honesty_humility(|h| {
            h.sincerity = s(0.8); h.fairness = s(0.7);
            h.greed_avoidance = s(0.6); h.modesty = s(0.5);
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.6); e.anxiety = s(-0.4);
            e.dependence = s(-0.7); e.sentimentality = s(0.2);
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.6); a.gentleness = s(0.7);
            a.flexibility = s(0.2); a.patience = s(0.8);
        })
        .conscientiousness(|c| {
            c.organization = s(0.4); c.diligence = s(0.8);
            c.perfectionism = s(0.6); c.prudence = s(0.7);
        })
        .build()
}

/// 교룡 — 야심적인 여검객. 자유를 갈망하며 관습을 거부한다.
pub fn make_교룡() -> Npc {
    let s = score;
    NpcBuilder::new("gyo_ryong", "교룡")
        .description("야심적인 여검객. 자유를 갈망하며 관습을 거부한다.")
        .honesty_humility(|h| {
            h.sincerity = s(-0.4); h.fairness = s(-0.5);
            h.greed_avoidance = s(-0.6); h.modesty = s(-0.7);
        })
        .emotionality(|e| {
            e.fearfulness = s(0.8); e.anxiety = s(0.7);
            e.dependence = s(0.5); e.sentimentality = s(0.6);
        })
        .extraversion(|x| {
            x.social_self_esteem = s(0.7); x.social_boldness = s(0.8);
            x.sociability = s(0.0); x.liveliness = s(0.6);
        })
        .agreeableness(|a| {
            a.forgiveness = s(-0.6); a.gentleness = s(-0.5);
            a.flexibility = s(-0.4); a.patience = s(-0.7);
        })
        .conscientiousness(|c| {
            c.organization = s(-0.5); c.diligence = s(-0.3);
            c.perfectionism = s(-0.4); c.prudence = s(-0.6);
        })
        .openness(|o| {
            o.aesthetic_appreciation = s(0.6); o.inquisitiveness = s(0.8);
            o.creativity = s(0.7); o.unconventionality = s(0.9);
        })
        .build()
}

/// 수련 — 절제의 여검객
pub fn make_수련() -> Npc {
    let s = score;
    NpcBuilder::new("shu_lien", "수련")
        .description("절제의 여검객. 의무와 명예를 삶의 기둥으로 삼는다.")
        .honesty_humility(|h| {
            h.sincerity = s(0.8); h.fairness = s(0.9);
            h.greed_avoidance = s(0.7); h.modesty = s(0.6);
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.3); e.anxiety = s(0.2);
            e.dependence = s(-0.5); e.sentimentality = s(0.7);
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.5); a.gentleness = s(0.6);
            a.flexibility = s(0.3); a.patience = s(0.9);
        })
        .conscientiousness(|c| {
            c.organization = s(0.6); c.diligence = s(0.8);
            c.perfectionism = s(0.5); c.prudence = s(0.9);
        })
        .build()
}

/// 소호 — 자유로운 낭인
pub fn make_소호() -> Npc {
    let s = score;
    NpcBuilder::new("so_ho", "소호")
        .description("자유로운 낭인. 직감과 행동으로 세상을 살아간다.")
        .honesty_humility(|h| {
            h.sincerity = s(0.1); h.fairness = s(0.5);
            h.greed_avoidance = s(0.3); h.modesty = s(-0.3);
        })
        .emotionality(|e| {
            e.fearfulness = s(-0.7); e.anxiety = s(-0.5);
            e.dependence = s(-0.8); e.sentimentality = s(0.4);
        })
        .extraversion(|x| {
            x.social_self_esteem = s(0.6); x.social_boldness = s(0.7);
            x.sociability = s(0.5); x.liveliness = s(0.4);
        })
        .agreeableness(|a| {
            a.forgiveness = s(0.1); a.gentleness = s(-0.4);
            a.flexibility = s(0.3); a.patience = s(-0.3);
        })
        .conscientiousness(|c| {
            c.organization = s(-0.6); c.diligence = s(0.2);
            c.perfectionism = s(-0.4); c.prudence = s(-0.5);
        })
        .build()
}
