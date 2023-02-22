use std::{convert::TryInto, str::FromStr};

use near_sdk::CryptoHash;

use crate::{*, utils::{get, check_and_set, check}};    
use utils::{get_content_hash, set_content};

// #[derive(Serialize, Deserialize)]
// #[serde(crate = "near_sdk::serde")]
// #[derive(Debug)]
// pub struct EncryptInfo {
//     content: EncryptArgs,
//     access: Access
// }

// #[derive(Serialize, Deserialize)]
// #[serde(crate = "near_sdk::serde")]
// #[derive(Debug)]
// pub struct Args {
//     text: Option<String>,
//     imgs: Option<Vec<String>>,
//     video: Option<String>,
//     audio: Option<String>,
// }

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub struct Hierarchy {
    pub target_hash: Base58CryptoHash,
    pub account_id: AccountId,
    pub options: Option<HashMap<String, String>>
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub struct InputArgs {
    hierarchies: Vec<Hierarchy>, 
    options: Option<HashMap<String, String>>
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub enum Report {
    Approve,
    Disapprove,
    Ignore
}


#[near_bindgen]
impl Community {

    pub fn add_content(&mut self, args: String, hierarchies: Vec<Hierarchy>, options: Option<HashMap<String, String>>) -> Base58CryptoHash {
        // TODO: avoid hash collision through a loop
        
        let sender_id = env::predecessor_account_id();
        let mut check_encryption_content_permission = false;
        if let Some(options) = options.clone() {
            if options.contains_key("access") {
                check_encryption_content_permission = true
            } 
        } 

        if check_encryption_content_permission {
            assert!(self.can_execute_action(sender_id.clone(), Permission::AddEncryptContent(hierarchies.len() as u8)), "not allowed");
        } else {
            assert!(self.can_execute_action(sender_id.clone(), Permission::AddContent(hierarchies.len() as u8)), "not allowed");
        }

        assert!(hierarchies.len() < MAX_LEVEL, "error");

        let hash_prefix = get_content_hash(hierarchies.clone(), None, false).expect("content not found");
        let target_hash = set_content(args.clone(), sender_id.clone(), hash_prefix.clone(), options.clone(), None);

        let mut prev_content_count = None;
        if hierarchies.len() > 0 {
            let prev_hash = CryptoHash::from(Base58CryptoHash::try_from(hash_prefix.clone()).unwrap()).to_vec();
            let mut val: u8 = get(&prev_hash).unwrap();
            prev_content_count = Some(val.clone());
            val += 1;
            if val > 1 {
                val = 1;
            }
            set(&prev_hash, val)
        }

        let drips = self.drip.set_content_drip(hierarchies.clone(), sender_id.clone(), prev_content_count);
        Event::log_add_content(
            args, 
            [hierarchies, vec![Hierarchy { 
                target_hash, 
                account_id: sender_id,
                options
            }]].concat(),
            Some(json!({
                "drips": drips
            }).to_string())
        );
        target_hash
    }

    pub fn like(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::Like), "not allowed");
        let hierarchy_hash = get_content_hash(hierarchies.clone(), None, false).expect("content not found");
        let hash = env::sha256(&(sender_id.to_string() + "like" + &hierarchy_hash.to_string()).into_bytes());
        let exist = check_and_set(&hash, 0);
        let mut drips = Vec::new();
        if !exist {
            drips = self.drip.set_like_drip(hierarchies.clone(), sender_id);
        }
        Event::log_like_content(
            hierarchies,
            Some(json!({
                "drips": drips
            }).to_string())
        );
    }

    pub fn unlike(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::Unlike), "not allowed");
        let hierarchy_hash = get_content_hash(hierarchies.clone(), None, false).expect("content not found");

        let hash = env::sha256(&(sender_id.to_string() + "like" + &hierarchy_hash.to_string()).into_bytes());
        assert!(check_and_set(&hash, 0), "illegal");
        Event::log_unlike_content(hierarchies, None);
    }

    pub fn del_content(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::DelContent), "not allowed");
        assert!(hierarchies.get(hierarchies.len() - 1).unwrap().account_id == sender_id, "not content owner");

        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, false) {
            Some(v) => v,
            None => return
        };
        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap();
        let hierarchy_hash = CryptoHash::from(hierarchy_hash).to_vec();
        remove(&hierarchy_hash);
        Event::log_del_content(hierarchies, None);
    }

    pub fn report_confirm(&mut self, hierarchies: Vec<Hierarchy>, report: Report) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::ReportConfirm), "not allowed");

        let hierarchy = hierarchies.get(hierarchies.len() - 1).unwrap();
        assert!(self.get_user_mod_level(&hierarchy.account_id) < self.get_user_mod_level(&sender_id) || sender_id == self.owner_id, "not allowed");

        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, false) {
            Some(v) => v,
            None => {
                self.internal_report_refund(hierarchies);
                return
            }
        };
        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap();
        let accounts = self.reports.get(&hierarchy_hash).unwrap_or(HashSet::new());
        let mut drips = vec![];
        match report {
            Report::Approve => {
                remove(&CryptoHash::from(hierarchy_hash).to_vec());
                for account_id in accounts {
                    if account_id == sender_id {
                        continue
                    }
                    drips = self.drip.set_report_drip(hierarchies.clone(), account_id);
                }
                Event::log_del_content(
                    hierarchies,
                    Some(json!({
                        "drips": drips
                    }).to_string())
                );
                
                //self.drip.set_report_confirm_drip(sender_id);
            },
            Report::Disapprove => {

            },
            Report::Ignore => {
                self.internal_report_refund(hierarchies);
            }
        }
        
    }

    pub fn revoke_report(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        let hierarchy_hash = get_content_hash(hierarchies.clone(), None, true).expect("content not found");
        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap();
        if let Some(accounts) = self.reports.get(&hierarchy_hash) {
            if let Some(_) = accounts.get(&sender_id) {
                let drips = self.drip.set_report_refund_drip(hierarchies.clone(), sender_id);
                Event::log_refund(
                    Some(json!({
                        "drips": drips
                    }).to_string())
                );
            }
        }
    }

    pub fn del_others_content(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::DelOthersContent), "not allowed");

        let hierarchy = hierarchies.get(hierarchies.len() - 1).unwrap();
        assert!(self.get_user_mod_level(&hierarchy.account_id) < self.get_user_mod_level(&sender_id), "not allowed");
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, false) {
            Some(v) => v,
            None => return
        };
        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap();
        remove(&CryptoHash::from(hierarchy_hash).to_vec());
        Event::log_del_content(hierarchies, None);
    }
}


