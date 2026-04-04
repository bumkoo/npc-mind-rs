use crate::state::ObjectEntry;

impl_crud_handlers!(ObjectEntry, objects, list_objects, upsert_object, delete_object);
