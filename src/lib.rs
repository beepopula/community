

use std::collections::HashMap;
use std::convert::TryInto;

use access::{Access, Condition, Relationship};
use bloom_filter::{Bloom, WrappedHash};
use events::Event;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base58CryptoHash, U128, U64};
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::serde_json::{json, self};
use near_sdk::{env, near_bindgen, AccountId, log, bs58, PanicOnDefault, Promise, BlockHeight, CryptoHash};
use near_sdk::collections::{LookupMap, UnorderedMap, Vector, LazyOption, UnorderedSet};
use drip::Drip;
use post::Report;
use utils::{check_args, verify, check_encrypt_args, refund_extra_storage_deposit, set_content};
use crate::post::Hierarchy;
use std::convert::TryFrom;


pub mod utils;
pub mod signature;
pub mod bloom_filter;
pub mod access;
pub mod post;
pub mod resolver;
pub mod owner;
pub mod moderator;
pub mod drip;
pub mod view;
pub mod events;



#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Community {
    owner_id: AccountId,
    public_key: String,
    moderators: UnorderedSet<AccountId>,
    public_bloom_filter: Bloom,
    encryption_bloom_filter: Bloom,
    relationship_bloom_filter: Bloom,
    access: Option<Access>,
    reports: UnorderedMap<AccountId, UnorderedMap<Base58CryptoHash, Report>>,
    drip: Drip
}

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug)]
pub struct Member (u32);


const MAX_LEVEL: usize = 3;


#[near_bindgen]
impl Community {

    #[init]
    pub fn new(owner_id: AccountId, public_key: String) -> Self {
        Self {
            owner_id: owner_id,
            public_key: public_key,
            moderators: UnorderedSet::new(b'm'),
            public_bloom_filter: Bloom::new_for_fp_rate_with_seed(1000000, 0.1, "public".to_string()),
            encryption_bloom_filter: Bloom::new_for_fp_rate_with_seed(1000000, 0.1, "encrypt".to_string()),
            relationship_bloom_filter: Bloom::new_for_fp_rate_with_seed(1000000, 0.1, "relationship".to_string()),
            access: None,
            reports: UnorderedMap::new(b'r'),
            drip: Drip::new()
        }
    }
    
    #[payable]
    pub fn join(&mut self) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        match &mut self.access {
            Some(v) => v.check_permission(sender_id),
            None => {
                self.drip.join(sender_id);
            }
        }
        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage, 0)
    }

    #[payable]
    pub fn quit(&mut self) {
        let sender_id = env::predecessor_account_id();
        self.drip.quit(sender_id);
    }

    pub fn add_item(&mut self, args: String) -> Base58CryptoHash {
        let sender_id = env::signer_account_id();
        let args = sender_id.to_string() + &args.clone();
        let target_hash = set_content(args.clone(), sender_id.clone(), "".to_string(), &mut self.public_bloom_filter);
        self.drip.set_content_drip(Vec::new(), sender_id.clone());
        Event::log_add_content(args, vec![Hierarchy { 
            target_hash, 
            account_id: sender_id
        }]);
        target_hash
    }

    pub fn collect_drip(&mut self) -> HashMap<String, U128> {
        let sender_id = env::signer_account_id();
        self.drip.get_and_clear_drip(sender_id)
    }
    
}





#[cfg(test)]
mod tests {


}