//! Rumor 애그리거트 — 소문 도메인 모델 (Step C1 foundation)
//!
//! 설계 문서: `docs/memory/03-implementation-design.md` §2.6
//!
//! 소문(Rumor)은 **확산 상태를 추적하는 애그리거트**이며, 실제 콘텐츠는 두 경로로
//! 해소된다:
//! 1. `topic`이 있으면 같은 topic의 Canonical `MemoryEntry(Seeded + World)`를 참조
//! 2. `topic`이 없거나 Canonical이 아직 없으면(= "예보된 사실") `seed_content`를 사용
//!
//! **불변식** (I-RU-1 ~ I-RU-6):
//! - I-RU-1: hop_index는 0부터 단조 증가한다.
//! - I-RU-2: distortion은 parent 체인을 통해 DAG를 이루며, 자기 자신·조상을 parent로
//!   가질 수 없다 (비순환).
//! - I-RU-3: status 전이는 `Active → Fading → Faded` 단방향.
//! - I-RU-4: `seed_content`는 생성자에서 1회 설정 후 불변.
//! - I-RU-5: Canonical 참조(`topic` 링크)는 불변.
//! - I-RU-6: append-only — hop/distortion은 추가만 가능하며 제거는 없다.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Rumor 생성 기원
// ---------------------------------------------------------------------------

/// 소문의 기원. `RumorSeeded` 이벤트 페이로드에도 그대로 실린다.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RumorOrigin {
    /// 시나리오 작가가 직접 시드한 소문
    Seeded,
    /// 특정 `WorldEventOccurred` 이벤트에서 파생 (Step D에서 발행)
    FromWorldEvent { event_id: u64 },
    /// NPC가 작성한 소문 (옵션으로 작성자)
    Authored { by: Option<String> },
}

// ---------------------------------------------------------------------------
// Reach — 소문 확산 범위 정책
// ---------------------------------------------------------------------------

/// 소문이 도달할 수 있는 범위.
///
/// 빈 vec은 "이 축은 제한 없음"을 의미한다. 모든 축이 비어 있으면 무제한.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReachPolicy {
    pub regions: Vec<String>,
    pub factions: Vec<String>,
    pub npc_ids: Vec<String>,
    /// 소문이 "가치 있다"고 판단되는 최소 significance. 0.0이면 제한 없음.
    pub min_significance: f32,
}

// ---------------------------------------------------------------------------
// Hop / Distortion — 확산 이력
// ---------------------------------------------------------------------------

/// 한 번의 소문 확산 이벤트. 단조 증가하는 `hop_index`로 정렬된다 (I-RU-1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RumorHop {
    pub hop_index: u32,
    /// 이 홉에서 전달된 컨텐츠 버전 (= DistortionId). None이면 원본 그대로.
    pub content_version: Option<String>,
    pub recipients: Vec<String>,
    pub spread_at: u64,
}

/// 컨텐츠 변형 노드. parent 체인을 따라가며 DAG를 이룬다 (I-RU-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RumorDistortion {
    pub id: String,
    /// 이 변형이 파생된 앞선 변형 id. None이면 원본에서 직접 파생.
    pub parent: Option<String>,
    pub content: String,
    pub created_at: u64,
}

// ---------------------------------------------------------------------------
// Status — 확산 상태 머신
// ---------------------------------------------------------------------------

/// 소문의 생애주기. I-RU-3: `Active → Fading → Faded` 단방향.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RumorStatus {
    Active,
    Fading,
    Faded,
}

// ---------------------------------------------------------------------------
// Rumor 애그리거트 루트
// ---------------------------------------------------------------------------

/// 소문 애그리거트. `id`가 애그리거트 루트 식별자.
///
/// 생성 후 `topic`·`seed_content`·`origin`·`reach_policy`는 불변(I-RU-4, I-RU-5).
/// hop/distortion은 전용 mutator로만 추가된다(I-RU-6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rumor {
    pub id: String,
    /// Canonical `MemoryEntry` 참조 키. None이면 고아 Rumor.
    pub topic: Option<String>,
    /// `topic=None`이거나 Canonical이 아직 시딩되지 않은 "예보된 사실"일 때 사용.
    pub seed_content: Option<String>,
    pub origin: RumorOrigin,
    pub reach_policy: ReachPolicy,
    hops: Vec<RumorHop>,
    distortions: Vec<RumorDistortion>,
    pub created_at: u64,
    status: RumorStatus,
}

