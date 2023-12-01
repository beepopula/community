use crate::{*, utils::{get_root_id}};
use utils::get_parent_contract_id;

#[near_bindgen]
impl Community {
    pub fn get_args(&self) -> HashMap<String, String> {
        self.args.clone()
    }

    // #[payable]
    // pub fn set_args(&mut self, args: HashMap<String, String>) {
    //     assert_one_yocto();
    //     let sender = get_predecessor_id();
    //     assert!(sender == self.owner_id || 
    //         get_parent_contract_id(env::current_account_id()) == get_predecessor_id() ||
    //         env::current_account_id() == get_predecessor_id()
    //     , "owner only");
    //     self.args = args
    // }

    #[payable]
    pub fn set_owner(&mut self, account_id: AccountId) {
        assert_one_yocto();
        let sender = get_predecessor_id();
        assert!(sender == self.owner_id || 
            get_parent_contract_id(env::current_account_id()) == get_predecessor_id() ||
            env::current_account_id() == get_predecessor_id()
        , "owner only");
        self.owner_id = account_id;
    }

    #[payable]
    pub fn del_contract(&mut self) {
        assert_one_yocto();
        let sender = get_predecessor_id();
        assert!(sender == self.owner_id || 
            get_parent_contract_id(env::current_account_id()) == get_predecessor_id() ||
            env::current_account_id() == get_predecessor_id()
        , "owner only");
        Promise::new(env::current_account_id()).delete_account(sender);
    }

    pub fn get_access_limit(&self) -> AccessLimit {
        self.access.clone()
    }

    #[payable]
    pub fn set_access_limit(&mut self, access: AccessLimit) {
        assert_one_yocto();
        let sender = get_predecessor_id();
        assert!(sender == self.owner_id || 
            get_parent_contract_id(env::current_account_id()) == get_predecessor_id() ||
            env::current_account_id() == get_predecessor_id()
        , "owner only");
        self.access = access;
    }

    #[private]  //TODO: open it?
    pub fn distribute(&mut self, list: Vec<(AccountId, AssetKey, U128)>) {
        let mut community = get_account(&env::current_account_id()).registered();
        for (account_id, asset, amount) in list {
            match asset {
                AssetKey::Drip(_) => panic!("not allowed"), 
                _ => {}
            };
            let mut account = match get_account(&account_id).get_registered() {
                Some(v) => v,
                None => continue
            };
            let balance = community.get_balance(&asset);
            if let Some(_) = balance.checked_sub(amount.0) {
                account.increase_balance(asset.clone(), amount.0);
                self.accounts.insert(&account_id, &account);
                community.decrease_balance(asset, amount.0)
            }
        }
        self.accounts.insert(&env::current_account_id(), &community);
    }
}

#[no_mangle]
pub extern "C" fn upgrade() {
    env::setup_panic_hook();
    let parent_contract_id = get_parent_contract_id(env::current_account_id());
    let root_contract_id = get_root_id(env::current_account_id());
    let owner_id = env::state_read::<Community>().unwrap().owner_id;
    assert!(parent_contract_id == get_predecessor_id(), "contract's parent only");
    assert!(root_contract_id == env::signer_account_id() || owner_id == env::signer_account_id(), "not owner");
    let input = env::input().unwrap();
    Promise::new(env::current_account_id()).deploy_contract(input);
}