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
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct FollowData {
    pub follower: AccountId,
    pub followee: AccountId
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct ContentAddData {
    pub args: String,
    pub hierarchies: Vec<Hierarchy>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct ContentHierarchyData {
    pub hierarchies: Vec<Hierarchy>
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

    pub fn log_follow(follower: AccountId, followee: AccountId) {
        Event::Follow(vec![
            FollowData {
                follower,
                followee
            }
        ]).log()
    }

    pub fn log_unfollow(follower: AccountId, followee: AccountId) {
        Event::Unfollow(vec![
            FollowData {
                follower,
                followee
            }
        ]).log()
    }

    pub fn log_add_content(args: String, hierarchies: Vec<Hierarchy>) {
        Event::ContentAdd(vec![
            ContentAddData {
                args,
                hierarchies
            }
        ]).log()
    }

    pub fn log_del_content(hierarchies: Vec<Hierarchy>) {
        Event::ContentDel(vec![
            ContentHierarchyData {
                hierarchies
            }
        ]).log()
    }

    pub fn log_like_content(hierarchies: Vec<Hierarchy>) {
        Event::ContentLike(vec![
            ContentHierarchyData {
                hierarchies
            }
        ]).log()
    }

    pub fn log_unlike_content(hierarchies: Vec<Hierarchy>) {
        Event::ContentLike(vec![
            ContentHierarchyData {
                hierarchies
            }
        ]).log()
    }
}