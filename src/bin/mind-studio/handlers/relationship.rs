use crate::state::RelationshipData;
use crate::events::StateEvent;

impl_crud_handlers!(RelationshipData, relationships, list_relationships, upsert_relationship, delete_relationship, relationship, StateEvent::RelationshipChanged);
