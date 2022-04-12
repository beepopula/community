

use access::{Access, Condition, Relationship};
use bloom_filter::{Bloom, WrappedHash};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base58CryptoHash, U128};
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::serde_json::{json, self};
use near_sdk::{env, near_bindgen, AccountId, log, bs58, PanicOnDefault, Promise, BlockHeight};
use near_sdk::collections::{LookupMap, UnorderedMap, Vector, LazyOption};
use utils::{check_args, verify, check_encrypt_args, refund_extra_storage_deposit};


pub mod utils;
pub mod signature;
pub mod bloom_filter;
pub mod access;
pub mod post;
pub mod resolver;




#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Community {
    owner_id: AccountId,
    public_key: String,
    post_bloom_filter: Bloom,
    encrypt_post_bloom_filter: Bloom,
    access: Option<Access>,
    members: UnorderedMap<AccountId, Member>,
}

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug)]
pub struct Member (u32);


#[near_bindgen]
impl Community {

    #[init]
    pub fn new(owner_id: AccountId, public_key: String) -> Self {
        Self {
            owner_id: owner_id,
            public_key: public_key,
            post_bloom_filter: Bloom::new_for_fp_rate_with_seed(1000000, 0.1, "public".to_string()),
            encrypt_post_bloom_filter: Bloom::new_for_fp_rate_with_seed(1000000, 0.1, "encrypt".to_string()),
            access: None,
            members: UnorderedMap::new(b'm')
        }
    }
    
    #[payable]
    pub fn join(&mut self) {
        let initial_storage_usage = env::storage_usage();
        let account_id = env::predecessor_account_id();
        match &mut self.access {
            Some(v) => v.check_permission(account_id),
            None => {
                self.members.insert(&account_id, &Member(1));
            }
        }
        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage, 0)
    }
    
}





#[cfg(test)]
mod tests {


}