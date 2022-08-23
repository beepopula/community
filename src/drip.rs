use std::{collections::HashMap, ops::Deref};

use crate::*;
use post::Hierarchy;
use uint::construct_uint;

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Debug)]
pub struct OldDrip {
    accounts: LookupMap<AccountId, HashMap<String, U128>>,  
}

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Debug)]
pub struct Drip {
    accounts: LookupMap<AccountId, DripAccount>,  
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Debug)]
pub struct DripAccount {
    balance: u128,     //post, comment, subcomment, comment to post, subcomment to post, subcomment to comment, like, report
    registered: bool,
    one_day_timestamp: u64,   //update after 24h
    content_count: u64
}

impl Default for DripAccount {
    fn default() -> Self {
        Self {
            balance: 0,
            registered: false,
            one_day_timestamp: env::block_timestamp(),
            content_count: 0
        }
    }
}

fn get_map_value(key: &String) -> u128 {
    let map: HashMap<String, u128> = serde_json::from_str(&json!({
        "content0": 200000000000000000000000 as u128,
        "content1": 200000000000000000000000 as u128,
        "content2": 200000000000000000000000 as u128,
        "content3": 100000000000000000000000 as u128,
        "content4": 40000000000000000000000 as u128,
        "content5": 100000000000000000000000 as u128,
        "like": 200000000000000000000000 as u128,
        "share": 200000000000000000000000 as u128,
        "be_shared": 50000000000000000000000 as u128,
        "be_liked": 50000000000000000000000 as u128,
    }).to_string()).unwrap();
    *map.get(key).unwrap_or(&(0 as u128))
}

fn get_account_decay(count: u64) -> u32 {
    if count >= 0 && count <= 10 {
        return 100
    } else if count > 10 && count <= 20 {
        return 50
    }
    25
}

impl Drip {
    pub fn new() -> Self {
        let mut this = Self { 
            accounts:  LookupMap::new("drip".as_bytes()),
        };
        this
    }

    fn set_drip(&mut self, key: String, hierarchy: Option<Hierarchy>, account_id: &AccountId) {
        let total_drip = U256::from(get_map_value(&key)) * U256::from(100 as u128);
        let mut drip = total_drip.clone();
        if let Some(hierarchy) = hierarchy {
            if let Some(options) = hierarchy.options.clone() {
                if let Some(royalties) = options.get("drip_royalties") {
                    let royalties: HashMap<AccountId, u32> = serde_json::from_str(&royalties).unwrap_or(HashMap::new());
                    for (account_id, royalty) in royalties {
                        let account_royalty = total_drip * royalty;
                        drip -= account_royalty;
                        let mut account = self.accounts.get(&hierarchy.account_id).unwrap_or_default();
                        account.balance += (account_royalty / U256::from(100 as u128)).as_u128();
                        self.accounts.insert(&account_id, &account);
                    }
                }
            }
        }
        let mut account = self.accounts.get(&account_id).unwrap_or_default();
        if env::block_timestamp() - account.one_day_timestamp > 60 * 60 * 24 * 1000_000_000 {
            account.one_day_timestamp = env::block_timestamp();
            account.content_count = 0
        }
        let decay = get_account_decay(account.content_count);
        drip *= decay;
        account.balance += (drip / U256::from(100 as u128) / U256::from(100 as u128)).as_u128();
        self.accounts.insert(&account_id, &account);
    }
    

    pub fn set_content_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) {
        let len = hierarchies.len();

        for (i, hierarchy) in hierarchies.iter().enumerate() { 
            if hierarchy.account_id == account_id {
                continue
            }
            let key = "content".to_string() + &(i + MAX_LEVEL + len - 1).to_string();
            self.set_drip(key, Some(hierarchy.clone()), &hierarchy.account_id);
        }

        let key = "content".to_string() + &(len).to_string();
        self.set_drip(key, None, &account_id);
    }

    pub fn set_like_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) {
        let hierarchy = hierarchies.get(hierarchies.len() - 1).unwrap();
        let content_account_id = hierarchy.account_id.clone();
        if content_account_id == account_id {
            return
        }
        let key = "be_liked".to_string();
    
        self.set_drip(key, Some(hierarchy.clone()), &content_account_id);

        let key = "like".to_string();
        self.set_drip(key, Some(hierarchy.clone()), &account_id);
    }

    pub fn set_report_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) {
        let hierarchy = hierarchies.get(hierarchies.len() - 1).unwrap();
        let content_account_id = hierarchy.account_id.clone();
        if content_account_id == account_id {
            return
        }
        let key = "report".to_string();
        self.set_drip(key, Some(hierarchy.clone()), &account_id);
    }

    pub fn set_report_confirm_drip(&mut self, account_id: AccountId) {
        let key = "report_confirm".to_string();
        self.set_drip(key, None, &account_id);
    }

    pub fn set_share_drip(&mut self, hierarchies: Vec<Hierarchy>, account_id: AccountId) {
        let content_account_id = hierarchies.get(hierarchies.len() - 1).unwrap().account_id.clone();
        if content_account_id == account_id {
            return
        }
        let hierarchy = hierarchies.get(hierarchies.len() - 1).unwrap();
        let key = "be_shared".to_string();
        self.set_drip(key, Some(hierarchy.clone()), &content_account_id);

        let key = "share".to_string();
        match self.accounts.get(&account_id) {
            Some(_) => {
                self.set_drip(key, None, &account_id);
            },
            None => return
        }
    }

    pub fn get_and_clear_drip(&mut self, account_id: AccountId) -> U128 {
        let mut account = self.accounts.get(&account_id).unwrap_or_default();
        assert!(account.registered, "not registered");
        let balance = account.balance.clone();
        account.balance = 0;
        self.accounts.insert(&account_id, &account);
        balance.into()
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