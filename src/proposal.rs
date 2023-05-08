use std::collections::HashMap;

use crate::*;
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
    /// If quorum voted yes, this proposal is successfully approved.
    Approved,
    /// If quorum voted no, this proposal is rejected. Bond is returned.
    Rejected,
    /// If quorum voted to remove (e.g. spam), this proposal is rejected and bond is not returned.
    /// Interfaces shouldn't show removed proposals.
    /// Expired after period of time.
    Expired,
    Failed,
    Finished
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
    args: Base64VecU8,
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
    token_id: AccountId,
    receiver_id: AccountId,
    amount: U128,
    msg: Option<String>,
}

/// Votes recorded in the proposal.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq, PartialOrd)]
#[serde(crate = "near_sdk::serde")]
pub enum Vote {
    Approve,
    Reject
}


/// Proposal that are sent to this DAO.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct Proposal {
    pub method: String,
    pub proposer: AccountId,
    pub asset: Option<AssetKey>,
    pub description: String,
    pub action_kind: String,
    pub args: String,
    pub vote_counts: HashMap<Vote, Balance>,
    pub votes: HashMap<AccountId, (Vote, U128)>,
    pub until: U64,
    pub quorum: U128,
    pub threshold: u32,
    pub execution_status: ExecutionStatus
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ProposalInput {
    pub method: String,
    /// Description of this proposal.
    pub description: String,
    /// Kind of proposal with relevant information.
    pub action_kind: String,
    pub args: String,
    pub asset: Option<AssetKey>,
    pub until: U64,
    pub quorum: U128,
    pub threshold: u32,
}

impl From<ProposalInput> for Proposal {
    fn from(input: ProposalInput) -> Self {
        Self {
            method: input.method,
            proposer: env::predecessor_account_id(),
            description: input.description,
            asset: input.asset,
            action_kind: input.action_kind,
            args: input.args,
            vote_counts: HashMap::default(),
            votes: HashMap::default(),
            until: input.until,
            quorum: input.quorum,
            threshold: input.threshold,
            execution_status: ExecutionStatus::NotStart
        }
    }
}

impl Proposal {
    /// Adds vote of the given user with given `amount` of weight. If user already voted, fails.
    pub fn update_vote(
        &mut self,
        account_id: &AccountId,
        vote: Vote,
        amount: u128
    ) {
        assert!(self.votes.contains_key(&account_id), "already voted");
        let amount = match &self.asset {
            Some(asset) => {
                let mut accounts = LookupMap::new(StorageKey::Account);
                let mut account: Account = accounts.get(account_id).unwrap();
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
                accounts.insert(account_id, &account);
                amount
            }
            None => 1,
        };
        let amount = match self.method.as_str() {
            _ => amount
        };
        self.votes.insert(account_id.clone(), (vote.clone(), amount.into()));
        let mut vote_count = *self.vote_counts.get(&vote).unwrap_or(&0);
        vote_count += amount;
        self.vote_counts.insert(vote, vote_count);
    }

    pub fn redeem_vote(&mut self, account_id: &AccountId) {
        let status = self.get_status();
        assert!(
            matches!(status, ProposalStatus::InProgress),
            "not ready for redeem"
        );
        assert!(self.votes.contains_key(&account_id), "account not found");
        match &self.asset {
            Some(asset) => {
                let mut accounts = LookupMap::new(StorageKey::Account);
                let mut account: Account = accounts.get(account_id).unwrap();
                let (vote, amount) = self.votes.get(account_id).unwrap();
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
                accounts.insert(account_id, &account);
            },
            None => ()
        }
    }

    pub fn get_status(
        &self,
    ) -> ProposalStatus {
        if self.until.0 < env::block_timestamp() {
            return ProposalStatus::InProgress
        }

        if (self.votes.len() as u128) < self.quorum.0 {
            return ProposalStatus::Expired
        }
        let mut total = 0;
        self.vote_counts.iter().for_each(|vote| {
            total += vote.1
        });
        let mut max_vote = (None, 0);
        for (vote, count) in self.vote_counts.iter() {
            if count * 100 / total > self.threshold as u128 {
                if max_vote.1 < *count {
                    max_vote = (Some(vote.clone()), *count)
                }
            }
        }
        match max_vote.0 {
            Some(vote) => match vote {
                Vote::Approve => ProposalStatus::Approved,
                Vote::Reject => ProposalStatus::Rejected
            },
            None => ProposalStatus::Expired
        }
    }

    /// Executes given proposal and updates the contract's state.
    pub fn execute(
        &mut self,
        proposal_id: String
    ) -> PromiseOrValue<()> {
        let result = match self.action_kind.as_str() {
            "functionCall" => {
                let args = serde_json::from_str::<FunctionCall>(&self.args).unwrap();
                let mut promise = Promise::new(args.receiver_id.clone().into());
                for action in args.actions {
                    promise = promise.function_call(
                        action.method_name.clone().into(),
                        action.args.clone().into(),
                        action.deposit.0,
                        Gas(action.gas.0),
                    )
                }
                promise.into()
            }
            "transfer" => {
                let args = serde_json::from_str::<Transfer>(&self.args).unwrap();
                self.payout(
                &args.token_id,
                &args.receiver_id,
                args.amount.0,
                self.description.clone(),
                args.msg.clone(),
                )
            },
            _ => PromiseOrValue::Value(())
        };
        match result {
            PromiseOrValue::Promise(promise) => promise
                .then(Community::ext(env::current_account_id())
                    .on_proposal_callback(
                        proposal_id,
                    ))
                .into(),
            PromiseOrValue::Value(()) => PromiseOrValue::Value(()),
        }
    }

