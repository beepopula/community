use crate::*;
use crate::events::Metadata;

#[near_bindgen]
impl Community {
    pub fn set_metadata(&mut self, items: Vec<(String, String)>) {
        let sender_id = get_predecessor_id();
        let mut metadata = vec![];
        for (key, val) in items {
            assert!(self.can_execute_action(None, None, Permission::Other(Some(key.clone()))));
            metadata.push(Metadata {
                key,
                val
            })
        }
        Event::log_set_metadata(metadata)
    }
}