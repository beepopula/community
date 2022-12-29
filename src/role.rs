use crate::*;
use crate::access::{FTCondition, Condition, Relationship};
use crate::account::Deposit;
use crate::utils::{is_registered};
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, AccountId, Balance};

// #[derive(BorshSerialize, BorshDeserialize)]
// #[derive(Debug)]
// pub struct Member {
//     until: Option<u64>,
//     after_action: Vec<String>,
//     before_action: Vec<String>
// }

// member keys:
//   until: U64,     for time limit groups

#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug, Clone)]
pub struct Role {
    /// Kind of the role: defines which users this permissions apply.
    pub alias: String,
    pub members: Vec<u8>,
    pub permissions: HashSet<Permission>,
    pub mod_level: u32,
    pub override_level: u32    // can override lower level group permissions, like black list
}

#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[derive(Debug, Clone, Hash, Eq, PartialEq, PartialOrd)]
pub enum Permission {
    AddContent(u8),   //hierarchy level count of content
    DelContent,
    AddEncryptContent(u8),   //hierarchy level count of content
    DelEncryptContent,
    Like,
    Unlike,
    Share,
    Report,



    ReportConfirm,
    DelOthersContent,
    SetRole(Option<String>),
    DelRole(Option<String>),
    AddMember(Option<String>),
    RemoveMember(Option<String>),
    Other(Option<String>)   //off-chain permission
}


#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug)]
pub struct RoleManagement {
    pub roles: HashMap<String, Role>,
    pub global_role: HashMap<Permission, (Relationship, Option<Access>)>,
}

impl RoleManagement {
    pub fn new() -> Self {
        let mut permissions = HashMap::new();
        permissions.insert(Permission::AddContent(0), (Relationship::Or, None));
        permissions.insert(Permission::AddContent(1), (Relationship::Or, None));
        permissions.insert(Permission::AddContent(2), (Relationship::Or, None));
        permissions.insert(Permission::DelContent, (Relationship::Or, None));
        permissions.insert(Permission::AddEncryptContent(0), (Relationship::Or, None));
        permissions.insert(Permission::AddEncryptContent(1), (Relationship::Or, None));
        permissions.insert(Permission::AddEncryptContent(2), (Relationship::Or, None));
        permissions.insert(Permission::DelEncryptContent, (Relationship::Or, None));
        permissions.insert(Permission::Like, (Relationship::Or, None));
        permissions.insert(Permission::Unlike, (Relationship::Or, None));
        permissions.insert(Permission::Report, (Relationship::Or, None));
        permissions.insert(Permission::ReportConfirm, (Relationship::And, None));
        permissions.insert(Permission::DelOthersContent, (Relationship::And, None));
        permissions.insert(Permission::SetRole(None), (Relationship::And, None));
        permissions.insert(Permission::DelRole(None), (Relationship::And, None));
        permissions.insert(Permission::AddMember(None), (Relationship::And, None));
        permissions.insert(Permission::RemoveMember(None), (Relationship::And, None));
        permissions.insert(Permission::Other(None), (Relationship::And, None));
        let mut this = Self {
            roles: HashMap::new(),
            global_role: permissions.clone()
        };
        this.roles.insert("ban".to_string(), Role { 
            alias: "ban".to_string(),
            members: "ban_member".to_string().into_bytes(), 
            permissions:  HashSet::new(),
            mod_level: 0,
            override_level: 99
        });
        this
    }
}


#[near_bindgen]
impl Community {

    #[payable]
    pub fn set_global_role(&mut self, permissions: Vec<Permission>, options: Vec<(Relationship, Option<Access>)>) {
        for i in 0..permissions.len() {
            self.role_management.global_role.insert(permissions[i].clone(), options[i].clone());
        }
    }

