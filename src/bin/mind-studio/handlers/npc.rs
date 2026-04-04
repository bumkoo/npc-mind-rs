use crate::state::NpcProfile;

impl_crud_handlers!(NpcProfile, npcs, list_npcs, upsert_npc, delete_npc);
