

use std::convert::TryFrom;

use ed25519_dalek::Verifier;
use near_sdk::{Balance, StorageUsage, Promise, log};

use crate::*;


pub(crate) fn refund_extra_storage_deposit(storage_used: StorageUsage, used_balance: Balance) {
    let required_cost = env::storage_byte_cost() * Balance::from(storage_used);
    let attached_deposit = env::attached_deposit()
        .checked_sub(used_balance)
        .expect("not enough attached balance");

    assert!(
        required_cost <= attached_deposit,
        "not enough attached balance {}",
        required_cost,
    );

    let refund = attached_deposit - required_cost;
    if refund > 1 {
        Promise::new(env::predecessor_account_id()).transfer(refund);
    }
}

pub(crate) fn verify(message: Vec<u8>, sign: Vec<u8>, pk: Vec<u8>) {
    let pk = ed25519_dalek::PublicKey::from_bytes(&pk).unwrap();
    if sign.len() != 64 {
        panic!("Invalid signature data length.");
    }
    let mut sig_data: [u8; 64] = [0; 64];
    for i in 0..64 {
        sig_data[i] = sign.get(i).unwrap_or(&0).clone();
    }
    let sign = ed25519_dalek::Signature::try_from(sig_data).unwrap();
    match pk.verify(&message, &sign) {
        Ok(_) => log!("verify ok"),
        Err(_) => panic!("verify error"),
    }
}

pub(crate) fn getSeed() -> String {
    "seed should be replaced here".to_string()
}

pub(crate) fn get_parent_contract_id(contract_id: AccountId) -> AccountId {
    let current_id = contract_id.to_string();
    let index = current_id.find('.').unwrap();
    let parent_id = current_id[index + 1..].to_string();
    AccountId::try_from(parent_id).unwrap()
}

pub(crate) fn get_root_id(contract_id: AccountId) -> AccountId {
    let contract_id = contract_id.to_string();
    //let index = contract_id.find('.').unwrap();
    let arr: Vec<String> = contract_id.split('.').map(|v| v.to_string()).collect();
    //let parent_id = contract_id[index + 1..].to_string();
    let root_id = arr.get(arr.len() - 2).unwrap().clone() + "." + arr.get(arr.len() - 1).unwrap();
    AccountId::try_from(root_id).unwrap()
}


pub(crate) fn get_content_hash(hierarchies: Vec<Hierarchy>, extra: Option<String>, tree: &BitTree) -> Option<String> {
    let mut hash_prefix = "".to_string();
    for (_, hierarchy) in hierarchies.iter().enumerate() {
        let mut hierarchy_str = hash_prefix + &hierarchy.account_id.to_string() + &String::from(&hierarchy.target_hash);
        if let Some(options) = hierarchy.options.clone() {
            hierarchy_str += &json!(options).to_string();
        }
        if let Some(extra) = extra.clone() {
            hierarchy_str += &extra;
        }
        let hierarchy_hash = env::sha256(&hierarchy_str.into_bytes());
        if !tree.check(&hierarchy_hash) {
            return None
        }
        let hierarchy_hash: [u8;32] = hierarchy_hash[..].try_into().unwrap();
        hash_prefix = String::from(&Base58CryptoHash::from(hierarchy_hash));
    }
    Some(hash_prefix)
}

pub(crate) fn set_content(args: String, account_id: AccountId, hash_prefix: String, options:Option<HashMap<String, String>>, extra: Option<String>, tree: &mut BitTree) -> Base58CryptoHash {
    let args = args.clone() + &bs58::encode(env::random_seed()).into_string();
    let target_hash = env::sha256(&args.clone().into_bytes());
    let target_hash: [u8;32] = target_hash[..].try_into().unwrap();

    let mut hierarchy_str = hash_prefix + &account_id.to_string() + &String::from(&Base58CryptoHash::from(target_hash));
    if let Some(options) = options {
        hierarchy_str += &json!(options).to_string();
    }
    if let Some(extra) = extra {
        hierarchy_str += &extra;
    }    

    let hash = env::sha256(&hierarchy_str.into_bytes());
    //let hash: CryptoHash = hash[..].try_into().unwrap();
    
    tree.set(&hash, 0);
    Base58CryptoHash::from(target_hash)
}

pub(crate) fn is_registered(account_id: &AccountId) -> bool {
    let accounts: LookupMap<AccountId, Account> = LookupMap::new(StorageKey::Account);
    match accounts.get(&account_id) {
        Some(v) => v.is_registered(),
        None => false
    }
}

pub(crate) fn get_arg<T>(key: &str) -> Option<T> 
where T: std::str::FromStr
{
    let this: Community = env::state_read().unwrap();
    let value = match this.args.get(key) {
        Some(v) => v,
        None => return None
    };
    match T::from_str(value) {
        Ok(res) => Some(res),
        Err(_) => None
    }
}

pub(crate) fn get_access_limit() -> AccessLimit {
    let this: Community = env::state_read().unwrap();
    this.access
}