#[no_mangle]
pub extern "C" fn add_long_content() {
    env::setup_panic_hook();
    let raw_input = env::input().unwrap();
    let args_length = u32::from_le_bytes(raw_input[0..4].try_into().unwrap());
    let args: InputArgs = serde_json::from_slice(&raw_input[4..(args_length as usize + 4)]).unwrap();
    let hierarchies = args.hierarchies.clone();
    let options = args.options.clone();
    let sender_id = env::predecessor_account_id();
    let mut contract: Community = env::state_read().unwrap();
    assert!(contract.can_execute_action(sender_id.clone(), Permission::AddContent(hierarchies.len() as u8)), "not allowed");

    assert!(hierarchies.len() < MAX_LEVEL, "error");

    let hash_prefix = get_content_hash(hierarchies.clone(), None, false).expect("content not found");
    let target_hash = set_content(json!(args.clone()).to_string(), sender_id.clone(), hash_prefix.clone(), options.clone(), None);

    let mut prev_content_count = None;
    if hierarchies.len() > 0 {
        let prev_hash = CryptoHash::from(Base58CryptoHash::try_from(hash_prefix.clone()).unwrap()).to_vec();
        let mut val = get::<u8>(&prev_hash).unwrap();
        prev_content_count = Some(val.clone());
        val += 1;
        if val > 3 {
            val = 3;
        }
        set(&prev_hash, val)
    }

    let drips = contract.drip.set_content_drip(hierarchies.clone(), sender_id.clone(), prev_content_count);
    Event::log_add_content(
        "".to_string(), 
        [hierarchies, vec![Hierarchy { 
            target_hash, 
            account_id: sender_id,
            options
        }]].concat(),
        Some(json!({
            "drips": drips
        }).to_string())
    );
    // String::from(&target_hash)
}


