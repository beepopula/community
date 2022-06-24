use crate::*;
use utils::get_content_hash;
use post::Hierarchy;

#[near_bindgen]
impl Community {

    pub fn report_confirm(&mut self, account_id: AccountId, hierarchies: Vec<Hierarchy>, del: bool) {
        let sender_id = env::predecessor_account_id();
        assert!(account_id != sender_id, "signer_id = account_id");
        assert!(sender_id != self.owner_id || self.moderators.contains(&sender_id), "no authorization");

        let hierarchy_hash = match get_content_hash(hierarchies.clone(), &self.public_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), &self.encryption_bloom_filter).expect("content not found")
        };
        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap();

        let mut account = self.reports.get(&account_id).unwrap();
        let mut report = account.get(&hierarchy_hash).unwrap();
        assert!(report.del.is_none(), "resolved");
        report.del = Some(del); 
        account.insert(&hierarchy_hash, &report);
        self.reports.insert(&account_id, &account);

        if del == true {
            let hierarchy_hash = CryptoHash::from(hierarchy_hash);
            self.public_bloom_filter.set(&WrappedHash::from(hierarchy_hash), false);
            self.encryption_bloom_filter.set(&WrappedHash::from(hierarchy_hash), false);

            self.drip.set_report_drip(hierarchies, account_id);
            self.drip.set_report_confirm_drip(sender_id);
        }
    }
}
