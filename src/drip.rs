use std::collections::HashMap;

use crate::*;
use post::Hierarchy;


#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Debug)]
pub struct Drip {
    accounts: LookupMap<AccountId, HashMap<String, U128>>,     //post, comment, subcomment, comment to post, subcomment to post, subcomment to comment, like, report
}

fn set_drip(key: String, map: &mut HashMap<String, U128>) {
    let drip = map.get(&key).unwrap_or(&U128::from(0)).0;
    let drip = U128::from(drip + 1);
    map.insert(key, drip);
}

impl Drip {
    pub fn new() -> Self{
        Drip { 
            accounts:  LookupMap::new("drip".as_bytes())
        }
    }

    pub fn set_content_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) {
        let len = hierarchies.len();

        for (i, hierarchy) in hierarchies.iter().enumerate() { 
            if hierarchy.account_id == account_id {
                continue
            }
            if let Some(mut account) = self.accounts.get(&hierarchy.account_id) {
                let key = "content".to_string() + &(i + MAX_LEVEL + len - 1).to_string();
                set_drip(key, &mut account);
                self.accounts.insert(&hierarchy.account_id, &account);
            }  
        }

        if let Some(mut sender) = self.accounts.get(&account_id) {
            let key = "content".to_string() + &(len).to_string();
            set_drip(key, &mut sender);
            self.accounts.insert(&account_id, &sender);
        }
    }

    pub fn set_like_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) {
        let content_account_id = hierarchies.get(hierarchies.len() - 1).unwrap().account_id.clone();
        if content_account_id == account_id {
            return
        }
        
        if let Some(mut content_account) = self.accounts.get(&content_account_id) {
            let key = "be_liked".to_string();
            set_drip(key, &mut content_account);
            self.accounts.insert(&content_account_id, &content_account);
        }
        
        if let Some(mut account) = self.accounts.get(&account_id) {
            let key = "like".to_string();
            set_drip(key, &mut account);
            self.accounts.insert(&account_id, &account);
        }
    }

    pub fn set_report_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) {
        let content_account_id = hierarchies.get(hierarchies.len() - 1).unwrap().account_id.clone();
        if content_account_id == account_id {
            return
        }

        if let Some(mut account) = self.accounts.get(&account_id) {
            let key = "report".to_string();
            set_drip(key, &mut account);
            self.accounts.insert(&account_id, &account);
        }
    }

    pub fn set_report_confirm_drip(&mut self, account_id: AccountId) {
        if let Some(mut account) = self.accounts.get(&account_id) {
            let key = "report_confirm".to_string();
            set_drip(key, &mut account);
            self.accounts.insert(&account_id, &account);
        }
        
    }

    pub fn set_share_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) {
        let content_account_id = hierarchies.get(hierarchies.len() - 1).unwrap().account_id.clone();
        if content_account_id == account_id {
            return
        }

        if let Some(mut content_account) = self.accounts.get(&content_account_id) {
            let key = "be_shared".to_string();
            set_drip(key, &mut content_account);
            self.accounts.insert(&content_account_id, &content_account);
        }
        
        if let Some(mut account) = self.accounts.get(&account_id) {
            let key = "share".to_string();
            set_drip(key, &mut account);
            self.accounts.insert(&account_id, &account);
        }
        
    }

    pub fn set_share_view_drip(&mut self, account_id: AccountId) {
        if let Some(mut account) = self.accounts.get(&account_id) {
            let key = "share_view".to_string();
            set_drip(key, &mut account);
            self.accounts.insert(&account_id, &account);
        }
        
    }




    pub fn get_and_clear_drip(&mut self, account_id: AccountId) -> HashMap<String, U128> {
        let mut account = match self.accounts.get(&account_id) {
            Some(v) => v,
            None => HashMap::new()
        };
        let ret = account.clone();
        for (_, value) in account.iter_mut() {
            *value = U128::from(0);
        }
        self.accounts.insert(&account_id, &account);
        ret
    }

    pub fn get_drip(&self, account_id: AccountId) -> HashMap<String, U128> {
        let account = self.accounts.get(&account_id).unwrap_or(HashMap::new());
        account
    }

    
}



impl Drip {
    pub fn join(&mut self, account_id: AccountId) {
        match self.accounts.get(&account_id) {
            Some(_) => log!("registred"),
            None => {self.accounts.insert(&account_id, &HashMap::new());}
        }
    }

    pub fn quit(&mut self, account_id: AccountId) {
        self.accounts.remove(&account_id);
    }
}