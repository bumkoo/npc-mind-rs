//! Unit of Work (작업 단위) 패턴 구현
//!
//! 단일 비즈니스 트랜잭션 내에서 변경된 애그리거트들을 추적하고,
//! 트랜잭션 종료 시 저장소에 일괄 반영(commit)하는 책임을 가진다.

use std::sync::{Arc, Mutex};
use crate::domain::emotion::{EmotionState, Scene};
use crate::domain::relationship::Relationship;
use crate::ports::MindRepository;
use crate::application::command::handler_v2::HandlerError;

use std::fmt;

/// 트랜잭션 내에서 변경된 상태를 관리하는 객체
pub struct UnitOfWork<'a, R: MindRepository> {
    repository: &'a Arc<Mutex<R>>,
    
    // --- 추적 중인 애그리거트들 (Dirty Checking 대용) ---
    /// (NPC ID, 변경된 상태)
    pub(crate) emotion_state: Option<(String, EmotionState)>,
    pub(crate) relationship: Option<Relationship>,
    pub(crate) scene: Option<Scene>,
    
    // --- 삭제/초기화 시그널 ---
    pub(crate) clear_emotion_for: Option<String>,
    pub(crate) clear_scene: bool,
    
    // --- 결과물 ---
    pub(crate) guide: Option<crate::domain::guide::ActingGuide>,
}

impl<'a, R: MindRepository> fmt::Debug for UnitOfWork<'a, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnitOfWork")
            .field("emotion_state", &self.emotion_state)
            .field("relationship", &self.relationship)
            .field("scene", &self.scene)
            .field("clear_emotion_for", &self.clear_emotion_for)
            .field("clear_scene", &self.clear_scene)
            .field("guide", &self.guide)
            .finish()
    }
}

impl<'a, R: MindRepository> UnitOfWork<'a, R> {
    pub fn new(repository: &'a Arc<Mutex<R>>) -> Self {
        Self {
            repository,
            emotion_state: None,
            relationship: None,
            scene: None,
            clear_emotion_for: None,
            clear_scene: false,
            guide: None,
        }
    }

    /// 감정 상태를 작업 단위에 등록 (변경 예고)
    pub fn save_emotion_state(&mut self, npc_id: String, state: EmotionState) {
        self.emotion_state = Some((npc_id, state));
    }

    /// 관계 정보를 작업 단위에 등록 (변경 예고)
    pub fn save_relationship(&mut self, relationship: Relationship) {
        self.relationship = Some(relationship);
    }

    /// 장면 정보를 작업 단위에 등록 (변경 예고)
    pub fn save_scene(&mut self, scene: Scene) {
        self.scene = Some(scene);
    }

    /// 감정 상태 삭제 예약
    pub fn clear_emotion_for(&mut self, npc_id: String) {
        self.clear_emotion_for = Some(npc_id);
    }

    /// 장면 종료 예약
    pub fn clear_scene(&mut self) {
        self.clear_scene = true;
    }

    /// 변경 사항을 리포지토리에 영구 반영 (Transactional Commit)
    pub fn commit(self) -> Result<(), HandlerError> {
        let mut repo = self.repository.lock()
            .map_err(|_| HandlerError::Infrastructure("repository mutex poisoned"))?;

        // 1. 저장 (Update/Create)
        if let Some((npc_id, state)) = self.emotion_state {
            repo.save_emotion_state(&npc_id, state);
        }

        if let Some(rel) = self.relationship {
            let owner_id = rel.owner_id().to_string();
            let target_id = rel.target_id().to_string();
            repo.save_relationship(&owner_id, &target_id, rel);
        }

        if let Some(scene) = self.scene {
            repo.save_scene(scene);
        }

        // 2. 삭제 (Delete/Clear)
        if let Some(npc_id) = self.clear_emotion_for {
            repo.clear_emotion_state(&npc_id);
        }

        if self.clear_scene {
            repo.clear_scene();
        }

        Ok(())
    }
}
