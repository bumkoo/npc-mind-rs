//! OCC 감정 유형과 감정 상태 정의

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// OCC 감정 유형 (22개)
// ---------------------------------------------------------------------------

/// OCC 모델의 22개 감정 유형
///
/// 3개 분기: Event(사건), Action(행동), Object(대상)
/// 각 감정은 양/음의 valence를 가짐
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmotionType {
    // === Event-based: 사건의 결과 ===

    // Well-being (자기 복지)
    /// 자신에게 바람직한 사건 → 기쁨
    Joy,
    /// 자신에게 바람직하지 않은 사건 → 고통
    Distress,

    // Fortune-of-others (타인의 운)
    /// 타인에게 바람직한 사건 + 나도 기쁨 → 대리기쁨
    HappyFor,
    /// 타인에게 바람직하지 않은 사건 + 내가 안타까움 → 동정
    Pity,
    /// 타인에게 바람직하지 않은 사건 + 내가 기쁨 → 고소함
    Gloating,
    /// 타인에게 바람직한 사건 + 내가 불쾌 → 시기/원망
    Resentment,

    // Prospect-based (전망)
    /// 바람직한 사건이 일어날 가능성 → 희망
    Hope,
    /// 바람직하지 않은 사건이 일어날 가능성 → 두려움
    Fear,
    /// 바랐던 일이 실현됨 → 만족
    Satisfaction,
    /// 바랐던 일이 실현되지 않음 → 실망
    Disappointment,
    /// 두려워했던 일이 일어나지 않음 → 안도
    Relief,
    /// 두려워했던 일이 실현됨 → 공포확인
    FearsConfirmed,

    // === Action-based: 행위자의 행동 ===

    // Attribution (귀인)
    /// 자신의 행동을 긍정 평가 → 자부심
    Pride,
    /// 자신의 행동을 부정 평가 → 수치심
    Shame,
    /// 타인의 행동을 긍정 평가 → 감탄
    Admiration,
    /// 타인의 행동을 부정 평가 → 비난
    Reproach,

    // Compound: Well-being + Attribution
    /// Pride + Joy → 자신의 좋은 행동이 좋은 결과를 낳음
    Gratification,
    /// Shame + Distress → 자신의 나쁜 행동이 나쁜 결과를 낳음
    Remorse,
    /// Admiration + Joy → 타인의 좋은 행동이 나에게 좋은 결과
    Gratitude,
    /// Reproach + Distress → 타인의 나쁜 행동이 나에게 나쁜 결과
    Anger,

    // === Object-based: 대상에 대한 반응 ===
    /// 매력적인 대상 → 좋아함
    Love,
    /// 비매력적인 대상 → 싫어함
    Hate,
}

impl EmotionType {
    /// 이 감정의 고유 인덱스 (0~21)
    /// 배열 기반의 빠른 접근을 위해 사용합니다.
    pub fn index(&self) -> usize {
        match self {
            Self::Joy => 0, Self::Distress => 1,
            Self::HappyFor => 2, Self::Pity => 3, Self::Gloating => 4, Self::Resentment => 5,
            Self::Hope => 6, Self::Fear => 7,
            Self::Satisfaction => 8, Self::Disappointment => 9,
            Self::Relief => 10, Self::FearsConfirmed => 11,
            Self::Pride => 12, Self::Shame => 13,
            Self::Admiration => 14, Self::Reproach => 15,
            Self::Gratification => 16, Self::Remorse => 17,
            Self::Gratitude => 18, Self::Anger => 19,
            Self::Love => 20, Self::Hate => 21,
        }
    }

