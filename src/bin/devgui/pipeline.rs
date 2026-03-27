//! GuiState → 도메인 타입 변환 + 파이프라인 실행

use npc_mind::domain::emotion::*;
use npc_mind::domain::guide::ActingGuide;
use npc_mind::domain::pad::Pad;
use npc_mind::domain::personality::*;
use npc_mind::domain::relationship::{Relationship, RelationshipBuilder};
use npc_mind::presentation::korean::KoreanFormatter;
use npc_mind::ports::GuideFormatter;

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
    let focuses: Vec<SituationFocus> = s
        .focuses
        .iter()
        .map(|f| match f.focus_type {
            FocusType::Event => {
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
                SituationFocus::Event(EventFocus {
                    desirability_for_self: f.desirability_for_self,
                    desirability_for_other: other,
                    prospect,
                })
            }
            FocusType::Action => SituationFocus::Action(ActionFocus {
                is_self_agent: f.is_self_agent,
                praiseworthiness: f.praiseworthiness,
            }),
            FocusType::Object => SituationFocus::Object(ObjectFocus {
                appealingness: f.appealingness,
            }),
        })
        .collect();

    Situation {
        description: s.situation_description.clone(),
        focuses,
    }
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
// 파이프라인 실행 결과
// ---------------------------------------------------------------------------

/// 결과 종류: Base는 전체 3열, Incremental은 기존 Base 위에 누적
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResultKind {
    /// 감정 평가, 가이드 생성 — 전체 3열을 채움
    Base,
    /// PAD 평가, 자극 적용, 대화 종료 — 감정 추적 열에 추가
    Incremental,
}

pub struct PipelineResult {
    pub title: String,
    pub sections: Vec<ResultSection>,
    pub kind: ResultKind,
}

