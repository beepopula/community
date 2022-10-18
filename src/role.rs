use crate::*;
use crate::access::{FTCondition, Condition};
use crate::account::Deposit;
use crate::utils::is_registered;
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

#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug)]
pub struct Group {
    pub members: LookupMap<AccountId, HashMap<String, String>>,
}

// member keys:
//   until: U64,     for time limit groups


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
    Access(Access)
}

#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug)]
pub enum RoleKind {
    /// Matches everyone, who is not matched by other roles.
    Everyone,
    //support NEP141,"near" for near token
    // Member(AccountId, U128),
    /// Set of accounts.
    Group(Group),
    Access(Access),
}

impl RoleKind {
    /// Checks if user matches given role.
    pub fn match_user(&self, account_id: &AccountId) -> bool {
        match self {
            RoleKind::Everyone => true,
            RoleKind::Group(group) => {
                let member = match group.members.get(&account_id) {
                    Some(v) => v,
                    None => return false
                };
                if let Some(until) = member.get("until") {
                    let until: u64 = serde_json::from_str(until).unwrap();
                    env::block_timestamp() < until
                } else {
                    return true
                }
            },
            RoleKind::Access(access) => access.check_account(&account_id)
        }
    }

    // /// Returns the number of people in the this role or None if not supported role kind.
    // pub fn get_role_size(&self) -> Option<usize> {
    //     match self {
    //         RoleKind::Group(group) => Some(env::storage_read() as usize),
    //         _ => None,
    //     }
    // }

    pub fn add_member_to_group(&mut self, member_id: &AccountId, map: &HashMap<String, String>) -> Result<(), ()> {
        match self {
            RoleKind::Group(group) => {
                group.members.insert(member_id, map);
                Ok(())
            }
            _ => Err(()),
        }
    }

    pub fn remove_member_from_group(&mut self, member_id: &AccountId) -> Result<(), ()> {
        match self {
            RoleKind::Group(group) => {
                group.members.remove(member_id);
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
    pub alias: String,
    pub kind: RoleKind,
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
    Other(String)   //off-chain permission
}

pub fn init_roles(this: &mut Community) {
    let mut permissions = HashSet::new();
    permissions.insert(Permission::AddContent(0));
    permissions.insert(Permission::AddContent(1));
    permissions.insert(Permission::AddContent(2));
    permissions.insert(Permission::DelContent);
    permissions.insert(Permission::AddEncryptContent(0));
    permissions.insert(Permission::AddEncryptContent(1));
    permissions.insert(Permission::AddEncryptContent(2));
    permissions.insert(Permission::DelEncryptContent);
    permissions.insert(Permission::Like);
    permissions.insert(Permission::Unlike);
    permissions.insert(Permission::Report);
    this.roles.insert(&"all".to_string(), &Role { 
        alias: "all".to_string(),
        kind: RoleKind::Everyone, 
        permissions:  permissions.clone(),
        mod_level: 0,
        override_level: 0
    });
    this.roles.insert(&"ban".to_string(), &Role { 
        alias: "ban".to_string(),
        kind: RoleKind::Group(Group {
            members: LookupMap::new("ban_member".as_bytes())
        }), 
        permissions:  HashSet::new(),
        mod_level: 0,
        override_level: 99
    });
}


#[near_bindgen]
impl Community {

    #[payable]
    pub fn add_role(&mut self, alias: String, kind: RoleKindInput, permissions: Vec<Permission>, mod_level: u32, override_level: u32) -> String {
        let initial_storage_usage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::SetRole(None)), "not allowed");
        let hash = bs58::encode(env::sha256((alias.clone() + &env::block_timestamp().to_string()).as_bytes())).into_string();
        let mut role = match self.roles.get(&hash) {
            Some(v) => panic!("role already exist"),
            None => Role {
                alias,
                kind: RoleKind::Everyone,
                permissions: HashSet::new(),
                mod_level: if self.get_user_mod_level(&sender_id) < mod_level { mod_level } else { 0 },
                override_level: override_level
            }
        };
        
        match kind {
            RoleKindInput::Everyone => {
                role.kind = RoleKind::Everyone
            },
            RoleKindInput::Group => {
                role.kind = RoleKind::Group(Group { 
                    members: LookupMap::new(format!("{}_member", hash).as_bytes()),
                })
            },
            RoleKindInput::Access(access) => {
                role.kind = RoleKind::Access(access)
            }
        };
        for permission in permissions {
            role.permissions.insert(permission);
        }

        self.roles.insert(&hash, &role);

        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage, 0);

        hash
    }