    /// 인덱스로부터 감정 유형 반환
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Joy), 1 => Some(Self::Distress),
            2 => Some(Self::HappyFor), 3 => Some(Self::Pity), 4 => Some(Self::Gloating), 5 => Some(Self::Resentment),
            6 => Some(Self::Hope), 7 => Some(Self::Fear),
            8 => Some(Self::Satisfaction), 9 => Some(Self::Disappointment),
            10 => Some(Self::Relief), 11 => Some(Self::FearsConfirmed),
            12 => Some(Self::Pride), 13 => Some(Self::Shame),
            14 => Some(Self::Admiration), 15 => Some(Self::Reproach),
            16 => Some(Self::Gratification), 17 => Some(Self::Remorse),
            18 => Some(Self::Gratitude), 19 => Some(Self::Anger),
            20 => Some(Self::Love), 21 => Some(Self::Hate),
            _ => None,
        }
    }

    /// 이 감정의 기본 valence (양수=긍정, 음수=부정)
    pub fn base_valence(&self) -> f32 {
        match self {
            Self::Joy | Self::HappyFor | Self::Hope |
            Self::Satisfaction | Self::Relief |
            Self::Pride | Self::Admiration |
            Self::Gratification | Self::Gratitude |
            Self::Love => 1.0,

            Self::Distress | Self::Pity | Self::Fear |
            Self::Disappointment | Self::FearsConfirmed |
            Self::Shame | Self::Reproach |
            Self::Remorse | Self::Anger |
            Self::Hate => -1.0,

            // Gloating/Resentment: 복합 valence
            Self::Gloating => 0.5,    // 긍정이지만 어두운 기쁨
            Self::Resentment => -0.5, // 부정이지만 질투 성격
        }
    }

    /// OCC 분기 분류
    pub fn branch(&self) -> EmotionBranch {
        match self {
            Self::Joy | Self::Distress |
            Self::HappyFor | Self::Pity | Self::Gloating | Self::Resentment |
            Self::Hope | Self::Fear |
            Self::Satisfaction | Self::Disappointment |
            Self::Relief | Self::FearsConfirmed => EmotionBranch::Event,

            Self::Pride | Self::Shame |
            Self::Admiration | Self::Reproach |
            Self::Gratification | Self::Remorse |
            Self::Gratitude | Self::Anger => EmotionBranch::Action,

            Self::Love | Self::Hate => EmotionBranch::Object,
        }
    }
}

/// OCC 3대 분기
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmotionBranch {
    Event,  // 사건의 결과
    Action, // 행위자의 행동
    Object, // 대상의 속성
}

// ---------------------------------------------------------------------------
// 감정 인스턴스 (특정 감정 + 강도)
// ---------------------------------------------------------------------------

/// 하나의 감정 인스턴스: 감정 유형 + 강도
///
/// 필드는 캡슐화되어 있으며, getter를 통해 접근한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emotion {
    emotion_type: EmotionType,
    /// 감정 강도 (0.0 ~ 1.0, 0이면 느끼지 않음)
    intensity: f32,
    /// 감정의 원인/맥락 설명 (LLM 프롬프트에 포함됨)
    context: Option<String>,
}

impl Emotion {
    /// 감정 인스턴스 생성
    ///
    /// 강도는 의도적으로 0.0~1.0 범위로 클램핑된다.
    /// AppraisalEngine이 성격 가중치 곱셈 과정에서 범위를 초과하는
    /// 중간값을 생성할 수 있으므로, 정규화를 위해 클램핑을 사용한다.
    pub fn new(emotion_type: EmotionType, intensity: f32) -> Self {
        Self {
            emotion_type,
            intensity: intensity.clamp(0.0, 1.0),
            context: None,
        }
    }

    /// context 포함 감정 생성
    pub fn with_context(emotion_type: EmotionType, intensity: f32, context: impl Into<String>) -> Self {
        Self {
            emotion_type,
            intensity: intensity.clamp(0.0, 1.0),
            context: Some(context.into()),
        }
    }

    /// 감정 유형
    pub fn emotion_type(&self) -> EmotionType {
        self.emotion_type
    }

    /// 감정 강도 (0.0 ~ 1.0)
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    /// 감정의 원인/맥락 설명
    pub fn context(&self) -> Option<&str> {
        self.context.as_deref()
    }

    /// 이 감정이 유의미한지 (강도가 threshold 이상)
    pub fn is_significant(&self, threshold: f32) -> bool {
        self.intensity >= threshold
    }

    /// 강도에 값을 추가 (클램핑)
    pub(super) fn add_intensity(&mut self, amount: f32) {
        self.intensity = (self.intensity + amount).clamp(0.0, 1.0);
    }
}

