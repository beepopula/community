

use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::str::FromStr;

use account::Account;
// use near_fixed_bit_tree::BitTree;
use events::Event;
use near_fixed_bit_tree::BitTree;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base58CryptoHash, U128, U64};
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::serde_json::{json, self};
use near_sdk::{env, near_bindgen, AccountId, log, bs58, PanicOnDefault, Promise, BlockHeight, CryptoHash, assert_one_yocto, BorshStorageKey};
use near_sdk::collections::{LookupMap, UnorderedMap, Vector, LazyOption, UnorderedSet};
use drip::{Drip};
use role::{RoleManagement};
use utils::{refund_extra_storage_deposit, set, remove, set_storage_usage};
use crate::post::Hierarchy;
use crate::utils::{get_arg, get_access_limit, verify};
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


const JOIN_DEPOSIT: u128 = 50000000000000000000000;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Community {
    owner_id: AccountId,
    args: HashMap<String, String>,
    accounts: LookupMap<AccountId, Account>,
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
    role_management: RoleManagement,
    access: AccessLimit
}


const MAX_LEVEL: usize = 3;

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKey {
    Report,
    Account,
    Roles
}

#[derive(Serialize, Deserialize, Clone)]
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
            // content_tree: BitTree::new(28, vec![0], u16::BITS as u8),
            // relationship_tree: BitTree::new(28, vec![1], 0),
            reports: UnorderedMap::new(StorageKey::Report),
            drip: Drip::new(),
            role_management: RoleManagement::new(),
            access: AccessLimit::Registy
        };
        let mut account = this.accounts.get(&owner_id).unwrap_or_default();
        account.set_registered(true);
        account.increase_deposit(Deposit::FT(AccountId::from_str("near").unwrap()), JOIN_DEPOSIT);
        this.accounts.insert(&owner_id, &account);
        this
    }

    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let prev: OldCommunity = env::state_read().expect("ERR_NOT_INITIALIZED");
        assert!(env::predecessor_account_id() == prev.owner_id || env::predecessor_account_id() == env::current_account_id(), "owner only");
        
        let this = Community {
            owner_id: prev.owner_id,
            args: prev.args,
            accounts: prev.accounts,
            reports: prev.reports,
            drip: prev.drip,
            role_management: prev.role_management,
            access: prev.access
        };
        env::state_write::<Community>(&this);
        this
    }

    pub fn follow(&mut self, account_id: AccountId) {
        let sender_id = env::predecessor_account_id();
        let hash = env::sha256(&(sender_id.to_string() + "follwing" + &account_id.to_string()).into_bytes());
        set(&hash, 0);
        Event::log_follow(sender_id, account_id,None);
    }

    pub fn unfollow(&mut self, account_id: AccountId) {
        let sender_id = env::predecessor_account_id();
        let hash = env::sha256(&(sender_id.to_string() + "follwing" + &account_id.to_string()).into_bytes());
        remove(&hash);
        Event::log_unfollow(sender_id, account_id, None);
    }
    
    #[payable]
    pub fn join(&mut self, inviter_id: Option<AccountId>) {
        let initial_storage_usage = env::storage_usage();
        if let AccessLimit::Free = self.access {
            return
        }
        match self.access {
            AccessLimit::Free => {},
            _ => assert!(env::attached_deposit() >= JOIN_DEPOSIT, "not enough deposit")
        }
        
        let sender_id = env::predecessor_account_id();
        let mut account = match self.accounts.get(&sender_id) {
            Some(mut account)=> {
                account.set_registered(true);
                account
            },
            None => {
                let mut account = Account::default();
                account.set_registered(true);
                if let Some(inviter_id) = inviter_id {
                    let drips = self.drip.set_invite_drip(inviter_id.clone(), sender_id.clone());
                    Event::log_invite(
                        inviter_id, 
                        sender_id.clone(), 
                        Some(json!({
                            "drips": drips
                        }).to_string())
                    )
                }
                account
            }
        };
        account.increase_deposit(Deposit::FT(AccountId::from_str("near").unwrap()), env::attached_deposit());
        self.accounts.insert(&sender_id, &account);
        set_storage_usage(initial_storage_usage, None);
        

    }

    #[payable]
    pub fn quit(&mut self) {
        if let AccessLimit::Free = self.access {
            return
        }
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let account = self.accounts.get(&sender_id);
        if let Some(mut account) = account {
            account.set_registered(false);
            self.accounts.insert(&sender_id, &account);
        }
    }

    #[payable]
    pub fn deposit(&mut self) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        let mut account = self.accounts.get(&sender_id).unwrap();
        account.increase_deposit(Deposit::FT(AccountId::from_str("near").unwrap()), env::attached_deposit());
        self.accounts.insert(&sender_id, &account);
        set_storage_usage(initial_storage_usage, None);
    }

    pub fn withdraw(&mut self, amount: U128) {
        let sender_id = env::predecessor_account_id();
        let mut account = self.accounts.get(&sender_id).unwrap_or_default();
        account.decrease_deposit(Deposit::FT(AccountId::from_str("near").unwrap()), amount.0);
        self.accounts.insert(&sender_id, &account);
        Promise::new(sender_id).transfer(amount.0);
    }

    #[payable]
    pub fn collect_drip(&mut self) -> U128 {
        assert_one_yocto();
        let sender_id = env::signer_account_id();
        assert!(self.accounts.get(&sender_id).is_some(), "account not found");
        self.drip.get_and_clear_drip(sender_id)
    }

    #[payable]
    pub fn collect_drip_from_non_near_account(&mut self, account_id: AccountId, sign: String, timestamp: U64) -> U128 {
        assert_one_yocto();
        assert!(self.accounts.get(&account_id).is_some(), "account not found");
        let timestamp = u64::from(timestamp);
        assert!(env::block_timestamp() - timestamp < 120_000_000_000, "signature expired");
        let sender_id = env::signer_account_id();
        let message = (account_id.to_string() + &sender_id.to_string() + &timestamp.to_string()).as_bytes().to_vec();
        let sign = bs58::decode(sign).into_vec().unwrap();
        let pk = account_id.as_bytes().to_vec();
        assert!(verify(message, sign, pk), "not verified");
        self.drip.get_and_clear_drip(account_id)
    }
}





#[cfg(test)]
mod tests {


}