    #[payable]
    pub fn set_role(&mut self, hash: String, alias: Option<String>, kind: Option<RoleKindInput>, permissions: Option<Vec<Permission>>, mod_level: Option<u32>, override_level: Option<u32>) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::SetRole(Some(hash.clone()))), "not allowed");
        let mut role = match self.roles.get(&hash) {
            Some(v) => v,
            None => panic!("role not exist")
        };

        if let Some(alias) = alias {
            role.alias = alias
        }

        if let Some(kind) = kind {
            match kind {
                RoleKindInput::Access(access) => {
                    role.kind = RoleKind::Access(access)
                },
                RoleKindInput::Group => {
                    let prefix = format!("{}_member", hash);
                    
                },
                RoleKindInput::Everyone => {
                    role.kind = RoleKind::Everyone
                },
            }
        }
        
        if let Some(mod_level) = mod_level {
            if mod_level < self.get_user_mod_level(&sender_id) {
                role.mod_level = role.override_level
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
        self.roles.insert(&hash, &role);
        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage, 0);
    }

    pub fn remove_role(&mut self, hash: String) {
        Base58CryptoHash::try_from(hash.clone()).unwrap();    //exclude "all" and "ban"
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::DelRole(Some(hash.clone()))), "not allowed");
        self.roles.remove(&hash);
    }

    #[payable]
    pub fn add_member_to_role(&mut self, hash: String, members: Vec<(AccountId, Option<HashMap<String, String>>)>) {
        let initial_storage_usage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::AddMember(Some(hash.clone()))), "not allowed");
        let mut role = self.roles.get(&hash).expect(format!("{} not found", hash.as_str()).as_str());
        for (account_id, options) in members {
            if !is_registered(&account_id) {
                continue
            }
            role.kind.add_member_to_group(&account_id, &options.unwrap_or(HashMap::new())).unwrap();
        }
        self.roles.insert(&hash, &role);
        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage, 0);
    }

    pub fn remove_member_from_role(&mut self, hash: String, members: Vec<AccountId>) {
        let sender_id = env::predecessor_account_id();
        assert!(self.can_execute_action(sender_id.clone(), Permission::RemoveMember(Some(hash.clone()))), "not allowed");
        let mut role = self.roles.get(&hash).expect(format!("{} not found", hash.as_str()).as_str());
        for account_id in members {
            role.kind.remove_member_from_group(&account_id).unwrap();
        }
        self.roles.insert(&hash, &role);
    }

    pub fn get_user_mod_level(&self, account_id: &AccountId) -> u32 {
        if *account_id == self.owner_id {
            return u32::MAX
        }
        let mut max_override_level = 0;
        for (hash, role) in self.roles.iter() {
            if role.override_level > max_override_level && role.kind.match_user(&account_id) {
                max_override_level = role.override_level
            }

        }
        let mut max_mod_level = 0;
        for (hash, role) in self.roles.iter() {
            if role.override_level >= max_override_level && role.kind.match_user(&account_id) {
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
        for (hash, role) in self.roles.iter() {
            if role.override_level > max_override_level && role.kind.match_user(&account_id) {
                max_override_level = role.override_level
            }

        }
        for (hash, role) in self.roles.iter() {
            if role.override_level >= max_override_level && role.kind.match_user(&account_id) {
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

    fn check_allowed(&self, permission: &Permission, permissions: &HashSet<Permission>, account_id: &AccountId) -> bool {
        if *account_id == self.owner_id {
            return true
        }
        match permission {
            Permission::SetRole(hash) => {
                let allowed = permissions.contains(&permission) || permissions.contains(&Permission::SetRole(None));
                allowed && match hash {
                    Some(hash) => self.roles.get(hash).unwrap().mod_level < self.get_user_mod_level(&account_id),
                    None => true,
                }
            },
            Permission::DelRole(hash) => {
                let allowed = permissions.contains(&permission) || permissions.contains(&Permission::DelRole(None));
                allowed && match hash {
                    Some(hash) => self.roles.get(hash).unwrap().mod_level < self.get_user_mod_level(&account_id),
                    None => true,
                }
            },
            Permission::AddMember(hash) => {
                let allowed = permissions.contains(&permission) || permissions.contains(&Permission::AddMember(None));
                allowed && match hash {
                    Some(hash) => self.roles.get(hash).unwrap().mod_level < self.get_user_mod_level(&account_id),
                    None => true,
                }
            },
            Permission::RemoveMember(hash) => {
                let allowed = permissions.contains(&permission) || permissions.contains(&Permission::RemoveMember(None));
                allowed && match hash {
                    Some(hash) => self.roles.get(hash).unwrap().mod_level < self.get_user_mod_level(&account_id),
                    None => true,
                }
            },
            _ => permissions.contains(&permission)
        }
    }

}