use std::collections::HashMap;

use crate::*;
use crate::drip::get_map_value;
use crate::utils::{get_account, set_account};
use ed25519_dalek::{ExpandedSecretKey, SecretKey};
use near_contract_standards::fungible_token;
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base64VecU8, U128, U64};
use near_sdk::{log, AccountId, Balance, Gas, PromiseOrValue, ext_contract, PromiseResult, PublicKey};

pub const GAS_FOR_FT_TRANSFER: Gas = Gas(10_000_000_000_000);

/// Status of a proposal.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum ProposalStatus {
    InProgress,
    Expired,
    Result(u32)   //represents the option
}

/// Status of a proposal.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum ExecutionStatus {
    NotStart,
    Failed,
    Finished
}

/// Function call arguments.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Clone, Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct ActionCall {
    method_name: String,
    args: String,
    deposit: U128,
    gas: U64,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Clone, Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct FunctionCall {
    receiver_id: AccountId,
    actions: Vec<ActionCall>
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Clone, Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct Transfer {
    asset: AssetKey,
    receiver_id: AccountId,
    msg: String,
    memo: Option<String>,
    amount: U128,
    // token_id: Option<String>  //NFT token id
}


/// Proposal that are sent to this DAO.
#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug)]
pub struct Proposal {
    pub method: String,
    pub options: Vec<Opt>,
    pub asset: Option<AssetKey>,
    pub bond: Option<(AssetKey, U128)>,
    pub begin: U64,
    pub until: U64,
    pub quorum: U64,
    pub threshold: u32,

    pub proposer: AccountId,
    pub votes: UnorderedMap<AccountId, (u32, U128, U64)>,   //option, balance, index
    pub execution_status: ExecutionStatus
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[derive(Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Opt {
    pub action_kind: String,
    pub args: String,
    pub description: String,
    pub vote_count: U128,
    pub accounts: U64
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ProposalInput {
    pub method: String,
    pub options: Vec<(String, String, String)>, //action_kind, args, description
    pub asset: Option<AssetKey>,
    pub bond: Option<(AssetKey, U128)>,
    pub begin: U64,
    pub until: U64,
    pub quorum: U64,
    pub threshold: u32,
}

impl From<ProposalInput> for Proposal {
    fn from(input: ProposalInput) -> Self {
        let id = bs58::encode(env::sha256(json!(input).to_string().as_bytes())).into_string();
        let mut options = vec![];
        input.options.iter().for_each(|option| {
            options.push(Opt {
                action_kind: option.0.clone(),
                args: option.1.clone(),
                description: option.2.clone(),
                vote_count: 0.into(),
                accounts: 0.into()
            })
        });
        Self {
            method: input.method,
            options: options,
            asset: input.asset,
            bond: input.bond,
            begin: input.begin,
            until: input.until,
            quorum: input.quorum,
            threshold: input.threshold,

            proposer: env::predecessor_account_id(),
            votes: UnorderedMap::new(id.as_bytes()),
            execution_status: ExecutionStatus::NotStart
        }
    }
}

impl Proposal {

    /// Adds vote of the given user with given `amount` of weight. If user already voted, fails.
    pub fn update_vote(
        &mut self,
        account_id: &AccountId,
        vote: u32,
        amount: u128
    ) {
        assert!(self.votes.get(&account_id).is_none(), "already voted");
        let mut account: Account = get_account(account_id).registered();
        match &self.bond {
            Some((bond, amount)) => {
                assert!(account.get_balance(bond) >= amount.0, "not enough bond");
            },
            None => {}
        }

        let amount = match &self.asset {
            Some(asset) => {
                let amount = match asset {
                    AssetKey::Drip((token_id, contract_id)) => {
                        if *token_id == None && *contract_id == env::current_account_id() {
                            let total = account.get_balance(&asset);
                            let consumed_asset = AssetKey::Drip(
                                (Some(AccountId::from_str("vote").unwrap()), env::current_account_id())
                            );
                            let consumed = account.get_balance(&consumed_asset);
                            assert!(total - consumed >= amount, "not enough balance");
                            account.increase_balance(consumed_asset, amount);
                            amount
                        } else {
                            amount
                        }
                    },
                    _ => amount,
                };
                account.decrease_balance(asset.clone(), amount);
                set_account(account_id, &account);
                amount
            }
            None => 1,
        };
        let amount = match self.method.as_str() {
            _ => amount
        };
        let mut option = self.options.get_mut(vote as usize).unwrap();
        let index = option.accounts.0 + 1;
        self.votes.insert(&account_id, &(vote.clone(), amount.into(), index.into()));
        option.vote_count = (option.vote_count.0 + amount).into();
        option.accounts = (option.accounts.0 + 1).into();
        
    }

    pub fn redeem_vote(&mut self, account_id: &AccountId) {
        let status = self.get_status();
        assert!(
            !matches!(status, ProposalStatus::InProgress),
            "not ready for redeem"
        );
        let (vote, amount, index) = self.votes.get(account_id).unwrap();
        assert!(self.votes.get(&account_id).is_some(), "account not found");
        match &self.asset {
            Some(asset) => {
                let mut account: Account = get_account(account_id).registered();
                match asset {
                    AssetKey::Drip((token_id, contract_id)) => {
                        if *token_id == None && *contract_id == env::current_account_id() {
                            let consumed_asset = AssetKey::Drip(
                                (Some(AccountId::from_str("vote").unwrap()), env::current_account_id())
                            );
                            account.decrease_balance(consumed_asset, amount.0);
                        } else {
                            account.increase_balance(asset.clone(), amount.0)
                        }
                    },
                    _ => account.increase_balance(asset.clone(), amount.0)
                }
                set_account(account_id, &account);
            },
            None => ()
        }
        self.votes.insert(account_id, &(vote, 0.into(), index));
    }

    pub fn get_status(
        &self,
    ) -> ProposalStatus {
        if self.until.0 > env::block_timestamp() {
            return ProposalStatus::InProgress
        }

        if self.votes.len() < self.quorum.0 {
            return ProposalStatus::Expired
        }
        let mut total = 0;
        self.options.iter().for_each(|option| {
            total += option.vote_count.0
        });
        if total == 0 {
            return ProposalStatus::Expired
        }
        let mut max_vote = (None, 0);
        for (index, option )in self.options.iter().enumerate() {
            if option.vote_count.0 * 100 / total > self.threshold as u128 {
                if max_vote.1 < option.vote_count.0 {
                    max_vote = (Some(index as u32), option.vote_count.0)
                }
            }
        }
        match max_vote.0 {
            Some(option) => ProposalStatus::Result(option),
            None => ProposalStatus::Expired
        }
    }

    /// Executes given proposal and updates the contract's state.
    pub fn execute(
        &mut self,
        proposal_id: String,
        option: Opt
    ) -> PromiseOrValue<()> {
        assert!(self.execution_status != ExecutionStatus::Finished, "already executed");
        let result = match option.action_kind.as_str() {
            "functionCall" => {
                let args = serde_json::from_str::<FunctionCall>(&option.args).unwrap();
                let mut promise = Promise::new(args.receiver_id.clone().into());
                for action in args.actions {
                    promise = promise.function_call(
                        action.method_name.clone().into(),
                        action.args.clone().as_bytes().to_vec(),
                        action.deposit.0,
                        Gas(action.gas.0),
                    )
                }
                promise.into()
            }
            "transfer" => {
                let community: Account = get_account(&env::current_account_id()).registered();
                let args = serde_json::from_str::<Transfer>(&option.args).unwrap();
                match &args.asset {
                    AssetKey::FT(token_id) => {
                        assert!(community.get_balance(&args.asset) >= args.amount.0, "not enough balance");
                        if token_id.to_string() == "near" {
                            Promise::new(args.receiver_id.clone()).transfer(args.amount.0).into()
                        } else {
                            ext_ft_core::ext(token_id.clone()).ft_transfer_call(args.receiver_id, args.amount, args.memo, args.msg).into()
                        }
                    },
                    AssetKey::NFT(contract_id, token_id) => PromiseOrValue::Value(()),
                    _  => PromiseOrValue::Value(())
                }
                
            },
            _ => {
                self.execution_status = ExecutionStatus::Finished;
                PromiseOrValue::Value(())
            }
        };
        match result {
            PromiseOrValue::Promise(promise) => promise
                .then(Community::ext(env::current_account_id())
                    .on_proposal_callback(
                        proposal_id,
                    ))
                .into(),
            PromiseOrValue::Value(()) => PromiseOrValue::Value(())

        }
    }
}



#[near_bindgen]
impl Community {
    /// Add proposal to this DAO.
    pub fn add_proposal(&mut self, proposal: ProposalInput) -> String {
        let initial_storage_usage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        let mut have_action = false;
        for option in proposal.options.iter() {
            if !option.0.is_empty() {
                have_action = true
            } else {
                match option.0.as_str() {
                    "functionCall" => {
                        let args = serde_json::from_str::<FunctionCall>(&option.1).unwrap();
                        args.actions.iter().for_each(|action| {
                            assert!(!action.method_name.contains("transfer"), "transfer is not allowed")
                        })
                        
                    },
                    _ => {}
                }
                
            }
        }
        assert!(self.can_execute_action(sender_id.clone(), Permission::AddProposal(have_action)), "not allowed");
        // TODO
        // if have_action {
        //     assert!(proposal.until.0 - proposal.begin.0 > 1440 * 60 * 1000 * 1000000, "duration too small");   //1 day
        // }
        let id_string= sender_id.to_string() + &json!(proposal).to_string() + &env::block_timestamp().to_string();
        let id = bs58::encode(env::sha256(id_string.as_bytes())).into_string();
        self.proposals.insert(&id, &proposal.into());
        let access_key = SecretKey::from_bytes(&env::sha256(id_string.as_bytes())).unwrap();
        let pk: ed25519_dalek::PublicKey = (&access_key).into();
        let public_key = PublicKey::try_from([vec![0], pk.as_bytes().to_vec()].concat()).unwrap();
        Promise::new(env::current_account_id()).add_access_key(public_key, 250000000000000000000000, env::current_account_id(), "act_proposal".to_string());
        set_storage_usage(initial_storage_usage, None);
        id
    }

    pub fn vote(&mut self, id: String, vote: u32, amount: U128) {
        let initial_storage_usage = env::storage_usage();
        let mut proposal: Proposal = self.proposals.get(&id).unwrap().into();
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::Vote), "not allowed");
        assert!(
            matches!(proposal.get_status(), ProposalStatus::InProgress),
            "Expired"
        );
        proposal.update_vote(
            &sender_id,
            vote,
            amount.0
        ); 
        let drips = self.drip.set_proposal_drip(proposal.proposer.clone());
        self.proposals.insert(&id, &proposal);
        Event::log_other(
            Some(json!({
                "drips": drips
            }).to_string())
        );
        set_storage_usage(initial_storage_usage, None);
    }

    /// Act on given proposal by id, if permissions allow.
    /// Memo is logged but not stored in the state. Can be used to leave notes or explain the action.
    #[private]
    pub fn act_proposal(&mut self, id: String) {
        let mut proposal: Proposal = self.proposals.get(&id).unwrap().into();
        let status = proposal.get_status();
        assert!(
            !matches!(status, ProposalStatus::InProgress),
            "not ready for action"
        );
        // Updates proposal status with new votes using the policy.
        match status {
            ProposalStatus::Result(option) => proposal.execute(id.clone(), proposal.options.get(option as usize).unwrap().clone()),
            _ => PromiseOrValue::Value(())
        };

        self.proposals.insert(&id, &proposal);   
    }

    /// Receiving callback after the proposal has been finalized.
    /// If successful, returns bond money to the proposal originator.
    /// If the proposal execution failed (funds didn't transfer or function call failure),
    /// move proposal to "Failed" state.
    #[private]
    pub fn on_proposal_callback(&mut self, proposal_id: String) -> PromiseOrValue<()> {
        let mut proposal: Proposal = self.proposals
            .get(&proposal_id)
            .expect("ERR_NO_PROPOSAL")
            .into();
        assert_eq!(
            env::promise_results_count(),
            1,
            "ERR_UNEXPECTED_CALLBACK_PROMISES"
        );
        let result = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {
                let mut community: Account = get_account(&env::current_account_id()).registered();
                let status = proposal.get_status();
                if let ProposalStatus::Result(index) = status {
                    let option = proposal.options.get(index as usize).unwrap();
                    if option.action_kind == "transfer".to_string() {
                        let args = serde_json::from_str::<Transfer>(&option.args).unwrap();
                        community.decrease_balance(args.asset, args.amount.0);
                        self.accounts.insert(&env::current_account_id(), &community);
                    }
                }

                let access_key = SecretKey::from_bytes(&bs58::decode(proposal_id.clone()).into_vec().unwrap()).unwrap();
                let pk: ed25519_dalek::PublicKey = (&access_key).into();
                let public_key = PublicKey::try_from([vec![0], pk.as_bytes().to_vec()].concat()).unwrap();
                Promise::new(env::current_account_id()).delete_key(public_key);
                proposal.execution_status = ExecutionStatus::Finished;
                PromiseOrValue::Value(())
            },
            PromiseResult::Failed => {
                proposal.execution_status = ExecutionStatus::Failed;
                PromiseOrValue::Value(())
            },
        };
        self.proposals.insert(&proposal_id, &proposal);
        result
    }

    pub fn redeem_vote(&mut self, proposal_id: String) {
        let mut proposal: Proposal = self.proposals.get(&proposal_id).unwrap().into();
        let sender_id = env::predecessor_account_id();
        proposal.redeem_vote(&sender_id);
        self.proposals.insert(&proposal_id, &proposal);  

        let voter = proposal.votes.get(&sender_id).unwrap();
        let drips = match proposal.get_status() {
            ProposalStatus::Expired => self.drip.set_vote_drip(sender_id, 100),
            ProposalStatus::Result(option) => {
                if option == voter.0 {       //bonus
                    let base_drip = U128::from(get_map_value(&"vote".to_string())).0;
                    let total_drips = proposal.votes.len() as u128 * base_drip;
                    let opt = proposal.options.get(option as usize).unwrap();
                    let option_drips = opt.accounts.0 as u128 * base_drip;
                    let rest_drips = total_drips - option_drips;
                    let index_threshold = opt.accounts.0 * 2 / 10;     //only 20%
                    let index = voter.2.0;
                    let mut amount_per_account = base_drip;
                    if index <= index_threshold {
                        amount_per_account += rest_drips * 8 / 10 / (index_threshold as u128);
                    } else {
                        amount_per_account += rest_drips * 2 / 10 / ((opt.accounts.0 - index_threshold) as u128);
                    }
                    self.drip.set_vote_drip(sender_id, (amount_per_account * 100 / base_drip) as u32)
                } else {
                    vec![]
                }
            },
            _ => panic!("in progress")
        };
        if drips.len() > 0 {
            Event::log_other(
                Some(json!({
                    "drips": drips
                }).to_string())
            )
        }
        
    }

    pub fn get_voter(&self, voter_id: AccountId, proposal_id: String) -> Option<(u32, U128, U64)> {
        let proposal: Proposal = self.proposals.get(&proposal_id).unwrap().into();
        proposal.votes.get(&voter_id)
    }
}


#[cfg(test)]
mod tests {
    use std::{str::FromStr, convert::TryFrom};

    use ed25519_dalek::{SecretKey, ExpandedSecretKey, Sha512};
    use near_sdk::{bs58, env, PublicKey, json_types::{U64, U128, Base64VecU8}, serde_json::{json, self}, AccountId, Promise};

    use crate::view::ProposalOutput;

    use super::{ProposalInput, Opt, FunctionCall, ActionCall, Proposal};

    #[test]
    pub fn test_pk() {
        let text = "1".to_string();
        let access_key = SecretKey::from_bytes(&env::sha256(text.as_bytes())).unwrap();
        let pk: ed25519_dalek::PublicKey = (&access_key).into();
        println!("{:?}", bs58::encode(pk.as_bytes().to_vec()).into_string());
        
        let access_key = ExpandedSecretKey::from(&access_key);
        let pk: ed25519_dalek::PublicKey = (&access_key).into();
        println!("{:?}", bs58::encode(pk.as_bytes().to_vec()).into_string());
        println!("{:?}", pk.as_bytes().to_vec());
        let public_key = PublicKey::try_from([vec![0], pk.as_bytes().to_vec()].concat()).unwrap();
    }

    #[test]
    pub fn test() {
        let id = bs58::encode(env::sha256("123".to_string().as_bytes())).into_string();
        let public_key = PublicKey::from_str(&id).unwrap();
        println!("{:?}", public_key)
    }

    #[test]
    pub fn test_proposal() {
        let proposal = ProposalInput {
            method: "".to_string(),
            options: vec![("".to_string(), "".to_string(), "1".to_string())],
            asset: None,
            bond: None,
            begin: U64::from(1684764073137000000),
            until: U64::from(1684850473137000000),
            quorum: U64::from(0),
            threshold: 0,
        };
        let j = json!(proposal).to_string();
        println!("{:?}", j);
        let bytes = bs58::encode(env::sha256(j.as_bytes())).into_string();
        println!("{:?}", bytes)
    }

    #[test]
    pub fn test_execution() {
        let args = FunctionCall {
            receiver_id: AccountId::from_str("2023-5.community-genesis2.bhc8521.testnet").unwrap(),
            actions: vec![ActionCall {
                method_name: "distribute".to_string(),
                args: "{\"list\":[[\"edzwiggle.testnet\",{\"FT\":\"near\"},\"20000000000000000000000\"],[\"wcs.testnet\",{\"FT\":\"near\"},\"10000000000000000000000\"],[\"gugu1997.testnet\",{\"FT\":\"near\"},\"40000000000000000000000\"],[\"kinkrit.testnet\",{\"FT\":\"near\"},\"800000000000000000000000\"],[\"171111.testnet\",{\"FT\":\"near\"},\"130000000000000000000000\"]],\"extra\":{\"token\":\"near\",\"amount\":\"1000000000000000000000000\",\"distribution\":\"proportionally\",\"receiver\":\"group\",\"group\":\"9USnArW9LZS8b2NAV1M3CiLkcCRBNewSn8TezJu1tJh4\"}}".to_string(),
                gas: 300000000000000.into(),
                deposit: 0.into()
            }]
        };
        let option = Opt {
            action_kind: "functionCall".to_string(),
            args: json!(args).to_string(),
            description: "Yes".to_string(),
            vote_count: U128::from(1),
            accounts: U64::from(0)
        };
        let result = match option.action_kind.as_str() {
            "functionCall" => {
                serde_json::from_str::<FunctionCall>(&option.args).unwrap()
            },
            _ => panic!("error")
        };
        println!("{:?}", result)
    }
    

    #[test]
    pub fn test_vote() {
        let proposalInput = ProposalInput {
            method: "".to_string(),
            options: vec![("".to_string(), "".to_string(), "haha".to_string())],
            asset: None,
            bond: None,
            begin: 0.into(),
            until: 0.into(),
            quorum: 0.into(),
            threshold: 0
        };
        let mut proposal: Proposal = proposalInput.into();
        proposal.update_vote(&AccountId::from_str("kinkrit.testnet").unwrap(), 0, 0);
        let status = proposal.get_status();
        let output = ProposalOutput {
            method: proposal.method,
            options: proposal.options,
            asset: proposal.asset,
            bond: proposal.bond,
            begin: proposal.begin,
            until: proposal.until,
            quorum: proposal.quorum,
            threshold: proposal.threshold,

            proposer: proposal.proposer,
            status: status,
            execution_status: proposal.execution_status
        };
        println!("{:?}", output);
    }

}