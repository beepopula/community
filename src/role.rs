use crate::*;
use crate::account::AssetKey;
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
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
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
    Other(Option<String>),   //off-chain permission

    AddProposal(bool),  //false for no action proposal
    Vote,
}


#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug)]
pub struct RoleManagement {
    pub roles: HashMap<String, Role>,
    pub global_role: HashMap<Permission, (Relationship, Option<Access>)>,
}

#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug)]
pub struct OldRoleManagement {
    pub roles: HashMap<String, Role>,
    pub global_role: HashMap<Permission, (Relationship, Option<OldAccess>)>,
}

impl RoleManagement {
    pub fn new() -> Self {
        let mut global_permissions = HashMap::new();
        global_permissions.insert(Permission::AddContent(0), (Relationship::Or, None));
        global_permissions.insert(Permission::AddContent(1), (Relationship::Or, None));
        global_permissions.insert(Permission::AddContent(2), (Relationship::Or, None));
        global_permissions.insert(Permission::DelContent, (Relationship::Or, None));
        global_permissions.insert(Permission::AddEncryptContent(0), (Relationship::Or, None));
        global_permissions.insert(Permission::AddEncryptContent(1), (Relationship::Or, None));
        global_permissions.insert(Permission::AddEncryptContent(2), (Relationship::Or, None));
        global_permissions.insert(Permission::DelEncryptContent, (Relationship::Or, None));
        global_permissions.insert(Permission::Like, (Relationship::Or, None));
        global_permissions.insert(Permission::Unlike, (Relationship::Or, None));
        global_permissions.insert(Permission::Report, (Relationship::Or, None));
        global_permissions.insert(Permission::Vote, (Relationship::Or, None));
        global_permissions.insert(Permission::AddProposal(false), (Relationship::Or, None));

        global_permissions.insert(Permission::AddProposal(true), (Relationship::And, None));
        global_permissions.insert(Permission::ReportConfirm, (Relationship::And, None));
        global_permissions.insert(Permission::DelOthersContent, (Relationship::And, None));
        global_permissions.insert(Permission::SetRole(None), (Relationship::And, None));
        global_permissions.insert(Permission::DelRole(None), (Relationship::And, None));
        global_permissions.insert(Permission::AddMember(None), (Relationship::And, None));
        global_permissions.insert(Permission::RemoveMember(None), (Relationship::And, None));
        global_permissions.insert(Permission::Other(None), (Relationship::And, None));
        let mut this = Self {
            roles: HashMap::new(),
            global_role: global_permissions.clone()
        };
        this.roles.insert("ban".to_string(), Role { 
            alias: "Banned".to_string(),
            members: "ban_member".to_string().into_bytes(), 
            permissions:  HashSet::new(),
            mod_level: 0,
            override_level: 99
        });
        let mut mod_permissions = HashSet::new();
        mod_permissions.insert(Permission::AddContent(0));
        mod_permissions.insert(Permission::AddContent(1));
        mod_permissions.insert(Permission::AddContent(2));
        mod_permissions.insert(Permission::DelContent);
        mod_permissions.insert(Permission::AddEncryptContent(0));
        mod_permissions.insert(Permission::AddEncryptContent(1));
        mod_permissions.insert(Permission::AddEncryptContent(2));
        mod_permissions.insert(Permission::DelEncryptContent);
        mod_permissions.insert(Permission::Like);
        mod_permissions.insert(Permission::Unlike);
        mod_permissions.insert(Permission::Report);
        mod_permissions.insert(Permission::Vote);
        mod_permissions.insert(Permission::AddProposal(false));
        mod_permissions.insert(Permission::AddProposal(true));
        mod_permissions.insert(Permission::ReportConfirm);
        mod_permissions.insert(Permission::DelOthersContent);
        mod_permissions.insert(Permission::SetRole(None));
        mod_permissions.insert(Permission::DelRole(None));
        mod_permissions.insert(Permission::AddMember(None));
        mod_permissions.insert(Permission::RemoveMember(None));
        mod_permissions.insert(Permission::Other(None));
        this.roles.insert("mod".to_string(), Role { 
            alias: "Mod".to_string(),
            members: "mod_member".to_string().into_bytes(), 
            permissions:  mod_permissions,
            mod_level: 2,
            override_level: 0
        });
        this
    }
}


#[near_bindgen]
impl Community {

    pub fn set_global_role(&mut self, permissions: Vec<Permission>, options: Vec<(Relationship, Option<Access>)>) {
        let initial_storage_usage = env::storage_usage();
        assert!(self.can_execute_action(None, None, Permission::SetRole(None)), "not allowed");
        self.role_management.global_role.clear();
        for i in 0..permissions.len() {
            self.role_management.global_role.insert(permissions[i].clone(), options[i].clone());
        }
        set_storage_usage(initial_storage_usage, None);
    }

