use std::collections::HashMap;

use crate::*;
use utils::get_content_hash;
use post::Hierarchy;

#[near_bindgen]
impl Community {
    pub fn check_follow(&self, followee: AccountId, follower: AccountId) -> bool {
        let target_hash = env::sha256(&(followee.to_string() + "follwed_by" + &follower.to_string()).into_bytes());
        //let target_hash: [u8;32] = target_hash[..].try_into().unwrap();
        self.relationship_tree.check(&target_hash)
    }

    pub fn get_drip(&self, account_id: AccountId) -> U128 {
        self.drip.get_drip(account_id)
    }

    pub fn get_account_decay(&self, account_id: AccountId) -> u32 {
        self.drip.get_account_decay(account_id)
    }

    pub fn get_content_decay(&self, hierarchies: Vec<Hierarchy>) -> u32 {
        let mut content_count = 0;
        if hierarchies.len() > 0 {
            let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_tree) {
                Some(v) => v,
                None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_tree).expect("content not found")
            };
            let prev_hash = CryptoHash::from(Base58CryptoHash::try_from(hierarchy_hash).unwrap());
            content_count = self.content_tree.get(&prev_hash).unwrap();
        }
        self.drip.get_content_decay(content_count as u32)
    }

    pub fn check_viewed(&self, hierarchies: Vec<Hierarchy>, inviter_id: AccountId, account_id: AccountId) -> bool {
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_tree) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_tree).expect("content not found")
        };
        let view_hash = env::sha256(&(account_id.to_string() + "viewed" + &hierarchy_hash + "through" + &inviter_id.to_string()).into_bytes());
        //let view_hash: CryptoHash = view_hash[..].try_into().unwrap();
        self.relationship_tree.check(&view_hash)
    }

    pub fn get_reports(&self, account_id: AccountId) -> Vec<Report> {
        let account = match self.reports.get(&account_id) {
            Some(v) => v,
            None => return Vec::new()
        };
        account.values().collect()
    }
    
}