
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::str::FromStr;

use account::{Account, OldAccess};
use events::Event;
use internal::Instruction;
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base58CryptoHash, U128, U64};
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::serde_json::{json, self, to_string, Value};
use near_sdk::{near_bindgen, AccountId, log, bs58, PanicOnDefault, Promise, BlockHeight, CryptoHash, assert_one_yocto, BorshStorageKey, env, PromiseOrValue, sys, PromiseResult, Gas};
use near_sdk::collections::{LookupMap, UnorderedMap, Vector, LazyOption, UnorderedSet};
use drip::{Drip, PendingDrip};
use proposal::{Proposal, FunctionCall, ActionCall};
use role::{RoleManagement, OldRoleManagement};
use uint::hex;
use utils::{refund_extra_storage_deposit, set, remove, set_storage_usage, get_account, set_account, get_account_id, init_callback};
use crate::post::Hierarchy;
use crate::proposal::ProposalInput;
use crate::role::Role;
use crate::utils::{get_arg, get_access_limit, verify, from_rpc_sig, get_predecessor_id, get_root_id};
use std::convert::TryFrom;
use role::Permission;
use account::AssetKey;
use account::{Access, Relationship};


pub mod utils;
pub mod post;
pub mod owner;
pub mod drip;
pub mod view;
pub mod events;
pub mod role;
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
    reports: UnorderedMap<Base58CryptoHash, HashSet<AccountId>>,
    drip: Drip,
    role_management: OldRoleManagement,
    proposals: UnorderedMap<String, Proposal>,
    access: OldAccessLimit
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

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[derive(BorshDeserialize, BorshSerialize, BorshStorageKey)]
pub enum OldAccessLimit {
    Free,
    Registry,
    TokenLimit(OldAccess)
}

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
        let mut account = Account::new(&owner_id);
        account.set_registered(true);
        account.set_timestamp(None, u64::MAX.into());
        account.increase_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), JOIN_DEPOSIT);
        this.accounts.insert(&owner_id, &account);
        let mut account = Account::new(&env::current_account_id());
        account.set_registered(true);
        account.set_timestamp(None, u64::MAX.into());
        this.accounts.insert(&env::current_account_id(), &account);
        this
    }

    // #[init(ignore_state)]
    // pub fn migrate() -> Self {

    //     let old_this: OldCommunity = env::state_read().expect("ERR_NOT_INITIALIZED");
    //     assert!(get_predecessor_id() == old_this.owner_id || get_predecessor_id() == env::current_account_id(), "owner only");
        
    //     let mut global_permissions = HashMap::new();
    //     global_permissions.insert(Permission::AddContent(0), (Relationship::Or, None));
    //     global_permissions.insert(Permission::AddContent(1), (Relationship::Or, None));
    //     global_permissions.insert(Permission::AddContent(2), (Relationship::Or, None));
    //     global_permissions.insert(Permission::DelContent, (Relationship::Or, None));
    //     global_permissions.insert(Permission::AddEncryptContent(0), (Relationship::Or, None));
    //     global_permissions.insert(Permission::AddEncryptContent(1), (Relationship::Or, None));
    //     global_permissions.insert(Permission::AddEncryptContent(2), (Relationship::Or, None));
    //     global_permissions.insert(Permission::DelEncryptContent, (Relationship::Or, None));
    //     global_permissions.insert(Permission::Like, (Relationship::Or, None));
    //     global_permissions.insert(Permission::Unlike, (Relationship::Or, None));
    //     global_permissions.insert(Permission::Report, (Relationship::Or, None));
    //     global_permissions.insert(Permission::Vote, (Relationship::Or, None));
    //     global_permissions.insert(Permission::AddProposal(false), (Relationship::Or, None));

    //     global_permissions.insert(Permission::AddProposal(true), (Relationship::And, None));
    //     global_permissions.insert(Permission::ReportConfirm, (Relationship::And, None));
    //     global_permissions.insert(Permission::DelOthersContent, (Relationship::And, None));
    //     global_permissions.insert(Permission::SetRole(None), (Relationship::And, None));
    //     global_permissions.insert(Permission::DelRole(None), (Relationship::And, None));
    //     global_permissions.insert(Permission::AddMember(None), (Relationship::And, None));
    //     global_permissions.insert(Permission::RemoveMember(None), (Relationship::And, None));
    //     global_permissions.insert(Permission::Other(None), (Relationship::And, None));

    //     let this = Community {
    //         owner_id: old_this.owner_id,
    //         args: old_this.args,
    //         accounts: old_this.accounts,
    //         reports: old_this.reports,
    //         drip: old_this.drip,
    //         role_management: RoleManagement {
    //             roles: old_this.role_management.roles,
    //             global_role: global_permissions
    //         },
    //         proposals: old_this.proposals,
    //         access: match old_this.access {
    //             OldAccessLimit::TokenLimit(_) => AccessLimit::Registry,
    //             OldAccessLimit::Registry => AccessLimit::Registry,
    //             OldAccessLimit::Free => AccessLimit::Free
    //         }
    //     };
    //     env::state_write::<Community>(&this);
    //     this
    // }

    #[payable]
    pub fn set_access_limit(&mut self, access: AccessLimit) {
        assert_one_yocto();
        self.can_execute_action(None, None, Permission::SetRole(None));
        self.access = access;
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
    pub fn join(&mut self, account_id: Option<AccountId>, inviter_id: Option<AccountId>, options: Option<HashMap<String, String>>) {
        init_callback();
        if let AccessLimit::Free = self.access {
            return
        }

        let initial_storage_usage = env::storage_usage();
        let sender_id = account_id.unwrap_or(get_predecessor_id());
        
        let mut account = match self.accounts.get(&sender_id) {
            Some(account)=> {
                account
            },
            None => {
                let account = Account::new(&sender_id);
                if let Some(inviter_id) = inviter_id {
                    let pending_drips = self.drip.add_pending_drip(inviter_id, "invite".to_string(), sender_id.to_string(), PendingDrip::Draw(vec![10,11,12,13,14,15,20]));
                    Event::log_other(
                        Some(json!({
                            "pending_drips": pending_drips
                        }).to_string())
                    );
                }
                account
            }
        };
        // let near_deposit = account.get_balance(&AssetKey::FT(AccountId::from_str("near").unwrap()));
        // if near_deposit == 0 {
        //     // assert!(env::attached_deposit() >= JOIN_DEPOSIT, "not enough deposit");
        // }
        account.increase_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), env::attached_deposit());
        
        if self.can_execute_action(None, None, Permission::SetRole(None)) {
            account.set_registered(true);
            let options = options.unwrap();
            let timestamp: U64 = u64::from_str(options.get("timestamp").unwrap()).unwrap().into();
            account.set_timestamp(None, timestamp);
            // account.set_permanent(true);
        } else {
            match self.access.clone() {
                AccessLimit::Free => {assert!(options.is_none(), "options are not allowed")},
                AccessLimit::Registry => {
                    assert!(options.is_none(), "options are not allowed");
                    account.set_registered(true)
                },
                AccessLimit::TokenLimit(access) => {
                    assert!(account.set_condition(&access, options), "not allowed");
                    account.set_registered(true)
                }
            }
        }
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
    pub fn deposit(&mut self, account_id: Option<AccountId>) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = account_id.unwrap_or(get_predecessor_id());
        let mut account = get_account(&sender_id);
        account.increase_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), env::attached_deposit());
        self.accounts.insert(&sender_id, &account);
        set_storage_usage(initial_storage_usage, None);
    }

    #[payable]
    pub fn withdraw(&mut self, asset: AssetKey, amount: U128) -> PromiseOrValue<()> {
        let initial_storage_usage = env::storage_usage();
        let sender_id = get_predecessor_id();
        let mut account = get_account(&sender_id).registered();
        assert!(account.get_balance(&asset).checked_sub(amount.0).is_some(), "not enough balance");
        let result = match &asset {
            AssetKey::FT(token_id) => {
                account.decrease_balance(asset.clone(), amount.0);
                set_account(&account);
                if token_id.to_string() == "near" {
                    Promise::new(sender_id.clone()).transfer(amount.0).into()
                } else {
                    ext_ft_core::ext(token_id.clone()).with_attached_deposit(1).ft_transfer(sender_id.clone(), amount, None).into()
                }
            },
            AssetKey::NFT(_, _) => PromiseOrValue::Value(()),
            AssetKey::Drip(_) => PromiseOrValue::Value(())
        };
        set_storage_usage(initial_storage_usage, None);
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
        let initial_storage_usage = env::storage_usage();
        let mut account = get_account(&env::current_account_id()).registered();
        account.increase_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), env::attached_deposit());
        set_account(&account);
        set_storage_usage(initial_storage_usage, None);
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
        let initial_storage_usage = env::storage_usage();
        let timestamp = u64::from(timestamp);
        assert!(env::block_timestamp() - timestamp < 120_000_000_000, "signature expired");
        let sender_id = env::signer_account_id();
        let message = (sender_id.to_string() + &timestamp.to_string()).as_bytes().to_vec();
        let non_near_account_id = get_account_id(id, message, sign, public_key);
        let drips = self.drip.gather_drip(non_near_account_id.clone(), sender_id.clone());
        set_storage_usage(initial_storage_usage, None);
        Event::log_other(
            Some(json!({
                "drips": drips
            }).to_string())
        );
    }

    pub fn resolve_pending_drip(&mut self, reason: String, option: String) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = get_predecessor_id();
        let drips = self.drip.set_pending_drip(sender_id, reason, option);
        set_storage_usage(initial_storage_usage, None);
        Event::log_other(
            Some(json!({
                "drips": drips
            }).to_string())
        )
    }

    // pub fn stake(&mut self, contract_id: AccountId) {
    //     let sender_id = get_predecessor_id();
    //     let mut instructions = vec![];
    //     for account_id in vec![sender_id, env::current_account_id()] {
    //         let account = get_account(&account_id);
    //         let staking = account.get_data(&get_predecessor_id().to_string()).unwrap_or(HashMap::new());
    //         let mut staking = Account::from_data(staking);
    //         let reward = staking.get_data("reward").unwrap_or(HashMap::new());
    //         staking.increase_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), env::attached_deposit());
            
    //         instructions.push(Instruction::Write((account_id, staking.data)));
    //     }
    //     Promise::new(contract_id).function_call("stake".to_string(), json!({}).to_string().as_bytes().to_vec(), env::attached_deposit(), env::prepaid_gas() / 3).then(
    //         Community::ext(env::current_account_id()).on_call(instructions)
    //     );
    // }

    // pub fn unstake(&mut self, contract_id: AccountId) {
    //     let sender_id = get_predecessor_id();
    //     let mut instructions = vec![];
    //     for account_id in vec![env::current_account_id(), sender_id] {
    //         let account = get_account(&account_id);
    //         let staking = account.get_data(&get_predecessor_id().to_string()).unwrap_or(HashMap::new());
    //         let mut staking = Account::from_data(staking);
    //         staking.decrease_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), env::attached_deposit());
    //         instructions.push(Instruction::Write((account_id, staking.data)));
    //     }
    //     Promise::new(contract_id).function_call("unstake".to_string(), json!({}).to_string().as_bytes().to_vec(), env::attached_deposit(), env::prepaid_gas() / 3).then(
    //         Community::ext(env::current_account_id()).on_call(instructions)
    //     );
    // }

    // pub fn reward(&mut self, contract_id: AccountId) {
        
    // }

    pub fn call(&mut self, to: AccountId, method_name: String, args: String, read_accounts: Vec<(AccountId, Vec<String>)>, permissions: Vec<Permission>, deposit: Option<U128>, gas: Option<U64>) {    //read_account:  account, fields
        assert!(get_root_id(&to) == get_root_id(&env::current_account_id()), "not allowed");
        let sender_id = get_predecessor_id();
        let mut permitteds = HashSet::new();
        for permission in permissions.clone() {
            if self.can_execute_action(None, None, permission.clone()) {
                permitteds.insert(permission);
            }
        }
        
        let mut accounts = HashMap::new();
        for (account_id, fields)in read_accounts {
            let account = get_account(&account_id);
            let mut data: HashMap<String, String> = account.get_data(&to.to_string()).unwrap_or_default();
            for field in fields {
                match account.get_data::<String>(&field) {
                    Some(v) => { data.insert(field, v); }, 
                    None => continue
                }
            }
            accounts.insert(account_id, json!(data).to_string());
        }
        let deposit = match deposit {
            Some(v) => {
                let mut account = get_account(&env::signer_account_id());
                account.decrease_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), v.0);
                set_account(&account);
                v.0
            },
            None => 0
        };
        let gas = match gas {
            Some(v) => Gas::from(v.0),
            None => env::prepaid_gas() / 3
        };
        
        Promise::new(to).function_call(method_name, json!({
            "args": args,
            "sender_id": sender_id,
            "accounts": accounts,
            "permissions": permitteds
        }).to_string().into_bytes(), deposit, gas).then(
            Community::ext(env::current_account_id()).on_call(deposit.into())
        );
    }

    #[private]
    pub fn on_call(&mut self, amount: U128) {
        
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(instructions) => {
                let initial_storage_usage = env::storage_usage();
                let instructions = serde_json::from_slice(&instructions).unwrap();
                self.internal_execute_instructions(instructions);
                set_storage_usage(initial_storage_usage, None);
                PromiseOrValue::Value(())
            },
            PromiseResult::Failed => {
                let mut account = get_account(&env::signer_account_id());
                account.increase_balance(AssetKey::FT(AccountId::from_str("near").unwrap()), amount.0);
                set_account(&account);
                PromiseOrValue::Value(())
            }
        };
    }



    // pub fn decode(&mut self, id: String, public_key: String, action: ActionCall, sign: String, timestamp: U64) -> Option<String> {
    //     Promise::new(env::signer_account_id()).function_call("on_callback".to_string(), json!({}).to_string().into_bytes(), 0, env::prepaid_gas() / 3);
    //     let timestamp = u64::from(timestamp);
    //     assert!(env::block_timestamp() - timestamp < 120_000_000_000, "signature expired");
    //     let message = (public_key.to_string() + &json!(action).to_string() + &timestamp.to_string()).as_bytes().to_vec();
    //     let account_id = get_account_id(id, message, sign, public_key);
    //     let register_account_id = account_id.try_to_vec().unwrap();
    //     let args_map = match serde_json::from_str(&action.args).unwrap() {
    //         Value::Object(map) => map,
    //         _ => panic!("invalid args")
    //     };
    //     unsafe {
    //         sys::write_register(PREDECESSOR_REGISTER, register_account_id.len() as u64, register_account_id.as_ptr() as u64);
    //     }
    //     match action.method_name.as_str() {
    //         "agree_rules" => {
    //             None
    //         },
    //         "add_content" => {
    //             let args = args_map.get("args").unwrap().to_string();
    //             let hierarchies = serde_json::from_str::<Vec<Hierarchy>>(&args_map.get("hierarchies").unwrap().to_string()).unwrap();
    //             let options = match args_map.get("options") {
    //                 Some(v) => Some(serde_json::from_str::<HashMap<String, String>>(&v.to_string()).unwrap()),
    //                 None => None
    //             };
    //             Some(String::from(&self.add_content(args, hierarchies, options)))
    //         },
    //         "like" => {
    //             let hierarchies = serde_json::from_str::<Vec<Hierarchy>>(&args_map.get("hierarchies").unwrap().to_string()).unwrap();
    //             self.like(hierarchies);
    //             None
    //         },
    //         // "unlike" => {
    //         //     let hierarchies = serde_json::from_str::<Vec<Hierarchy>>(&args_map.get("hierarchies").unwrap().to_string()).unwrap();
    //         //     self.unlike(hierarchies);
    //         //     None
    //         // },
    //         "add_proposal" => {
    //             let proposal = serde_json::from_str::<ProposalInput>(&args_map.get("proposal").unwrap().to_string()).unwrap();
    //             Some(self.add_proposal(proposal))
    //         },
    //         "vote" => {
    //             let id = args_map.get("id").unwrap().to_string();
    //             let vote = serde_json::from_str::<u32>(&args_map.get("vote").unwrap().to_string()).unwrap();
    //             let amount = serde_json::from_str::<U128>(&args_map.get("amount").unwrap().to_string()).unwrap();
    //             self.vote(id, vote, amount);
    //             None
    //         },
    //         _ => panic!("not support")
    //     }
        
    // }
}


