//! GuiState → 도메인 타입 변환 + 파이프라인 실행

use npc_mind::domain::emotion::*;
use npc_mind::domain::guide::ActingGuide;
use npc_mind::domain::pad::Pad;
use npc_mind::domain::personality::*;
use npc_mind::domain::relationship::{Relationship, RelationshipBuilder};
use npc_mind::presentation::korean::KoreanFormatter;
use npc_mind::ports::{AppraisalWeights, GuideFormatter};
use crate::trace_collector::AppraisalCollector;

use crate::state::{FocusType, GuiState, ProspectChoice};

// ---------------------------------------------------------------------------
// 도메인 변환
// ---------------------------------------------------------------------------

pub fn build_npc(s: &GuiState) -> Npc {
    let sc = Score::clamped;
    NpcBuilder::new(&s.npc_id, &s.npc_name)
        .description(&s.npc_description)
        .honesty_humility(|h| {
            h.sincerity = sc(s.sincerity);
            h.fairness = sc(s.fairness);
            h.greed_avoidance = sc(s.greed_avoidance);
            h.modesty = sc(s.modesty);
        })
        .emotionality(|e| {
            e.fearfulness = sc(s.fearfulness);
            e.anxiety = sc(s.anxiety);
            e.dependence = sc(s.dependence);
            e.sentimentality = sc(s.sentimentality);
        })
        .extraversion(|x| {
            x.social_self_esteem = sc(s.social_self_esteem);
            x.social_boldness = sc(s.social_boldness);
            x.sociability = sc(s.sociability);
            x.liveliness = sc(s.liveliness);
        })
        .agreeableness(|a| {
            a.forgiveness = sc(s.forgiveness);
            a.gentleness = sc(s.gentleness);
            a.flexibility = sc(s.flexibility);
            a.patience = sc(s.patience);
        })
        .conscientiousness(|c| {
            c.organization = sc(s.organization);
            c.diligence = sc(s.diligence);
            c.perfectionism = sc(s.perfectionism);
            c.prudence = sc(s.prudence);
        })
        .openness(|o| {
            o.aesthetic_appreciation = sc(s.aesthetic_appreciation);
            o.inquisitiveness = sc(s.inquisitiveness);
            o.creativity = sc(s.creativity);
            o.unconventionality = sc(s.unconventionality);
        })
        .build()
}

pub fn build_situation(s: &GuiState) -> Situation {
    let event = s.focuses.iter().find(|f| f.focus_type == FocusType::Event).map(|f| {
        let other = if f.has_other {
            Some(DesirabilityForOther {
                target_id: f.other_target_id.clone(),
                desirability: f.desirability_for_other,
                relationship: RelationshipBuilder::new(&s.npc_id, &f.other_target_id)
                    .closeness(Score::clamped(f.other_closeness))
                    .trust(Score::clamped(f.other_trust))
                    .power(Score::clamped(f.other_power))
                    .build(),
            })
        } else {
            None
        };
        let prospect = match f.prospect {
            ProspectChoice::None => None,
            ProspectChoice::Anticipation => Some(Prospect::Anticipation),
            ProspectChoice::HopeFulfilled => {
                Some(Prospect::Confirmation(ProspectResult::HopeFulfilled))
            }
            ProspectChoice::HopeUnfulfilled => {
                Some(Prospect::Confirmation(ProspectResult::HopeUnfulfilled))
            }
            ProspectChoice::FearUnrealized => {
                Some(Prospect::Confirmation(ProspectResult::FearUnrealized))
            }
            ProspectChoice::FearConfirmed => {
                Some(Prospect::Confirmation(ProspectResult::FearConfirmed))
            }
        };
        EventFocus {
            description: f.event_description.clone(),
            desirability_for_self: f.desirability_for_self,
            desirability_for_other: other,
            prospect,
        }
    });

    let action = s.focuses.iter().find(|f| f.focus_type == FocusType::Action).map(|f| {
        ActionFocus {
            description: f.action_description.clone(),
            agent_id: if f.is_self_agent { None } else { Some("partner".into()) },
            praiseworthiness: f.praiseworthiness,
            relationship: None, // devgui에서는 대화 상대만 (제3자 미지원)
        }
    });

    let object = s.focuses.iter().find(|f| f.focus_type == FocusType::Object).map(|f| {
        ObjectFocus {
            target_id: f.object_target_id.clone(),
            target_description: f.object_target_description.clone(),
            appealingness: f.appealingness,
        }
    });

    // devgui에서는 unwrap — GUI가 최소 1개 Focus를 보장
    Situation::new(s.situation_description.clone(), event, action, object)
        .expect("최소 1개 Focus 필요")
}

pub fn build_relationship(s: &GuiState) -> Relationship {
    RelationshipBuilder::new(&s.rel_owner_id, &s.rel_target_id)
        .closeness(Score::clamped(s.closeness))
        .trust(Score::clamped(s.trust))
        .power(Score::clamped(s.power))
        .build()
}

