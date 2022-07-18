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


#[near_bindgen]
impl Community {
    pub fn add_content(&mut self, args: String, hierarchies: Vec<Hierarchy>) -> Base58CryptoHash {
        let sender_id = env::predecessor_account_id();
        let args_obj: Args = serde_json::from_str(&args).unwrap();
        check_args(args_obj.text, args_obj.imgs, args_obj.video, args_obj.audio);

        assert!(hierarchies.len() < MAX_LEVEL, "error");

        let hash_prefix = get_content_hash(hierarchies.clone(), &self.public_bloom_filter).expect("content not found");
        let target_hash = set_content(args.clone(), sender_id.clone(), hash_prefix, &mut self.public_bloom_filter);

        self.drip.set_content_drip(hierarchies.clone(), sender_id.clone());
        Event::log_add_content(args, [hierarchies, vec![Hierarchy { 
            target_hash, 
            account_id: sender_id
        }]].concat());
        target_hash
    }

    pub fn add_encrypt_content(&mut self, encrypt_args: String, access: Option<Access>, hierarchies: Vec<Hierarchy>, nonce: String, sign: String) -> Base58CryptoHash {
        let sender_id = env::predecessor_account_id();
        let pk: Vec<u8> = bs58::decode(self.public_key.clone()).into_vec().unwrap();

        let hash = env::sha256(&(encrypt_args.clone() + &nonce).into_bytes());
        let sign: Vec<u8> = bs58::decode(sign).into_vec().unwrap();
        verify(hash.clone(), sign.into(), pk);

        let args: EncryptArgs = serde_json::from_str(&encrypt_args).unwrap();
        check_encrypt_args(args.text, args.imgs, args.video, args.audio);

        assert!(hierarchies.len() < MAX_LEVEL, "error");

        let hash_prefix = get_content_hash(hierarchies.clone(), &self.encryption_bloom_filter).expect("content not found");

        let target_hash = set_content(encrypt_args.clone(), sender_id.clone(), hash_prefix, &mut self.encryption_bloom_filter);
        
        self.drip.set_content_drip(hierarchies.clone(), sender_id.clone());
        Event::log_add_content(encrypt_args, [hierarchies, vec![Hierarchy { 
            target_hash, 
            account_id: sender_id
        }]].concat());
        target_hash
    }

    pub fn like(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), &self.public_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), &self.encryption_bloom_filter).expect("content not found")
        };
        let hash = env::sha256(&(sender_id.to_string() + "like" + &hierarchy_hash.to_string()).into_bytes());
        let hash: CryptoHash = hash[..].try_into().unwrap();
        let exist = self.relationship_bloom_filter.check_and_set(&WrappedHash::from(hash), true);
        if !exist {
            self.drip.set_like_drip(hierarchies.clone(), sender_id);
        }
        Event::log_like_content(hierarchies);
    }

    pub fn unlike(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), &self.public_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), &self.encryption_bloom_filter).expect("content not found")
        };

        let hash = env::sha256(&(sender_id.to_string() + "like" + &hierarchy_hash.to_string()).into_bytes());
        let hash: CryptoHash = hash[..].try_into().unwrap();
        assert!(self.relationship_bloom_filter.check_and_set(&WrappedHash::from(hash), false), "illegal");
        Event::log_unlike_content(hierarchies);
    }

    #[payable]
    pub fn report(&mut self, hierarchies: Vec<Hierarchy>) {
        assert!(5_000_000_000_000_000_000_000_000 <= env::attached_deposit(), "not enough deposit");

        let sender_id = env::predecessor_account_id();
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), &self.public_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), &self.encryption_bloom_filter).expect("content not found")
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

    pub fn del_content(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        assert!(hierarchies.get(hierarchies.len() - 1).unwrap().account_id == sender_id, "not content owner");

        let hierarchy_hash = match get_content_hash(hierarchies.clone(), &self.public_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), &self.encryption_bloom_filter).expect("content not found")
        };
        let hierarchy_hash = Base58CryptoHash::try_from(hierarchy_hash).unwrap().try_to_vec().unwrap();
        let hierarchy_hash: CryptoHash = hierarchy_hash[..].try_into().unwrap();
        self.public_bloom_filter.set(&WrappedHash::from(hierarchy_hash), false);
        self.encryption_bloom_filter.set(&WrappedHash::from(hierarchy_hash), false);
        Event::log_del_content(hierarchies);
    }

    pub fn share(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), &self.public_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies.clone(), &self.encryption_bloom_filter).expect("content not found")
        };
        let share_hash = env::sha256(&(sender_id.to_string() + "shared" + &hierarchy_hash).into_bytes());
        let share_hash: CryptoHash = share_hash[..].try_into().unwrap();
        let exist = self.relationship_bloom_filter.check_and_set(&WrappedHash::from(share_hash), true);
        if !exist {
            self.drip.set_share_drip(hierarchies, sender_id)
        }
    }

    pub fn share_view(&mut self, hierarchies: Vec<Hierarchy>, inviter_id: AccountId) {
        let sender_id = env::predecessor_account_id();
        assert!(inviter_id != sender_id, "failed");
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), &self.public_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies, &self.encryption_bloom_filter).expect("content not found")
        };
        let share_hash = env::sha256(&(sender_id.to_string() + "shared" + &hierarchy_hash).into_bytes());
        let share_hash: CryptoHash = share_hash[..].try_into().unwrap();
        assert!(self.relationship_bloom_filter.check(&WrappedHash::from(share_hash)), "not shared");

        let share_hash = String::from(&Base58CryptoHash::from(share_hash));
        let view_hash = env::sha256(&(sender_id.to_string() + "viewed" + &share_hash).into_bytes());
        let view_hash: CryptoHash = view_hash[..].try_into().unwrap();
        let exist = self.relationship_bloom_filter.check_and_set(&WrappedHash::from(view_hash), true);
        if !exist {
            self.drip.set_share_view_drip(inviter_id)
        }
    }

    pub fn redeem_report_deposit(&mut self, hierarchies: Vec<Hierarchy>) {
        let sender_id = env::predecessor_account_id();
        let hierarchy_hash = match get_content_hash(hierarchies.clone(), &self.public_bloom_filter) {
            Some(v) => v,
            None => get_content_hash(hierarchies, &self.encryption_bloom_filter).expect("content not found")
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