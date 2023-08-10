
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::str::FromStr;

use account::Account;
use events::Event;
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base58CryptoHash, U128, U64};
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::serde_json::{json, self, to_string, Value};
use near_sdk::{near_bindgen, AccountId, log, bs58, PanicOnDefault, Promise, BlockHeight, CryptoHash, assert_one_yocto, BorshStorageKey, env, PromiseOrValue, sys};
use near_sdk::collections::{LookupMap, UnorderedMap, Vector, LazyOption, UnorderedSet};
use drip::{Drip};
use proposal::{Proposal, FunctionCall, ActionCall};
use role::{RoleManagement};
use uint::hex;
use utils::{refund_extra_storage_deposit, set, remove, set_storage_usage, get_account, set_account, get_account_id, init_callback};
use crate::access::Relationship;
use crate::post::Hierarchy;
use crate::proposal::ProposalInput;
use crate::role::Role;
use crate::utils::{get_arg, get_access_limit, verify, from_rpc_sig, get_predecessor_id};
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
    proposals: UnorderedMap<String, Proposal>,
    access: AccessLimit
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct OldCommunity {
    owner_id: AccountId,
    args: HashMap<String, String>,
    accounts: LookupMap<AccountId, Account>,
    // content_tree: BitTree,
    // relationship_tree: BitTree,
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
    Registry,
    TokenLimit(Access)
}

/*
args : {
    drip_contract: AccountId,
    open_access: bool   //no join action needed
}
*/
const DRIP_CONTRACT: &str = "drip_contract";
const PREDECESSOR_REGISTER: u64 = std::u64::MAX - 3;


#[near_bindgen]
impl Community {

