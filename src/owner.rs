use crate::*;
use utils::get_parent_contract_id;

#[near_bindgen]
impl Community {
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

#[no_mangle]
pub extern "C" fn upgrade() {
    env::setup_panic_hook();
    assert!(get_parent_contract_id() == env::predecessor_account_id(), "contract's parent only");
    let input = env::input().unwrap();
    Promise::new(env::predecessor_account_id()).deploy_contract(input);
}