pub fn build_pad(s: &GuiState) -> Pad {
    Pad {
        pleasure: s.pad_pleasure,
        arousal: s.pad_arousal,
        dominance: s.pad_dominance,
    }
}

// ---------------------------------------------------------------------------
// 열별 출력 구조체
// ---------------------------------------------------------------------------

/// Column 0: 감정 평가 결과 (감정 상태 초기값)
pub struct AppraisalOutput {
    pub title: String,
    /// 중간 계산값 + 공식
    pub intermediates: String,
    /// 감정 생성 추적
    pub trace: String,
    /// (초기) 감정 상태
    pub emotion_state: String,
}

/// Column 1: 자극 적용 이력 엔트리
pub enum Col1Entry {
    /// 임베딩 → PAD 변환 결과
    PadEval { content: String },
    /// 자극 적용 (감정 변동 + 감정 상태)
    Stimulus { delta: String, emotion_state: String },
}

/// Column 2: 가이드 출력
pub struct GuideOutput {
    /// 연기 지시
    pub directive: String,
    /// 프롬프트
    pub prompt: String,
}

// ---------------------------------------------------------------------------
// 파이프라인 실행
// ---------------------------------------------------------------------------

pub fn run_appraise(s: &GuiState, collector: &AppraisalCollector) -> (EmotionState, AppraisalOutput) {
    let npc = build_npc(s);
    let situation = build_situation(s);
    let relationship = build_relationship(s);

    // collector 비우고 appraise 실행 → trace 이벤트 수집
    collector.take_entries();
    let state = AppraisalEngine::appraise(npc.personality(), &situation, &relationship);
    let trace_entries = collector.take_entries();
    let trace = trace_entries.join("\n");
    let intermediates = format_weights(npc.personality(), &relationship);
    let emotion_state = format_emotion_state(&state);

    let output = AppraisalOutput {
        title: format!("[감정 평가] {} — {}", npc.name(), s.situation_description),
        intermediates,
        trace,
        emotion_state,
    };

    (state, output)
}

/// 현재 감정 상태 기준으로 가이드 생성 (감정 평가 재실행하지 않음)
pub fn run_guide(
    s: &GuiState,
    current_state: &EmotionState,
) -> GuideOutput {
    let npc = build_npc(s);
    let relationship = build_relationship(s);

    let guide = ActingGuide::build(
        &npc,
        current_state,
        Some(s.situation_description.clone()),
        Some(&relationship),
    );

    let formatter = KoreanFormatter::new();
    let prompt = formatter.format_prompt(&guide);

    GuideOutput {
        directive: format!(
            "어조: {:?}\n태도: {:?}\n행동: {:?}\n금지: {:?}",
            guide.directive.tone,
            guide.directive.attitude,
            guide.directive.behavioral_tendency,
            guide.directive.restrictions,
        ),
        prompt,
    }
}

/// 자극 적용 → (새 EmotionState, 감정 변동 텍스트, 감정 상태 텍스트)
pub fn run_stimulus(
    s: &GuiState,
    current_state: &EmotionState,
) -> (EmotionState, String, String) {
    let npc = build_npc(s);
    let pad = build_pad(s);

    let new_state = StimulusEngine::apply_stimulus(npc.personality(), current_state, &pad);

    // 변동 비교
    let mut deltas = String::new();
    for emotion in new_state.emotions() {
        let old_intensity = current_state
            .emotions()
            .iter()
            .find(|e| e.emotion_type() == emotion.emotion_type())
            .map(|e| e.intensity())
            .unwrap_or(0.0);
        let new_intensity = emotion.intensity();
        if (new_intensity - old_intensity).abs() > 0.001 {
            let arrow = if new_intensity > old_intensity {
                "+"
            } else {
                ""
            };
            deltas.push_str(&format!(
                "{:?}: {:.3} -> {:.3} ({}{:.3})\n",
                emotion.emotion_type(),
                old_intensity,
                new_intensity,
                arrow,
                new_intensity - old_intensity,
            ));
        }
    }
    if deltas.is_empty() {
        deltas = "변동 없음".into();
    }

    let emotion_state = format_emotion_state(&new_state);

    (new_state, deltas, emotion_state)
}

/// 대화 종료 → (새 Relationship, 관계 갱신 텍스트)
pub fn run_after_dialogue(
    s: &GuiState,
    current_state: &EmotionState,
) -> (Relationship, String) {
    let relationship = build_relationship(s);
    let situation = build_situation(s);

    let pw = situation.action.as_ref().map(|a| a.praiseworthiness);
    let new_rel = relationship.after_dialogue(current_state, pw);

    let text = format!(
        "친밀도: {:.3} -> {:.3}\n신뢰도: {:.3} -> {:.3}\n상하:   {:.3} -> {:.3}",
        relationship.closeness().value(),
        new_rel.closeness().value(),
        relationship.trust().value(),
        new_rel.trust().value(),
        relationship.power().value(),
        new_rel.power().value(),
    );

    (new_rel, text)
}