    #[init]
    pub fn new(owner_id: AccountId, args: HashMap<String, String>) -> Self {
        let mut this = Self {
            owner_id: owner_id.clone(),
            args,
            accounts: LookupMap::new(StorageKey::Account),
            reports: UnorderedMap::new(StorageKey::Report),
            drip: Drip::new(),
            role_management: RoleManagement::new(),
            proposals: UnorderedMap::new(StorageKey::Proposals),
            access: AccessLimit::Registry
        };
        let mut account = this.accounts.get(&owner_id).unwrap_or_default();
        account.set_registered(true);
        account.increase_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), JOIN_DEPOSIT);
        this.accounts.insert(&owner_id, &account);
        let mut account = this.accounts.get(&env::current_account_id()).unwrap_or_default();
        account.set_registered(true);
        this.accounts.insert(&env::current_account_id(), &account);
        this
    }

    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let mut this: Community = env::state_read().expect("ERR_NOT_INITIALIZED");
        assert!(get_predecessor_id() == this.owner_id || get_predecessor_id() == env::current_account_id(), "owner only");
        
        let mut account = this.accounts.get(&env::current_account_id()).unwrap_or_default();
        account.set_registered(true);
        this.accounts.insert(&env::current_account_id(), &account);
        let mut mod_permissions = HashSet::new();
        mod_permissions.insert(Permission::AddContent(0));
        mod_permissions.insert(Permission::AddContent(1));
        mod_permissions.insert(Permission::AddContent(2));
        mod_permissions.insert(Permission::DelContent);
        mod_permissions.insert(Permission::AddEncryptContent(0));
        mod_permissions.insert(Permission::AddEncryptContent(1));
        mod_permissions.insert(Permission::AddEncryptContent(2));
        mod_permissions.insert(Permission::DelEncryptContent);
        mod_permissions.insert(Permission::Like);
        mod_permissions.insert(Permission::Unlike);
        mod_permissions.insert(Permission::Report);
        mod_permissions.insert(Permission::Vote);
        mod_permissions.insert(Permission::AddProposal(false));
        mod_permissions.insert(Permission::AddProposal(true));
        mod_permissions.insert(Permission::ReportConfirm);
        mod_permissions.insert(Permission::DelOthersContent);
        mod_permissions.insert(Permission::SetRole(None));
        mod_permissions.insert(Permission::DelRole(None));
        mod_permissions.insert(Permission::AddMember(None));
        mod_permissions.insert(Permission::RemoveMember(None));
        mod_permissions.insert(Permission::Other(None));
        this.role_management.roles.insert("mod".to_string(), Role { 
            alias: "Mod".to_string(),
            members: "mod_member".to_string().into_bytes(), 
            permissions:  mod_permissions,
            mod_level: 2,
            override_level: 0
        });
        this.role_management.global_role.insert(Permission::Vote, (Relationship::Or, None));
        this.role_management.global_role.insert(Permission::AddProposal(false), (Relationship::Or, None));
        this.role_management.global_role.insert(Permission::AddProposal(true), (Relationship::And, None));
        env::state_write::<Community>(&this);
        this
    }

    pub fn follow(&mut self, account_id: AccountId) {
        init_callback();
        let sender_id = get_predecessor_id();
        let hash = env::sha256(&(sender_id.to_string() + "follwing" + &account_id.to_string()).into_bytes());
        set(&hash, 0);
        Event::log_follow(sender_id, account_id,None);
    }

    pub fn unfollow(&mut self, account_id: AccountId) {
        let sender_id = get_predecessor_id();
        let hash = env::sha256(&(sender_id.to_string() + "follwing" + &account_id.to_string()).into_bytes());
        remove(&hash);
        Event::log_unfollow(sender_id, account_id, None);
    }

    pub fn agree_rules(&mut self) {
        init_callback()
    }
    
    #[payable]
    pub fn join(&mut self, account_id: Option<AccountId>, inviter_id: Option<AccountId>) {
        let initial_storage_usage = env::storage_usage();
        if let AccessLimit::Free = self.access {
            return
        }
        match self.access {
            AccessLimit::Free => {},
            _ => assert!(env::attached_deposit() >= JOIN_DEPOSIT, "not enough deposit")
        }
        
        let sender_id = account_id.unwrap_or(get_predecessor_id());
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
        let sender_id = get_predecessor_id();
        let account = get_account(&sender_id).get_registered();
        if let Some(mut account) = account {
            account.set_registered(false);
            self.accounts.insert(&sender_id, &account);
        }
    }

    #[payable]
    pub fn deposit(&mut self) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = get_predecessor_id();
        let mut account = get_account(&sender_id).registered();
        account.increase_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), env::attached_deposit());
        self.accounts.insert(&sender_id, &account);
        set_storage_usage(initial_storage_usage, None);
    }

    #[payable]
    pub fn withdraw(&mut self, asset: AssetKey, amount: U128) -> PromiseOrValue<()> {
        let sender_id = get_predecessor_id();
        let account = get_account(&sender_id).registered();
        assert!(account.get_balance(&asset).checked_sub(amount.0).is_some(), "not enough balance");
        let result = match &asset {
            AssetKey::FT(token_id) => {
                if token_id.to_string() == "near" {
                    Promise::new(sender_id.clone()).transfer(amount.0).into()
                } else {
                    ext_ft_core::ext(token_id.clone()).with_attached_deposit(1).ft_transfer(sender_id.clone(), amount, None).into()
                }
            },
            AssetKey::NFT(_, _) => PromiseOrValue::Value(()),
            AssetKey::Drip(_) => PromiseOrValue::Value(())
        };
        match result {
            PromiseOrValue::Promise(promise) => promise
                .then(
                    Self::ext(env::current_account_id()).on_withdraw_callback(sender_id, asset, amount)
                ).into(),
            PromiseOrValue::Value(()) => PromiseOrValue::Value(())
        }
    }

    #[payable]
    pub fn donate(&mut self) {
        let mut account = get_account(&env::current_account_id()).registered();
        account.increase_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), env::attached_deposit());
        set_account(&env::current_account_id(), &account);
    }

    #[payable]
    pub fn collect_drip(&mut self) -> U128 {
        assert_one_yocto();
        let sender_id = env::signer_account_id();
        assert!(get_account(&sender_id).is_registered(), "account not found");
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
        let non_near_account_id = get_account_id(id, message, sign, public_key);
        let drips = self.drip.gather_drip(non_near_account_id.clone(), sender_id.clone());

        Event::log_other(
            Some(json!({
                "drips": drips
            }).to_string())
        );
    }

    pub fn decode(&mut self, id: String, public_key: String, action: ActionCall, sign: String, timestamp: U64) -> Option<String> {
        Promise::new(env::signer_account_id()).function_call("on_callback".to_string(), json!({}).to_string().into_bytes(), 0, env::prepaid_gas() / 3);
        let timestamp = u64::from(timestamp);
        assert!(env::block_timestamp() - timestamp < 120_000_000_000, "signature expired");
        let message = (public_key.to_string() + &json!(action).to_string() + &timestamp.to_string()).as_bytes().to_vec();
        let account_id = get_account_id(id, message, sign, public_key);
        let account_id = account_id.try_to_vec().unwrap();
        let args_map = match serde_json::from_str(&action.args).unwrap() {
            Value::Object(map) => map,
            _ => panic!("invalid args")
        };
        unsafe {
            sys::write_register(PREDECESSOR_REGISTER, account_id.len() as u64, account_id.as_ptr() as u64);
        }
        match action.method_name.as_str() {
            "agree_rules" => {
                None
            },
            "add_content" => {
                let args = args_map.get("args").unwrap().to_string();
                let hierarchies = serde_json::from_str::<Vec<Hierarchy>>(&args_map.get("hierarchies").unwrap().to_string()).unwrap();
                let options = match args_map.get("options") {
                    Some(v) => Some(serde_json::from_str::<HashMap<String, String>>(&v.to_string()).unwrap()),
                    None => None
                };
                Some(String::from(&self.add_content(args, hierarchies, options)))
            },
            "like" => {
                let hierarchies = serde_json::from_str::<Vec<Hierarchy>>(&args_map.get("hierarchies").unwrap().to_string()).unwrap();
                self.like(hierarchies);
                None
            },
            // "unlike" => {
            //     let hierarchies = serde_json::from_str::<Vec<Hierarchy>>(&args_map.get("hierarchies").unwrap().to_string()).unwrap();
            //     self.unlike(hierarchies);
            //     None
            // },
            "add_proposal" => {
                let proposal = serde_json::from_str::<ProposalInput>(&args_map.get("proposal").unwrap().to_string()).unwrap();
                Some(self.add_proposal(proposal))
            },
            "vote" => {
                let id = args_map.get("id").unwrap().to_string();
                let vote = serde_json::from_str::<u32>(&args_map.get("vote").unwrap().to_string()).unwrap();
                let amount = serde_json::from_str::<U128>(&args_map.get("amount").unwrap().to_string()).unwrap();
                self.vote(id, vote, amount);
                None
            }
            _ => panic!("not support")
        }
        
    }
}


