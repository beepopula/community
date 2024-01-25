use std::{collections::HashMap, ops::Deref};

use crate::{*, utils::{set_account, get}};
use account::Account;
use post::Hierarchy;
use uint::construct_uint;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub enum PendingDrip {
    Draw(Vec<u8>)  //between..to  integer
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
        "report":     "1000000000000000000000000",      //report                     passive
        "report_deposit": "1000000000000000000000000",//report_deposit            passive
        "be_voted":        "200000000000000000000000",  //be_voted                 passive
        "vote":        "200000000000000000000000"      //vote                     active
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
        let mut drip = total_drip.clone() * per;

        if let Some(options) = options.clone() {
            if let Some(royalties) = options.get("drip_royalties") {
                let royalties: HashMap<AccountId, u32> = serde_json::from_str(&royalties).unwrap_or(HashMap::new());
                for (account_id, royalty) in royalties {
                    let account_royalty = total_drip * royalty;
                    drip -= account_royalty;
                    let account_royalty = (account_royalty / U256::from(100 as u128)).as_u128();
                    let mut account = get_account(&account_id);
                    account.increase_drip(account_royalty);
                    set_account(&account);
                    drip_items.push((account_id, key.clone() + ":royalty", account_royalty.into()));
                }
            }
        }
        
        let mut account = get_account(&account_id);
        let drip = (drip / U256::from(100 as u128)).as_u128();
        account.increase_drip(drip);
        self.cum_active_drip(drip);
        set_account(&account);
        drip_items.push((account_id.clone(), key, drip.into()));
        drip_items
    }

    fn cum_active_drip(&mut self, drip: u128) {
        let asset = AssetKey::Drip((Some(AccountId::from_str("active").unwrap()), env::current_account_id()));
        let mut account = get_account(&env::current_account_id());
        let total_drip = account.get_balance(&asset);
        if let Some(new_total_drip) = total_drip.checked_add(drip) {
            account.increase_balance(asset, drip);
        }
        self.accounts.insert(&env::current_account_id(), &account);
    }

    pub fn set_custom_drip(&mut self, key: String, account_id: &AccountId, amount: u128, active_drip: bool) -> Vec<(AccountId, String, U128)> {
        let mut account = get_account(&account_id);
        let drip = amount;
        account.increase_drip(drip);
        if active_drip {
            self.cum_active_drip(drip);
        }
        set_account(&account);
        let mut drip_items: Vec<(AccountId, String, U128)> = Vec::new();
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
                let post = hierarchies.get(0).unwrap();
                if per > 100 && post.account_id == account_id {
                    per = 100
                }
            }
        }

        let mut account = get_account(&account_id);
        per = account.get_account_decay() * per / 100;
        account.increase_content_count();
        set_account(&account);
        
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

    pub fn set_report_confirm_drip(&mut self, account_id: AccountId) -> Vec<(AccountId, String, U128)> {
        let key = "report_confirm".to_string();
        self.set_drip(key, None, &account_id, 100)
    }

    // pub fn set_invite_drip(&mut self, inviter_id: AccountId, invitee_id: AccountId) -> Vec<(AccountId, String, U128)> {
    //     if inviter_id == invitee_id {
    //         return vec![]
    //     }
    //     let key = "invite".to_string();
    //     self.set_drip(key, None, &inviter_id, 100)
    // }

    // pub fn set_vote_drip(&mut self, voter_id: AccountId, amount: u128) -> Vec<(AccountId, String, U128)> {
    //     let key = "vote".to_string();
    //     self.set_custom_drip(key, &voter_id, amount)
    // }

    pub fn set_proposal_drip(&mut self, proposer_id: AccountId, account_id: AccountId) -> Vec<(AccountId, String, U128)> {
        if proposer_id == account_id {
            return vec![]
        }
        let key = "be_voted".to_string();
        self.set_drip(key, None, &proposer_id, 100)
    }

    pub fn gather_drip(&mut self, from: AccountId, to: AccountId) -> Vec<(AccountId, String, U128)> {
        if from == to {
            return vec![]
        }

        let mut from_account = get_account(&from);
        let mut to_account = get_account(&to).registered();
        let amount = from_account.get_drip();
        from_account.decrease_drip(amount);
        self.accounts.insert(&from, &from_account);
        to_account.increase_drip(amount);
        set_account(&to_account);
        vec![(to, "gather".to_string(), amount.into())]
    }
    

    pub fn add_pending_drip(&mut self, account_id: AccountId, reason: String, option: String, pending_drip: PendingDrip) -> Vec<(AccountId, String, String)> {
        let id = env::sha256((account_id.to_string() + &reason + &option).as_bytes());
        set::<PendingDrip>(&id, pending_drip);
        vec![(account_id, "invite".to_string(), option)]
    }

    pub fn set_pending_drip(&mut self, account_id: AccountId, reason: String, option: String) -> Vec<(AccountId, String, U128)> {
        let id = env::sha256((account_id.to_string() + &reason + &option).as_bytes());
        match get::<PendingDrip>(&id) {
            Some(pending) => {
                let amount = resolve_pending(pending);
                remove(&id);
                self.set_custom_drip(reason.clone(), &account_id, amount, true);
                vec![(account_id, reason, amount.into())]
            },
            None => panic!("not found")
        }
    } 

    pub fn get_and_clear_drip(&mut self, account_id: AccountId) -> U128 {
        let mut account = get_account(&account_id).registered();
        let balance = account.get_drip();
        account.decrease_drip(balance);
        let asset = AssetKey::Drip((None, env::current_account_id()));
        let total_drip = account.get_balance(&asset);
        if let Some(new_total_drip) = total_drip.checked_add(balance) {
            account.increase_balance(asset, balance);
        }
        self.accounts.insert(&account_id, &account);
        balance.into()
    }

    pub fn get_drip(&self, account_id: AccountId) -> U128 {
        let account = get_account(&account_id);
        account.get_drip().into()
    }

    pub fn get_account_decay(&self, account_id: AccountId) -> u32 {
        let account = get_account(&account_id);
        account.get_account_decay()
    }

    pub fn get_content_decay(&self, content_count: u32) -> u32 {
        get_content_decay(content_count as u8)
    }
}


