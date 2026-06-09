use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DingTalkLoginIdentity {
    pub dingtalk_user_id: String,
    pub union_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DingTalkDepartment {
    pub dingtalk_dept_id: i64,
    pub parent_dingtalk_dept_id: Option<i64>,
    pub name: String,
    pub order_no: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DingTalkUser {
    pub dingtalk_user_id: String,
    pub union_id: Option<String>,
    pub name: String,
    pub avatar_url: Option<String>,
    pub mobile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DingTalkWorkNotification {
    pub receiver_dingtalk_user_id: String,
    pub message: String,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum DingtalkClientError {
    #[error("login failed")]
    LoginFailed,
    #[error("sync failed")]
    SyncFailed,
    #[error("notify failed")]
    NotifyFailed,
}

#[async_trait]
pub trait DingTalkClient: Clone + Send + Sync + 'static {
    async fn exchange_login_code(
        &self,
        code: &str,
    ) -> Result<DingTalkLoginIdentity, DingtalkClientError>;

    async fn list_departments(&self) -> Result<Vec<DingTalkDepartment>, DingtalkClientError>;

    async fn list_users_by_department(
        &self,
        dingtalk_dept_id: i64,
    ) -> Result<Vec<DingTalkUser>, DingtalkClientError>;

    async fn send_work_notification(
        &self,
        receiver_dingtalk_user_id: &str,
        message: &str,
    ) -> Result<(), DingtalkClientError>;
}

#[derive(Debug, Clone, Default)]
pub struct MockDingTalkClient {
    inner: Arc<Mutex<MockDingTalkState>>,
}

#[derive(Debug, Default)]
struct MockDingTalkState {
    login_identities: HashMap<String, DingTalkLoginIdentity>,
    departments: Vec<DingTalkDepartment>,
    department_users: HashMap<i64, Vec<DingTalkUser>>,
    notifications: Vec<DingTalkWorkNotification>,
    fail_notifications: bool,
}

impl MockDingTalkClient {
    pub fn with_login_identity(
        self,
        code: impl Into<String>,
        identity: DingTalkLoginIdentity,
    ) -> Self {
        self.inner
            .lock()
            .expect("mock dingtalk lock")
            .login_identities
            .insert(code.into(), identity);
        self
    }

    pub fn with_departments(self, departments: Vec<DingTalkDepartment>) -> Self {
        self.inner.lock().expect("mock dingtalk lock").departments = departments;
        self
    }

    pub fn with_department_users(self, dingtalk_dept_id: i64, users: Vec<DingTalkUser>) -> Self {
        self.inner
            .lock()
            .expect("mock dingtalk lock")
            .department_users
            .insert(dingtalk_dept_id, users);
        self
    }

    pub fn with_notification_failure(self) -> Self {
        self.inner
            .lock()
            .expect("mock dingtalk lock")
            .fail_notifications = true;
        self
    }

    pub fn sent_notifications(&self) -> Vec<DingTalkWorkNotification> {
        self.inner
            .lock()
            .expect("mock dingtalk lock")
            .notifications
            .clone()
    }
}

#[async_trait]
impl DingTalkClient for MockDingTalkClient {
    async fn exchange_login_code(
        &self,
        code: &str,
    ) -> Result<DingTalkLoginIdentity, DingtalkClientError> {
        self.inner
            .lock()
            .expect("mock dingtalk lock")
            .login_identities
            .get(code)
            .cloned()
            .ok_or(DingtalkClientError::LoginFailed)
    }

    async fn list_departments(&self) -> Result<Vec<DingTalkDepartment>, DingtalkClientError> {
        Ok(self
            .inner
            .lock()
            .expect("mock dingtalk lock")
            .departments
            .clone())
    }

    async fn list_users_by_department(
        &self,
        dingtalk_dept_id: i64,
    ) -> Result<Vec<DingTalkUser>, DingtalkClientError> {
        Ok(self
            .inner
            .lock()
            .expect("mock dingtalk lock")
            .department_users
            .get(&dingtalk_dept_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn send_work_notification(
        &self,
        receiver_dingtalk_user_id: &str,
        message: &str,
    ) -> Result<(), DingtalkClientError> {
        let mut state = self.inner.lock().expect("mock dingtalk lock");
        if state.fail_notifications {
            return Err(DingtalkClientError::NotifyFailed);
        }

        state.notifications.push(DingTalkWorkNotification {
            receiver_dingtalk_user_id: receiver_dingtalk_user_id.to_string(),
            message: message.to_string(),
        });
        Ok(())
    }
}
