use crate::*;

#[near_bindgen]
impl Community {
    #[private]
    pub fn join_resolver(&mut self, account_id: AccountId) {
        let initial_storage_usage = env::storage_usage();
        let result_count = env::promise_results_count();
        let mut success = true;
        let access = self.access.clone().unwrap();
        for i in 0..result_count {
            match env::promise_result(i) {
                near_sdk::PromiseResult::Successful(result) => {
                    let result: U128 = serde_json::from_slice(&result).unwrap();
                    let condition = &access.conditions[i as usize];
                    let fill = match condition {
                        Condition::FTCondition(ft_condition) => {
                            u128::from(result) >= u128::from(ft_condition.amount_to_access)
                        },
                    };
                    match access.relationship {
                        Relationship::Or => {
                            if fill {
                                success = true;
                                break
                            }
                        },
                        Relationship::And => {
                            if !fill {
                                success = false;
                                break
                            }
                        }
                    }
                },
                _ => panic!("failed to join")
            }
        }
        if success {
            self.members.insert(&account_id, &Member(1));
        }

        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage, 0)
    }
}