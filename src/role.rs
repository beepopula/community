use crate::*;
use std::collections::{HashMap, HashSet};

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, AccountId, Balance};


#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug)]
pub enum RoleKindInput {
    /// Matches everyone, who is not matched by other roles.
    Everyone,
    //support NEP141,"near" for near token
    // Member(AccountId, U128),
    /// Set of accounts.
    Group,
}

#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug)]
pub enum RoleKind {
    /// Matches everyone, who is not matched by other roles.
    Everyone,
    //support NEP141,"near" for near token
    // Member(AccountId, U128),
    /// Set of accounts.
    Group(UnorderedSet<AccountId>),
}

impl RoleKind {
    /// Checks if user matches given role.
    pub fn match_user(&self, account_id: &AccountId) -> bool {
        match self {
            RoleKind::Everyone => true,
            RoleKind::Group(accounts) => accounts.contains(&account_id),
        }
    }

    /// Returns the number of people in the this role or None if not supported role kind.
    pub fn get_role_size(&self) -> Option<usize> {
        match self {
            RoleKind::Group(accounts) => Some(accounts.len() as usize),
            _ => None,
        }
    }

    pub fn add_member_to_group(&mut self, member_id: &AccountId) -> Result<(), ()> {
        match self {
            RoleKind::Group(accounts) => {
                accounts.insert(&member_id);
                Ok(())
            }
            _ => Err(()),
        }
    }

    pub fn remove_member_from_group(&mut self, member_id: &AccountId) -> Result<(), ()> {
        match self {
            RoleKind::Group(accounts) => {
                accounts.remove(member_id);
                Ok(())
            }
            _ => Err(()),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug)]
pub struct Role {
    /// Kind of the role: defines which users this permissions apply.
    pub kind: RoleKind,
    // /// Set of actions on which proposals that this role is allowed to execute.
    // /// <proposal_kind>:<action>
    pub permissions: HashSet<Permission>,
}

#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone, Hash, Eq, PartialEq, PartialOrd)]
pub enum Permission {
    AddContent,
    DelContent,
    Like,
    Unlike,
    Report,
    ReportConfirm,
    SetRole,
    DelRole,
    AddMember,
    RemoveMember
}


#[near_bindgen]
impl Community {
    pub fn set_role(&mut self, name: String, kind: Option<RoleKindInput>, permissions: Option<Vec<Permission>>) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::SetRole), "not allowed");

        let mut role = match self.roles.get(&name) {
            Some(v) => v,
            None => Role {
                kind: RoleKind::Everyone,
                permissions: HashSet::new()
            }
        };
        if let Some(kind) = kind {
            match role.kind {
                RoleKind::Everyone => {},
                RoleKind::Group(mut group) => {
                    match kind {
                        RoleKindInput::Group=>{group.clear();}
                        RoleKindInput::Everyone => {}
                    };
                }
            }
            match kind {
                RoleKindInput::Everyone => {
                    role.kind = RoleKind::Everyone
                },
                RoleKindInput::Group => {
                    role.kind = RoleKind::Group(UnorderedSet::new(format!("{}_member", name).as_bytes()))
                }
            }
        }
        if let Some(permissions) = permissions {
            for permission in permissions {
                role.permissions.insert(permission);
            }
        }
        self.roles.insert(&name, &role);
    }

    pub fn remove_role(&mut self, name: String) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::DelRole), "not allowed");
        self.roles.remove(&name);
    }

    pub fn add_member_to_role(&mut self, name: String, member_id: AccountId) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::AddMember), "not allowed");
        let mut role = self.roles.get(&name).expect(format!("{} not found", name.as_str()).as_str());
        role.kind.add_member_to_group(&member_id).unwrap();
        self.roles.insert(&name, &role);
    }

    pub fn remove_member_from_role(&mut self, name: String, member_id: AccountId) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::RemoveMember), "not allowed");
        let mut role = self.roles.get(&name).expect(format!("{} not found", name.as_str()).as_str());
        role.kind.remove_member_from_group(&member_id).unwrap();
        self.roles.insert(&name, &role);
    }

    /// Returns set of roles that this user is member of permissions for given user across all the roles it's member of.
    fn get_user_roles(&self, account_id: AccountId) -> HashMap<String, HashSet<Permission>> {
        let mut roles = HashMap::default();
        for (name, role) in self.roles.iter() {
            if role.kind.match_user(&account_id) {
                roles.insert(name.clone(), role.permissions.clone());
            }
        }
        roles
    }

    /// Can given user execute given action on this proposal.
    /// Returns all roles that allow this action.
    pub fn can_execute_action(
        &self,
        account_id: AccountId,
        permission: Permission
    ) -> bool {
        if !self.drip.is_member(account_id.clone()) {
            return false
        }
        if account_id == self.owner_id {
            return true
        }
        let roles = self.get_user_roles(account_id);
        let mut allowed = false;
        roles
            .into_iter()
            .for_each(|(_, permissions)| {
                let allowed_role = permissions.contains(&permission);
                allowed = allowed || allowed_role;
            });
        allowed
    }

    pub fn get_allowed_roles(&self,
        account_id: AccountId,
        permission: Option<Permission>
    ) -> Vec<String> {
        if !self.drip.is_member(account_id.clone()) {
            return Vec::new()
        }
        let roles = self.get_user_roles(account_id);
        let mut allowed = false;
        let allowed_roles = roles
            .into_iter()
            .filter_map(|(role, permissions)| {
                match &permission {
                    Some(permission) => {
                        let allowed_role = permissions.contains(permission);
                        allowed = allowed || allowed_role;
                        if allowed_role {
                            Some(role)
                        } else {
                            None
                        }
                    },
                    None => {
                        Some(role)
                    }
                }
                
            })
            .collect();
        allowed_roles
    }

}