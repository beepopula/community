
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
use near_sdk::serde_json::{json, self, to_string};
use near_sdk::{near_bindgen, AccountId, log, bs58, PanicOnDefault, Promise, BlockHeight, CryptoHash, assert_one_yocto, BorshStorageKey, env};
use near_sdk::collections::{LookupMap, UnorderedMap, Vector, LazyOption, UnorderedSet};
use drip::{Drip};
use role::{RoleManagement};
use uint::hex;
use utils::{refund_extra_storage_deposit, set, remove, set_storage_usage};
use crate::post::Hierarchy;
use crate::utils::{get_arg, get_access_limit, verify, from_rpc_sig};
use std::convert::TryFrom;
use role::Permission;
use access::Access;
use account::AssetKey;


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
pub mod proposal;


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
    Roles,
    Proposals
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
        account.increase_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), JOIN_DEPOSIT);
        let mut account = this.accounts.get(&env::current_account_id()).unwrap_or_default();
        account.set_registered(true);
        account.increase_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), u128::MAX);
        this.accounts.insert(&owner_id, &account);
        this
    }

    // #[init(ignore_state)]
    // pub fn migrate() -> Self {
    //     let prev: OldCommunity = env::state_read().expect("ERR_NOT_INITIALIZED");
    //     assert!(env::predecessor_account_id() == prev.owner_id || env::predecessor_account_id() == env::current_account_id(), "owner only");
        
    //     let this = Community {
    //         owner_id: prev.owner_id,
    //         args: prev.args,
    //         accounts: prev.accounts,
    //         reports: prev.reports,
    //         drip: prev.drip,
    //         role_management: prev.role_management,
    //         access: prev.access
    //     };
    //     env::state_write::<Community>(&this);
    //     this
    // }

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
        account.increase_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), env::attached_deposit());
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
        account.increase_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), env::attached_deposit());
        self.accounts.insert(&sender_id, &account);
        set_storage_usage(initial_storage_usage, None);
    }

    pub fn withdraw(&mut self, amount: U128) {
        let sender_id = env::predecessor_account_id();
        let mut account = self.accounts.get(&sender_id).unwrap_or_default();
        account.decrease_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), amount.0);
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
    // #[cfg(feature = "unstable")]
    pub fn gather_drip_from_non_near_account(&mut self, id: String, public_key: String, sign: String, timestamp: U64) {
        assert_one_yocto();
        let timestamp = u64::from(timestamp);
        assert!(env::block_timestamp() - timestamp < 120_000_000_000, "signature expired");
        let sender_id = env::signer_account_id();
        let message = (sender_id.to_string() + &timestamp.to_string()).as_bytes().to_vec();
        let non_near_account_id = match id.as_str() {
            "eth" => {
                let prefix = ("\u{0019}Ethereum Signed Message:\n".to_string() + &message.len().to_string()).as_bytes().to_vec();
                let hash = env::keccak256(&[prefix, message].concat());
                let sign = hex::decode(sign).unwrap();
                let (sign, v) = from_rpc_sig(&sign);
                
                let public_key = env::ecrecover(&hash, &sign, v, false).unwrap();
                let address = "0x".to_string() + &hex::encode(env::keccak256(&public_key.to_vec())[12..].to_vec());
                AccountId::from_str(&address).unwrap()
            },
            _ => {
                let sign = bs58::decode(sign).into_vec().unwrap();
                let pk = hex::decode(public_key.clone()).unwrap();
                assert!(verify(message, sign, pk), "not verified");
                AccountId::from_str(&public_key).unwrap()
            }
        };
        let drips = self.drip.gather_drip(non_near_account_id.clone(), sender_id.clone());

        Event::log_other(
            Some(json!({
                "drips": drips
            }).to_string())
        );
    }
}


