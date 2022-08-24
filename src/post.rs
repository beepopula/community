use std::convert::TryInto;

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

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug)]
pub struct Args {
    text: Option<String>,
    imgs: Option<Vec<String>>,
    video: Option<String>,
    audio: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug)]
pub struct EncryptArgs {
    text: Option<String>,
    imgs: Option<String>,
    video: Option<String>,
    audio: Option<String>
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub struct Hierarchy {
    pub target_hash: Base58CryptoHash,
    pub account_id: AccountId,
    pub options: Option<HashMap<String, String>>
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub struct Report {
    pub hierarchies: Vec<Hierarchy>,
    pub timestamp: U64,
    pub deposit: U128,
    pub del: Option<bool>
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub struct Access
{
    pub conditions: Vec<Condition>,
    pub relationship: Relationship
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub enum Condition {
    FTCondition(FTCondition)
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub struct FTCondition {
    pub token_id: AccountId,
    pub amount_to_access: U128
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub enum Relationship {
    Or,
    And
}


#[near_bindgen]
impl Community {

    pub fn add_item(&mut self, args: String, options: Option<HashMap<String, String>>) -> Base58CryptoHash {
        let sender_id = env::signer_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::AddContent), "not allowed");
        let args = sender_id.to_string() + &args.clone();
        let target_hash = set_content(args.clone(), sender_id.clone(), "".to_string(), options.clone(), None, &mut self.content_bloom_filter);
        self.drip.set_content_drip(Vec::new(), sender_id.clone());
        Event::log_add_content(args, vec![Hierarchy { 
            target_hash, 
            account_id: sender_id,
            options
        }]);
        target_hash
    }

    pub fn add_content(&mut self, args: String, hierarchies: Vec<Hierarchy>, options: Option<HashMap<String, String>>) -> Base58CryptoHash {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::AddContent), "not allowed");
        let args_obj: Args = serde_json::from_str(&args).unwrap();
        check_args(args_obj.text, args_obj.imgs, args_obj.video, args_obj.audio);

        assert!(hierarchies.len() < MAX_LEVEL, "error");

        let hash_prefix = get_content_hash(hierarchies.clone(), None, &self.content_bloom_filter).expect("content not found");
        let target_hash = set_content(args.clone(), sender_id.clone(), hash_prefix, options.clone(), None, &mut self.content_bloom_filter);

        self.drip.set_content_drip(hierarchies.clone(), sender_id.clone());
        Event::log_add_content(args, [hierarchies, vec![Hierarchy { 
            target_hash, 
            account_id: sender_id,
            options
        }]].concat());
        target_hash
    }

    pub fn add_encrypt_content(&mut self, encrypt_args: String, access: Option<Access>, hierarchies: Vec<Hierarchy>, options: Option<HashMap<String, String>>, nonce: String, sign: String) -> Base58CryptoHash {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::AddContent), "not allowed");
        let pk: Vec<u8> = bs58::decode(self.public_key.clone()).into_vec().unwrap();

        let hash = env::sha256(&(encrypt_args.clone() + &nonce).into_bytes());
        let sign: Vec<u8> = bs58::decode(sign).into_vec().unwrap();
        verify(hash.clone(), sign.into(), pk);

        let args: EncryptArgs = serde_json::from_str(&encrypt_args).unwrap();
        check_encrypt_args(args.text, args.imgs, args.video, args.audio);

        assert!(hierarchies.len() < MAX_LEVEL, "error");

        let hash_prefix = get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_bloom_filter).expect("content not found");

        let target_hash = set_content(encrypt_args.clone(), sender_id.clone(), hash_prefix, options.clone(), Some("encrypted".to_string()), &mut self.content_bloom_filter);
        