    fn payout(
        &mut self,
        token_id: &AccountId,
        receiver_id: &AccountId,
        amount: Balance,
        memo: String,
        msg: Option<String>,
    ) -> PromiseOrValue<()> {
        if token_id.to_string() == "near" {
            Promise::new(receiver_id.clone()).transfer(amount).into()
        } else {
            if let Some(msg) = msg {
                ext_ft_core::ext(token_id.clone())
                    .with_attached_deposit(1)
                    .ft_transfer_call(
                        receiver_id.clone(),
                        U128(amount),
                        Some(memo),
                        msg,
                    )
            } else {
                ext_ft_core::ext(token_id.clone())
                    .with_attached_deposit(1)
                    .ft_transfer(
                        receiver_id.clone(),
                        U128(amount),
                        Some(memo),
                    )
                    
            }
            .into()
        }
    }
}



#[near_bindgen]
impl Community {
    /// Add proposal to this DAO.
    pub fn add_proposal(&mut self, proposal: ProposalInput) -> String {
        let initial_storage_usage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::AddProposal(Some(proposal.action_kind.clone()))), "not allowed");
        match proposal.action_kind.as_str() {
            "functionCall" => {
                let args = serde_json::from_str::<FunctionCall>(&proposal.args).unwrap();
                args.actions.iter().for_each(|action| {
                    assert!(action.method_name.find("transfer").is_none() && action.method_name.find("approve").is_none(), "transfer is not allowed");
                });
            },
            "transfer" => {
            },
            _ => ()
        }
        let mut proposals: UnorderedMap<String, Proposal> = UnorderedMap::new(StorageKey::Proposals);
        let id = bs58::encode(env::sha256(json!(proposal).to_string().as_bytes())).into_string();
        proposals.insert(&id, &proposal.into());
        let public_key = PublicKey::from_str(&id).unwrap();
        Promise::new(env::current_account_id()).add_access_key(public_key, 250000000000000000000000, env::current_account_id(), "act_proposal".to_string());
        set_storage_usage(initial_storage_usage, None);
        id
    }

    pub fn vote(&mut self, id: String, vote: Vote, amount: U128) {
        let initial_storage_usage = env::storage_usage();
        let mut proposals: UnorderedMap<String, Proposal> = UnorderedMap::new(StorageKey::Proposals);
        let mut proposal: Proposal = proposals.get(&id).unwrap().into();
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::Vote(Some(proposal.action_kind.clone()))), "not allowed");
        assert!(
            matches!(proposal.get_status(), ProposalStatus::InProgress),
            "Expired"
        );
        proposal.update_vote(
            &sender_id,
            vote,
            amount.0
        ); 
        proposals.insert(&id, &proposal);
        set_storage_usage(initial_storage_usage, None);
    }

    /// Act on given proposal by id, if permissions allow.
    /// Memo is logged but not stored in the state. Can be used to leave notes or explain the action.
    #[private]
    pub fn act_proposal(&mut self, id: String, memo: Option<String>) {
        let mut proposals: UnorderedMap<String, Proposal> = UnorderedMap::new(StorageKey::Proposals);
        let mut proposal: Proposal = proposals.get(&id).unwrap().into();
        let status = proposal.get_status();
        assert!(
            matches!(status, ProposalStatus::InProgress),
            "not ready for action"
        );
        // Updates proposal status with new votes using the policy.
        match status {
            ProposalStatus::Approved => proposal.execute(id.clone()),
            ProposalStatus::Rejected => {
                proposal.execution_status = ExecutionStatus::Finished;
                PromiseOrValue::Value(())
            },
            _ => PromiseOrValue::Value(())
        };

        proposals.insert(&id, &proposal);   
        if let Some(memo) = memo {
            log!("Memo: {}", memo);
        }
    }

    /// Receiving callback after the proposal has been finalized.
    /// If successful, returns bond money to the proposal originator.
    /// If the proposal execution failed (funds didn't transfer or function call failure),
    /// move proposal to "Failed" state.
    #[private]
    pub fn on_proposal_callback(&mut self, proposal_id: String) -> PromiseOrValue<()> {
        let mut proposals: UnorderedMap<String, Proposal> = UnorderedMap::new(StorageKey::Proposals);
        let mut proposal: Proposal = proposals
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
                let public_key = PublicKey::from_str(&proposal_id).unwrap();
                Promise::new(env::current_account_id()).delete_key(public_key);
                proposal.execution_status = ExecutionStatus::Finished;
                PromiseOrValue::Value(())
            },
            PromiseResult::Failed => {
                proposal.execution_status = ExecutionStatus::Finished;
                PromiseOrValue::Value(())
            },
        };
        proposals.insert(&proposal_id, &proposal);
        result
    }

    pub fn redeem_vote(&mut self, proposal_id: String) {
        let mut proposals: UnorderedMap<String, Proposal> = UnorderedMap::new(StorageKey::Proposals);
        let mut proposal: Proposal = proposals.get(&proposal_id).unwrap().into();
        let sender_id = env::predecessor_account_id();
        proposal.redeem_vote(&sender_id);
        proposals.insert(&proposal_id, &proposal);
        // TODO: drip rewards
    }
}


#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use near_sdk::{bs58, env, PublicKey};

    #[test]
    pub fn test() {
        let id = bs58::encode(env::sha256("123".to_string().as_bytes())).into_string();
        let public_key = PublicKey::from_str(&id).unwrap();
        println!("{:?}", public_key)
    }

}