use std::future::Pending;

use crate::{*, utils::{get_access_limit, verify_secp256k1, get}};

const ACCOUNT_ID: &str = "account_id";
const REGISTERED: &str = "registered";
const DRIP: &str = "drip";
const ONE_DAY_TIMESTAMP: &str = "one_day_timestamp";
const CONTENT_COUNT: &str = "content_count";
const TOTAL_CONTENT_COUNT: &str = "total_content_count";
const PERMANENT: &str = "permanent";

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub enum AssetKey {
    FT(AccountId),
    NFT(AccountId, Option<String>),               //nft token id, token id
    Drip((Option<AccountId>, AccountId))  //drip token id, contract id
}


fn get_account_decay(count: u64) -> u32 {
    if count < 20 {
        return 100
    } 
    40
}

#[derive(BorshDeserialize, BorshSerialize, Clone)]
#[derive(Debug)]
pub struct Account {
    data: HashMap<String, String>,
    // one_day_timestamp: u64,   //update after 24h
    // content_count: u64
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub enum Relationship {
    Or,
    And
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub struct OldAccess
{
    pub conditions: Vec<Condition>,
    pub relationship: Relationship,
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub struct Access
{
    pub condition: Condition,
    pub expire_duration: Option<U64>,
    pub is_payment: bool,
    pub options: Option<HashMap<String, String>>
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub enum Condition {
    FTCondition(FTCondition),
    NFTCondition(NFTCondition),
    DripCondition(DripCondition),
    SignCondition(SignCondition)
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
pub struct NFTCondition {
    pub token_id: AccountId,
    pub amount_to_access: U128,
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub struct DripCondition {
    pub token_id: Option<AccountId>,    //for total drips
    pub contract_id: AccountId,
    pub amount_to_access: U128,
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub struct SignCondition {
    pub message: String,
    pub public_key: String
}

impl Account {

    pub fn new(account_id: &AccountId) -> Self {
        let mut this = Self {
            data: HashMap::new(),
        };
        this.data.insert(ACCOUNT_ID.to_string(), account_id.to_string());

        this.data.insert(REGISTERED.to_string(), json!(false).to_string());
        this.data.insert(DRIP.to_string(), 0.to_string());
        this.data.insert(ONE_DAY_TIMESTAMP.to_string(), env::block_timestamp().to_string());
        this.data.insert(CONTENT_COUNT.to_string(), 0.to_string());
        this.data.insert(TOTAL_CONTENT_COUNT.to_string(), 0.to_string());
        
        this
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.data.insert(key.to_string(), value.to_string());
    }

    pub fn from_data(data: HashMap<String, String>) -> Self {
        Account {
            data
        }
    }

    pub fn data(&self) -> HashMap<String, String> {
        let mut data = self.data.clone();
        data.insert(REGISTERED.to_string(), json!(self.is_registered()).to_string());
        data
    }

    pub fn get_data<T>(&self, key: &str) -> Option<T> 
    where T: for<'a> Deserialize<'a>
    {
        let value = match self.data.get(key) {
            Some(v) => v,
            None => return None
        };
        match serde_json::from_str::<T>(value) {
            Ok(res) => Some(res),
            Err(_) => None
        }
    }

    pub fn set_data<T>(&mut self, key: &str, value: T) 
    where T: Serialize
    {
        self.data.insert(key.to_string(), json!(value).to_string());
    }

    pub fn account_id(&self) -> AccountId {
        AccountId::from_str(self.data.get(ACCOUNT_ID).unwrap()).unwrap()
    }

    pub fn registered(self) -> Self {
        if self.is_registered() {
            self
        } else {
            panic!("not registered")
        }
    }

    pub fn get_registered(self) -> Option<Self> {
        if self.is_registered() {
            Some(self)
        } else {
            None
        }
    }

    pub fn is_registered(&self) -> bool {
        let this: Community = env::state_read().unwrap();
        if this.owner_id == self.account_id() {
            return true
        }
        let access_limit = get_access_limit();
        let mut registered = match access_limit {
            AccessLimit::Free => return true,
            _ => self.get_data::<bool>(REGISTERED).unwrap()
        };
        if registered {
            if self.is_permanent() {
                return true
            }
            registered = match access_limit {
                AccessLimit::TokenLimit(access) => self.check_condition(&access),
                _ => return true
            }
        }
        registered
    }

    pub fn set_registered(&mut self, registered: bool) {
        self.data.insert(REGISTERED.to_string(), json!(registered).to_string());
    }

    pub fn is_permanent(&self) -> bool {  // deprecated for new communities
        match self.get_data::<bool>(PERMANENT) {
            Some(v) => v,
            None => !self.is_expired(None)
        }
    }

    // pub fn set_permanent(&mut self, is_permanent: bool) {
    //     self.data.insert(PERMANENT.to_string(), json!(is_permanent).to_string());
    // }

    pub fn is_expired(&self, access: Option<&Access>) -> bool {
        match access {
            Some(access) => {
                let key = json!(access.condition).to_string();
                match access.expire_duration {
                    Some(expire_duration) => {
                        let timestamp = self.get_data::<U64>(&key).unwrap_or(U64::from(0));
                        env::block_timestamp() > timestamp.0 + expire_duration.0
                    },
                    None => false
                }
            },
            None => {
                let timestamp = self.get_data::<U64>(PERMANENT).unwrap_or(U64::from(0));
                env::block_timestamp() > timestamp.0
            }
        }
    }

    pub fn set_timestamp(&mut self, access: Option<&Access>, timestamp: U64) {
        match access {
            Some(access) => {
                let key = json!(access.condition).to_string();
                self.data.insert(key, json!(timestamp).to_string());
            },
            None => {
                self.data.insert(PERMANENT.to_string(), json!(timestamp).to_string());
            }
        }
        
    }

    pub fn get_drip(&self) -> u128 {
        let drip = self.get_data::<U128>(DRIP).unwrap_or(U128::from(0));
        drip.0
    }

    pub fn increase_drip(&mut self, amount: u128) {
        let drip = self.get_drip();

        if let Some(new_drip) = drip.checked_add(amount) {
            let drip: U128 = new_drip.into();
            self.data.insert(DRIP.to_string(), json!(drip).to_string());
        }
    }

    pub fn decrease_drip(&mut self, amount: u128) {
        let mut drip = self.get_drip();
        if let Some(new_drip) = drip.checked_sub(amount) {
            drip = new_drip;
        } else {
            panic!("not enough balance");
        }
        let drip: U128 = drip.into();
        self.data.insert(DRIP.to_string(), json!(drip).to_string());
    }

    pub fn get_account_decay(&self) -> u32 {
        let timestamp: u64 = self.data.get(&ONE_DAY_TIMESTAMP.to_string()).unwrap_or(&env::block_timestamp().to_string()).parse().unwrap();
        let mut content_count = 0;
        if env::block_timestamp() - timestamp < 60 * 60 * 24 * 1000_000_000 {
            content_count = (self.data.get(&CONTENT_COUNT.to_string()).unwrap_or(&0.to_string())).parse().unwrap();
        }
        get_account_decay(content_count)
    }

    pub fn increase_content_count(&mut self) {
        let timestamp: u64 = self.data.get(&ONE_DAY_TIMESTAMP.to_string()).unwrap_or(&env::block_timestamp().to_string()).parse().unwrap();
        if env::block_timestamp() - timestamp > 60 * 60 * 24 * 1000_000_000 {
            self.data.insert(ONE_DAY_TIMESTAMP.to_string(), env::block_timestamp().to_string());
            self.data.insert(CONTENT_COUNT.to_string(), 0.to_string());
        }
        let content_count: u32 = (self.data.get(&CONTENT_COUNT.to_string()).unwrap_or(&0.to_string())).parse().unwrap();
        self.data.insert(CONTENT_COUNT.to_string(), (content_count + 1).to_string());
        let total_content_count: u32 = (self.data.get(&TOTAL_CONTENT_COUNT.to_string()).unwrap_or(&0.to_string())).parse().unwrap();
        self.data.insert(TOTAL_CONTENT_COUNT.to_string(), (total_content_count + 1).to_string());
    }


//////////////////////////////////////////////////////////  Deposit Part ////////////////////////////////////////////////////////////

    pub fn get_balance(&self, balance: &AssetKey) -> u128 {
        let drip: U128 = serde_json::from_str(self.data.get(&json!(balance).to_string()).unwrap_or(&"0".to_string())).unwrap_or(U128::from(0));
        drip.0
    }

    pub fn increase_balance(&mut self, asset: AssetKey, amount: u128) {
        let mut balance = self.get_balance(&asset);
        if let Some(new_balance) = balance.checked_add(amount) {
            balance = new_balance;
        }
        let balance: U128 = balance.into();
        self.data.insert(json!(asset).to_string(), json!(balance).to_string());
    }

    pub fn decrease_balance(&mut self, asset: AssetKey, amount: u128) {
        if let AssetKey::Drip((token_id, _)) = asset.clone() {
            if token_id == None {
                return
            }
        }
        let mut balance = self.get_balance(&asset);
        if let Some(new_balance) = balance.checked_sub(amount) {
            balance = new_balance;
        } else {
            panic!("not enough balance");
        }
        let balance: U128 = balance.into();
        self.data.insert(json!(asset).to_string(), json!(balance).to_string());
    }
////////////////////////////////////////////////////////  Condition Part ////////////////////////////////////////////////////////////////

    pub fn get_signature(&self, public_key: String) -> Option<(String, U64)> {
        let key = public_key + "_signature";
        self.get_data::<(String, U64)>(&key)
    }

    pub fn set_signature(&mut self, public_key: String, signature: String, timestamp: U64) {
        self.data.insert(public_key + "_signature", json!((signature, timestamp)).to_string());
    } 

    pub fn check_condition(&self, access: &Access) -> bool {
        if !self.is_expired(Some(access)) || !self.is_expired(None) {
            return true
        }
        match &access.condition {
            Condition::FTCondition(ft) => {
                self.get_balance(&AssetKey::FT(ft.token_id.clone())) >= ft.amount_to_access.0 && !access.is_payment
            }
            Condition::NFTCondition(_) => todo!(),
            Condition::DripCondition(drip) => {
                self.get_balance(&AssetKey::Drip((drip.token_id.clone(), drip.contract_id.clone()))) >= drip.amount_to_access.0 && !access.is_payment
            },
            Condition::SignCondition(sign) => {
                match self.get_signature(sign.public_key.clone()) {
                    Some(v) => {
                        let message = self.account_id().to_string() + &sign.message + &v.1.0.to_string();
                        let mut can = match sign.public_key.strip_prefix("0x") {
                            Some(pk) => verify_secp256k1(message.as_bytes().to_vec(), v.0.clone(), pk.to_string()),
                            None => verify(message.as_bytes(), v.0.as_bytes(), sign.public_key.as_bytes())
                        };
                        
                        if let Some(expire_duration) = access.expire_duration {
                            log!("{:?}, {:?}", env::block_timestamp(), v.1.0 + expire_duration.0);
                            can = can && (env::block_timestamp() < v.1.0 + expire_duration.0);
                        }
                        can
                    },
                    None => false
                }
            }
        }
    }


    pub fn set_condition(&mut self, access: &Access, options: Option<HashMap<String, String>>) -> bool {
        // let account = get_account(account_id);
        match &access.condition {
            Condition::FTCondition(ft) => {
                if self.get_balance(&AssetKey::FT(ft.token_id.clone())) >= ft.amount_to_access.0 {
                    if access.is_payment {
                        self.decrease_balance(AssetKey::FT(ft.token_id.clone()), ft.amount_to_access.0);
                    }
                    if let Some(expire_duration) = access.expire_duration{
                        let timestamp = U64::from(env::block_timestamp());
                        self.set_timestamp(Some(access), timestamp);
                    }
                    true
                } else {
                    false
                }
            }
            Condition::NFTCondition(_) => todo!(),
            Condition::DripCondition(drip) => {
                if self.get_balance(&AssetKey::Drip((drip.token_id.clone(), drip.contract_id.clone()))) >= drip.amount_to_access.0 {
                    if access.is_payment && drip.token_id.is_some() {
                        self.decrease_balance(AssetKey::Drip((drip.token_id.clone(), drip.contract_id.clone())), drip.amount_to_access.0);
                    }
                    true
                } else {
                    false
                }
            },
            Condition::SignCondition(sign_condition) => {
                let options = options.unwrap();
                let sign = options.get("sign").unwrap();
                let timestamp: U64 = u64::from_str(options.get("timestamp").unwrap()).unwrap().into();
                let message = self.account_id().to_string() + &sign_condition.message + &timestamp.0.to_string();
                let can = match sign_condition.public_key.strip_prefix("0x") {
                    Some(pk) => verify_secp256k1(message.as_bytes().to_vec(), sign.clone(), pk.to_string()),
                    None => verify(message.as_bytes(), sign.as_bytes(), sign_condition.public_key.as_bytes())
                };
                if !can {
                    return false
                }

                if let Some(expire_duration) = access.expire_duration{
                    assert!(timestamp.0 + expire_duration.0 >= env::block_timestamp(), "signature expired");
                    self.set_timestamp(Some(access), timestamp);
                }
                self.set_signature(sign_condition.public_key.clone(), sign.clone(), timestamp);
                true
            }
        }
    } 

}


mod test {
    use std::collections::HashMap;

    use near_sdk::{json_types::U64, serde_json::{json, self}};

    use crate::utils::{verify_secp256k1, verify};


    #[test]
    pub fn test() {
        let v = u64::from(123123 as u32);
        let msg = "hello";
        let account_id = "5566".to_string();
        let message = account_id + &msg + &v.to_string();
        println!("{:?}", message);
    }

    #[test]
    pub fn test_sign() {
        let sign = "da6845aaf49973e77412d98cf1fe3e403a69d6ac22e84ce834ac9bacf9f27acc08af57a2c97b9e8471e699d94159add90f4d0047ff50b22454cd24e98e442c821c".to_string();
        let message = "billkin.testnet has W0rdl3 #11693923940974000000".to_string();
        let public_key = "313043dbb2679ec57f83a46d6675bca8d2cc9c109bc82a2160f86ece7eb6a4d972aaa43af2d38db68ed2d480ddc29d917294d40b29f92d7418884700bea361e2".to_string();
        let mut pass = verify_secp256k1(message.as_bytes().to_vec(), sign.to_string(), public_key);
        println!("{:?}", pass);
    }
}