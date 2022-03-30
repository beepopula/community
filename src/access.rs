
use crate::*;

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


impl Access {
    pub fn new(conditions: Vec<Condition>, relationship: Relationship) -> Self {
        Self {
            conditions: conditions,
            relationship: relationship
        }
    }

    pub fn set(&mut self, conditions: Vec<Condition>, relationship: Relationship) {
        self.conditions = conditions;
        self.relationship = relationship;
    }

    pub fn check_permission(&mut self, account_id: AccountId) {
        let mut promises: Vec<u64> = Vec::new();
        for condition in self.conditions.iter() {
            match condition {
                Condition::FTCondition(v) => {
                    let new_promise = env::promise_create(v.token_id.clone(), "ft_balance_of", &json!({
                        "account_id": account_id.clone()
                    }).to_string().into_bytes(), 0, env::prepaid_gas() / (self.conditions.len() as u64 + 2));
                    promises.push(new_promise);
                },
            }
        }
        let batch_promise = env::promise_and(&promises[..]);
        env::promise_then(batch_promise, env::current_account_id(), "join_resolver", &json!({
            "account_id": account_id
        }).to_string().into_bytes(), env::attached_deposit(), env::prepaid_gas() / (self.conditions.len() as u64 + 2));
    }
    
}