// ---------------------------------------------------------------------------
// 헬퍼
// ---------------------------------------------------------------------------

/// 감정 상태를 포맷팅된 문자열로 변환
pub fn format_emotion_state(state: &EmotionState) -> String {
    let mut emotions_text = String::new();
    let mut emotions: Vec<_> = state
        .emotions()
        .into_iter()
        .filter(|e| e.intensity() > 0.001)
        .collect();
    emotions.sort_by(|a, b| b.intensity().partial_cmp(&a.intensity()).unwrap());

    for e in &emotions {
        let bar_len = (e.intensity() * 20.0) as usize;
        let bar: String = "#".repeat(bar_len);
        let space: String = " ".repeat(20 - bar_len);
        emotions_text.push_str(&format!(
            "{:<20} [{}{}] {:.3}\n",
            format!("{:?}", e.emotion_type()),
            bar,
            space,
            e.intensity(),
        ));
    }
    if emotions_text.is_empty() {
        emotions_text = "감정 없음\n".into();
    }

    let dominant = state.dominant();
    let valence = state.overall_valence();

    emotions_text.push_str(&format!(
        "\n지배 감정: {}\n전반적 기분: {:.3}",
        dominant
            .as_ref()
            .map(|e| format!("{:?} ({:.3})", e.emotion_type(), e.intensity()))
            .unwrap_or_else(|| "없음".into()),
        valence,
    ));

    emotions_text
}

/// 성격 weight + 관계 modifier 요약 (AppraisalWeights 포트를 통해 조회)
fn format_weights(
    p: &HexacoProfile,
    rel: &Relationship,
) -> String {
    let avg = p.dimension_averages();
    let mut text = String::new();

    text.push_str("── HEXACO 차원 평균 ──\n");
    text.push_str(&format!(
        "  H={:.2}  E={:.2}  X={:.2}  A={:.2}  C={:.2}  O={:.2}\n\n",
        avg.h.value(), avg.e.value(), avg.x.value(),
        avg.a.value(), avg.c.value(), avg.o.value()
    ));

    text.push_str("── AppraisalWeights (성격 가중치) ──\n");
    text.push_str(&format!(
        "  desirability_self_weight(+0.8) = {:.3}\n",
        p.desirability_self_weight(0.8)));
    text.push_str(&format!(
        "  desirability_self_weight(-0.8) = {:.3}\n",
        p.desirability_self_weight(-0.8)));
    text.push_str(&format!(
        "  desirability_prospect_weight(+0.8) = {:.3}\n",
        p.desirability_prospect_weight(0.8)));
    text.push_str(&format!(
        "  desirability_prospect_weight(-0.8) = {:.3}\n",
        p.desirability_prospect_weight(-0.8)));
    text.push_str(&format!(
        "  desirability_confirmation_weight = {:.3}\n",
        p.desirability_confirmation_weight(0.8)));
    text.push_str(&format!(
        "  empathy_weight(+0.8) = {:.3}\n",
        p.empathy_weight(0.8)));
    text.push_str(&format!(
        "  empathy_weight(-0.8) = {:.3}\n",
        p.empathy_weight(-0.8)));
    text.push_str(&format!(
        "  hostility_weight(+0.8) = {:.3}\n",
        p.hostility_weight(0.8)));
    text.push_str(&format!(
        "  hostility_weight(-0.8) = {:.3}\n",
        p.hostility_weight(-0.8)));
    text.push_str(&format!(
        "  praiseworthiness_weight(self, +0.8) = {:.3}\n",
        p.praiseworthiness_weight(true, 0.8)));
    text.push_str(&format!(
        "  praiseworthiness_weight(self, -0.8) = {:.3}\n",
        p.praiseworthiness_weight(true, -0.8)));
    text.push_str(&format!(
        "  praiseworthiness_weight(other, +0.8) = {:.3}\n",
        p.praiseworthiness_weight(false, 0.8)));
    text.push_str(&format!(
        "  praiseworthiness_weight(other, -0.8) = {:.3}\n",
        p.praiseworthiness_weight(false, -0.8)));
    text.push_str(&format!(
        "  appealingness_weight = {:.3}\n\n",
        p.appealingness_weight(0.8)));
    text.push_str("── 관계 계수 ──\n");
    text.push_str(&format!(
        "  emotion_intensity_multiplier = {:.3}\n",
        rel.emotion_intensity_multiplier()));
    text.push_str(&format!(
        "  trust_emotion_modifier = {:.3}\n",
        rel.trust_emotion_modifier()));
    text.push_str(&format!(
        "  empathy_rel_modifier = {:.3}\n",
        rel.empathy_rel_modifier()));
    text.push_str(&format!(
        "  hostility_rel_modifier = {:.3}\n",
        rel.hostility_rel_modifier()));

    text
}
