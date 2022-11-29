

use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::str::FromStr;

use account::Account;
use near_fixed_bit_tree::BitTree;
use events::Event;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base58CryptoHash, U128, U64};
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::serde_json::{json, self};
use near_sdk::{env, near_bindgen, AccountId, log, bs58, PanicOnDefault, Promise, BlockHeight, CryptoHash, assert_one_yocto, BorshStorageKey};
use near_sdk::collections::{LookupMap, UnorderedMap, Vector, LazyOption, UnorderedSet};
use drip::{Drip};
use role::{RoleManagement};
use utils::{refund_extra_storage_deposit};
use crate::post::Hierarchy;
use crate::utils::get_arg;
use std::convert::TryFrom;
use role::Permission;
use access::Access;
use account::Deposit;


pub mod utils;
pub mod post;
pub mod owner;
pub mod drip;
pub mod view;
pub mod events;
pub mod role;
pub mod access;
pub mod account;
pub mod resolver;
pub mod internal;
pub mod metadata;


#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Community {
    owner_id: AccountId,
    args: HashMap<String, String>,
    accounts: LookupMap<AccountId, Account>,
    content_tree: BitTree,
    relationship_tree: BitTree,
    reports: UnorderedMap<Base58CryptoHash, HashSet<AccountId>>,
    drip: Drip,
    role_management: RoleManagement,
    access: AccessLimit
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct OldCommunity {
    owner_id: AccountId,
    args: HashMap<String, String>,
    accounts: LookupMap<AccountId, Account>,
    content_tree: BitTree,
    relationship_tree: BitTree,
    reports: UnorderedMap<Base58CryptoHash, HashSet<AccountId>>,
    drip: Drip,
    roles: Vec<u8>
}


const MAX_LEVEL: usize = 3;

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKey {
    Report,
    Account,
    Roles
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(BorshDeserialize, BorshSerialize, BorshStorageKey)]
pub enum AccessLimit {
    Free,
    Registy,
    TokenLimit(Access)
}

/*
args : {
    drip_contract: AccountId,
    open_access: bool   //no join action needed
}
*/
const DRIP_CONTRACT: &str = "drip_contract";


#[near_bindgen]
impl Community {

    #[init]
    pub fn new(owner_id: AccountId, args: HashMap<String, String>) -> Self {
        let mut this = Self {
            owner_id: owner_id.clone(),
            args,
            accounts: LookupMap::new(StorageKey::Account),
            content_tree: BitTree::new(28, vec![0], u16::BITS as u8),
            relationship_tree: BitTree::new(28, vec![1], 0),
            reports: UnorderedMap::new(StorageKey::Report),
            drip: Drip::new(),
            role_management: RoleManagement::new(),
            access: AccessLimit::Registy
        };
        let mut account = this.accounts.get(&owner_id).unwrap_or_default();
        account.set_registered(true);
        this.accounts.insert(&owner_id, &account);
        this
    }

    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let prev: OldCommunity = env::state_read().expect("ERR_NOT_INITIALIZED");
        assert_eq!(
            env::predecessor_account_id(),
            prev.owner_id,
            "Only owner"
        );
        let mut args = HashMap::new();
        
        let this = Community {
            owner_id: env::predecessor_account_id(),
            args: args,
            accounts: LookupMap::new(StorageKey::Account),
            content_tree: BitTree::new(28, vec![0], u16::BITS as u8),
            relationship_tree: BitTree::new(28, vec![1], 0),
            reports: UnorderedMap::new(b'r'),
            drip: Drip::new(),
            role_management: RoleManagement::new(),
            access: AccessLimit::Registy
        };
        this
    }

    pub fn follow(&mut self, account_id: AccountId) {
        let sender_id = env::predecessor_account_id();
        let hash = env::sha256(&(sender_id.to_string() + "follwing" + &account_id.to_string()).into_bytes());
        self.relationship_tree.set(&hash, 0);
        Event::log_follow(sender_id, account_id,None);
    }

    pub fn unfollow(&mut self, account_id: AccountId) {
        let sender_id = env::predecessor_account_id();
        let hash = env::sha256(&(sender_id.to_string() + "follwing" + &account_id.to_string()).into_bytes());
        self.relationship_tree.del(&hash);
        Event::log_unfollow(sender_id, account_id, None);
    }
    
    #[payable]
    pub fn join(&mut self) {
        let sender_id = env::predecessor_account_id();
        let mut account = self.accounts.get(&sender_id).unwrap_or_default();
        account.set_registered(true);
        self.accounts.insert(&sender_id, &account);
    }

    #[payable]
    pub fn quit(&mut self) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let account = self.accounts.get(&sender_id);
        if let Some(mut account) = account {
            if account.get_drip() == 0 {
                self.accounts.remove(&sender_id);
            } else {
                account.set_registered(false);
            }
        }
    }

    pub fn withdraw(&mut self, deposit: Deposit, amount: U128) {
        let sender_id = env::predecessor_account_id();
        match deposit {
            Deposit::FT(account_id) => {
                let mut account = self.accounts.get(&sender_id).unwrap_or_default();
                account.decrease_deposit(Deposit::FT(account_id), amount.0);
                self.accounts.insert(&sender_id, &account);
            }
            _ => {}
        }
    }

    #[payable]
    pub fn collect_drip(&mut self) -> U128 {
        assert_one_yocto();
        let sender_id = env::signer_account_id();
        self.drip.get_and_clear_drip(sender_id)
    }
}





#[cfg(test)]
mod tests {


}