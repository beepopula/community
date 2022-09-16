

use std::collections::{HashMap, HashSet};
use std::convert::TryInto;

use account::Account;
use bit_tree::BitTree;
use events::Event;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base58CryptoHash, U128, U64};
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::serde_json::{json, self};
use near_sdk::{env, near_bindgen, AccountId, log, bs58, PanicOnDefault, Promise, BlockHeight, CryptoHash, assert_one_yocto, BorshStorageKey};
use near_sdk::collections::{LookupMap, UnorderedMap, Vector, LazyOption, UnorderedSet};
use drip::{Drip};
use post::{Report};
use role::{Role, RoleKind};
use utils::{check_args, verify, check_encrypt_args, refund_extra_storage_deposit, set_content};
use crate::post::Hierarchy;
use std::convert::TryFrom;
use role::Permission;
use access::Access;


pub mod utils;
pub mod signature;
pub mod bit_tree;
pub mod post;
pub mod owner;
pub mod drip;
pub mod view;
pub mod events;
pub mod role;
pub mod access;
pub mod account;


#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Community {
    owner_id: AccountId,
    public_key: String,
    content_tree: BitTree,
    relationship_tree: BitTree,
    reports: UnorderedMap<AccountId, UnorderedMap<Base58CryptoHash, Report>>,
    drip: Drip,
    roles: UnorderedMap<String, Role>
}

#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Value(Vec<(u8, u16)>);

impl Value {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}


const MAX_LEVEL: usize = 3;

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKey {
    Report,
    Account
}


#[near_bindgen]
impl Community {

    #[init]
    pub fn new(owner_id: AccountId, public_key: String) -> Self {
        let mut this = Self {
            owner_id: owner_id.clone(),
            public_key: public_key,
            content_tree: BitTree::new(28, vec![0], u16::BITS as u8),
            relationship_tree: BitTree::new(28, vec![1], 0),
            reports: UnorderedMap::new(StorageKey::Report),
            drip: Drip::new(),
            roles: UnorderedMap::new("roles".as_bytes())
        };
        this.join();
        let mut permissions = HashSet::new();
        permissions.insert(Permission::AddContent(0));
        permissions.insert(Permission::AddContent(1));
        permissions.insert(Permission::AddContent(2));
        permissions.insert(Permission::DelContent);
        permissions.insert(Permission::AddEncryptContent(0));
        permissions.insert(Permission::AddEncryptContent(1));
        permissions.insert(Permission::AddEncryptContent(2));
        permissions.insert(Permission::DelEncryptContent);
        permissions.insert(Permission::Like);
        permissions.insert(Permission::Unlike);
        permissions.insert(Permission::Report);
        this.roles.insert(&"all".to_string(), &Role { 
            kind: RoleKind::Everyone, 
            permissions:  permissions,
            mod_level: 0,
            override_level: 0
        });
        this
    }

    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let prev: Community = env::state_read().expect("ERR_NOT_INITIALIZED");
        assert_eq!(
            env::predecessor_account_id(),
            prev.owner_id,
            "Only owner"
        );
        
        let this = Community {
            owner_id: prev.owner_id,
            public_key: prev.public_key,
            content_tree: BitTree::new(28, vec![0], u16::BITS as u8),
            relationship_tree: BitTree::new(28, vec![1], 0),
            reports: UnorderedMap::new(b'r'),
            drip: Drip::new(),
            roles: UnorderedMap::new("roles".as_bytes())
        };
        this
    }
    
    #[payable]
    pub fn join(&mut self) {
        let sender_id = env::predecessor_account_id();
        let mut accounts: UnorderedMap<AccountId, Account> = UnorderedMap::new(StorageKey::Account);
        let mut account = accounts.get(&sender_id).unwrap_or_default();
        account.set_registered(true);
        accounts.insert(&sender_id, &account);
    }

    #[payable]
    pub fn quit(&mut self) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let mut accounts: UnorderedMap<AccountId, Account> = UnorderedMap::new(StorageKey::Account);
        let account = accounts.get(&sender_id);
        if let Some(mut account) = account {
            if account.get_drip() == 0 {
                accounts.remove(&sender_id);
            } else {
                account.set_registered(true);
            }
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