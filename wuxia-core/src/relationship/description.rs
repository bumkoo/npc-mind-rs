// wuxia-core/src/relationship/description.rs
//
// 관계 설명 데이터 타입.
//
// descriptions.toml의 구조를 Rust 타입으로 정의한다.
// 이 타입은 wuxia-data(로딩)와 wuxia-llm(프롬프트 조립) 모두에서 사용한다.
//
// 의존성 방향:
//   wuxia-core (타입 정의) ← wuxia-data (로딩) + wuxia-llm (사용)
//   → wuxia-data ↔ wuxia-llm 직접 참조 없음.
//
// 비유: 강호 인맥첩의 "해설 양식"
//   숫자 → enum (types.rs)
//   enum → 자연어 (이 파일의 타입으로 toml에서 로딩)
//   자연어 → 프롬프트 (wuxia-llm에서 조립)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// LocalizedDesc — 국가별 단어 + 행동 정의
// ---------------------------------------------------------------------------

/// 하나의 enum 값에 대한 국가별 라벨과 행동 정의.
///
/// `ko_desc`와 `en_desc`는 LLM 프롬프트에 삽입되어 NPC 연기 지침이 된다.
///
/// # Example (descriptions.toml)
/// ```toml
/// [relationship_level.Friendly]
/// ko = "친근"
/// ko_desc = "상대에게 호감이 있다. 가벼운 농담도 한다."
/// en = "Friendly"
/// en_desc = "Has affinity. Makes casual jokes."
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalizedDesc {
    /// 한국어 라벨 (UI 표시용). 예: "친근"
    pub ko: String,
    /// 한국어 행동 정의 (LLM 프롬프트용). 예: "상대에게 호감이 있다."
    pub ko_desc: String,
    /// English label. e.g. "Friendly"
    pub en: String,
    /// English behavioral definition. e.g. "Has affinity."
    pub en_desc: String,
}

impl LocalizedDesc {
    /// locale에 맞는 (라벨, 행동 정의) 튜플을 반환한다.
    ///
    /// 지원 locale: "ko", "en". 그 외는 영어 fallback.
    pub fn get(&self, locale: &str) -> (&str, &str) {
        match locale {
            "ko" => (&self.ko, &self.ko_desc),
            _ => (&self.en, &self.en_desc), // en fallback
        }
    }
}

use super::types::Impression;

// ... (LocalizedDesc remains same)

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationshipDescriptions {
    /// RelationshipLevel enum → 설명.
    pub relationship_level: HashMap<String, LocalizedDesc>,
    /// TrustLevel enum → 설명.
    pub trust_level: HashMap<String, LocalizedDesc>,
    /// [v4.5] 도메인 인상(Impression) 기반 설명.
    pub impression: ImpressionDescription,
}

/// [v4.5] 도메인 인상(Impression) 상태별 국가별 묘사.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImpressionDescription {
    /// 첫 만남 (FirstMeeting)
    pub first_meeting: InteractionLocalizedDesc,
    /// 호의적 재회 (WarmReunion)
    pub warm_reunion: InteractionLocalizedDesc,
    /// 적대적 재회 (ColdReunion)
    pub cold_reunion: InteractionLocalizedDesc,
    /// 서먹한 재회 (NeutralReunion)
    pub neutral_reunion: InteractionLocalizedDesc,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionLocalizedDesc {
    pub ko: String,
    pub en: String,
}

impl InteractionLocalizedDesc {
    pub fn get(&self, locale: &str) -> &str {
        match locale {
            "ko" => &self.ko,
            _ => &self.en,
        }
    }
}

impl RelationshipDescriptions {
    /// 도메인 인상 상태와 횟수를 기반으로 자연어 설명을 생성한다 (도메인 규칙).
    pub fn describe_impression(&self, impression: Impression, count: u32, locale: &str) -> String {
        let desc = match impression {
            Impression::FirstMeeting => &self.impression.first_meeting,
            Impression::WarmReunion => &self.impression.warm_reunion,
            Impression::ColdReunion => &self.impression.cold_reunion,
            Impression::NeutralReunion => &self.impression.neutral_reunion,
        };

        desc.get(locale).replace("{count}", &count.to_string())
    }