/// Rumor 애그리거트 불변식 위반.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum RumorError {
    #[error("hop_index는 단조 증가해야 한다 (I-RU-1): 마지막 {last}, 신규 {new}")]
    HopIndexNotMonotonic { last: u32, new: u32 },

    #[error("distortion '{child}'의 parent '{parent}'가 존재하지 않는다 (I-RU-2)")]
    DistortionParentNotFound { child: String, parent: String },

    #[error("distortion '{id}'가 자기 자신을 parent로 가리킨다 (I-RU-2)")]
    DistortionSelfParent { id: String },

    #[error("distortion '{id}'가 조상을 parent로 가져 순환 DAG가 된다 (I-RU-2)")]
    DistortionCycle { id: String },

    #[error("status 전이 역행 (I-RU-3): {from:?} → {to:?}")]
    InvalidStatusTransition { from: RumorStatus, to: RumorStatus },

    #[error("topic 없는 Rumor는 seed_content가 있어야 한다 (1차 §3.4.6)")]
    OrphanRumorMissingSeed,

    #[error("중복된 distortion id: '{id}'")]
    DuplicateDistortionId { id: String },

    #[error("중복된 hop_index: {index}")]
    DuplicateHopIndex { index: u32 },

    #[error(
        "RumorHop.content_version '{cv}'이 이 rumor의 distortion 목록에 없다 (참조 무결성 위반)"
    )]
    HopContentVersionUnknown { cv: String },
}

