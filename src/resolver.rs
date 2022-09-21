use near_non_transferrable_token::fungible_token::receiver::FungibleTokenReceiver;

use crate::*;
use near_sdk::PromiseOrValue;

#[near_bindgen]
impl FungibleTokenReceiver for Community {

    fn ft_on_deposit(&mut self,owner_id:AccountId,contract_id:AccountId,token_source:Option<near_non_transferrable_token::fungible_token::core::TokenSource> ,amount:U128,msg:String,) -> PromiseOrValue<U128>  {
        todo!()
    }


    fn ft_on_burn(&mut self,owner_id:AccountId,contract_id:AccountId,token_source:Option<near_non_transferrable_token::fungible_token::core::TokenSource> ,amount:U128,msg:String) -> PromiseOrValue<U128>  {
        todo!()
    }

    fn ft_on_withdraw(&mut self,owner_id:AccountId,contract_id:AccountId,token_source:Option<near_non_transferrable_token::fungible_token::core::TokenSource> ,amount:U128,msg:String,) -> PromiseOrValue<U128>  {
        todo!()
    }

}