    /// RelationshipLevel 키로 조회한다.
    ///
    /// # Example
    /// ```
    /// use wuxia_core::relationship::RelationshipLevel;
    /// // descs.lookup_relationship_level("Friendly", "ko")
    /// //   → Some(("친근", "상대에게 호감이 있다. 가벼운 농담도 한다."))
    /// ```
    pub fn lookup_relationship_level<'a>(
        &'a self,
        key: &str,
        locale: &str,
    ) -> Option<(&'a str, &'a str)> {
        self.relationship_level.get(key).map(|d| d.get(locale))
    }

    /// TrustLevel 키로 조회한다.
    pub fn lookup_trust_level<'a>(
        &'a self,
        key: &str,
        locale: &str,
    ) -> Option<(&'a str, &'a str)> {
        self.trust_level.get(key).map(|d| d.get(locale))
    }

}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_desc(ko: &str, ko_desc: &str, en: &str, en_desc: &str) -> LocalizedDesc {
        LocalizedDesc {
            ko: ko.to_string(),
            ko_desc: ko_desc.to_string(),
            en: en.to_string(),
            en_desc: en_desc.to_string(),
        }
    }

    fn make_descriptions() -> RelationshipDescriptions {
        let mut rl = HashMap::new();
        rl.insert(
            "Friendly".to_string(),
            make_desc(
                "친근",
                "상대에게 호감이 있다. 가벼운 농담도 한다.",
                "Friendly",
                "Has affinity. Makes casual jokes.",
            ),
        );
        rl.insert(
            "Enemy".to_string(),
            make_desc(
                "원수",
                "상대를 적으로 본다. 대화를 거부하거나 위협한다.",
                "Enemy",
                "Sees the other as an enemy. Refuses or threatens.",
            ),
        );

        let mut tl = HashMap::new();
        tl.insert(
            "Cautious".to_string(),
            make_desc(
                "조심스러운 신뢰",
                "어느 정도 신뢰하지만, 비밀을 털어놓을 정도는 아니다.",
                "Cautious Trust",
                "Somewhat trusts, but not enough to share secrets.",
            ),
        );

        RelationshipDescriptions {
            relationship_level: rl,
            trust_level: tl,
            impression: ImpressionDescription {
                first_meeting: InteractionLocalizedDesc {
                    ko: "첫 만남이다.".to_string(),
                    en: "First meeting.".to_string(),
                },
                warm_reunion: InteractionLocalizedDesc {
                    ko: "반가운 재회다({count}회).".to_string(),
                    en: "Warm reunion ({count} times).".to_string(),
                },
                cold_reunion: InteractionLocalizedDesc {
                    ko: "불쾌한 재회다({count}회).".to_string(),
                    en: "Cold reunion ({count} times).".to_string(),
                },
                neutral_reunion: InteractionLocalizedDesc {
                    ko: "서먹한 재회다({count}회).".to_string(),
                    en: "Neutral reunion ({count} times).".to_string(),
                },
            },
        }
    }

    #[test]
    fn localized_desc_get_ko() {
        let desc = make_desc("친근", "호감이 있다.", "Friendly", "Has affinity.");
        let (label, definition) = desc.get("ko");
        assert_eq!(label, "친근");
        assert_eq!(definition, "호감이 있다.");
    }

    #[test]
    fn localized_desc_get_en() {
        let desc = make_desc("친근", "호감이 있다.", "Friendly", "Has affinity.");
        let (label, definition) = desc.get("en");
        assert_eq!(label, "Friendly");
        assert_eq!(definition, "Has affinity.");
    }

    #[test]
    fn localized_desc_unknown_locale_falls_back_to_en() {
        let desc = make_desc("친근", "호감이 있다.", "Friendly", "Has affinity.");
        let (label, _) = desc.get("ja");
        assert_eq!(label, "Friendly");
    }

    #[test]
    fn lookup_relationship_level_found() {
        let descs = make_descriptions();
        let result = descs.lookup_relationship_level("Friendly", "ko");
        assert!(result.is_some());
        let (label, desc) = result.unwrap();
        assert_eq!(label, "친근");
        assert!(desc.contains("호감"));
    }

    #[test]
    fn lookup_relationship_level_not_found() {
        let descs = make_descriptions();
        assert!(descs.lookup_relationship_level("Unknown", "ko").is_none());
    }

    #[test]
    fn lookup_trust_level_found() {
        let descs = make_descriptions();
        let result = descs.lookup_trust_level("Cautious", "ko");
        assert!(result.is_some());
        let (label, _) = result.unwrap();
        assert_eq!(label, "조심스러운 신뢰");
    }

    #[test]
    fn serde_roundtrip() {
        let descs = make_descriptions();
        let json = serde_json::to_string(&descs).expect("serialize");
        let restored: RelationshipDescriptions =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(descs, restored);
    }
}
