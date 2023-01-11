use std::fmt::Display;
use crate::*;
use post::Hierarchy;


#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum Event {
    Follow(Vec<FollowData>),
    Unfollow(Vec<FollowData>),
    ContentAdd(Vec<ContentAddData>),
    ContentDel(Vec<ContentHierarchyData>),
    ContentLike(Vec<ContentHierarchyData>),
    ContentUnlike(Vec<ContentHierarchyData>),

    //custome events
    Invite(Vec<InviteData>),
    Refund(Vec<RefundData>),
    SetMetadata(Vec<Metadata>)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Metadata {
    pub key: String,
    pub val: String
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct RefundData {
    pub memo: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct FollowData {
    pub follower: AccountId,
    pub followee: AccountId,
    pub memo: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct ContentAddData {
    pub args: String,
    pub hierarchies: Vec<Hierarchy>,
    pub memo: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct ContentHierarchyData {
    pub hierarchies: Vec<Hierarchy>,
    pub memo: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct InviteData {
    pub inviter_id: AccountId,
    pub invitee_id: AccountId,
    pub memo: Option<String>
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("EVENT_JSON:{}", self.to_json_string()))
    }
}

impl Event {

    pub(crate) fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn log(&self) {
        log!(&format!("EVENT_JSON:{}", self.to_json_string()));
    }

    pub fn log_follow(follower: AccountId, followee: AccountId, memo: Option<String>) {
        Event::Follow(vec![
            FollowData {
                follower,
                followee,
                memo
            }
        ]).log()
    }

    pub fn log_unfollow(follower: AccountId, followee: AccountId, memo: Option<String>) {
        Event::Unfollow(vec![
            FollowData {
                follower,
                followee,
                memo
            }
        ]).log()
    }

    pub fn log_add_content(args: String, hierarchies: Vec<Hierarchy>, memo: Option<String>) {
        Event::ContentAdd(vec![
            ContentAddData {
                args,
                hierarchies,
                memo
            }
        ]).log()
    }

    pub fn log_del_content(hierarchies: Vec<Hierarchy>, memo: Option<String>) {
        Event::ContentDel(vec![
            ContentHierarchyData {
                hierarchies,
                memo
            }
        ]).log()
    }

    pub fn log_like_content(hierarchies: Vec<Hierarchy>, memo: Option<String>) {
        Event::ContentLike(vec![
            ContentHierarchyData {
                hierarchies,
                memo
            }
        ]).log()
    }

    pub fn log_unlike_content(hierarchies: Vec<Hierarchy>, memo: Option<String>) {
        Event::ContentLike(vec![
            ContentHierarchyData {
                hierarchies,
                memo
            }
        ]).log()
    }

    pub fn log_invite(inviter_id: AccountId, invitee_id: AccountId, memo: Option<String>) {
        Event::Invite(vec![
            InviteData {
                inviter_id,
                invitee_id,
                memo
            }
        ]).log()
    }

    pub fn log_refund(memo: Option<String>) {
        Event::Refund(vec![
            RefundData {
                memo
            }
        ]).log()
    }

    pub fn log_set_metadata(metadata: Vec<Metadata>) {
        Event::SetMetadata(metadata).log()
    }
}