    pub fn add_role(&mut self, alias: String, permissions: Vec<Permission>, mod_level: u32, override_level: u32) -> String {
        let initial_storage_usage = env::storage_usage();
        let sender_id = get_predecessor_id();
        assert!(self.can_execute_action(None, None, Permission::SetRole(None)), "not allowed");
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

        set_storage_usage(initial_storage_usage, None);

        hash
    }


    pub fn set_role(&mut self, hash: Base58CryptoHash, alias: Option<String>, permissions: Option<Vec<Permission>>, mod_level: Option<u32>, override_level: Option<u32>) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = get_predecessor_id();
        let hash = String::from(&hash);
        assert!(self.can_execute_action(None, None, Permission::SetRole(Some(hash.clone()))), "not allowed");
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
            role.permissions.clear();
            for permission in permissions {
                role.permissions.insert(permission);
            }
        }
        self.role_management.roles.insert(hash, role);
        set_storage_usage(initial_storage_usage, None);
    }

    pub fn remove_role(&mut self, hash: String) {
        let initial_storage_usage = env::storage_usage();
        Base58CryptoHash::try_from(hash.clone()).unwrap();    //exclude "all" and "ban"
        let sender_id = get_predecessor_id();
        assert!(self.can_execute_action(None, None, Permission::DelRole(Some(hash.clone()))), "not allowed");
        self.role_management.roles.remove(&hash);
        set_storage_usage(initial_storage_usage, None);
    }

    #[payable]
    pub fn add_member_to_role(&mut self, hash: String, members: Vec<(AccountId, Option<HashMap<String, String>>)>) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = get_predecessor_id();
        assert!(self.can_execute_action(None, None, Permission::AddMember(Some(hash.clone()))), "not allowed");
        let role = self.role_management.roles.get(&hash).expect(format!("{} not found", hash.as_str()).as_str());
        let mut role_members: LookupMap<AccountId, HashMap<String, String>> = LookupMap::new(role.members.clone());
        let mod_level = self.get_user_mod_level(&sender_id);
        for (account_id, options) in members {
            assert!(mod_level > self.get_user_mod_level(&account_id), "not allowed");
            role_members.insert(&account_id, &options.unwrap_or(HashMap::new()));
        }
        set_storage_usage(initial_storage_usage, None);
    }

    pub fn remove_member_from_role(&mut self, hash: String, members: Vec<AccountId>) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = get_predecessor_id();
        assert!(self.can_execute_action(None, None, Permission::RemoveMember(Some(hash.clone()))), "not allowed");
        let role = self.role_management.roles.get(&hash).expect(format!("{} not found", hash.as_str()).as_str());
        let mut role_members: LookupMap<AccountId, HashMap<String, String>> = LookupMap::new(role.members.clone());
        let mod_level = self.get_user_mod_level(&sender_id);
        for account_id in members {
            assert!(mod_level > self.get_user_mod_level(&account_id), "not allowed");
            role_members.remove(&account_id);
        }
        set_storage_usage(initial_storage_usage, None);
    }

    pub fn set_members(&mut self, add: HashMap<String, Vec<AccountId>>, remove: HashMap<String, Vec<AccountId>>) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = get_predecessor_id();
        let mod_level = self.get_user_mod_level(&sender_id);
        for (hash, members) in add.iter() {
            assert!(self.can_execute_action(None, None, Permission::AddMember(Some(hash.clone()))), "not allowed");
            let role = match self.role_management.roles.get(hash) {
                Some(role) => role,
                None => continue,
            };
            let mut role_members: LookupMap<AccountId, HashMap<String, String>> = LookupMap::new(role.members.clone());
            for account_id in members {
                assert!(mod_level > self.get_user_mod_level(&account_id), "not allowed");
                role_members.insert(&account_id, &HashMap::new());
            }
        }

        for (hash, members) in remove.iter() {
            assert!(self.can_execute_action(None, None, Permission::RemoveMember(Some(hash.clone()))), "not allowed");
            let role = match self.role_management.roles.get(hash) {
                Some(role) => role,
                None => continue,
            };
            let mut role_members: LookupMap<AccountId, HashMap<String, String>> = LookupMap::new(role.members.clone());
            for account_id in members {
                assert!(self.can_execute_action(None, None, Permission::RemoveMember(Some(hash.clone()))), "not allowed");
                role_members.remove(&account_id);
            }
        }
        set_storage_usage(initial_storage_usage, None);
    }

    pub fn get_user_mod_level(&self, account_id: &AccountId) -> u32 {
        if *account_id == self.owner_id || *account_id == env::current_account_id() {
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
    pub fn get_user_roles(&self, account_id: &AccountId) -> HashMap<String, Role> {
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
                roles.insert(hash.clone(), role.clone());
            }
        }
        roles
    }

    /// Can given user execute given action on this proposal.
    /// Returns all roles that allow this action.
    pub fn can_execute_action(
        &mut self,
        signer_id: Option<AccountId>,
        sender_id: Option<AccountId>,
        permission: Permission
    ) -> bool {
        let signer_id = match signer_id {
            Some(v) => v,
            None => env::signer_account_id()
        };
        let sender_id = match sender_id {
            Some(v) => v,
            None => get_predecessor_id()
        };
        if signer_id == self.owner_id || signer_id == env::current_account_id() {
            return true
        }

        if !get_account(&signer_id).is_registered() {
            return false
        }

        let mut account_ids = vec![signer_id.clone()];
        if signer_id != sender_id {
            account_ids.push(sender_id);
        }


        for account_id in account_ids {
            let mut allowed = false;
            let mut max_override_level = 0;
            let roles = self.get_user_roles(&account_id);
            for (_, role) in roles.into_iter() {
                max_override_level = role.override_level;
                if self.check_allowed(&permission, &role.permissions, &account_id) {
                    allowed = true;
                    break
                }
            }
            if max_override_level == 0 && allowed == false {
                match self.check_global_allowed(&permission, &account_id) {
                    Some(a) => {
                        allowed = a;
                    },
                    None => {}
                }
            }
            if allowed == false {
                return false
            }
        }
        true
    }

    pub fn get_allowed_roles(&self,
        account_id: AccountId,
        permission: Option<Permission>
    ) -> Vec<String> {
        if !get_account(&account_id).is_registered() {
            return Vec::new()
        }
        let roles = self.get_user_roles(&account_id);
        let mut allowed = false;
        let allowed_roles = roles
            .into_iter()
            .filter_map(|(role_name, role)| {
                match &permission {
                    Some(permission) => {
                        let allowed_role = self.check_allowed(&permission, &role.permissions, &account_id);
                        allowed = allowed || allowed_role;
                        if allowed_role {
                            Some(role_name)
                        } else {
                            None
                        }
                    },
                    None => {
                        Some(role_name)
                    }
                }
                
            })
            .collect();
        allowed_roles
    }

    fn check_global_allowed(&mut self, permission: &Permission, account_id: &AccountId) -> Option<bool> {
        if *account_id == self.owner_id || *account_id == env::current_account_id() {
            return Some(true)
        }
        let permissions = self.role_management.global_role.clone();
        let (relationship, option) = match permissions.get(&permission) {
            Some(val) => val,
            None => {
                match permission {
                    Permission::SetRole(hash) => { 
                        match permissions.get(&Permission::SetRole(None)) {
                            Some(val) => val,
                            None => return Some(false)
                        }
                    },
                    Permission::DelRole(hash) => {
                        match permissions.get(&Permission::DelRole(None)) {
                            Some(val) => val,
                            None => return Some(false)
                        }
                    },
                    Permission::AddMember(hash) => {
                        match permissions.get(&Permission::AddMember(None)) {
                            Some(val) => val,
                            None => return Some(false)
                        }
                    },
                    Permission::RemoveMember(hash) => {
                        match permissions.get(&Permission::RemoveMember(None)) {
                            Some(val) => val,
                            None => return Some(false)
                        }
                    },
                    Permission::Other(_) => {
                        match permissions.get(&Permission::Other(None)) {
                            Some(val) => val,
                            None => return Some(false)
                        }
                    },
                    _ => return Some(false)
                }
            }
        };
        
        match relationship {
            Relationship::Or => {
                if let Some(mut access) = option.clone() {
                    let mut account = get_account(account_id);
                    if account.check_condition(&access) || account.set_condition(&access, None) {
                        set_account(&account);
                        return Some(true)
                    } 
                    return None
                }
                return Some(true)
            },
            Relationship::And => {
                if let Some(mut access) = option.clone() {
                    let mut account = get_account(account_id);
                    if !account.check_condition(&access) && !account.set_condition(&access, None) {
                        set_account(&account);
                        return Some(false)
                    }
                    return None
                }
                return None
            },
        }
        
        
    }

    fn check_allowed(&self, permission: &Permission, permissions: &HashSet<Permission>, account_id: &AccountId) -> bool {
        if *account_id == self.owner_id || *account_id == env::current_account_id() {
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

    use crate::account::{Relationship, Access};

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
                permissions.contains(&permission) || permissions.contains(&Permission::SetRole(None))
            },
            Permission::DelRole(hash) => {
                permissions.contains(&permission) || permissions.contains(&Permission::DelRole(None))
            },
            Permission::AddMember(hash) => {
                permissions.contains(&permission) || permissions.contains(&Permission::AddMember(None))
            },
            Permission::RemoveMember(hash) => {
                permissions.contains(&permission) || permissions.contains(&Permission::RemoveMember(None))
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
        let res = check_global_allowed(&Permission::AddContent(0 as u8), permissions);
        print!("1: {:?}", res);

        let mut permissions = HashSet::new();
        permissions.insert(Permission::AddContent(0));
        let res = check_allowed(&Permission::AddContent(0 as u8), &permissions);
        print!("2: {:?}", res)
    }

}