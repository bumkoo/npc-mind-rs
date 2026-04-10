use crate::state::NpcProfile;
use crate::events::StateEvent;

impl_crud_handlers!(NpcProfile, npcs, list_npcs, upsert_npc, delete_npc, StateEvent::NpcChanged);
