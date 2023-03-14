use crate::{*, utils::get_access_limit};


const REGISTERED: &str = "registered";
const DRIP: &str = "drip";
const TOTAL_DRIP: &str = "total_drip";
const ONE_DAY_TIMESTAMP: &str = "one_day_timestamp";
const CONTENT_COUNT: &str = "content_count";
const TOTAL_CONTENT_COUNT: &str = "total_content_count";

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub enum AssetKey {
    FT(AccountId),
    NFT(AccountId),
    Drip((Option<AccountId>, AccountId))
}


fn get_account_decay(count: u64) -> u32 {
    if count <= 10 {
        return 100
    } 
    40
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Debug)]
pub struct Account {
    pub data: HashMap<String, String>,
    // one_day_timestamp: u64,   //update after 24h
    // content_count: u64
}

impl Default for Account {
    fn default() -> Self {
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
}

impl Account {

    pub fn new() -> Self {
        let mut this = Self {
            data: HashMap::new(),
        };
        this.data.insert(REGISTERED.to_string(), json!(false).to_string());
        this.data.insert(DRIP.to_string(), 0.to_string());
        this.data.insert(TOTAL_DRIP.to_string(), 0.to_string());
        this.data.insert(ONE_DAY_TIMESTAMP.to_string(), env::block_timestamp().to_string());
        this.data.insert(CONTENT_COUNT.to_string(), 0.to_string());
        this.data.insert(TOTAL_CONTENT_COUNT.to_string(), 0.to_string());
        
        this
    }

    pub fn is_registered(&self) -> bool {
        match get_access_limit() {
            AccessLimit::Free => true,
            _ => match self.data.get(REGISTERED) {
                Some(v) => serde_json::from_str(v).unwrap(),
                None => false
            }
        }
        
    }

    pub fn set_registered(&mut self, registered: bool) {
        self.data.insert(REGISTERED.to_string(), json!(registered).to_string());
    }

    pub fn get_drip(&self) -> u128 {
        let drip: U128 = serde_json::from_str(self.data.get(DRIP).unwrap_or(&"0".to_string())).unwrap_or(U128::from(0));
        drip.0
    }

    pub fn increase_drip(&mut self, amount: u128) {
        let drip = self.get_drip();
        if let Some(new_drip) = drip.checked_add(amount) {
            let drip: U128 = new_drip.into();
            self.data.insert(DRIP.to_string(), json!(drip).to_string());
        }

        let balance = AssetKey::Drip((None, env::current_account_id()));
        let total_drip = self.get_balance(&balance);
        if let Some(new_total_drip) = total_drip.checked_add(amount) {
            self.set_balance(balance, new_total_drip);
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
        let mut balance = self.get_balance(&asset);
        if let Some(new_balance) = balance.checked_sub(amount) {
            balance = new_balance;
        } else {
            panic!("not enough balance");
        }
        let balance: U128 = balance.into();
        self.data.insert(json!(asset).to_string(), json!(balance).to_string());
    }

    pub fn set_balance(&mut self, asset: AssetKey, balance: u128) {
        let balance: U128 = balance.into();
        self.data.insert(json!(asset).to_string(), json!(balance).to_string());
    }
////////////////////////////////////////////////////////  Report Part ////////////////////////////////////////////////////////////////

    // pub fn report_deposit(&mut self, amount: )
}