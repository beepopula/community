use crate::{*, utils::get_access_limit};

const REGISTERED: &str = "registered";
const DRIP: &str = "drip";
const ONE_DAY_TIMESTAMP: &str = "one_day_timestamp";
const CONTENT_COUNT: &str = "content_count";
const TOTAL_CONTENT_COUNT: &str = "total_content_count";
const EXPIRED_AT: &str = "expired_at";

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
    if count <= 10 {
        return 100
    } 
    40
}

#[derive(BorshDeserialize, BorshSerialize, Clone)]
#[derive(Debug)]
pub struct Account {
    pub data: HashMap<String, String>,
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
    pub is_payment: bool
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
        this.data.insert(REGISTERED.to_string(), json!(false).to_string());
        this.data.insert(DRIP.to_string(), 0.to_string());
        this.data.insert(ONE_DAY_TIMESTAMP.to_string(), env::block_timestamp().to_string());
        this.data.insert(CONTENT_COUNT.to_string(), 0.to_string());
        this.data.insert(TOTAL_CONTENT_COUNT.to_string(), 0.to_string());
        
        this
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
        match get_access_limit() {
            AccessLimit::Free => true,
            AccessLimit::Registry => self.get_data::<bool>(REGISTERED).unwrap(),
            AccessLimit::TokenLimit(access) => self.get_data::<bool>(REGISTERED).unwrap() || self.check_condition(&access)
        }
    }

    pub fn set_registered(&mut self, registered: bool) {
        self.data.insert(REGISTERED.to_string(), json!(registered).to_string());
    }

    pub fn is_expired(&self) -> bool {
        let expired_at = self.get_data::<U64>(EXPIRED_AT).unwrap_or(U64::from(0));
        env::block_timestamp() > expired_at.0
    }

    pub fn set_expired(&mut self, expired_at: U64) {
        self.data.insert(EXPIRED_AT.to_string(), json!(expired_at).to_string());
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

    pub fn get_signature(&self, public_key: String) -> Option<String> {
        let key = public_key + "_signature";
        self.get_data::<String>(&key)
    }

    pub fn set_signature(&mut self, public_key: String, signature: String) {
        self.data.insert(public_key, signature);
    } 

    pub fn check_condition(&self, access: &Access) -> bool {
        if let Some(expire_duration) = access.expire_duration{
            if !self.is_expired() {
                return true
            }
        }

        match &access.condition {
            Condition::FTCondition(ft) => {
                self.get_balance(&AssetKey::FT(ft.token_id.clone())) >= ft.amount_to_access.0 && !access.is_payment
            }
            Condition::NFTCondition(_) => todo!(),
            Condition::DripCondition(drip) => {
                self.get_balance(&AssetKey::Drip((drip.token_id.clone(), drip.contract_id.clone()))) >= drip.amount_to_access.0
            },
            Condition::SignCondition(sign) => {
                match self.get_signature(sign.public_key.clone()) {
                    Some(v) => {
                        verify(sign.message.as_bytes(), v.as_bytes(), sign.public_key.as_bytes())
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
                        let expired_at = U64::from(env::block_timestamp() + expire_duration.0);
                        self.set_expired(expired_at);
                    }
                    true
                } else {
                    false
                }
            }
            Condition::NFTCondition(_) => todo!(),
            Condition::DripCondition(drip) => {
                self.get_balance(&AssetKey::Drip((drip.token_id.clone(), drip.contract_id.clone()))) >= drip.amount_to_access.0
            },
            Condition::SignCondition(sign) => {
                match &self.get_signature(sign.public_key.clone()) {
                    Some(v) => {
                        verify(sign.message.as_bytes(), v.as_bytes(), sign.public_key.as_bytes())
                    },
                    None => {
                        match &options {
                            Some(map) => match map.get("sign") {
                                Some(v) => {
                                    if verify(sign.message.as_bytes(), v.as_bytes(), sign.public_key.as_bytes()) {
                                        if let Some(expire_duration) = access.expire_duration{
                                            let expired_at = U64::from(env::block_timestamp() + expire_duration.0);
                                            self.set_expired(expired_at);
                                        }
                                        true
                                    } else {
                                        false
                                    }
                                },
                                None => false
                            },
                            None => false
                        }
                    }
                }
                
            }
        }
    }



}