use crate::state::ObjectEntry;
use crate::events::StateEvent;

impl_crud_handlers!(ObjectEntry, objects, list_objects, upsert_object, delete_object, StateEvent::ObjectChanged);
