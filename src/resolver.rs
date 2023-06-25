use near_non_transferable_token::fungible_token::receiver::FungibleTokenReceiver as NtftReceiver;

use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver as FtReceiver;

use crate::{*, utils::set_account};
use crate::account::AssetKey;
use crate::drip::get_map_value;
use crate::utils::get_parent_contract_id;
use near_sdk::{PromiseOrValue, PromiseResult};

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub enum MsgInput {
    Report(ReportInput),
    RevokeReport(ReportInput),
    Deposit,
    Donate
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone)]
pub struct ReportInput {
    hierarchies: Vec<Hierarchy>,
    reason: String
}

#[near_bindgen]
impl Community {

    #[private]
    pub fn on_withdraw_callback(&mut self, account_id: AccountId, asset: AssetKey, amount: U128) {
        assert_eq!(
            env::promise_results_count(),
            1,
            "unexpected promise count"
        );
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {
                let mut account = get_account(&account_id).registered();
                account.decrease_balance(asset, amount.0);
                set_account(&account_id, &account);
                PromiseOrValue::Value(())
            },
            PromiseResult::Failed => {
                PromiseOrValue::Value(())
            },
        };
    }
}

#[near_bindgen]
impl FtReceiver for Community {

    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128>  {
        let msg_input = serde_json::from_str(&msg).unwrap();
        match msg_input {
            MsgInput::Deposit => {
                let mut accounts: LookupMap<AccountId, Account> = LookupMap::new(StorageKey::Account);
                let mut account = get_account(&sender_id);
                account.increase_balance(AssetKey::FT(env::predecessor_account_id()), amount.0);
                accounts.insert(&sender_id, &account);
                PromiseOrValue::Value(0.into())
            },
            MsgInput::Donate => {
                let mut accounts: LookupMap<AccountId, Account> = LookupMap::new(StorageKey::Account);
                let mut account = get_account(&env::current_account_id()).registered();
                account.increase_balance(AssetKey::FT(env::predecessor_account_id()), amount.0);
                accounts.insert(&env::current_account_id(), &account);
                PromiseOrValue::Value(0.into())
            },
            _ => {PromiseOrValue::Value(amount)}
        }
    }

}


#[near_bindgen]
impl NtftReceiver for Community {

    #[payable]
    fn ft_on_deposit(&mut self, owner_id: AccountId, contract_id: AccountId ,amount: U128, msg: String) -> PromiseOrValue<U128>  {
        let msg_input: MsgInput = serde_json::from_str(&msg).unwrap();
        match msg_input {
            MsgInput::Report(report_input) => {
                assert!(get_arg::<AccountId>(DRIP_CONTRACT).unwrap_or(AccountId::new_unchecked("".to_string())) == env::predecessor_account_id(), "wrong token id");
                assert!(contract_id == env::current_account_id(), "wrong drip");
                let need_amount = get_map_value(&"report_deposit".to_string());
                assert!(amount.0 >= need_amount, "not enough drip");
                self.internal_report(owner_id, report_input.hierarchies);
                PromiseOrValue::Value((amount.0 - need_amount).into())
            },
            _ => {
                let mut accounts: LookupMap<AccountId, Account> = LookupMap::new(StorageKey::Account);
                let mut account = get_account(&owner_id);
                account.increase_balance(AssetKey::Drip((Some(env::predecessor_account_id()), contract_id.clone())), amount.0);
                accounts.insert(&owner_id, &account);
                PromiseOrValue::Value(0.into())
            }
            
            
        }
    }

    fn ft_on_withdraw(&mut self, owner_id: AccountId, contract_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128>  {
        let accounts: LookupMap<AccountId, Account> = LookupMap::new(StorageKey::Account);
        let mut account = match get_account(&owner_id).get_registered() {
            Some(v) => v,
            None => return PromiseOrValue::Value(amount)
        };

        let msg_input: MsgInput = serde_json::from_str(&msg).unwrap();
        match msg_input {
            MsgInput::RevokeReport(report_input) => {
                assert!(get_arg::<AccountId>(DRIP_CONTRACT).unwrap_or(AccountId::new_unchecked("".to_string())) == env::predecessor_account_id(), "wrong token id");
                assert!(contract_id == env::current_account_id(), "wrong drip");
                let need_amount = get_map_value(&"report_deposit".to_string());
                assert!(amount.0 > need_amount, "not enough amount");
                self.internal_revoke_report(owner_id, report_input.hierarchies);
                PromiseOrValue::Value((amount.0 - need_amount).into())
            },
            _ => {
                account.decrease_balance(AssetKey::Drip((Some(env::predecessor_account_id()), contract_id.clone())), amount.0);
                PromiseOrValue::Value(0.into())
            }
        }
    }


    #[payable]
    fn ft_on_burn(&mut self, owner_id: AccountId, contract_id: AccountId ,amount: U128, msg: String) -> PromiseOrValue<U128>  {
        let msg_input: MsgInput = serde_json::from_str(&msg).unwrap();
        match msg_input {
            _ => {PromiseOrValue::Value(amount)}
        }
        
    }

}


#[cfg(test)]
mod test {
    use std::{collections::HashMap, convert::{TryInto, TryFrom}, str::FromStr};

    use near_sdk::{serde_json::{json, self}, AccountId, serde::{Deserialize, de::IntoDeserializer}};

    use super::{MsgInput, ReportInput};

    fn get_arg<T>(key: &str) -> Option<T> 
    where T: std::str::FromStr
    {
        let mut args = HashMap::new();
        args.insert("drip_contract".to_string(), "drip4.popula.testnet".to_string());
        
        let value = match args.get(&key.to_string()) {
            Some(v) => v,
            None => return None
        };

        match T::from_str(value) {
            Ok(res) => Some(res),
            Err(_) => None
        }
    }

    #[test]
    fn test() {
        let msg_input = MsgInput::Report(ReportInput {
            hierarchies: vec![],
            reason: "".to_string(),
        });
        print!("{:?}", json!(msg_input).to_string());

    }

    #[test]
    fn test_args() {
        print!("{}", get_arg::<AccountId>("drip_contract").unwrap().to_string());
        // assert!(false)
    }
}