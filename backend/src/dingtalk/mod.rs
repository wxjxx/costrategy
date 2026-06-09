mod client;
mod sync;

pub use client::{
    DingTalkClient, DingTalkDepartment, DingTalkLoginIdentity, DingTalkUser,
    DingTalkWorkNotification, DingtalkClientError, MockDingTalkClient,
};
pub use sync::{DingtalkSyncResult, DingtalkSyncService};