#[cfg(test)]
mod tests {
    use std::{convert::TryInto, str::FromStr};

    use near_sdk::{AccountId, env, json_types::Base58CryptoHash};

    use super::Hierarchy;


    #[test]
    pub fn test() {
        let raw_input: Vec<u8> = vec![0x12, 0x00, 0x00, 0x00, 0x7b, 0x22, 0x68, 0x69, 0x65, 0x72, 0x61, 0x72, 0x63, 0x68, 0x69, 0x65, 0x73, 0x22, 0x3a, 0x5b, 0x5d, 0x7d, 0x31, 0x32, 0x33];
        let raw_args_length: [u8; 4] = raw_input[0..4].try_into().unwrap();
        let args_length = u32::from_le_bytes(raw_args_length);
        print!("{:?}", args_length);
        // let args: InputArgs = serde_json::from_slice(&raw_input[4..(args_length as usize + 4)]).unwrap();
        // let hierarchies = args.hierarchies.clone();
        // let options = args.options.clone();
    }

    fn get_content_hash(hierarchies: Vec<Hierarchy>, extra: Option<String>) -> Option<String> {
        let mut hash_prefix = "".to_string();
        for (_, hierarchy) in hierarchies.iter().enumerate() {
            let mut hierarchy_str = hash_prefix + &hierarchy.account_id.to_string() + &String::from(&hierarchy.target_hash);
            let hierarchy_hash = env::sha256(&hierarchy_str.into_bytes());
            let hierarchy_hash: [u8;32] = hierarchy_hash[..].try_into().unwrap();
            hash_prefix = String::from(&Base58CryptoHash::from(hierarchy_hash));
        }
        Some(hash_prefix)
    }

    #[test]
    pub fn test_like() {
        let hierarchies = vec![
            Hierarchy {
                target_hash: Base58CryptoHash::from_str("5EVZZTdCcMQ6Di5fq2Zw1HuFd4chQ9KK4DG3byVwiSyp").unwrap(),
                account_id: AccountId::from_str("tokenq.testnet").unwrap(),
                options: None
            },
            Hierarchy {
                target_hash: Base58CryptoHash::from_str("6UjBiuEVunhN5t36wERYbpVTdRW5oCYjGKGp23hmQRkF").unwrap(),
                account_id: AccountId::from_str("tokenq.testnet").unwrap(),
                options: None
            },
            Hierarchy {
                target_hash: Base58CryptoHash::from_str("5vXhcBEwVqSeYzkDbckEfrvxMG3zp9Y9QkYWqfStn6aE").unwrap(),
                account_id: AccountId::from_str("tokenq.testnet").unwrap(),
                options: None
            },
        ];
        let hierarchy_hash = get_content_hash(hierarchies.clone(), None).unwrap();
        let hash = env::sha256(&("bhc11.testnet".to_string() + "like" + &hierarchy_hash.to_string()).into_bytes());
        println!("{:?}", hash)
    }

    #[test]
    pub fn test_share_view() {
        let hierarchies = vec![
            Hierarchy {
                target_hash: Base58CryptoHash::from_str("5EVZZTdCcMQ6Di5fq2Zw1HuFd4chQ9KK4DG3byVwiSyp").unwrap(),
                account_id: AccountId::from_str("tokenq.testnet").unwrap(),
                options: None
            },
            Hierarchy {
                target_hash: Base58CryptoHash::from_str("6UjBiuEVunhN5t36wERYbpVTdRW5oCYjGKGp23hmQRkF").unwrap(),
                account_id: AccountId::from_str("tokenq.testnet").unwrap(),
                options: None
            },
            Hierarchy {
                target_hash: Base58CryptoHash::from_str("5vXhcBEwVqSeYzkDbckEfrvxMG3zp9Y9QkYWqfStn6aE").unwrap(),
                account_id: AccountId::from_str("tokenq.testnet").unwrap(),
                options: None
            }
        ];
        let hierarchy_hash = get_content_hash(hierarchies.clone(), None).unwrap();
        let hash = env::sha256(&("testdrip0830.testnet".to_string() + "viewed" + &hierarchy_hash + "through" + "tokenq.testnet").into_bytes());
        println!("{:?}", hash)
        
    }

}