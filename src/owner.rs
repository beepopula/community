use crate::*;
use utils::get_parent_contract_id;

#[near_bindgen]
impl Community {
    pub fn get_public_key(&self) -> String {
        self.public_key.clone()
    }

    #[payable]
    pub fn set_public_key(&mut self, public_key: String) {
        assert_one_yocto();
        let sender = env::predecessor_account_id();
        assert!(sender == self.owner_id, "owner only");
        self.public_key = public_key;
    }

    #[payable]
    pub fn set_owner(&mut self, account_id: AccountId) {
        assert_one_yocto();
        let sender = env::predecessor_account_id();
        assert!(sender == self.owner_id, "owner only");
        self.owner_id = account_id;
    }

    #[payable]
    pub fn del_contract(&mut self) {
        assert_one_yocto();
        let sender = env::predecessor_account_id();
        assert!(sender == self.owner_id, "owner only");
        Promise::new(env::current_account_id()).delete_account(sender);
    }
}

#[no_mangle]
pub extern "C" fn upgrade() {
    env::setup_panic_hook();
    assert!(get_parent_contract_id(env::current_account_id()) == env::predecessor_account_id(), "contract's parent only");
    let input = env::input().unwrap();
    Promise::new(env::current_account_id()).deploy_contract(input);
}