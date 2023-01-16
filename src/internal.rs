use crate::{utils::get_content_hash};
use crate::*;

impl Community {
    pub(crate) fn internal_report(&mut self, sender_id: AccountId, hierarchies: Vec<Hierarchy>) {
        let initial_storage_usage = env::storage_usage();

        assert!(self.can_execute_action(sender_id.clone(), Permission::Report), "not allowed");

        let hierarchy = hierarchies.get(hierarchies.len() - 1).unwrap();
        let content_account_id = hierarchy.account_id.clone();
        assert!(content_account_id != sender_id, "can not be content owner");

        let hierarchy_hash = get_content_hash(hierarchies.clone(), None, &self.content_tree).expect("content not found");
        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap();

        let mut report_accounts = self.reports.get(&hierarchy_hash).unwrap_or(HashSet::new());
        assert!(report_accounts.len() < 5, "can not report");
        assert!(!report_accounts.contains(&sender_id), "already report");
        report_accounts.insert(sender_id);
        self.reports.insert(&hierarchy_hash, &report_accounts);
        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage, 0)
    }
}