    #[payable]
    pub fn add_role(&mut self, alias: String, permissions: Vec<Permission>, mod_level: u32, override_level: u32) -> String {
        let initial_storage_usage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::SetRole(None)), "not allowed");
        let hash = bs58::encode(env::sha256((alias.clone() + &env::block_timestamp().to_string()).as_bytes())).into_string();
        let mut role = match self.role_management.roles.get(&hash) {
            Some(v) => panic!("role already exist"),
            None => Role {
                alias,
                members: format!("{}_member", hash).into_bytes(),
                permissions: HashSet::new(),
                mod_level: if self.get_user_mod_level(&sender_id) < mod_level { 0 } else { mod_level },
                override_level: override_level
            }
        };

        for permission in permissions {
            role.permissions.insert(permission);
        }

        self.role_management.roles.insert(hash.clone(), role);

        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage, 0);

        hash
    }


    #[payable]
    pub fn set_role(&mut self, hash: String, alias: Option<String>, permissions: Option<Vec<Permission>>, mod_level: Option<u32>, override_level: Option<u32>) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::SetRole(Some(hash.clone()))), "not allowed");
        let mut role = match self.role_management.roles.get(&hash) {
            Some(v) => v.clone(),
            None => panic!("role not exist")
        };

        if let Some(alias) = alias {
            role.alias = alias
        }
        
        if let Some(mod_level) = mod_level {
            if mod_level < (*self).get_user_mod_level(&sender_id) {
                role.mod_level = mod_level
            }
        }

        if let Some(override_level) = override_level {
            role.override_level = override_level;
        }
        
        if let Some(permissions) = permissions {
            for permission in permissions {
                role.permissions.insert(permission);
            }
        }
        self.role_management.roles.insert(hash, role);
        let storage_usage = match env::storage_usage().checked_sub(initial_storage_usage) {
            Some(storage_usage) => storage_usage,
            None => 0,
        };
        refund_extra_storage_deposit(storage_usage, 0);
    }

    pub fn remove_role(&mut self, hash: String) {
        Base58CryptoHash::try_from(hash.clone()).unwrap();    //exclude "all" and "ban"
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::DelRole(Some(hash.clone()))), "not allowed");
        self.role_management.roles.remove(&hash);
    }

    #[payable]
    pub fn add_member_to_role(&mut self, hash: String, members: Vec<(AccountId, Option<HashMap<String, String>>)>) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::AddMember(Some(hash.clone()))), "not allowed");
        let role = self.role_management.roles.get(&hash).expect(format!("{} not found", hash.as_str()).as_str());
        let mut role_members: LookupMap<AccountId, HashMap<String, String>> = LookupMap::new(role.members.clone());
        for (account_id, options) in members {
            if !is_registered(&account_id) {
                continue
            }
            role_members.insert(&account_id, &options.unwrap_or(HashMap::new()));
        }
        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage, 0);
    }

    pub fn remove_member_from_role(&mut self, hash: String, members: Vec<AccountId>) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::RemoveMember(Some(hash.clone()))), "not allowed");
        let role = self.role_management.roles.get(&hash).expect(format!("{} not found", hash.as_str()).as_str());
        let mut role_members: LookupMap<AccountId, HashMap<String, String>> = LookupMap::new(role.members.clone());
        for account_id in members {
            role_members.remove(&account_id);
        }
    }

    #[payable]
    pub fn set_members(&mut self, add: HashMap<String, Vec<AccountId>>, remove: HashMap<String, Vec<AccountId>>) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        for (hash, members) in add.iter() {
            if !self.can_execute_action(sender_id.clone(), Permission::AddMember(Some(hash.clone()))) {
                continue
            }
            let role = match self.role_management.roles.get(hash) {
                Some(role) => role,
                None => continue,
            };
            let mut role_members: LookupMap<AccountId, HashMap<String, String>> = LookupMap::new(role.members.clone());
            for account_id in members {
                if !is_registered(&account_id) {
                    continue
                }
                role_members.insert(&account_id, &HashMap::new());
            }
        }

        for (hash, members) in remove.iter() {
            if !self.can_execute_action(sender_id.clone(), Permission::RemoveMember(Some(hash.clone()))) {
                continue
            }
            let role = match self.role_management.roles.get(hash) {
                Some(role) => role,
                None => continue,
            };
            let mut role_members: LookupMap<AccountId, HashMap<String, String>> = LookupMap::new(role.members.clone());
            for account_id in members {
                if !is_registered(&account_id) {
                    continue
                }
                role_members.remove(&account_id);
            }
        }
        let storage_usage = match env::storage_usage().checked_sub(initial_storage_usage) {
            Some(storage_usage) => storage_usage,
            None => 0,
        };
        refund_extra_storage_deposit(storage_usage, 0);
    }

    pub fn get_user_mod_level(&self, account_id: &AccountId) -> u32 {
        if *account_id == self.owner_id {
            return u32::MAX
        }
        let mut max_override_level = 0;
        for (hash, role) in self.role_management.roles.iter() {
            let role_members: LookupMap<AccountId, HashMap<String, String>> = LookupMap::new(role.members.clone());
            if role.override_level > max_override_level && role_members.contains_key(&account_id) {
                max_override_level = role.override_level
            }

        }
        let mut max_mod_level = 0;
        for (hash, role) in self.role_management.roles.iter() {
            let role_members: LookupMap<AccountId, HashMap<String, String>> = LookupMap::new(role.members.clone());
            if role.override_level >= max_override_level && role_members.contains_key(&account_id) {
                if role.mod_level > max_mod_level {
                    max_mod_level = role.mod_level;
                }
            }
        }
        max_mod_level
    }

    /// Returns set of roles that this user is member of permissions for given user across all the roles it's member of.
    pub fn get_user_roles(&self, account_id: &AccountId) -> HashMap<String, HashSet<Permission>> {
        let mut roles = HashMap::default();
        let mut max_override_level = 0;
        for (hash, role) in self.role_management.roles.iter() {
            let role_members: LookupMap<AccountId, HashMap<String, String>> = LookupMap::new(role.members.clone());
            if role.override_level > max_override_level && role_members.contains_key(&account_id) {
                max_override_level = role.override_level
            }

        }
        for (hash, role) in self.role_management.roles.iter() {
            let role_members: LookupMap<AccountId, HashMap<String, String>> = LookupMap::new(role.members.clone());
            if role.override_level >= max_override_level && role_members.contains_key(&account_id) {
                roles.insert(hash.clone(), role.permissions.clone());
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
        if !is_registered(&account_id) {
            return false
        }
        if account_id == self.owner_id {
            return true
        }

        match self.check_global_allowed(&permission, &account_id) {
            Some(allowed) => return allowed,
            None => {}
        }

        let roles = self.get_user_roles(&account_id);
        let mut allowed = false;
        roles
            .into_iter()
            .for_each(|(_, permissions)| {
                let allowed_role = self.check_allowed(&permission, &permissions, &account_id);
                allowed = allowed || allowed_role;
            });
        allowed
    }

    pub fn get_allowed_roles(&self,
        account_id: AccountId,
        permission: Option<Permission>
    ) -> Vec<String> {
        if !is_registered(&account_id) {
            return Vec::new()
        }
        let roles = self.get_user_roles(&account_id);
        let mut allowed = false;
        let allowed_roles = roles
            .into_iter()
            .filter_map(|(role, permissions)| {
                match &permission {
                    Some(permission) => {
                        let allowed_role = self.check_allowed(&permission, &permissions, &account_id);
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

    fn check_global_allowed(&self, permission: &Permission, account_id: &AccountId) -> Option<bool> {
        if *account_id == self.owner_id {
            return Some(true)
        }
        let permissions = self.role_management.global_role.clone();
        let relationship = match permission {
            Permission::SetRole(hash) => {
                match permissions.get(&permission) {
                    Some(val) => val,
                    None => match permissions.get(&Permission::SetRole(None)) {
                        Some(val) => val,
                        None => return Some(false)
                    }
                }
            },
            Permission::DelRole(hash) => {
                match permissions.get(&permission) {
                    Some(val) => val,
                    None => match permissions.get(&Permission::DelRole(None)) {
                        Some(val) => val,
                        None => return Some(false)
                    }
                }
            },
            Permission::AddMember(hash) => {
                match permissions.get(&permission) {
                    Some(val) => val,
                    None => match permissions.get(&Permission::AddMember(None)) {
                        Some(val) => val,
                        None => return Some(false)
                    }
                }
            },
            Permission::RemoveMember(hash) => {
                match permissions.get(&permission) {
                    Some(val) => val,
                    None => match permissions.get(&Permission::RemoveMember(None)) {
                        Some(val) => val,
                        None => return Some(false)
                    }
                }
            },
            Permission::Other(hash) => {
                match permissions.get(&permission) {
                    Some(val) => val,
                    None => match permissions.get(&Permission::Other(None)) {
                        Some(val) => val,
                        None => return Some(false)
                    }
                }
            },
            _ => match permissions.get(&permission) {
                Some(val) => val,
                None => return Some(false)
            }
        };
        match relationship.0 {
            Relationship::Or => {
                if let Some(access) = &relationship.1 {
                    if access.check_account(&account_id) {
                        return Some(true)
                    }
                    return None
                }
                return Some(true)
            },
            Relationship::And => {
                if let Some(access) = &relationship.1 {
                    if !access.check_account(&account_id) {
                        return Some(false)
                    }
                    return None
                }
                return None
            },
        }
        
        
    }

    fn check_allowed(&self, permission: &Permission, permissions: &HashSet<Permission>, account_id: &AccountId) -> bool {
        if *account_id == self.owner_id {
            return true
        }
        match permission {
            Permission::SetRole(hash) => {
                let allowed = permissions.contains(&permission) || permissions.contains(&Permission::SetRole(None));
                allowed && match hash {
                    Some(hash) => self.role_management.roles.get(hash).unwrap().mod_level < self.get_user_mod_level(&account_id),
                    None => true,
                }
            },
            Permission::DelRole(hash) => {
                let allowed = permissions.contains(&permission) || permissions.contains(&Permission::DelRole(None));
                allowed && match hash {
                    Some(hash) => self.role_management.roles.get(hash).unwrap().mod_level < self.get_user_mod_level(&account_id),
                    None => true,
                }
            },
            Permission::AddMember(hash) => {
                let allowed = permissions.contains(&permission) || permissions.contains(&Permission::AddMember(None));
                allowed && match hash {
                    Some(hash) => self.role_management.roles.get(hash).unwrap().mod_level < self.get_user_mod_level(&account_id),
                    None => true,
                }
            },
            Permission::RemoveMember(hash) => {
                let allowed = permissions.contains(&permission) || permissions.contains(&Permission::RemoveMember(None));
                allowed && match hash {
                    Some(hash) => self.role_management.roles.get(hash).unwrap().mod_level < self.get_user_mod_level(&account_id),
                    None => true,
                }
            },
            Permission::Other(_) => permissions.contains(&permission) || permissions.contains(&Permission::Other(None)),
            _ => permissions.contains(&permission)
        }
    }

}



#[cfg(test)]
mod tests {
    use std::{convert::TryInto, collections::{HashMap, HashSet}};

    use near_sdk::AccountId;

    use crate::access::{Relationship, Access};

    use super::{RoleManagement, Permission};

    fn check_global_allowed(permission: &Permission, permissions: HashMap<Permission, (Relationship, Option<Access>)>) -> Option<bool> {
        let relationship = match permission {
            Permission::SetRole(hash) => {
                match permissions.get(&permission) {
                    Some(val) => val,
                    None => match permissions.get(&Permission::SetRole(None)) {
                        Some(val) => val,
                        None => return Some(false)
                    }
                }
            },
            Permission::DelRole(hash) => {
                match permissions.get(&permission) {
                    Some(val) => val,
                    None => match permissions.get(&Permission::DelRole(None)) {
                        Some(val) => val,
                        None => return Some(false)
                    }
                }
            },
            Permission::AddMember(hash) => {
                match permissions.get(&permission) {
                    Some(val) => val,
                    None => match permissions.get(&Permission::AddMember(None)) {
                        Some(val) => val,
                        None => return Some(false)
                    }
                }
            },
            Permission::RemoveMember(hash) => {
                match permissions.get(&permission) {
                    Some(val) => val,
                    None => match permissions.get(&Permission::RemoveMember(None)) {
                        Some(val) => val,
                        None => return Some(false)
                    }
                }
            },
            Permission::Other(hash) => {
                match permissions.get(&permission) {
                    Some(val) => val,
                    None => match permissions.get(&Permission::Other(None)) {
                        Some(val) => val,
                        None => return Some(false)
                    }
                }
            },
            _ => match permissions.get(&permission) {
                Some(val) => val,
                None => return Some(false)
            }
        };
        match relationship.0 {
            Relationship::Or => {
                if let Some(access) = &relationship.1 {
                    return None
                }
                return Some(true)
            },
            Relationship::And => {
                if let Some(access) = &relationship.1 {
                    return None
                }
                return None
            },
        }
        
        
    }

    fn check_allowed(permission: &Permission, permissions: &HashSet<Permission>) -> bool {
        match permission {
            Permission::SetRole(hash) => {
                let allowed = permissions.contains(&permission) || permissions.contains(&Permission::SetRole(None));
                allowed && match hash {
                    Some(hash) => true,
                    None => true,
                }
            },
            Permission::DelRole(hash) => {
                let allowed = permissions.contains(&permission) || permissions.contains(&Permission::DelRole(None));
                allowed && match hash {
                    Some(hash) => true,
                    None => true,
                }
            },
            Permission::AddMember(hash) => {
                let allowed = permissions.contains(&permission) || permissions.contains(&Permission::AddMember(None));
                allowed && match hash {
                    Some(hash) => true,
                    None => true,
                }
            },
            Permission::RemoveMember(hash) => {
                let allowed = permissions.contains(&permission) || permissions.contains(&Permission::RemoveMember(None));
                allowed && match hash {
                    Some(hash) => true,
                    None => true,
                }
            },
            Permission::Other(_) => permissions.contains(&permission) || permissions.contains(&Permission::Other(None)),
            _ => permissions.contains(&permission)
        }
    }

    #[test]
    pub fn test() {
        let mut permissions = HashMap::new();
        permissions.insert(Permission::AddContent(0), (Relationship::Or, None));
        permissions.insert(Permission::AddContent(1), (Relationship::Or, None));
        permissions.insert(Permission::AddContent(2), (Relationship::Or, None));
        permissions.insert(Permission::DelContent, (Relationship::Or, None));
        permissions.insert(Permission::AddEncryptContent(0), (Relationship::Or, None));
        permissions.insert(Permission::AddEncryptContent(1), (Relationship::Or, None));
        permissions.insert(Permission::AddEncryptContent(2), (Relationship::Or, None));
        permissions.insert(Permission::DelEncryptContent, (Relationship::Or, None));
        permissions.insert(Permission::Like, (Relationship::Or, None));
        permissions.insert(Permission::Unlike, (Relationship::Or, None));
        permissions.insert(Permission::Report, (Relationship::Or, None));
        permissions.insert(Permission::ReportConfirm, (Relationship::And, None));
        permissions.insert(Permission::DelOthersContent, (Relationship::And, None));
        permissions.insert(Permission::SetRole(None), (Relationship::And, None));
        permissions.insert(Permission::DelRole(None), (Relationship::And, None));
        permissions.insert(Permission::AddMember(None), (Relationship::And, None));
        permissions.insert(Permission::RemoveMember(None), (Relationship::And, None));
        permissions.insert(Permission::Other(None), (Relationship::And, None));
        let res = check_global_allowed(&Permission::AddMember(Some("ban".to_string())), permissions);
        print!("1: {:?}", res);

        let mut permissions = HashSet::new();
        permissions.insert(Permission::AddMember(None));
        let res = check_allowed(&Permission::AddMember(Some("ban".to_string())), &permissions);
        print!("2: {:?}", res)
    }

}