fn resolve_pending(pending: PendingDrip) -> u128 {
    match pending {
        PendingDrip::Draw(items) => {
            let r = u128::from_be_bytes(env::random_seed()[0..16].try_into().unwrap()) as usize;
            let mut v = items[r % items.len()] as u128;
            v = v * 1000000000000000000000000;
            v
        }
    }
}


mod test {
    use std::{collections::HashMap, str::FromStr, hash::Hash, convert::TryInto};

    use near_sdk::{json_types::U128, AccountId, serde_json::json, serde_json, env};

    use crate::account::{self, Account};

    use super::{U256, get_map_value, Drip, PendingDrip, resolve_pending};

    #[test]
    pub fn test_resolve_pending() {
        let pending = PendingDrip::Draw(vec![10,11,12,13,14,15,20]);
        let res = resolve_pending(pending);
        println!("{:?}", res);
        let r = u128::from_be_bytes(env::random_seed()[16..32].try_into().unwrap()) as usize;
        println!("{:?}", r)
    }


    #[test]
    pub fn test() {
        let mut options: HashMap<String, String> = HashMap::new();
        let r = json!({
            "billkin.testnet": 5
        }).to_string();
        options.insert("drip_royalties".to_string(), r);
        let total_drip: U256 = U256::from(get_map_value(&"like".to_string()));
        let mut drip = total_drip.clone() * U256::from(100 as u128);


        if let Some(royalties) = options.get("drip_royalties") {
            let royalties: HashMap<AccountId, u32> = serde_json::from_str(&royalties).unwrap_or(HashMap::new());
            for (account_id, royalty) in royalties {
                let account_royalty = total_drip * royalty;
                println!("account_royalty: {:?}, drip: {:?}", account_royalty, drip);
                drip -= account_royalty;
                let account_royalty = (account_royalty / U256::from(100 as u128)).as_u128();
                println!("{:?}", account_royalty)
            }
        }
        
        let drip = (drip / U256::from(100 as u128)).as_u128();
        println!("{:?}", drip);
    }

    #[test]
    pub fn decay() {
        print!("{:?}", env::block_timestamp());
        let account_id = AccountId::from_str("gugu2029.testnet").unwrap();
        let mut account = Account::new(&account_id);
        account.data.insert("content_count".to_string(), "13".to_string());
        account.data.insert("one_day_timestamp".to_string(), "1697013468067413865".to_string());
        let mut drip = Drip::new();
        drip.accounts.insert(&account_id, &account);
        drip.set_content_drip(vec![], account_id.clone(), None);
        println!("{:?}", drip.accounts.get(&account_id).unwrap())
    }
}