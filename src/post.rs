use std::convert::TryInto;

use near_sdk::CryptoHash;

use crate::*;    

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug)]
pub struct Args {
    text: Option<String>,
    imgs: Option<Vec<String>>,
    video: Option<String>,
    audio: Option<String>,
    options: Option<Vec<Options>>
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

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug)]
pub enum Options {
    At(AccountId)
}

#[near_bindgen]
impl Community {
    pub fn add_post(&mut self, args: String) -> Base58CryptoHash {
        let args_obj: Args = serde_json::from_str(&args).unwrap();
        check_args(args_obj.text, args_obj.imgs, args_obj.video, args_obj.audio);
        
        let args = args.clone() + &bs58::encode(env::random_seed()).into_string();
        let hash = env::sha256(&args.clone().into_bytes());
        let hash: CryptoHash = hash[..].try_into().unwrap();
        self.public_bloom_filter.set(&WrappedHash::from(hash), true);
        let hash = Base58CryptoHash::from(hash);
        hash
    }

    pub fn add_encrypt_post(&mut self, encrypt_args: String, access: Access, text_sign: String, contract_id_sign: String) -> Base58CryptoHash {
        let pk: Vec<u8> = bs58::decode(self.public_key.clone()).into_vec().unwrap();
        
        let hash = env::sha256(&env::current_account_id().to_string().into_bytes());
        let sign: Vec<u8> = bs58::decode(contract_id_sign).into_vec().unwrap();
        verify(hash.clone(), sign.into(), pk.clone());

        let hash = env::sha256(&encrypt_args.clone().into_bytes());
        let sign: Vec<u8> = bs58::decode(text_sign).into_vec().unwrap();
        verify(hash.clone(), sign.into(), pk);

        let args: EncryptArgs = serde_json::from_str(&encrypt_args).unwrap();
        check_encrypt_args(args.text, args.imgs, args.video, args.audio);

        let encrypt_info = encrypt_args.clone() + &bs58::encode(env::random_seed()).into_string();
        let hash = env::sha256(&encrypt_info.clone().into_bytes());
        let hash: CryptoHash = hash[..].try_into().unwrap();
        self.encryption_bloom_filter.set(&WrappedHash::from(hash), true);
        let hash = Base58CryptoHash::from(hash);
        hash
    }

    pub fn like(&mut self, target_hash: Base58CryptoHash) {
        let target_hash = target_hash.try_to_vec().unwrap();
        let target_hash:[u8;32] = target_hash[..].try_into().unwrap();
        assert!(self.public_bloom_filter.check(&WrappedHash::from(target_hash)) || self.encryption_bloom_filter.check(&WrappedHash::from(target_hash)), "content not found");
    }

    pub fn unlike(&mut self, target_hash: Base58CryptoHash) {
        let target_hash = target_hash.try_to_vec().unwrap();
        let target_hash:[u8;32] = target_hash[..].try_into().unwrap();
        assert!(self.public_bloom_filter.check(&WrappedHash::from(target_hash)) || self.encryption_bloom_filter.check(&WrappedHash::from(target_hash)), "content not found");
    }

    pub fn add_comment(&mut self, args: String, target_hash: Base58CryptoHash) -> Base58CryptoHash {
        let target_hash = CryptoHash::from(target_hash);
        assert!(self.public_bloom_filter.check(&WrappedHash::from(target_hash)), "content not found");

        let args_obj: Args = serde_json::from_str(&args).unwrap();
        check_args(args_obj.text, args_obj.imgs, args_obj.video, args_obj.audio);

        let args = args.clone() + &env::block_height().to_string();
        let hash = env::sha256(&args.clone().into_bytes());
        let hash: CryptoHash = hash[..].try_into().unwrap();
        self.public_bloom_filter.set(&WrappedHash::from(hash), true);
        let hash = Base58CryptoHash::from(hash);
        hash
    }

    pub fn add_encrypt_comment(&mut self, encrypt_args: String, text_sign: String, contract_id_sign: String, target_hash: Base58CryptoHash) -> Base58CryptoHash {
        let target_hash = CryptoHash::from(target_hash);
        assert!(self.encryption_bloom_filter.check(&WrappedHash::from(target_hash)), "content not found");

        let pk: Vec<u8> = bs58::decode(self.public_key.clone()).into_vec().unwrap();
        
        let hash = env::sha256(&env::current_account_id().to_string().into_bytes());
        let sign: Vec<u8> = bs58::decode(contract_id_sign).into_vec().unwrap();
        verify(hash.clone(), sign.into(), pk.clone().into());

        let hash = env::sha256(&encrypt_args.clone().into_bytes());
        let sign: Vec<u8> = bs58::decode(text_sign).into_vec().unwrap();
        verify(hash.clone(), sign.into(), pk.into());

        let args: EncryptArgs = serde_json::from_str(&encrypt_args).unwrap();
        check_encrypt_args(args.text, args.imgs, args.video, args.audio);

        let encrypt_info = encrypt_args.clone() + &env::block_height().to_string();
        let hash = env::sha256(&encrypt_info.clone().into_bytes());
        let hash: CryptoHash = hash[..].try_into().unwrap();
        self.encryption_bloom_filter.set(&WrappedHash::from(hash), true);
        let hash = Base58CryptoHash::from(hash);
        hash

    }
}

    