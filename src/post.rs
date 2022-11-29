use std::{convert::TryInto, str::FromStr};

use near_sdk::CryptoHash;

use crate::*;    
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

    // pub fn add_item(&mut self, args: String, options: Option<HashMap<String, String>>) -> Base58CryptoHash {
    //     let sender_id = env::signer_account_id();
    //     assert!(self.can_execute_action(sender_id.clone(), Permission::AddContent(0)), "not allowed");
    //     let target_hash = set_content(args.clone(), sender_id.clone(), "".to_string(), options.clone(), None, &mut self.content_tree);
    //     let drips = self.drip.set_content_drip(Vec::new(), sender_id.clone(), None);
    //     Event::log_add_content(
    //         args, 
    //         vec![Hierarchy { 
    //             target_hash, 
    //             account_id: sender_id,
    //             options
    //         }], 
    //         Some(json!({
    //             "drips": drips
    //         }).to_string())
    //     );
    //     target_hash
    // }

    pub fn add_content(&mut self, args: String, hierarchies: Vec<Hierarchy>, options: Option<HashMap<String, String>>) -> Base58CryptoHash {
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

        let hash_prefix = get_content_hash(hierarchies.clone(), None, &self.content_tree).expect("content not found");
        let target_hash = set_content(args.clone(), sender_id.clone(), hash_prefix.clone(), options.clone(), None, &mut self.content_tree);

        let mut prev_content_count = 0;
        if hierarchies.len() > 0 {
            let prev_hash = CryptoHash::from(Base58CryptoHash::try_from(hash_prefix.clone()).unwrap());
            let mut val = self.content_tree.get(&prev_hash).unwrap();
            prev_content_count = val.clone();
            val += 1;
            if val > 3 {
                val = 3;
            }
            self.content_tree.set(&prev_hash, val)
        }

        let drips = self.drip.set_content_drip(hierarchies.clone(), sender_id.clone(), Some(prev_content_count));
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
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_tree) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_tree).expect("content not found")
        };
        let hash = env::sha256(&(sender_id.to_string() + "like" + &hierarchy_hash.to_string()).into_bytes());
        let exist = self.relationship_tree.check_and_set(&hash, 0);
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
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_tree) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_tree).expect("content not found")
        };

        let hash = env::sha256(&(sender_id.to_string() + "like" + &hierarchy_hash.to_string()).into_bytes());
        let hash: CryptoHash = hash[..].try_into().unwrap();
        assert!(self.relationship_tree.check_and_set(&hash, 0), "illegal");
        Event::log_unlike_content(hierarchies, None);
    }

    pub fn del_content(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::DelContent), "not allowed");
        assert!(hierarchies.get(hierarchies.len() - 1).unwrap().account_id == sender_id, "not content owner");

        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_tree) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_tree).expect("content not found")
        };
        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap().try_to_vec().unwrap();
        self.content_tree.del(&hierarchy_hash);
        Event::log_del_content(hierarchies, None);
    }

    pub fn share_view(&mut self, hierarchies: Vec<Hierarchy>, inviter_id: AccountId) {
        let sender_id = env::predecessor_account_id();
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_tree) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_tree).expect("content not found")
        };

        let view_hash = env::sha256(&(sender_id.to_string() + "viewed" + &hierarchy_hash + "through" + &inviter_id.to_string()).into_bytes());
        let view_hash: CryptoHash = view_hash[..].try_into().unwrap();
        let exist = self.relationship_tree.check_and_set(&view_hash, 0);
        let mut drips = Vec::new();
        if !exist {
            drips = self.drip.set_share_drip(hierarchies.clone(), inviter_id.clone());
        }

        Event::log_share_content(
            hierarchies,
            inviter_id,
            sender_id,
            Some(json!({
                "drips": drips
            }).to_string())
        );
    }

    pub fn report_confirm(&mut self, hierarchies: Vec<Hierarchy>, report: Report) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::ReportConfirm), "not allowed");

        let hierarchy = hierarchies.get(hierarchies.len() - 1).unwrap();
        assert!(self.get_user_mod_level(&hierarchy.account_id) < self.get_user_mod_level(&sender_id), "not allowed");

        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_tree) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_tree).expect("content not found")
        };

        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap();
        let accounts = self.reports.get(&hierarchy_hash).unwrap_or(HashSet::new());
        let mut drips = vec![];
        match report {
            Report::Approve => {
                self.content_tree.del(&hierarchy_hash.try_to_vec().unwrap());
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
                for account_id in accounts {
                    if account_id == sender_id {
                        continue
                    }
                    drips = self.drip.set_report_refund_drip(hierarchies.clone(), account_id);
                }
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
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_tree) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_tree).expect("content not found")
        };
        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap();

        self.content_tree.del(&hierarchy_hash.try_to_vec().unwrap());
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

    let hash_prefix = get_content_hash(hierarchies.clone(), None, &contract.content_tree).expect("content not found");
    let target_hash = set_content(json!(args.clone()).to_string(), sender_id.clone(), hash_prefix.clone(), options.clone(), None, &mut contract.content_tree);

    let mut prev_content_count = 0;
    if hierarchies.len() > 0 {
        let prev_hash = CryptoHash::from(Base58CryptoHash::try_from(hash_prefix.clone()).unwrap());
        let mut val = contract.content_tree.get(&prev_hash).unwrap();
        prev_content_count = val.clone();
        val += 1;
        if val > 3 {
            val = 3;
        }
        contract.content_tree.set(&prev_hash, val)
    }

    let drips = contract.drip.set_content_drip(hierarchies.clone(), sender_id.clone(), Some(prev_content_count));
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
    use std::convert::TryInto;


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

}