/// NPC의 현재 감정 상태: 여러 감정의 조합
///
/// 리팩토링: Vec<Emotion> 대신 고정 크기 배열 [f32; 22]를 사용하여
/// 감정 추가 및 조회 성능을 최적화했습니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionState {
    /// 각 감정 유형별 강도 (인덱스는 EmotionType::index()와 대응)
    intensities: [f32; 22],
    /// 각 감정 유형별 맥락 설명
    contexts: [Option<String>; 22],
}

impl Default for EmotionState {
    fn default() -> Self {
        Self::new()
    }
}

impl EmotionState {
    /// 빈 감정 상태 생성
    pub fn new() -> Self {
        Self {
            intensities: [0.0; 22],
            contexts: Default::default(),
        }
    }

    /// 감정 목록을 Vec<Emotion>으로 변환하여 반환 (강도가 0보다 큰 것만)
    /// 외부와의 호환성을 위해 제공됩니다.
    pub fn emotions(&self) -> Vec<Emotion> {
        self.intensities.iter().enumerate()
            .filter(|(_, &i)| i > 0.0)
            .filter_map(|(idx, &i)| {
                EmotionType::from_index(idx).map(|t| {
                    match &self.contexts[idx] {
                        Some(ctx) => Emotion::with_context(t, i, ctx.clone()),
                        None => Emotion::new(t, i),
                    }
                })
            })
            .collect()
    }

    /// 감정 추가 (같은 유형이면 강도 합산, context는 최초 것 유지)
    pub fn add(&mut self, emotion: Emotion) {
        let idx = emotion.emotion_type().index();
        self.intensities[idx] = (self.intensities[idx] + emotion.intensity()).clamp(0.0, 1.0);
        if self.contexts[idx].is_none() {
            self.contexts[idx] = emotion.context;
        }
    }

    /// 가장 강한 감정 반환
    pub fn dominant(&self) -> Option<Emotion> {
        self.intensities.iter().enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .and_then(|(idx, &i)| {
                if i > 0.0 {
                    EmotionType::from_index(idx).map(|t| {
                        match &self.contexts[idx] {
                            Some(ctx) => Emotion::with_context(t, i, ctx.clone()),
                            None => Emotion::new(t, i),
                        }
                    })
                } else {
                    None
                }
            })
    }

    /// threshold 이상의 유의미한 감정들만 반환 (강도 내림차순)
    pub fn significant(&self, threshold: f32) -> Vec<Emotion> {
        let mut result = self.emotions();
        result.retain(|e| e.is_significant(threshold));
        result.sort_by(|a, b| b.intensity().partial_cmp(&a.intensity()).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    /// 특정 감정의 강도 조회
    pub fn intensity_of(&self, emotion_type: EmotionType) -> f32 {
        self.intensities[emotion_type.index()]
    }

    /// 특정 감정의 맥락 조회
    pub fn context_of(&self, emotion_type: EmotionType) -> Option<&str> {
        self.contexts[emotion_type.index()].as_deref()
    }

    /// 특정 감정의 강도를 직접 설정 (apply_stimulus용)
    pub fn set_intensity(&mut self, emotion_type: EmotionType, intensity: f32) {
        let idx = emotion_type.index();
        self.intensities[idx] = intensity.clamp(0.0, 1.0);
    }

    /// 특정 감정 제거 (자연 소멸, apply_stimulus용)
    pub fn remove(&mut self, emotion_type: EmotionType) {
        let idx = emotion_type.index();
        self.intensities[idx] = 0.0;
    }

    /// 전체 감정 valence (양수=긍정적 상태, 음수=부정적 상태)
    pub fn overall_valence(&self) -> f32 {
        let mut sum = 0.0;
        let mut count = 0;

        for (idx, &intensity) in self.intensities.iter().enumerate() {
            if intensity > 0.0 {
                if let Some(t) = EmotionType::from_index(idx) {
                    sum += t.base_valence() * intensity;
                    count += 1;
                }
            }
        }

        if count == 0 { return 0.0; }
        (sum / count as f32).clamp(-1.0, 1.0)
    }
}