#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr};

    use near_sdk::{base64, serde_json::{self, json}, borsh::{BorshDeserialize, BorshSerialize}, AccountId, env, json_types::{U64, U128}, log};
    use uint::hex;
    use crate::{Community, OldCommunity, utils::{from_rpc_sig}, proposal::ActionCall};


    #[test]
    pub fn test() {
        let state = base64::decode("EAAAAHBhdmVsZ29kLnRlc3RuZXQBAAAADQAAAGRyaXBfY29udHJhY3QZAAAAdjItZHJpcC5iZWVwb3B1bGEudGVzdG5ldAEAAAABAgAAAABpAAAAAAAAAAACAAAAAGsAAAAAAAAAAAIAAAAAdgEAAAABAgAAAAMAAABiYW4GAAAAQmFubmVkCgAAAGJhbl9tZW1iZXIAAAAAAAAAAGMAAAADAAAAbW9kAwAAAE1vZAoAAABtb2RfbWVtYmVyFQAAAAAAAAEAAgECAAIBAgIDBAUHCAkKAAsADAANAA4ADwAPARACAAAAAAAAABUAAAAAAAAAAAEAAAACAAABAAACAAAAAgEAAAICAAADAAAEAAAFAAAHAAAIAQAJAQAKAAEACwABAAwAAQANAAEADgABAA8AAAAPAQEAEAAAAgAAAANpAAAAAAAAAAACAAAAA2sAAAAAAAAAAAIAAAADdgE=").unwrap();
        let state = OldCommunity::try_from_slice(&state).unwrap();
        // let mut map = HashMap::new();
        // map.insert("drip_contract".to_string(), "v2-drip.beepopula.testnet".to_string());
        // let community = Community::new(AccountId::from_str("filo.testnet").unwrap(), map);
        // let text = community.try_to_vec().unwrap();
        // let text = base64::encode(text);
        println!("{:?}", state.args);
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

    #[test]
    pub fn test_decode() {
        let action = ActionCall {
            method_name: "add_content".to_string(),
            args: "{\"args\":\"{\\\"text\\\":\\\"bbbbbb\\\",\\\"imgs\\\":[]}\",\"hierarchies\":[],\"extra\":{\"at\":\"[]\",\"drip_royalty\":5}}".to_string(),
            deposit: U128::from(0),
            gas: U64::from(0)
        };
        let timestamp = "1690383153326000000".to_string();
        let message = (json!(action).to_string() + &timestamp);
        // let prefix = ("\u{0019}Ethereum Signed Message:\n".to_string() + &message.len().to_string()).as_bytes().to_vec();
        // let hash = hex::encode(env::keccak256(&[prefix, message].concat()));
        log!("{:?}", message)
    }

}