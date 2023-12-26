use crate::utils::get_root_id;
use crate::{utils::get_content_hash};
use crate::*;

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[derive(BorshDeserialize, BorshSerialize, BorshStorageKey)]
pub enum Instruction {
    Write(HashMap<String, String>),
    Drip(Vec<(AccountId, String, U128)>)
}

impl Community {
    pub(crate) fn internal_report(&mut self, sender_id: AccountId, hierarchies: Vec<Hierarchy>) {
        let initial_storage_usage = env::storage_usage();

        assert!(self.can_execute_action(None, None, Permission::Report), "not allowed");

        let hierarchy = hierarchies.get(hierarchies.len() - 1).unwrap();
        let content_account_id = hierarchy.account_id.clone();
        assert!(content_account_id != sender_id, "can not be content owner");

        let hierarchy_hash = get_content_hash(hierarchies.clone(), None, false).expect("content not found");
        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap();

        let mut report_accounts = self.reports.get(&hierarchy_hash).unwrap_or(HashSet::new());
        assert!(report_accounts.len() < 5, "can not report");
        assert!(!report_accounts.contains(&sender_id), "already report");
        report_accounts.insert(sender_id.clone());
        self.reports.insert(&hierarchy_hash, &report_accounts);
        set_storage_usage(initial_storage_usage, Some(sender_id));
    }

    pub(crate) fn internal_revoke_report(&mut self, sender_id: AccountId, hierarchies: Vec<Hierarchy>) {
        let initial_storage_usage = env::storage_usage();
        let hierarchy_hash = get_content_hash(hierarchies.clone(), None, true).expect("content not found");
        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap();
        if let Some(mut accounts) = self.reports.get(&hierarchy_hash) {
            if let Some(_) = accounts.get(&sender_id) {
                accounts.remove(&sender_id);
                if accounts.is_empty() {
                    self.reports.remove(&hierarchy_hash);
                }
            }
        }
        
        set_storage_usage(initial_storage_usage, None);
    }

    pub(crate) fn internal_execute_instructions(&mut self, account_id: AccountId, instructions: Vec<Instruction>) {
        let mut account = get_account(&account_id);
        for instruction in instructions {
            match instruction {
                Instruction::Write(map) => {
                    account.data.insert(get_predecessor_id().to_string(), json!(map).to_string());
                },
                Instruction::Drip(drips) => {
                    if get_root_id(env::current_account_id()) == get_root_id(get_predecessor_id()) {
                        for (account_id, key, amount) in drips.clone() {
                            self.drip.set_custom_drip(key, &account_id, amount.0, false);
                        }
                        Event::log_other(
                            Some(json!({
                                "drips": drips
                            }).to_string())
                        )
                    }
                }
            }
        }
        set_account(&account_id, &account)
    }
}