pub struct ResultSection {
    pub heading: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// 파이프라인 실행
// ---------------------------------------------------------------------------

pub fn run_appraise(s: &GuiState) -> (EmotionState, PipelineResult) {
    let npc = build_npc(s);
    let situation = build_situation(s);
    let relationship = build_relationship(s);

    let state = AppraisalEngine::appraise(npc.personality(), &situation, &relationship);
    let trace = trace_appraisal(npc.personality(), &situation, &relationship);

    let mut sections = Vec::new();
    sections.push(intermediates_section(npc.personality(), &relationship, s));
    sections.push(ResultSection {
        heading: "감정 생성 추적".into(),
        content: trace,
    });
    sections.extend(emotion_state_sections(&state));

    let result = PipelineResult {
        title: format!("[감정 평가] {} — {}", npc.name(), s.situation_description),
        sections,
        kind: ResultKind::Base,
    };

    (state, result)
}

pub fn run_guide(s: &GuiState) -> (EmotionState, PipelineResult) {
    let npc = build_npc(s);
    let situation = build_situation(s);
    let relationship = build_relationship(s);

    let state = AppraisalEngine::appraise(npc.personality(), &situation, &relationship);
    let guide = ActingGuide::build(
        &npc,
        &state,
        Some(s.situation_description.clone()),
        Some(&relationship),
    );

    let formatter = KoreanFormatter::new();
    let prompt = formatter.format_prompt(&guide);

    let trace = trace_appraisal(npc.personality(), &situation, &relationship);

    let mut sections = Vec::new();
    sections.push(intermediates_section(npc.personality(), &relationship, s));
    sections.push(ResultSection {
        heading: "감정 생성 추적".into(),
        content: trace,
    });
    sections.extend(emotion_state_sections(&state));
    sections.push(ResultSection {
        heading: "연기 지시".into(),
        content: format!(
            "어조: {:?}\n태도: {:?}\n행동: {:?}\n금지: {:?}",
            guide.directive.tone,
            guide.directive.attitude,
            guide.directive.behavioral_tendency,
            guide.directive.restrictions,
        ),
    });
    sections.push(ResultSection {
        heading: "프롬프트".into(),
        content: prompt,
    });

    let result = PipelineResult {
        title: format!("[가이드 생성] {} — {}", npc.name(), s.situation_description),
        sections,
        kind: ResultKind::Base,
    };

    (state, result)
}

pub fn run_stimulus(
    s: &GuiState,
    current_state: &EmotionState,
) -> (EmotionState, PipelineResult) {
    let npc = build_npc(s);
    let pad = build_pad(s);

    let new_state = StimulusEngine::apply_stimulus(npc.personality(), current_state, &pad);

    let mut sections = Vec::new();

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
    sections.push(ResultSection {
        heading: "감정 변동".into(),
        content: deltas,
    });
    sections.extend(emotion_state_sections(&new_state));

    let title = if s.utterance_text.is_empty() {
        format!("[자극 적용] P={:.1} A={:.1} D={:.1}", pad.pleasure, pad.arousal, pad.dominance)
    } else {
        let preview: String = s.utterance_text.chars().take(20).collect();
        format!("[자극 적용] \"{}..\" P={:.1} A={:.1} D={:.1}", preview, pad.pleasure, pad.arousal, pad.dominance)
    };
    let result = PipelineResult {
        title,
        sections,
        kind: ResultKind::Incremental,
    };

    (new_state, result)
}

pub fn run_after_dialogue(
    s: &GuiState,
    current_state: &EmotionState,
) -> (Relationship, PipelineResult) {
    let relationship = build_relationship(s);
    let situation = build_situation(s);

    let new_rel = relationship.after_dialogue(current_state, &situation);

    let sections = vec![ResultSection {
        heading: "관계 갱신".into(),
        content: format!(
            "친밀도: {:.3} -> {:.3}\n신뢰도: {:.3} -> {:.3}\n상하:   {:.3} -> {:.3}",
            relationship.closeness().value(),
            new_rel.closeness().value(),
            relationship.trust().value(),
            new_rel.trust().value(),
            relationship.power().value(),
            new_rel.power().value(),
        ),
    }];

    let result = PipelineResult {
        title: "[대화 종료] 관계 갱신".into(),
        sections,
        kind: ResultKind::Incremental,
    };

    (new_rel, result)
}

// ---------------------------------------------------------------------------
// 헬퍼
// ---------------------------------------------------------------------------

/// AppraisalEngine 로직을 미러링하여 각 감정의 생성 사유 + 공식을 추적
fn trace_appraisal(
    p: &HexacoProfile,
    situation: &Situation,
    relationship: &Relationship,
) -> String {
    let mut out = String::new();
    let avg = p.dimension_averages();
    let w: f32 = 0.3;
    let rel_mul = relationship.emotion_intensity_multiplier();
    let trust_mod = relationship.trust_emotion_modifier();

    // Event 계수 (공통)
    let emotional_amp = avg.e.abs_modifier(w);
    let positive_amp = avg.x.pos_modifier(w);
    let negative_mod = avg.a.neg_modifier(w);
    let impulse_mod = p.conscientiousness.prudence.neg_modifier(w);
    let fear_amp = p.emotionality.fearfulness.abs_modifier(w);

    // Action 계수 (공통)
    let standards_amp = avg.c.abs_modifier(w);
    let pride_mod = p.honesty_humility.modesty.neg_modifier(w);
    let reproach_mod = p.agreeableness.gentleness.neg_modifier(w);
    let anger_mod = p.agreeableness.patience.modifier(-w);
    let gratitude_amp = p.honesty_humility.sincerity.pos_modifier(w);
    let aesthetic_amp = p.openness.aesthetic_appreciation.abs_modifier(w);

    let h = avg.h.value();
    let a = avg.a.value();
    let empathy_base: f32 = 0.5;
    let fortune_t: f32 = -0.2;

    for (fi, focus) in situation.focuses.iter().enumerate() {
        out.push_str(&format!("━━ Focus {} ━━\n", fi + 1));

        match focus {
            SituationFocus::Event(event) => {
                let d = event.desirability_for_self;
                out.push_str(&format!("  [Event] desirability_for_self = {d:.2}\n"));

                // Prospect Confirmation
                if let Some(Prospect::Confirmation(result)) = &event.prospect {
                    let base = d.abs() * emotional_amp * rel_mul;
                    let etype = match result {
                        ProspectResult::HopeFulfilled => "Satisfaction",
                        ProspectResult::HopeUnfulfilled => "Disappointment",
                        ProspectResult::FearUnrealized => "Relief",
                        ProspectResult::FearConfirmed => "FearsConfirmed",
                    };
                    out.push_str(&format!(
                        "  → {etype}: |{d:.2}| * emotional_amp({emotional_amp:.3}) * rel_mul({rel_mul:.3}) = {base:.3}\n"
                    ));
                    continue;
                }

                // Prospect Anticipation
                if let Some(Prospect::Anticipation) = &event.prospect {
                    if d > 0.0 {
                        let v = d * positive_amp * rel_mul;
                        out.push_str(&format!(
                            "  → Hope: {d:.2} * positive_amp({positive_amp:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                        ));
                    } else if d < 0.0 {
                        let v = d.abs() * emotional_amp * fear_amp * rel_mul;
                        out.push_str(&format!(
                            "  → Fear: |{d:.2}| * emotional_amp({emotional_amp:.3}) * fear_amp({fear_amp:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                        ));
                    }
                    continue;
                }

                // Well-being
                if d > 0.0 {
                    let v = d * emotional_amp * positive_amp * rel_mul;
                    out.push_str(&format!(
                        "  → Joy: {d:.2} * emotional_amp({emotional_amp:.3}) * positive_amp({positive_amp:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                    ));
                } else if d < 0.0 {
                    let v = d.abs() * emotional_amp * negative_mod * impulse_mod * rel_mul;
                    out.push_str(&format!(
                        "  → Distress: |{d:.2}| * emotional_amp({emotional_amp:.3}) * negative_mod({negative_mod:.3}) * impulse_mod({impulse_mod:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                    ));
                }

                // Fortune-of-others
                if let Some(other) = &event.desirability_for_other {
                    let do_ = other.desirability;
                    let c_val = other.relationship.closeness().value();
                    let affinity = other.relationship.closeness().modifier(w);
                    let hostility = other.relationship.closeness().modifier(-w);
                    out.push_str(&format!(
                        "  [타인 운] desir_other={do_:.2}, closeness={c_val:.2}, affinity_mod={affinity:.3}, hostility_mod={hostility:.3}\n"
                    ));

                    if do_ > 0.0 {
                        if h > 0.0 || a > 0.0 {
                            let empathy = (h.max(0.0) + a.max(0.0)) / 2.0;
                            let v = do_ * (empathy_base + empathy * empathy_base) * affinity;
                            out.push_str(&format!(
                                "  → HappyFor: {do_:.2} * (0.5 + empathy({empathy:.3}) * 0.5) * affinity({affinity:.3}) = {v:.3}\n"
                            ));
                        }
                        if h < fortune_t {
                            let v = do_ * h.abs() * negative_mod * hostility;
                            out.push_str(&format!(
                                "  → Resentment: {do_:.2} * |H({h:.2})| * negative_mod({negative_mod:.3}) * hostility({hostility:.3}) = {v:.3}\n"
                            ));
                        }
                    } else if do_ < 0.0 {
                        let abs_d = do_.abs();
                        let sent = p.emotionality.sentimentality.value();
                        if a > 0.0 || sent > 0.0 {
                            let compassion = (a.max(0.0) + sent.max(0.0)) / 2.0;
                            let v = abs_d * (empathy_base + compassion * empathy_base) * affinity;
                            out.push_str(&format!(
                                "  → Pity: |{do_:.2}| * (0.5 + compassion({compassion:.3}) * 0.5) * affinity({affinity:.3}) = {v:.3}\n"
                            ));
                        }
                        if h < fortune_t && a < fortune_t {
                            let cruelty = (h.abs() + a.abs()) / 2.0;
                            let v = abs_d * cruelty * hostility;
                            out.push_str(&format!(
                                "  → Gloating: |{do_:.2}| * cruelty({cruelty:.3}) * hostility({hostility:.3}) = {v:.3}\n"
                            ));
                        }
                    }
                }
            }

            SituationFocus::Action(action) => {
                let pw = action.praiseworthiness;
                let self_str = if action.is_self_agent { "자기" } else { "타인" };
                out.push_str(&format!(
                    "  [Action] {self_str} 행동, praiseworthiness = {pw:.2}\n"
                ));

                if action.is_self_agent {
                    if pw > 0.0 {
                        let v = pw * standards_amp * pride_mod * rel_mul;
                        out.push_str(&format!(
                            "  → Pride: {pw:.2} * standards_amp({standards_amp:.3}) * pride_mod({pride_mod:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                        ));
                    } else if pw < 0.0 {
                        let v = pw.abs() * standards_amp * rel_mul;
                        out.push_str(&format!(
                            "  → Shame: |{pw:.2}| * standards_amp({standards_amp:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                        ));
                    }
                } else {
                    if pw > 0.0 {
                        let v = pw * standards_amp * trust_mod * rel_mul;
                        out.push_str(&format!(
                            "  → Admiration: {pw:.2} * standards_amp({standards_amp:.3}) * trust_mod({trust_mod:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                        ));
                    } else if pw < 0.0 {
                        let v = pw.abs() * standards_amp * reproach_mod * trust_mod * rel_mul;
                        out.push_str(&format!(
                            "  → Reproach: |{pw:.2}| * standards_amp({standards_amp:.3}) * reproach_mod({reproach_mod:.3}) * trust_mod({trust_mod:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                        ));
                    }
                }
            }

            SituationFocus::Object(object) => {
                let ap = object.appealingness;
                out.push_str(&format!("  [Object] appealingness = {ap:.2}\n"));

                if ap > 0.0 {
                    let v = ap * aesthetic_amp * rel_mul;
                    out.push_str(&format!(
                        "  → Love: {ap:.2} * aesthetic_amp({aesthetic_amp:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                    ));
                } else if ap < 0.0 {
                    let v = ap.abs() * aesthetic_amp * rel_mul;
                    out.push_str(&format!(
                        "  → Hate: |{ap:.2}| * aesthetic_amp({aesthetic_amp:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                    ));
                }
            }
        }
    }

    // Compound 감정
    if let (Some(action), Some(event)) = (situation.find_action(), situation.find_event()) {
        let pw = action.praiseworthiness;
        let outcome = event.desirability_for_self;

        if (pw > 0.0 && outcome > 0.0) || (pw < 0.0 && outcome < 0.0) {
            out.push_str("\n━━ Compound (Action+Event) ━━\n");
            let self_str = if action.is_self_agent { "자기" } else { "타인" };
            out.push_str(&format!(
                "  {self_str} 행동, praiseworthiness={pw:.2}, outcome={outcome:.2}\n"
            ));

            if action.is_self_agent {
                if pw > 0.0 && outcome > 0.0 {
                    let v = (pw + outcome) / 2.0 * standards_amp * rel_mul;
                    out.push_str(&format!(
                        "  → Gratification: ({pw:.2}+{outcome:.2})/2 * standards_amp({standards_amp:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                    ));
                } else {
                    let v = (pw.abs() + outcome.abs()) / 2.0 * standards_amp * rel_mul;
                    out.push_str(&format!(
                        "  → Remorse: (|{pw:.2}|+|{outcome:.2}|)/2 * standards_amp({standards_amp:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                    ));
                }
            } else {
                if pw > 0.0 && outcome > 0.0 {
                    let v = (pw + outcome) / 2.0 * gratitude_amp * trust_mod * rel_mul;
                    out.push_str(&format!(
                        "  → Gratitude: ({pw:.2}+{outcome:.2})/2 * gratitude_amp({gratitude_amp:.3}) * trust_mod({trust_mod:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                    ));
                } else {
                    let v = (pw.abs() + outcome.abs()) / 2.0 * anger_mod * trust_mod * rel_mul;
                    out.push_str(&format!(
                        "  → Anger: (|{pw:.2}|+|{outcome:.2}|)/2 * anger_mod({anger_mod:.3}) * trust_mod({trust_mod:.3}) * rel_mul({rel_mul:.3}) = {v:.3}\n"
                    ));
                }
            }
        }
    }

    if out.is_empty() {
        out.push_str("포커스 없음 — 감정 생성 없음\n");
    }

    out
}

fn intermediates_section(
    p: &HexacoProfile,
    rel: &Relationship,
    s: &GuiState,
) -> ResultSection {
    let avg = p.dimension_averages();
    let w: f32 = 0.3; // AppraisalEngine::W

    // HEXACO 차원 평균
    let h = avg.h.value();
    let e = avg.e.value();
    let x = avg.x.value();
    let a = avg.a.value();
    let c = avg.c.value();
    let o = avg.o.value();

    // 감정 증폭/억제 계수
    let emotional_amp = avg.e.abs_modifier(w);
    let positive_amp = avg.x.pos_modifier(w);
    let negative_mod = avg.a.neg_modifier(w);
    let impulse_mod = p.conscientiousness.prudence.neg_modifier(w);
    let standards_amp = avg.c.abs_modifier(w);
    let fear_amp = p.emotionality.fearfulness.abs_modifier(w);
    let pride_mod = p.honesty_humility.modesty.neg_modifier(w);
    let reproach_mod = p.agreeableness.gentleness.neg_modifier(w);
    let anger_mod = p.agreeableness.patience.modifier(-w);
    let gratitude_amp = p.honesty_humility.sincerity.pos_modifier(w);
    let aesthetic_amp = p.openness.aesthetic_appreciation.abs_modifier(w);

    // 관계 계수
    let rel_mul = rel.emotion_intensity_multiplier();
    let trust_mod = rel.trust_emotion_modifier();

    let mut text = String::new();

    // HEXACO 평균
    text.push_str("── HEXACO 차원 평균 ──\n");
    text.push_str(&format!(
        "  H={:.2}  E={:.2}  X={:.2}  A={:.2}  C={:.2}  O={:.2}\n\n",
        h, e, x, a, c, o
    ));

    // Event 계수
    text.push_str("── Event 감정 계수 ──\n");
    text.push_str(&format!(
        "  emotional_amp = 1.0 + |E({:.2})| * {w} = {emotional_amp:.3}\n",
        e
    ));
    text.push_str(&format!(
        "  positive_amp  = 1.0 + max(0,X({:.2})) * {w} = {positive_amp:.3}\n",
        x
    ));
    text.push_str(&format!(
        "  negative_mod  = 1.0 - max(0,A({:.2})) * {w} = {negative_mod:.3}\n",
        a
    ));
    text.push_str(&format!(
        "  impulse_mod   = 1.0 - max(0,prudence({:.2})) * {w} = {impulse_mod:.3}\n",
        p.conscientiousness.prudence.value()
    ));
    text.push_str(&format!(
        "  fear_amp      = 1.0 + |fearfulness({:.2})| * {w} = {fear_amp:.3}\n\n",
        p.emotionality.fearfulness.value()
    ));

    // Action 계수
    text.push_str("── Action 감정 계수 ──\n");
    text.push_str(&format!(
        "  standards_amp = 1.0 + |C({:.2})| * {w} = {standards_amp:.3}\n",
        c
    ));
    text.push_str(&format!(
        "  pride_mod     = 1.0 - max(0,modesty({:.2})) * {w} = {pride_mod:.3}\n",
        p.honesty_humility.modesty.value()
    ));
    text.push_str(&format!(
        "  reproach_mod  = 1.0 - max(0,gentleness({:.2})) * {w} = {reproach_mod:.3}\n",
        p.agreeableness.gentleness.value()
    ));
    text.push_str(&format!(
        "  gratitude_amp = 1.0 + max(0,sincerity({:.2})) * {w} = {gratitude_amp:.3}\n",
        p.honesty_humility.sincerity.value()
    ));
    text.push_str(&format!(
        "  anger_mod     = 1.0 + patience({:.2}) * (-{w}) = {anger_mod:.3}\n",
        p.agreeableness.patience.value()
    ));
    text.push_str(&format!(
        "  aesthetic_amp = 1.0 + |aesthetic({:.2})| * {w} = {aesthetic_amp:.3}\n\n",
        p.openness.aesthetic_appreciation.value()
    ));

    // 관계 계수
    text.push_str("── 관계 계수 ──\n");
    text.push_str(&format!(
        "  rel_mul   = 1.0 + |closeness({:.2})| * 0.5 = {rel_mul:.3}\n",
        s.closeness
    ));
    text.push_str(&format!(
        "  trust_mod = 1.0 + trust({:.2}) * 0.3 = {trust_mod:.3}\n",
        s.trust
    ));

    ResultSection {
        heading: "중간 계산값 + 공식".into(),
        content: text,
    }
}

fn emotion_state_sections(state: &EmotionState) -> Vec<ResultSection> {
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

    vec![ResultSection {
        heading: "감정 상태".into(),
        content: emotions_text,
    }]
}
