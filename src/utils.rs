

use std::convert::TryFrom;

use near_sdk::{Balance, StorageUsage, Promise, log};

use crate::*;
use crate::signature::ed25519::{PublicKey, Signature};


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
    let pk = PublicKey::from_slice(&pk).unwrap();
    let sign = Signature::from_slice(&sign).unwrap();
    match pk.verify(message, &sign) {
        Ok(_) => log!("verify ok"),
        Err(_) => panic!("verify error")
    }
}

pub(crate) fn check_args(text: Option<String>, imgs: Option<Vec<String>>, video: Option<String>, audio: Option<String>) {
    assert!(text.is_some() || (imgs.is_some() && imgs.clone().unwrap().len() > 0) || video.is_some() || audio.is_some(), "at least one field");
}

pub(crate) fn check_encrypt_args(text: Option<String>, imgs: Option<String>, video: Option<String>, audio: Option<String>) {
    assert!(text.is_some() || imgs.is_some() || video.is_some() || audio.is_some(), "at least one field");
}

pub(crate) fn get_parent_contract_id() -> AccountId {
    let current_id = env::current_account_id().to_string();
    let index = current_id.find('.').unwrap();
    let parent_id = current_id[index + 1..].to_string();
    AccountId::try_from(parent_id).unwrap()
}


pub(crate) fn get_content_hash(hierarchies: Vec<Hierarchy>, bloom_filter: &Bloom) -> Option<String> {
    let mut hash_prefix = "".to_string();
    for (_, hierarchy) in hierarchies.iter().enumerate() {
        let hierarchy_hash = env::sha256(&(hash_prefix + &hierarchy.account_id.to_string() + &String::from(&hierarchy.target_hash)).into_bytes());
        let hierarchy_hash: [u8;32] = hierarchy_hash[..].try_into().unwrap();
        if !bloom_filter.check(&WrappedHash::from(hierarchy_hash)) {
            return None
        }
        hash_prefix = String::from(&Base58CryptoHash::from(hierarchy_hash));
    }
    Some(hash_prefix)
}

pub(crate) fn set_content(args: String, account_id: AccountId, hash_prefix: String, bloom_filter: &mut Bloom) -> Base58CryptoHash {
    let args = args.clone() + &bs58::encode(env::random_seed()).into_string();
    let target_hash = env::sha256(&args.clone().into_bytes());
    let target_hash: [u8;32] = target_hash[..].try_into().unwrap();
    let hash = env::sha256(&(hash_prefix + &account_id.to_string() + &String::from(&Base58CryptoHash::from(target_hash))).into_bytes());
    let hash: CryptoHash = hash[..].try_into().unwrap();
    bloom_filter.set(&WrappedHash::from(hash), true);
    Base58CryptoHash::from(target_hash)
}