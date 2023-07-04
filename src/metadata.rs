use crate::*;
use crate::events::Metadata;

#[near_bindgen]
impl Community {
    pub fn set_metadata(&self, items: Vec<(String, String)>) {
        let sender_id = env::predecessor_account_id();
        let mut metadata = vec![];
        for (key, val) in items {
            if !self.can_execute_action(None, Permission::Other(Some(key.clone()))) {
                continue
            }
            metadata.push(Metadata {
                key,
                val
            })
        }
        Event::log_set_metadata(metadata)
    }
}