impl Rumor {
    /// 일반 소문 생성 — `topic`이 있고 Canonical 존재를 가정.
    ///
    /// `seed_content`가 있으면 "예보된 사실"(§2.6 Canonical 해소 표) 경로로 취급되어
    /// Canonical 시딩 전까지 `seed_content`가 사용된다.
    pub fn new(
        id: impl Into<String>,
        topic: impl Into<String>,
        origin: RumorOrigin,
        reach_policy: ReachPolicy,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            topic: Some(topic.into()),
            seed_content: None,
            origin,
            reach_policy,
            hops: Vec::new(),
            distortions: Vec::new(),
            created_at,
            status: RumorStatus::Active,
        }
    }

    /// 예보된 사실 — `topic`은 있지만 Canonical이 아직 없어 `seed_content` 경로 사용.
    pub fn with_forecast_content(
        id: impl Into<String>,
        topic: impl Into<String>,
        seed_content: impl Into<String>,
        origin: RumorOrigin,
        reach_policy: ReachPolicy,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            topic: Some(topic.into()),
            seed_content: Some(seed_content.into()),
            origin,
            reach_policy,
            hops: Vec::new(),
            distortions: Vec::new(),
            created_at,
            status: RumorStatus::Active,
        }
    }

    /// 고아 Rumor — topic 없음, seed_content 필수.
    pub fn orphan(
        id: impl Into<String>,
        seed_content: impl Into<String>,
        origin: RumorOrigin,
        reach_policy: ReachPolicy,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            topic: None,
            seed_content: Some(seed_content.into()),
            origin,
            reach_policy,
            hops: Vec::new(),
            distortions: Vec::new(),
            created_at,
            status: RumorStatus::Active,
        }
    }

    pub fn hops(&self) -> &[RumorHop] {
        &self.hops
    }

    pub fn distortions(&self) -> &[RumorDistortion] {
        &self.distortions
    }

    pub fn status(&self) -> RumorStatus {
        self.status
    }

    /// 고아 여부 (topic 없음).
    pub fn is_orphan(&self) -> bool {
        self.topic.is_none()
    }

    /// 다음 홉 인덱스 — 현재 마지막 hop_index + 1, 비어 있으면 0.
    pub fn next_hop_index(&self) -> u32 {
        self.hops.last().map(|h| h.hop_index + 1).unwrap_or(0)
    }

    /// 새 홉 추가 — I-RU-1 (단조 증가) + content_version 참조 무결성 강제.
    pub fn add_hop(&mut self, hop: RumorHop) -> Result<(), RumorError> {
        // content_version이 있으면 distortions에 실존해야 한다 (참조 무결성).
        if let Some(cv) = &hop.content_version {
            if !self.distortions.iter().any(|d| &d.id == cv) {
                return Err(RumorError::HopContentVersionUnknown { cv: cv.clone() });
            }
        }
        if let Some(last) = self.hops.last() {
            if hop.hop_index <= last.hop_index {
                return Err(RumorError::HopIndexNotMonotonic {
                    last: last.hop_index,
                    new: hop.hop_index,
                });
            }
        }
        // 동일 hop_index 중복 방지 (last 이후 추가되는 hop이라도 중간 삽입 시 걸러냄).
        if self.hops.iter().any(|h| h.hop_index == hop.hop_index) {
            return Err(RumorError::DuplicateHopIndex {
                index: hop.hop_index,
            });
        }
        self.hops.push(hop);
        Ok(())
    }

    /// 새 변형 추가 — I-RU-2 (비순환 DAG) 강제.
    pub fn add_distortion(&mut self, distortion: RumorDistortion) -> Result<(), RumorError> {
        if self.distortions.iter().any(|d| d.id == distortion.id) {
            return Err(RumorError::DuplicateDistortionId {
                id: distortion.id.clone(),
            });
        }
        if let Some(parent) = &distortion.parent {
            if parent == &distortion.id {
                return Err(RumorError::DistortionSelfParent {
                    id: distortion.id.clone(),
                });
            }
            if !self.distortions.iter().any(|d| &d.id == parent) {
                return Err(RumorError::DistortionParentNotFound {
                    child: distortion.id.clone(),
                    parent: parent.clone(),
                });
            }
            // 순환 검사 불필요: 신규 id는 중복 검사를 이미 통과했으므로 기존 조상 체인에
            // 존재하지 않는다. 기존 distortions가 비순환이면 잎으로 붙여도 비순환 유지.
        }
        self.distortions.push(distortion);
        Ok(())
    }

    /// 상태 전이 — I-RU-3 단방향 강제.
    pub fn transition_to(&mut self, next: RumorStatus) -> Result<(), RumorError> {
        let ok = matches!(
            (self.status, next),
            (RumorStatus::Active, RumorStatus::Fading)
                | (RumorStatus::Active, RumorStatus::Faded)
                | (RumorStatus::Fading, RumorStatus::Faded)
        );
        if !ok && self.status != next {
            return Err(RumorError::InvalidStatusTransition {
                from: self.status,
                to: next,
            });
        }
        self.status = next;
        Ok(())
    }

    /// 편의 메서드 — `Active → Fading`.
    pub fn fade(&mut self) -> Result<(), RumorError> {
        self.transition_to(RumorStatus::Fading)
    }

    /// 편의 메서드 — 현재 상태에서 `Faded`로 종결.
    pub fn fade_out(&mut self) -> Result<(), RumorError> {
        self.transition_to(RumorStatus::Faded)
    }

    /// 불변식 자가 검증 — 저장소 로드 시 호출해 깨진 상태를 감지한다.
    pub fn validate(&self) -> Result<(), RumorError> {
        // I-RU-1: hop_index 단조성.
        let mut prev: Option<u32> = None;
        for h in &self.hops {
            if let Some(p) = prev {
                if h.hop_index <= p {
                    return Err(RumorError::HopIndexNotMonotonic {
                        last: p,
                        new: h.hop_index,
                    });
                }
            }
            prev = Some(h.hop_index);
        }

        // I-RU-2: distortion DAG 비순환 + parent 존재.
        //
        // 불변식: parent는 반드시 목록에서 **앞**에 나와야 한다 (토폴로지 정렬).
        // parent가 뒤에 있으면 = 순환 의심 (둘 이상의 노드가 서로를 참조). `add_distortion`이
        // 순차 추가만 허용하므로 위배는 `from_parts`로 저장된 데이터를 로드할 때만 발생.
        for (i, d) in self.distortions.iter().enumerate() {
            if let Some(parent) = &d.parent {
                if parent == &d.id {
                    return Err(RumorError::DistortionSelfParent { id: d.id.clone() });
                }
                if self.distortions[..i].iter().any(|p| &p.id == parent) {
                    continue; // OK — parent가 앞에 있음
                }
                // 앞에 없음: 뒤에 있는지 / 아예 없는지 분기
                if self.distortions[i + 1..].iter().any(|p| &p.id == parent) {
                    return Err(RumorError::DistortionCycle { id: d.id.clone() });
                }
                return Err(RumorError::DistortionParentNotFound {
                    child: d.id.clone(),
                    parent: parent.clone(),
                });
            }
        }

        // RumorHop.content_version 참조 무결성.
        for h in &self.hops {
            if let Some(cv) = &h.content_version {
                if !self.distortions.iter().any(|d| &d.id == cv) {
                    return Err(RumorError::HopContentVersionUnknown { cv: cv.clone() });
                }
            }
        }

        // 고아 Rumor는 seed_content 필수.
        if self.topic.is_none() && self.seed_content.is_none() {
            return Err(RumorError::OrphanRumorMissingSeed);
        }

        Ok(())
    }

    /// 저장소 로드용 — 원시 필드로 Rumor 재구성. 불변식 검증 포함.
    ///
    /// `hops`는 hop_index 오름차순으로, `distortions`는 토폴로지 순서(부모 먼저)로
    /// 주어져야 한다. 이 메서드는 **저장소 재구성 전용**이며 `transition_to`의
    /// 단방향 규칙(I-RU-3)을 우회한다 — 외부 consumer가 악용하지 못하도록
    /// `pub(crate)`로 제한한다.
    pub(crate) fn from_parts(
        id: String,
        topic: Option<String>,
        seed_content: Option<String>,
        origin: RumorOrigin,
        reach_policy: ReachPolicy,
        hops: Vec<RumorHop>,
        distortions: Vec<RumorDistortion>,
        created_at: u64,
        status: RumorStatus,
    ) -> Result<Self, RumorError> {
        let r = Self {
            id,
            topic,
            seed_content,
            origin,
            reach_policy,
            hops,
            distortions,
            created_at,
            status,
        };
        r.validate()?;
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_reach() -> ReachPolicy {
        ReachPolicy::default()
    }

    #[test]
    fn new_rumor_has_topic_no_seed_and_active() {
        let r = Rumor::new("r1", "topic-x", RumorOrigin::Seeded, empty_reach(), 100);
        assert_eq!(r.topic.as_deref(), Some("topic-x"));
        assert!(r.seed_content.is_none());
        assert_eq!(r.status(), RumorStatus::Active);
        assert!(!r.is_orphan());
    }

    #[test]
    fn orphan_rumor_has_no_topic_but_has_seed() {
        let r = Rumor::orphan(
            "r-orph",
            "강호에 이상한 기운이 돈다",
            RumorOrigin::Authored { by: None },
            empty_reach(),
            100,
        );
        assert!(r.topic.is_none());
        assert!(r.seed_content.is_some());
        assert!(r.is_orphan());
    }

    #[test]
    fn forecast_rumor_has_both_topic_and_seed() {
        let r = Rumor::with_forecast_content(
            "r-forecast",
            "moorim-leader-change",
            "조만간 무림맹주가 바뀐다더라",
            RumorOrigin::Authored {
                by: Some("informant".into()),
            },
            empty_reach(),
            100,
        );
        assert_eq!(r.topic.as_deref(), Some("moorim-leader-change"));
        assert_eq!(r.seed_content.as_deref(), Some("조만간 무림맹주가 바뀐다더라"));
    }

    #[test]
    fn add_hop_enforces_monotonic_index() {
        let mut r = Rumor::new("r1", "topic", RumorOrigin::Seeded, empty_reach(), 0);
        assert_eq!(r.next_hop_index(), 0);
        r.add_hop(RumorHop {
            hop_index: 0,
            content_version: None,
            recipients: vec!["a".into()],
            spread_at: 1,
        })
        .unwrap();
        assert_eq!(r.next_hop_index(), 1);

        // 역행 거부
        let err = r
            .add_hop(RumorHop {
                hop_index: 0,
                content_version: None,
                recipients: vec![],
                spread_at: 2,
            })
            .unwrap_err();
        assert!(matches!(err, RumorError::HopIndexNotMonotonic { .. }));

        // 순방향 OK
        r.add_hop(RumorHop {
            hop_index: 1,
            content_version: None,
            recipients: vec!["b".into()],
            spread_at: 3,
        })
        .unwrap();
        assert_eq!(r.hops().len(), 2);
    }

    #[test]
    fn add_distortion_rejects_missing_parent_and_self_parent() {
        let mut r = Rumor::new("r1", "topic", RumorOrigin::Seeded, empty_reach(), 0);

        // parent None: OK
        r.add_distortion(RumorDistortion {
            id: "d1".into(),
            parent: None,
            content: "원본 변형".into(),
            created_at: 1,
        })
        .unwrap();

        // 자기 자신을 parent로: 거부
        let err = r
            .add_distortion(RumorDistortion {
                id: "d2".into(),
                parent: Some("d2".into()),
                content: "self-loop".into(),
                created_at: 2,
            })
            .unwrap_err();
        assert!(matches!(err, RumorError::DistortionSelfParent { .. }));

        // 존재하지 않는 parent: 거부
        let err = r
            .add_distortion(RumorDistortion {
                id: "d3".into(),
                parent: Some("ghost".into()),
                content: "orphan-parent".into(),
                created_at: 3,
            })
            .unwrap_err();
        assert!(matches!(err, RumorError::DistortionParentNotFound { .. }));

        // 이미 존재하는 parent로 연쇄: OK
        r.add_distortion(RumorDistortion {
            id: "d4".into(),
            parent: Some("d1".into()),
            content: "chain".into(),
            created_at: 4,
        })
        .unwrap();
        assert_eq!(r.distortions().len(), 2);
    }

    #[test]
    fn add_distortion_rejects_duplicate_id() {
        let mut r = Rumor::new("r1", "topic", RumorOrigin::Seeded, empty_reach(), 0);
        r.add_distortion(RumorDistortion {
            id: "d1".into(),
            parent: None,
            content: "a".into(),
            created_at: 1,
        })
        .unwrap();
        let err = r
            .add_distortion(RumorDistortion {
                id: "d1".into(),
                parent: None,
                content: "b".into(),
                created_at: 2,
            })
            .unwrap_err();
        assert!(matches!(err, RumorError::DuplicateDistortionId { .. }));
    }

    #[test]
    fn status_transitions_active_to_fading_to_faded() {
        let mut r = Rumor::new("r1", "topic", RumorOrigin::Seeded, empty_reach(), 0);
        r.transition_to(RumorStatus::Fading).unwrap();
        assert_eq!(r.status(), RumorStatus::Fading);
        r.transition_to(RumorStatus::Faded).unwrap();
        assert_eq!(r.status(), RumorStatus::Faded);
    }

    #[test]
    fn status_transitions_allow_active_to_faded_directly() {
        let mut r = Rumor::new("r1", "topic", RumorOrigin::Seeded, empty_reach(), 0);
        r.transition_to(RumorStatus::Faded).unwrap();
        assert_eq!(r.status(), RumorStatus::Faded);
    }

    #[test]
    fn status_transitions_reject_backward_and_same() {
        // Faded → Active 는 거부. 동일 상태로의 전이는 허용(no-op).
        let mut r = Rumor::new("r1", "topic", RumorOrigin::Seeded, empty_reach(), 0);
        r.transition_to(RumorStatus::Faded).unwrap();
        let err = r.transition_to(RumorStatus::Active).unwrap_err();
        assert!(matches!(err, RumorError::InvalidStatusTransition { .. }));
        // same-state는 허용 (멱등성)
        r.transition_to(RumorStatus::Faded).unwrap();
    }

    #[test]
    fn from_parts_validates_orphan_requires_seed() {
        // 고아 + seed 없음 → 실패
        let err = Rumor::from_parts(
            "r1".into(),
            None,
            None,
            RumorOrigin::Seeded,
            empty_reach(),
            vec![],
            vec![],
            0,
            RumorStatus::Active,
        )
        .unwrap_err();
        assert_eq!(err, RumorError::OrphanRumorMissingSeed);
    }

    #[test]
    fn from_parts_validates_hop_monotonicity() {
        let err = Rumor::from_parts(
            "r1".into(),
            Some("t".into()),
            None,
            RumorOrigin::Seeded,
            empty_reach(),
            vec![
                RumorHop {
                    hop_index: 1,
                    content_version: None,
                    recipients: vec![],
                    spread_at: 1,
                },
                RumorHop {
                    hop_index: 0,
                    content_version: None,
                    recipients: vec![],
                    spread_at: 2,
                },
            ],
            vec![],
            0,
            RumorStatus::Active,
        )
        .unwrap_err();
        assert!(matches!(err, RumorError::HopIndexNotMonotonic { .. }));
    }

    #[test]
    fn from_parts_detects_two_node_cycle() {
        // d1 → d2 → d1 (상호 참조) — 토폴로지 정렬 불가능
        let err = Rumor::from_parts(
            "r1".into(),
            Some("t".into()),
            None,
            RumorOrigin::Seeded,
            empty_reach(),
            vec![],
            vec![
                RumorDistortion {
                    id: "d1".into(),
                    parent: Some("d2".into()),
                    content: "a".into(),
                    created_at: 1,
                },
                RumorDistortion {
                    id: "d2".into(),
                    parent: Some("d1".into()),
                    content: "b".into(),
                    created_at: 2,
                },
            ],
            0,
            RumorStatus::Active,
        )
        .unwrap_err();
        assert!(
            matches!(err, RumorError::DistortionCycle { .. }),
            "2-node cycle must be caught, got {err:?}"
        );
    }

    #[test]
    fn from_parts_detects_three_node_cycle() {
        // d1 → d2 → d3 → d1
        let err = Rumor::from_parts(
            "r1".into(),
            Some("t".into()),
            None,
            RumorOrigin::Seeded,
            empty_reach(),
            vec![],
            vec![
                RumorDistortion {
                    id: "d1".into(),
                    parent: Some("d3".into()),
                    content: "a".into(),
                    created_at: 1,
                },
                RumorDistortion {
                    id: "d2".into(),
                    parent: Some("d1".into()),
                    content: "b".into(),
                    created_at: 2,
                },
                RumorDistortion {
                    id: "d3".into(),
                    parent: Some("d2".into()),
                    content: "c".into(),
                    created_at: 3,
                },
            ],
            0,
            RumorStatus::Active,
        )
        .unwrap_err();
        assert!(
            matches!(err, RumorError::DistortionCycle { .. }),
            "3-node cycle must be caught, got {err:?}"
        );
    }

    #[test]
    fn from_parts_accepts_valid_topological_order() {
        // d1 (root) → d2 → d3 순서대로 저장된 정상 DAG
        let r = Rumor::from_parts(
            "r1".into(),
            Some("t".into()),
            None,
            RumorOrigin::Seeded,
            empty_reach(),
            vec![],
            vec![
                RumorDistortion {
                    id: "d1".into(),
                    parent: None,
                    content: "root".into(),
                    created_at: 1,
                },
                RumorDistortion {
                    id: "d2".into(),
                    parent: Some("d1".into()),
                    content: "child".into(),
                    created_at: 2,
                },
                RumorDistortion {
                    id: "d3".into(),
                    parent: Some("d2".into()),
                    content: "grandchild".into(),
                    created_at: 3,
                },
            ],
            0,
            RumorStatus::Active,
        )
        .unwrap();
        assert_eq!(r.distortions().len(), 3);
    }

    #[test]
    fn add_hop_rejects_unknown_content_version() {
        let mut r = Rumor::new("r1", "t", RumorOrigin::Seeded, empty_reach(), 0);
        let err = r
            .add_hop(RumorHop {
                hop_index: 0,
                content_version: Some("ghost".into()),
                recipients: vec!["a".into()],
                spread_at: 1,
            })
            .unwrap_err();
        assert!(matches!(err, RumorError::HopContentVersionUnknown { .. }));

        // distortion 추가 후에는 같은 id를 참조하는 hop이 통과
        r.add_distortion(RumorDistortion {
            id: "d1".into(),
            parent: None,
            content: "a".into(),
            created_at: 2,
        })
        .unwrap();
        r.add_hop(RumorHop {
            hop_index: 0,
            content_version: Some("d1".into()),
            recipients: vec!["a".into()],
            spread_at: 3,
        })
        .unwrap();
    }

    #[test]
    fn from_parts_rejects_unknown_content_version() {
        let err = Rumor::from_parts(
            "r1".into(),
            Some("t".into()),
            None,
            RumorOrigin::Seeded,
            empty_reach(),
            vec![RumorHop {
                hop_index: 0,
                content_version: Some("ghost".into()),
                recipients: vec![],
                spread_at: 1,
            }],
            vec![],
            0,
            RumorStatus::Active,
        )
        .unwrap_err();
        assert!(matches!(err, RumorError::HopContentVersionUnknown { .. }));
    }

    #[test]
    fn serde_roundtrip_preserves_status_and_origin_tag() {
        let mut r = Rumor::with_forecast_content(
            "r1",
            "t",
            "예보",
            RumorOrigin::FromWorldEvent { event_id: 42 },
            ReachPolicy {
                regions: vec!["central".into()],
                factions: vec!["moorim".into()],
                npc_ids: vec![],
                min_significance: 0.3,
            },
            123,
        );
        // distortion이 먼저 추가되어야 content_version 참조 무결성을 만족.
        r.add_distortion(RumorDistortion {
            id: "d1".into(),
            parent: None,
            content: "변형".into(),
            created_at: 150,
        })
        .unwrap();
        r.add_hop(RumorHop {
            hop_index: 0,
            content_version: Some("d1".into()),
            recipients: vec!["npc-a".into()],
            spread_at: 200,
        })
        .unwrap();

        let json = serde_json::to_string(&r).unwrap();
        let back: Rumor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }
}
