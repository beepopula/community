use crate::*;

#[near_bindgen]
impl Popula {
    pub fn get_public_key(&self) -> String {
        self.public_key.clone()
    }

    pub fn set_public_key(&mut self, public_key: String) {
        let sender = env::predecessor_account_id();
        assert!(sender == self.owner_id, "owner only");
        self.public_key = public_key;
    }

    pub fn set_access(&mut self, conditions: Vec<Condition>, relationship: Relationship) {
        match &mut self.access.clone() {
            Some(v) => v.set(conditions, relationship),
            None => self.access = Some(Access::new(conditions, relationship))
        }
    }
}
