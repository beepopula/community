use crate::*;


const REGISTERED: &str = "registered";
const DRIP: &str = "drip";
const ONE_DAY_TIMESTAMP: &str = "one_day_timestamp";
const CONTENT_COUNT: &str = "content_count";

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub enum Deposit {
    FT(AccountId),
    NFT(AccountId),
    Drip(AccountId)
}


fn get_account_decay(count: u64) -> u32 {
    if count <= 10 {
        return 100
    } else if count > 10 && count <= 20 {
        return 50
    }
    25
}

#[derive(BorshDeserialize, BorshSerialize)]
#[derive(Debug)]
pub struct Account {
    data: HashMap<String, String>,
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
        this
    }
}

impl Account {

    pub fn is_registered(&self) -> bool {
       match self.data.get(REGISTERED) {
           Some(v) => serde_json::from_str(v).unwrap(),
           None => false
       }
    }

    pub fn set_registered(&mut self, registered: bool) {
        self.data.insert(REGISTERED.to_string(), json!(registered).to_string());
    }

    pub fn get_drip(&self) -> u128 {
        serde_json::from_str(self.data.get(DRIP).unwrap_or(&"0".to_string())).unwrap_or(0)
    }

    pub fn increase_drip(&mut self, amount: u128) {
        let mut drip = self.get_drip();
        drip += amount;
        self.data.insert(DRIP.to_string(), json!(drip).to_string());
    }

    pub fn decrease_drip(&mut self, amount: u128) {
        let mut drip = self.get_drip();
        drip -= amount;
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
    }


//////////////////////////////////////////////////////////  Deposit Part ////////////////////////////////////////////////////////////

    pub fn get_deposit(&self, deposit: &Deposit) -> u128 {
        serde_json::from_str(self.data.get(&json!(deposit).to_string()).unwrap_or(&"0".to_string())).unwrap_or(0)
    }

    pub fn increase_deposit(&mut self, deposit: Deposit, amount: u128) {
        let mut balance = self.get_deposit(&deposit);
        balance += amount;
        self.data.insert(json!(deposit).to_string(), json!(balance).to_string());
    }

    pub fn decrease_deposit(&mut self, deposit: Deposit, amount: u128) {
        let mut balance = self.get_deposit(&deposit);
        balance -= amount;
        self.data.insert(json!(deposit).to_string(), json!(balance).to_string());
    }
}