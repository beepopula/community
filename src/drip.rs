use std::{collections::HashMap, ops::Deref};

use crate::*;
use account::Account;
use post::Hierarchy;
use uint::construct_uint;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Debug)]
pub struct Drip {
    accounts: LookupMap<AccountId, Account>,  
}

pub fn get_map_value(key: &String) -> u128 {
    let map: HashMap<String, U128> = serde_json::from_str(&json!({
        "content0":   "1000000000000000000000000",    //post                       active
        "content1":   "1000000000000000000000000",    //comment                    active
        "content2":   "1000000000000000000000000",    //subcomment                 active
        "content3":    "400000000000000000000000",    //comment to post            passive
        // "content4": "40000000000000000000000",     //subcomment to post         passive
        "content5":    "400000000000000000000000",    //subcomment to comment      passive
        "like":        "200000000000000000000000",         //like                       active
        "invite":     "10000000000000000000000000",        //invite                      active for inviter
        "be_liked":    "200000000000000000000000",     //be_liked                   passive
        "report":     "2000000000000000000000000",      //report                     passive
        "report_refund": "1000000000000000000000000",//report_refund             passive
    }).to_string()).unwrap();
    let val = *map.get(key).unwrap_or(&(U128::from(0)));
    val.0
}

fn get_content_decay(count: u8) -> u32 {
    match count {
        0 => 200,
        _ => 100
    }
}

impl Drip {
    pub fn new() -> Self {
        let mut this = Self { 
            accounts:  LookupMap::new(StorageKey::Account),
        };
        this
    }

    fn set_drip(&mut self, key: String, options: Option<HashMap<String, String>>, account_id: &AccountId, per: u32) -> Vec<(AccountId, String, U128)> {
        let total_drip = U256::from(get_map_value(&key));
        let mut drip_items: Vec<(AccountId, String, U128)> = Vec::new();
        let mut drip = total_drip.clone();

        if let Some(options) = options.clone() {
            if let Some(royalties) = options.get("drip_royalties") {
                let royalties: HashMap<AccountId, u32> = serde_json::from_str(&royalties).unwrap_or(HashMap::new());
                for (account_id, royalty) in royalties {
                    let account_royalty = total_drip * royalty;
                    drip -= account_royalty;
                    let account_royalty = (account_royalty / U256::from(100 as u128)).as_u128();
                    let mut account = self.accounts.get(&account_id).unwrap_or_default();
                    account.increase_drip(account_royalty);
                    self.accounts.insert(&account_id, &account);
                    drip_items.push((account_id, key.clone() + ":royalty", account_royalty.into()));
                }
            }
        }
        
        let mut account = self.accounts.get(&account_id).unwrap_or_default();
        drip *= per;
        let drip = (drip / U256::from(100 as u128)).as_u128();
        account.increase_drip(drip);
        self.accounts.insert(&account_id, &account);
        drip_items.push((account_id.clone(), key, drip.into()));
        drip_items
    }
    

    pub fn set_content_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId, prev_content_count: Option<u8>) -> Vec<(AccountId, String, U128)> {
        let len = hierarchies.len();
        let mut drip_items: Vec<(AccountId, String, U128)> = Vec::new();
        for (i, hierarchy) in hierarchies.iter().enumerate() { 
            if hierarchy.account_id == account_id {
                continue
            }
            let key = "content".to_string() + &(i + MAX_LEVEL + len - 1).to_string();
            let items = self.set_drip(key, hierarchy.options.clone(), &hierarchy.account_id, 100);
            drip_items = [drip_items, items].concat();
        }

        let key = "content".to_string() + &(len).to_string();
        let mut per = 100;
        // only comment can be doubled
        if let Some(prev_content_count) = prev_content_count{
            if len == 1 {   
                per = get_content_decay(prev_content_count);
            }
        }

        let mut account = self.accounts.get(&account_id).unwrap_or_default();
        per = account.get_account_decay() * per / 100;
        account.increase_content_count();
        self.accounts.insert(&account_id, &account);
        
        let items = self.set_drip(key, None, &account_id, per); 
        [drip_items, items].concat()
    }

    pub fn set_like_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) -> Vec<(AccountId, String, U128)> {
        let hierarchy = hierarchies.get(hierarchies.len() - 1).unwrap();
        let content_account_id = hierarchy.account_id.clone();
        if content_account_id == account_id {
            return vec![]
        }

        let mut drip_items: Vec<(AccountId, String, U128)> = Vec::new();
        let key = "be_liked".to_string();
        let items = self.set_drip(key, hierarchy.options.clone(), &content_account_id, 100);
        drip_items = [drip_items, items].concat();
        

        let key = "like".to_string();
        let items = self.set_drip(key, None, &account_id, 100);
        [drip_items, items].concat()
    }

    pub fn set_report_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) -> Vec<(AccountId, String, U128)> {
        let hierarchy = hierarchies.get(hierarchies.len() - 1).unwrap();
        let content_account_id = hierarchy.account_id.clone();
        if content_account_id == account_id {
            return vec![]
        }
        let key = "report".to_string();
        self.set_drip(key, None, &account_id, 100)
    }

    pub fn set_report_refund_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) -> Vec<(AccountId, String, U128)> {
        let hierarchy = hierarchies.get(hierarchies.len() - 1).unwrap();
        let content_account_id = hierarchy.account_id.clone();
        if content_account_id == account_id {
            return vec![]
        }
        let key = "report_refund".to_string();
        self.set_drip(key, None, &account_id, 100)
    }

    pub fn set_report_confirm_drip(&mut self, account_id: AccountId) -> Vec<(AccountId, String, U128)> {
        let key = "report_confirm".to_string();
        self.set_drip(key, None, &account_id, 100)
    }

    pub fn set_invite_drip(&mut self, inviter_id: AccountId, invitee_id: AccountId) -> Vec<(AccountId, String, U128)> {
        if inviter_id == invitee_id {
            return vec![]
        }
        let key = "invite".to_string();
        match self.accounts.get(&invitee_id) {
            Some(_) => {
                let items = self.set_drip(key, None, &invitee_id, 100);
                items
            },
            None => vec![]
        }
    }

    pub fn get_and_clear_drip(&mut self, account_id: AccountId) -> U128 {
        let mut account = self.accounts.get(&account_id).unwrap_or_default();
        assert!(account.is_registered(), "not registered");
        let balance = account.get_drip();
        account.decrease_drip(balance);
        self.accounts.insert(&account_id, &account);
        balance.into()
    }

    pub fn get_drip(&self, account_id: AccountId) -> U128 {
        let account = self.accounts.get(&account_id).unwrap_or_default();
        account.get_drip().into()
    }

    pub fn get_account_decay(&self, account_id: AccountId) -> u32 {
        let account = self.accounts.get(&account_id).unwrap_or_default();
        account.get_account_decay()
    }

    pub fn get_content_decay(&self, content_count: u32) -> u32 {
        get_content_decay(content_count as u8)
    }
}