        self.drip.set_content_drip(hierarchies.clone(), sender_id.clone());
        Event::log_add_content(encrypt_args, [hierarchies, vec![Hierarchy { 
            target_hash, 
            account_id: sender_id,
            options
        }]].concat());
        target_hash
    }

    pub fn like(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::Like), "not allowed");
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_bloom_filter).expect("content not found")
        };
        let hash = env::sha256(&(sender_id.to_string() + "like" + &hierarchy_hash.to_string()).into_bytes());
        let exist = self.relationship_bloom_filter.check_and_set(&hash, 0);
        if !exist {
            self.drip.set_like_drip(hierarchies.clone(), sender_id);
        }
        Event::log_like_content(hierarchies);
    }

    pub fn unlike(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::Unlike), "not allowed");
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_bloom_filter).expect("content not found")
        };

        let hash = env::sha256(&(sender_id.to_string() + "like" + &hierarchy_hash.to_string()).into_bytes());
        let hash: CryptoHash = hash[..].try_into().unwrap();
        assert!(self.relationship_bloom_filter.check_and_set(&hash, 0), "illegal");
        Event::log_unlike_content(hierarchies);
    }

    #[payable]
    pub fn report(&mut self, hierarchies: Vec<Hierarchy>) {
        assert!(5_000_000_000_000_000_000_000_000 <= env::attached_deposit(), "not enough deposit");

        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::Report), "not allowed");
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_bloom_filter).expect("content not found")
        };
        let mut account = self.reports.get(&sender_id).unwrap_or(UnorderedMap::new((sender_id.to_string() + "report").as_bytes()));
        account.insert(&Base58CryptoHash::try_from(hierarchy_hash).unwrap(), &Report{ 
            hierarchies,
            timestamp: env::block_timestamp().into(),
            deposit: env::attached_deposit().into(), 
            del: None 
        });
        self.reports.insert(&sender_id, &account);
    }

    pub fn report_confirm(&mut self, account_id: AccountId, hierarchies: Vec<Hierarchy>, del: bool) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::ReportConfirm), "not allowed");
        assert!(account_id != sender_id, "signer_id = account_id");

        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_bloom_filter).expect("content not found")
        };

        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap();
        let mut account = self.reports.get(&account_id).unwrap();
        let mut report = account.get(&hierarchy_hash).unwrap();
        assert!(report.del.is_none(), "resolved");
        report.del = Some(del); 
        account.insert(&hierarchy_hash, &report);
        self.reports.insert(&account_id, &account);

        if del == true {
            let hash = 
            self.content_bloom_filter.del(&hierarchy_hash.try_to_vec().unwrap());

            self.drip.set_report_drip(hierarchies, account_id);
            self.drip.set_report_confirm_drip(sender_id);
        }
    }

    pub fn del_content(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::DelContent), "not allowed");
        assert!(hierarchies.get(hierarchies.len() - 1).unwrap().account_id == sender_id, "not content owner");

        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_bloom_filter).expect("content not found")
        };
        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap().try_to_vec().unwrap();
        self.content_bloom_filter.del(&hierarchy_hash);
        Event::log_del_content(hierarchies);
    }

    pub fn share_view(&mut self, hierarchies: Vec<Hierarchy>, inviter_id: AccountId) {
        let sender_id = env::predecessor_account_id();
        assert!(inviter_id != sender_id, "failed");
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_bloom_filter).expect("content not found")
        };

        let view_hash = env::sha256(&(sender_id.to_string() + "viewed" + &hierarchy_hash + "through" + &inviter_id.to_string()).into_bytes());
        let view_hash: CryptoHash = view_hash[..].try_into().unwrap();
        let exist = self.relationship_bloom_filter.check_and_set(&view_hash, 0);
        if !exist {
            self.drip.set_share_drip(hierarchies, inviter_id)
        }
    }

    pub fn redeem_report_deposit(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), None, &self.content_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), Some("encrypted".to_string()), &self.content_bloom_filter).expect("content not found")
        };
        let mut account = self.reports.get(&sender_id).unwrap_or(UnorderedMap::new((sender_id.to_string() + "report").as_bytes()));
        let key = Base58CryptoHash::try_from(hierarchy_hash).unwrap();
        let report = account.get(&key).unwrap();
        assert!(report.del == Some(true) || env::block_timestamp() - report.timestamp.0 > 2_592_000_000_000_000, "redeem failed");
        Promise::new(sender_id.clone()).transfer(report.deposit.0);
        account.remove(&key);
        self.reports.insert(&sender_id, &account);
    }
}