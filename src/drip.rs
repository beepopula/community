use std::collections::HashMap;

use crate::*;
use post::Hierarchy;


#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Debug)]
pub struct Drip {
    accounts: LookupMap<AccountId, DripAccount>,  
    map: HashMap<String, u128>
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Debug)]
pub struct DripAccount {
    balance: u128,     //post, comment, subcomment, comment to post, subcomment to post, subcomment to comment, like, report
    registered: bool
}

impl Default for DripAccount {
    fn default() -> Self {
        Self {
            balance: 0,
            registered: false
        }
    }
}



impl Drip {
    pub fn new() -> Self{
        Drip { 
            accounts:  LookupMap::new("drip".as_bytes()),
            map: HashMap::new()
        }
    }

    fn set_drip(&self, key: String, balance: &mut u128) {
        *balance += self.map.get(&key).unwrap_or(&(0 as u128));
    }
    

    pub fn set_content_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) {
        let len = hierarchies.len();

        for (i, hierarchy) in hierarchies.iter().enumerate() { 
            if hierarchy.account_id == account_id {
                continue
            }
            let mut account = self.accounts.get(&hierarchy.account_id).unwrap_or_default();
            let key = "content".to_string() + &(i + MAX_LEVEL + len - 1).to_string();
            self.set_drip(key, &mut account.balance);
            self.accounts.insert(&hierarchy.account_id, &account);
        }

        let mut sender = self.accounts.get(&account_id).unwrap_or_default();
        let key = "content".to_string() + &(len).to_string();
        self.set_drip(key, &mut sender.balance);
        self.accounts.insert(&account_id, &sender);
    }

    pub fn set_like_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) {
        let content_account_id = hierarchies.get(hierarchies.len() - 1).unwrap().account_id.clone();
        if content_account_id == account_id {
            return
        }
        let key = "be_liked".to_string();
        let mut content_account = self.accounts.get(&content_account_id).unwrap_or_default();
        self.set_drip(key, &mut content_account.balance);
        self.accounts.insert(&content_account_id, &content_account);

        let key = "like".to_string();
        let mut account = self.accounts.get(&account_id).unwrap_or_default();
        self.set_drip(key, &mut account.balance);
        self.accounts.insert(&account_id, &account);
    }

    pub fn set_report_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) {
        let content_account_id = hierarchies.get(hierarchies.len() - 1).unwrap().account_id.clone();
        if content_account_id == account_id {
            return
        }
        let key = "report".to_string();
        let mut account = self.accounts.get(&account_id).unwrap_or_default();
        self.set_drip(key, &mut account.balance);
        self.accounts.insert(&account_id, &account);
    }

    pub fn set_report_confirm_drip(&mut self, account_id: AccountId) {
        let key = "report_confirm".to_string();
        let mut account = self.accounts.get(&account_id).unwrap_or_default();
        self.set_drip(key, &mut account.balance);
        self.accounts.insert(&account_id, &account);
    }

    // pub fn set_share_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) {
    //     let content_account_id = hierarchies.get(hierarchies.len() - 1).unwrap().account_id.clone();
    //     if content_account_id == account_id {
    //         return
    //     }

    //     let key = "be_shared".to_string();
    //     let mut content_account = self.accounts.get(&content_account_id).unwrap_or(HashMap::new());
    //     set_drip(key, &mut content_account);
    //     self.accounts.insert(&content_account_id, &content_account);

    //     let key = "share".to_string();
    //     let mut account = self.accounts.get(&account_id).unwrap_or(HashMap::new());
    //     set_drip(key, &mut account);
    //     self.accounts.insert(&account_id, &account);
    // }

    pub fn set_share_view_drip(&mut self, account_id: AccountId) {
        let key = "share_view".to_string();
        let mut account = self.accounts.get(&account_id).unwrap_or_default();
        self.set_drip(key, &mut account.balance);
        self.accounts.insert(&account_id, &account);
    }

    pub fn get_and_clear_drip(&mut self, account_id: AccountId) -> U128 {
        let mut account = self.accounts.get(&account_id).unwrap_or_default();
        account.balance.into()
    }

    pub fn get_drip(&self, account_id: AccountId) -> U128 {
        let account = self.accounts.get(&account_id).unwrap_or_default();
        account.balance.into()
    }

    
}



impl Drip {
    pub fn join(&mut self, account_id: AccountId) {
        let mut account = self.accounts.get(&account_id).unwrap_or_default();
        account.registered = true;
        self.accounts.insert(&account_id, &account);
    }

    pub fn quit(&mut self, account_id: AccountId) {
        let account = self.accounts.get(&account_id);
        if let Some(mut account) = account {
            if account.balance == 0 {
                self.accounts.remove(&account_id);
            } else {
                account.registered = false
            }
        }
    }

    pub fn is_member(&self, account_id: AccountId) -> bool {
        self.accounts.get(&account_id).is_some()
    }
}