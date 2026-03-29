//! 로케일 기반 연기 가이드 포맷터
//!
//! LocaleBundle(TOML)에서 로드한 번역 데이터를 사용하여
//! 언어에 무관하게 ActingGuide를 텍스트/JSON으로 변환한다.

use serde::Serialize;

use crate::domain::guide::*;
use crate::ports::GuideFormatter;
use super::locale::LocaleBundle;

/// 로케일 기반 포맷터 — TOML 로케일 파일 하나로 어떤 언어든 지원
pub struct LocaleFormatter {
    locale: LocaleBundle,
}

impl LocaleFormatter {
    /// LocaleBundle을 직접 주입하여 생성
    pub fn new(locale: LocaleBundle) -> Self {
        Self { locale }
    }

    /// TOML 문자열에서 직접 생성
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        Ok(Self { locale: LocaleBundle::from_toml(content)? })
    }

    /// 내부 로케일 번들 참조
    pub fn locale(&self) -> &LocaleBundle {
        &self.locale
    }
}

impl GuideFormatter for LocaleFormatter {
    fn format_prompt(&self, guide: &ActingGuide) -> String {
        let l = &self.locale;
        let t = &l.template;
        let mut lines = Vec::new();

        // --- NPC 기본 정보 ---
        lines.push(l.render_template(&t.section_npc, &[("name", &guide.npc_name)]));
        if !guide.npc_description.is_empty() {
            lines.push(guide.npc_description.clone());
        }
        lines.push(String::new());

        // --- 성격 ---
        lines.push(t.section_personality.clone());
        lines.push(l.format_traits(&guide.personality));
        lines.push(String::new());

        // --- 현재 감정 ---
        lines.push(t.section_emotion.clone());

        // 상위 3개 감정을 줄바꿈으로 표시, 지배 감정에 "(지배)" 표시
        if !guide.emotion.active_emotions.is_empty() {
            lines.push(t.emotion_composition.clone());

            let dominant_type = guide.emotion.dominant.as_ref().map(|d| d.emotion_type);
            let top3 = guide.emotion.active_emotions.iter().take(3);

            for entry in top3 {
                let emotion = l.emotion_name(&entry.emotion_type);
                let intensity_str = l.intensity_label(entry.intensity);
                let is_dominant = dominant_type == Some(entry.emotion_type);
                let label = if is_dominant {
                    format!("- {}({}, 지배)", emotion, intensity_str)
                } else {
                    format!("- {}({})", emotion, intensity_str)
                };
                let line = match &entry.context {
                    Some(ctx) if !ctx.is_empty() => format!("{} — {}", label, ctx),
                    _ => label,
                };
                lines.push(line);
            }
        }
        let mood_str = l.mood_label(guide.emotion.mood);
        lines.push(l.render_template(&t.overall_mood, &[("mood", mood_str)]));
        lines.push(String::new());

        // --- 상황 ---
        if let Some(ref desc) = guide.situation_description {
            lines.push(t.section_situation.clone());
            lines.push(desc.clone());
            lines.push(String::new());
        }

        // --- 연기 지시 ---
        lines.push(t.section_directive.clone());
        lines.push(l.render_template(&t.directive_tone, &[
            ("tone", l.tone_label(&guide.directive.tone)),
        ]));
        lines.push(l.render_template(&t.directive_attitude, &[
            ("attitude", l.attitude_label(&guide.directive.attitude)),
        ]));
        lines.push(l.render_template(&t.directive_behavior, &[
            ("behavior", l.behavioral_tendency_label(&guide.directive.behavioral_tendency)),
        ]));
        lines.push(String::new());

        // --- 말투 ---
        lines.push(t.section_speech.clone());
        lines.push(l.format_speech_styles(&guide.personality));
        lines.push(String::new());

        // --- 금지 사항 ---
        if !guide.directive.restrictions.is_empty() {
            lines.push(t.section_restriction.clone());
            for r in &guide.directive.restrictions {
                lines.push(l.render_template(&t.restriction_item, &[
                    ("restriction", l.restriction_label(r)),
                ]));
            }
            lines.push(String::new());
        }

        // --- 관계 ---
        if let Some(ref rel) = guide.relationship {
            lines.push(l.render_template(&t.section_relationship, &[]));
            lines.push(l.render_template(&t.relationship_closeness, &[
                ("level", l.closeness_level_label(&rel.closeness_level)),
            ]));
            lines.push(l.render_template(&t.relationship_trust, &[
                ("level", l.trust_level_label(&rel.trust_level)),
            ]));
            lines.push(l.render_template(&t.relationship_power, &[
                ("level", l.power_level_label(&rel.power_level)),
            ]));
        }

        lines.join("\n")
    }

    fn format_json(&self, guide: &ActingGuide) -> Result<String, serde_json::Error> {
        let l = &self.locale;

        let output = LocaleGuideOutput {
            npc_name: guide.npc_name.clone(),
            npc_description: guide.npc_description.clone(),
            personality: LocalePersonalityOutput {
                traits: l.format_traits(&guide.personality),
                speech_style: l.format_speech_styles(&guide.personality),
            },
            emotion: LocaleEmotionOutput {
                dominant: guide.emotion.dominant.as_ref().map(|entry|
                    format!("{}({})", l.emotion_name(&entry.emotion_type), l.intensity_label(entry.intensity))),
                active_emotions: guide.emotion.active_emotions.iter()
                    .map(|entry|
                        format!("{}({})", l.emotion_name(&entry.emotion_type), l.intensity_label(entry.intensity)))
                    .collect(),
                mood: guide.emotion.mood,
                mood_label: l.mood_label(guide.emotion.mood).to_string(),
            },
            directive: LocaleDirectiveOutput {
                tone: l.tone_label(&guide.directive.tone).to_string(),
                attitude: l.attitude_label(&guide.directive.attitude).to_string(),
                behavioral_tendency: l.behavioral_tendency_label(
                    &guide.directive.behavioral_tendency).to_string(),
                restrictions: guide.directive.restrictions.iter()
                    .map(|r| l.restriction_label(r).to_string())
                    .collect(),
            },
            situation_description: guide.situation_description.clone(),
            relationship: guide.relationship.as_ref().map(|rel| LocaleRelationshipOutput {
                target_name: rel.target_name.clone(),
                closeness: l.closeness_level_label(&rel.closeness_level).to_string(),
                trust: l.trust_level_label(&rel.trust_level).to_string(),
                power: l.power_level_label(&rel.power_level).to_string(),
            }),
        };

        serde_json::to_string_pretty(&output)
    }
}

// ---------------------------------------------------------------------------
// JSON 출력용 DTO — 로케일 기반 직렬화 전용 구조체
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct LocaleGuideOutput {
    npc_name: String,
    npc_description: String,
    personality: LocalePersonalityOutput,
    emotion: LocaleEmotionOutput,
    directive: LocaleDirectiveOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    situation_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    relationship: Option<LocaleRelationshipOutput>,
}

#[derive(Serialize)]
struct LocalePersonalityOutput {
    traits: String,
    speech_style: String,
}

#[derive(Serialize)]
struct LocaleEmotionOutput {
    dominant: Option<String>,
    active_emotions: Vec<String>,
    mood: f32,
    mood_label: String,
}

#[derive(Serialize)]
struct LocaleDirectiveOutput {
    tone: String,
    attitude: String,
    behavioral_tendency: String,
    restrictions: Vec<String>,
}

#[derive(Serialize)]
struct LocaleRelationshipOutput {
    target_name: String,
    closeness: String,
    trust: String,
    power: String,
}
