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

    pub fn check_viewed(&self, hierarchies: Vec<Hierarchy>, inviter_id: AccountId, account_id: AccountId) -> bool {
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_tree) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_tree).expect("content not found")
        };
        let view_hash = env::sha256(&(account_id.to_string() + "viewed" + &hierarchy_hash + "invited_by" + &inviter_id.to_string()).into_bytes());
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