#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr};

    use near_sdk::{base64, serde_json::{self, json}, borsh::{BorshDeserialize, BorshSerialize}, AccountId, env, json_types::{U64, U128}, log, bs58};
    use uint::hex;
    use crate::{Community, OldCommunity, utils::{from_rpc_sig}, proposal::ActionCall, account::{Access, SignCondition, Condition, Account}, drip::U256};


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

    #[test]
    pub fn test_join() {
        let mut options = HashMap::new();
        options.insert("sign".to_string(), "fdb0c81e771183de4ba7477a88f1bceda6c98744bcbad534763acde45a23495376b9908ec70bc38cc66cf52efb32f38c796c3a7714c411efa8d915d0812c65a71c".to_string());
        options.insert("timestamp".to_string(), "1693994267043000000".to_string());
        let access = Access {
            condition: Condition::SignCondition( SignCondition{
                message: " has W0rdl3 #1".to_string(),
                public_key: "0x313043dbb2679ec57f83a46d6675bca8d2cc9c109bc82a2160f86ece7eb6a4d972aaa43af2d38db68ed2d480ddc29d917294d40b29f92d7418884700bea361e2".to_string()
            }),
            expire_duration: Some(U64::from(86400000000).into()),
            is_payment: false,
            options: Some(HashMap::new())
        };
        let mut account = Account::new(&AccountId::from_str("bhc13.testnet").unwrap());
        let pass = account.set_condition(&access, Some(options));
        println!("{:?}", pass)
    }

    #[test]
    pub fn test_hashing() {
        let hash = bs58::decode("QmZVcKc16Xuyv8SfWfPaULGZDaxxvBaxUiAa5rLQQz8Eg3").into_vec().unwrap();
        let hash = hex::encode(hash);
        // let hash = env::keccak256(&message);
        
        // let hash = U256::from_big_endian(&hash);
        println!("{:?}", hash)
    }

}