#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr};

    use near_sdk::{base64, serde_json, borsh::{BorshDeserialize, BorshSerialize}, AccountId, env, json_types::U64};
    use uint::hex;
    use crate::{Community, OldCommunity, utils::{from_rpc_sig}};


    #[test]
    pub fn test() {
        let state = base64::decode("DAAAAGZpbG8udGVzdG5ldAIAAAANAAAAZHJpcF9jb250cmFjdBYAAABkcmlwLmJlZXBvcHVsYS50ZXN0bmV0CgAAAHB1YmxpY19rZXksAAAARllMVjV6dFNlMkZqblJUQ0JIZ0UxSFVvQWlvSHVRQmF2Y0VLU2g3b0RxOWQBAAAAAQIAAAAAaQIAAAAAAAAAAgAAAABrAgAAAAAAAAACAAAAAHYBAAAAAQQAAAAsAAAAN1gxWFp2emdkRjI4V2Rib1F0N1FIc1VBMWVTOWRFTVd6dnk4YktMeGNlb20IAAAAQ3VyYXRvcnMzAAAAN1gxWFp2emdkRjI4V2Rib1F0N1FIc1VBMWVTOWRFTVd6dnk4YktMeGNlb21fbWVtYmVyBAAAAAQFBgcAAAAAAAAAACwAAABCazhGWUhXcWJ2aEdERndGdGNDWFduVDRMcGFwY3l3TjFLZXQ0aWdvSlJIZgYAAABBZG1pbnMzAAAAQms4RllIV3FidmhHREZ3RnRjQ1hXblQ0THBhcGN5d04xS2V0NGlnb0pSSGZfbWVtYmVyCgAAAAQFBgcICQwBAwAAAGJhbg0BAwAAAGJhbg4BCQAAAE1hbmFnZVBpbg4BCwAAAE1hbmFnZVJ1bGVzAQAAAAAAAAAsAAAAR2dXYlZ1WUp2ZXRvZm5vWHdBQVh1SmdVREIxUHAycTZEVjZNVFRCNUxoM2EdAAAATkZUIFBhcmlzIE1hcmtldGluZyBDb21taXR0ZWUzAAAAR2dXYlZ1WUp2ZXRvZm5vWHdBQVh1SmdVREIxUHAycTZEVjZNVFRCNUxoM2FfbWVtYmVyBAAAAAQFBgcAAAAAAAAAAAMAAABiYW4GAAAAQmFubmVkCgAAAGJhbl9tZW1iZXIAAAAAAAAAAGMAAAATAAAAAAAAAQEAAAACFgAAAGRyaXAuYmVlcG9wdWxhLnRlc3RuZXQrAAAAcmVuZ2F1bm9mZmljaWFsLmNvbW11bml0eS5iZWVwb3B1bGEudGVzdG5ldAAAAKHtzM4bwtMAAAAAAAAAAAEAAAACAAABAAACAAABAQAAAAIWAAAAZHJpcC5iZWVwb3B1bGEudGVzdG5ldCsAAAByZW5nYXVub2ZmaWNpYWwuY29tbXVuaXR5LmJlZXBvcHVsYS50ZXN0bmV0AAAAoe3MzhvC0wAAAAAAAAACAQAAAgIAAAMAAAQAAAUAAAYAAAcAAAgBAAkBAAoAAQALAAEADAABAA0AAQAOAAEAAQ==").unwrap();
        let state = OldCommunity::try_from_slice(&state).unwrap();
        let mut map = HashMap::new();
        map.insert("drip_contract".to_string(), "v2-drip.beepopula.testnet".to_string());
        let community = Community::new(AccountId::from_str("filo.testnet").unwrap(), map);
        let text = community.try_to_vec().unwrap();
        let text = base64::encode(text);
        println!("{:?}", text);
    }

    #[test]
    pub fn test_address() {
        let public_key = "59b42ef8f3b1deb16ddb61f82a1e536a02cb887c38024bb57dbe3852a88c691a6effd3571a2afb98d68ceae968e0bf21da5e1cdb510f807982f8a3d5b7e8175f";
        let address = hex::encode(env::keccak256(&hex::decode(public_key).unwrap())[12..].to_vec());
        println!("{:?}, {:?}", address, address.len())
    }

    #[test]
    pub fn test_ecrecover() {
        use near_sdk::test_utils::test_env;

        test_env::setup_free();
        let sign = "0220991b4ce76c195432033e2198dde50a6788cba37aa29c5be2afcb4ff4d8c2d41b9d636b7353b2877b59775f5c3cf04514fc0dabf76eb0dbadfc25d89c59e4".to_string();
        let message = ("kinkrit.testnet").as_bytes().to_vec();
        let prefix = ("\u{0019}Ethereum Signed Message:\n".to_string() + &message.len().to_string()).as_bytes().to_vec();
        let hash = env::keccak256(&[prefix, message].concat());
        println!("{:?}", hash);
        let sign = hex::decode(sign).unwrap();
        
        let (sign, v) = from_rpc_sig(&sign);
        println!("{:?}, {}", sign, v);
        let public_key = env::ecrecover(&hash, &sign, v, false).unwrap();
        println!("{:?}", public_key);
        let account_id = "0x".to_string() + &hex::encode(env::keccak256(&public_key.to_vec())[12..].to_vec());
        println!("{:?}", account_id);
        // let address = to_checksum_address(account_id);
        // println!("{:?}", address)
        // AccountId::from_str(&().unwrap();
    }

}