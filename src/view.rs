use std::collections::HashMap;

use crate::{*, access::Relationship, utils::get, proposal::{ProposalStatus, Vote, Proposal}};
use near_sdk::Balance;
use utils::get_content_hash;
use post::Hierarchy;
use account::AssetKey;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug)]
pub struct RoleOutput {
    pub alias: String,
    pub permissions: HashSet<Permission>,
    pub mod_level: u32,
    pub override_level: u32  
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug)]
pub struct ProposalOutput {
    pub method: String,
    pub proposer: AccountId,
    pub asset: Option<AssetKey>,
    pub description: String,
    pub action_kind: String,
    pub args: String,
    pub vote_counts: HashMap<Vote, Balance>,
    pub until: U64,
    pub quorum: U128,
    pub threshold: u32,
    pub status: ProposalStatus
}



#[near_bindgen]
impl Community {

    pub fn get_drip(&self, account_id: AccountId) -> U128 {
        self.drip.get_drip(account_id)
    }

    pub fn get_account_decay(&self, account_id: AccountId) -> u32 {
        self.drip.get_account_decay(account_id)
    }

    pub fn get_account(&self, account_id: AccountId) -> Option<HashMap<String, String>> {
        match self.accounts.get(&account_id) {
            Some(v) => Some(v.data),
            None => None
        }
    }

    pub fn get_content_decay(&self, hierarchies: Vec<Hierarchy>) -> u32 {
        let mut content_count = 0;
        if hierarchies.len() > 0 {
            let hierarchy_hash = get_content_hash(hierarchies.clone(), None, false).expect("content not found");
            let prev_hash = CryptoHash::from(Base58CryptoHash::try_from(hierarchy_hash).unwrap()).to_vec();
            content_count = get::<u8>(&prev_hash).unwrap();
        }
        self.drip.get_content_decay(content_count as u32)
    }

    // pub fn check_invited(&self, inviter_id: AccountId, invitee_id: AccountId) -> bool {
    //     let view_hash = env::sha256(&(inviter_id.to_string() + "invite" + &invitee_id.to_string()).into_bytes());
    //     self.relationship_tree.check(&view_hash)
    // }

    pub fn get_global_role(&self) -> (Vec<Permission>, Vec<(Relationship, Option<Access>)>) {
        let mut keys = vec![];
        let mut vals = vec![];
        for (key, value) in self.role_management.global_role.iter() {
            keys.push(key.clone());
            vals.push(value.clone());
        }
        (keys, vals)
    }

    pub fn get_roles(&self) -> HashMap<String, RoleOutput> {
        let mut roles = HashMap::new();
        for (hash, role) in self.role_management.roles.iter() {
            roles.insert(hash.clone(), RoleOutput { 
                alias: role.alias.clone(), 
                permissions: role.permissions.clone(), 
                mod_level: role.mod_level, 
                override_level: role.override_level 
            });
        }
        roles
    }

    pub fn get_balance(&self, account_id: AccountId, balance: AssetKey) -> U128{
        match self.accounts.get(&account_id) {
            Some(account) => {
                account.get_balance(&balance).into()
            },
            None => 0.into()
        }
    }

    // pub fn get_reports(&self, account_id: AccountId) -> Vec<Report> {
    //     let account = match self.reports.get(&account_id) {
    //         Some(v) => v,
    //         None => return Vec::new()
    //     };
    //     account.values().collect()
    // }

    pub fn get_proposal(&self, id: String) -> ProposalOutput {
        let proposals: UnorderedMap<String, Proposal> = UnorderedMap::new(StorageKey::Proposals);
        let proposal: Proposal = proposals.get(&id).unwrap().into();
        let status = proposal.get_status();
        ProposalOutput {
            method: proposal.method,
            proposer: proposal.proposer,
            asset: proposal.asset,
            description: proposal.description,
            action_kind: proposal.action_kind,
            args: proposal.args,
            vote_counts: proposal.vote_counts,
            until: proposal.until,
            quorum: proposal.quorum,
            threshold: proposal.threshold,
            status: status
        }
    }
}


#[cfg(test)]
mod test {
    #[test]
    pub fn test() {

    }
}