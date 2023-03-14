use crate::*;
use account::AssetKey;

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
    FTCondition(FTCondition),
    NFTCondition(NFTCondition),
    DripCondition(DripCondition),
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
pub enum Relationship {
    Or,
    And
}

impl Access {
    pub fn check_account(&self, account_id: &AccountId) -> bool {
        let accounts: LookupMap<AccountId, Account> = LookupMap::new(StorageKey::Account);
        let account = match accounts.get(account_id) {
            Some(v)  => v,
            None => return false
        };
        let mut fullfill = true;
        match self.relationship {
            Relationship::Or => fullfill = false,
            Relationship::And => fullfill  = true
        }
        for condition in self.conditions.iter() {
            let access = match condition {
                Condition::FTCondition(ft) => {
                    account.get_balance(&AssetKey::FT(ft.token_id.clone())) >= ft.amount_to_access.0
                }
                Condition::NFTCondition(_) => todo!(),
                Condition::DripCondition(drip) => {
                    account.get_balance(&AssetKey::Drip((drip.token_id.clone(), drip.contract_id.clone()))) >= drip.amount_to_access.0
                },
            };
            match self.relationship {
                Relationship::Or => fullfill = fullfill || access,
                Relationship::And => fullfill = fullfill && access
            };
            
        }
        fullfill
    }
}
    