use crate::{utils::get_content_hash, post::REPORT_DEPOSIT};
use crate::*;

impl Community {
    pub(crate) fn internal_report(&mut self, sender_id: AccountId, hierarchies: Vec<Hierarchy>) {
        let initial_storage_usage = env::storage_usage();
        assert!( REPORT_DEPOSIT <= env::attached_deposit(), "not enough deposit");

        assert!(self.can_execute_action(sender_id.clone(), Permission::Report), "not allowed");
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_tree) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_tree).expect("content not found")
        };
        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap();

        let mut report_accounts = self.reports.get(&hierarchy_hash).unwrap_or(HashSet::new());
        if report_accounts.len() >= 10 {
            return
        }
        report_accounts.insert(sender_id);
        self.reports.insert(&hierarchy_hash, &report_accounts);
        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage, 0)
    }
}