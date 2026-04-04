use crate::state::RelationshipData;

impl_crud_handlers!(RelationshipData, relationships, list_relationships, upsert_relationship, delete_relationship, relationship);
