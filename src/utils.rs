

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
