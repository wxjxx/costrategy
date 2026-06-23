use crate::config::DingtalkConfig;
use async_trait::async_trait;
use reqwest::Url;
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
    #[error("config missing")]
    ConfigMissing,
    #[error("login failed")]
    LoginFailed,
    #[error("sync failed")]
    SyncFailed,
    #[error("notify failed")]
    NotifyFailed,
    #[error("{operation} failed: errcode={errcode}, errmsg={errmsg}")]
    ApiFailed {
        operation: &'static str,
        errcode: i64,
        errmsg: String,
    },
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

#[derive(Debug, Clone)]
pub enum ConfiguredDingTalkClient {
    Http(DingtalkHttpClient),
    Missing,
}

impl ConfiguredDingTalkClient {
    pub fn from_config(config: Option<DingtalkConfig>) -> Self {
        config
            .map(DingtalkHttpClient::new)
            .map(Self::Http)
            .unwrap_or(Self::Missing)
    }

    pub fn missing() -> Self {
        Self::Missing
    }
}

#[async_trait]
impl DingTalkClient for ConfiguredDingTalkClient {
    async fn exchange_login_code(
        &self,
        code: &str,
    ) -> Result<DingTalkLoginIdentity, DingtalkClientError> {
        match self {
            Self::Http(client) => client.exchange_login_code(code).await,
            Self::Missing => Err(DingtalkClientError::ConfigMissing),
        }
    }

    async fn list_departments(&self) -> Result<Vec<DingTalkDepartment>, DingtalkClientError> {
        match self {
            Self::Http(client) => client.list_departments().await,
            Self::Missing => Err(DingtalkClientError::ConfigMissing),
        }
    }

    async fn list_users_by_department(
        &self,
        dingtalk_dept_id: i64,
    ) -> Result<Vec<DingTalkUser>, DingtalkClientError> {
        match self {
            Self::Http(client) => client.list_users_by_department(dingtalk_dept_id).await,
            Self::Missing => Err(DingtalkClientError::ConfigMissing),
        }
    }

    async fn send_work_notification(
        &self,
        receiver_dingtalk_user_id: &str,
        message: &str,
    ) -> Result<(), DingtalkClientError> {
        match self {
            Self::Http(client) => {
                client
                    .send_work_notification(receiver_dingtalk_user_id, message)
                    .await
            }
            Self::Missing => Err(DingtalkClientError::ConfigMissing),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DingtalkHttpClient {
    config: DingtalkConfig,
    http: reqwest::Client,
    token_cache: Arc<Mutex<Option<CachedAccessToken>>>,
}

#[derive(Debug, Clone)]
struct CachedAccessToken {
    token: String,
    refresh_at: Instant,
}

impl DingtalkHttpClient {
    pub fn new(config: DingtalkConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            token_cache: Arc::new(Mutex::new(None)),
        }
    }

    async fn access_token(&self) -> Result<String, DingtalkClientError> {
        if let Some(token) = self.cached_access_token() {
            return Ok(token);
        }

        let mut url = self.oapi_url("/gettoken")?;
        url.query_pairs_mut()
            .append_pair("appkey", &self.config.client_id)
            .append_pair("appsecret", &self.config.client_secret);
        let response: AccessTokenResponse = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|error| {
                log::error!("dingtalk http: get access token request failed: {error}");
                DingtalkClientError::SyncFailed
            })?
            .json()
            .await
            .map_err(|error| {
                log::error!("dingtalk http: get access token response parse failed: {error}");
                DingtalkClientError::SyncFailed
            })?;
        let errcode = response.errcode.unwrap_or(0);
        if errcode != 0 {
            log::error!(
                "dingtalk http: get access token api failed, errcode={}, errmsg={}",
                errcode,
                response.errmsg.as_deref().unwrap_or("")
            );
            return Err(DingtalkClientError::SyncFailed);
        }
        let token = response.access_token.ok_or_else(|| {
            log::error!("dingtalk http: get access token response missing access_token");
            DingtalkClientError::SyncFailed
        })?;
        let expires_in = response.expires_in.unwrap_or(7200);
        let refresh_after = expires_in.saturating_sub(300).max(60);

        self.token_cache
            .lock()
            .expect("dingtalk token cache lock")
            .replace(CachedAccessToken {
                token: token.clone(),
                refresh_at: Instant::now() + Duration::from_secs(refresh_after),
            });

        Ok(token)
    }

    fn cached_access_token(&self) -> Option<String> {
        self.token_cache
            .lock()
            .expect("dingtalk token cache lock")
            .as_ref()
            .filter(|cached| cached.refresh_at > Instant::now())
            .map(|cached| cached.token.clone())
    }

    fn oapi_url(&self, path: &str) -> Result<Url, DingtalkClientError> {
        let base = self.config.oapi_base_url.trim_end_matches('/');
        Url::parse(&format!("{base}{path}")).map_err(|_| DingtalkClientError::ConfigMissing)
    }

    async fn post_oapi<T>(
        &self,
        path: &str,
        body: serde_json::Value,
        fallback_error: DingtalkClientError,
    ) -> Result<T, DingtalkClientError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let access_token = self.access_token().await.map_err(|error| match error {
            DingtalkClientError::ConfigMissing => DingtalkClientError::ConfigMissing,
            _ => fallback_error.clone(),
        })?;
        let mut url = self.oapi_url(path)?;
        url.query_pairs_mut()
            .append_pair("access_token", &access_token);

        let raw_response = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                log::error!("dingtalk http: request failed, path={path}: {error}");
                fallback_error.clone()
            })?;
        let status = raw_response.status();
        if !status.is_success() {
            log::error!("dingtalk http: non-success http status, path={path}, status={status}");
        }
        let response: DingtalkApiResponse<T> = raw_response.json().await.map_err(|error| {
            log::error!(
                "dingtalk http: response parse failed, path={path}, status={status}: {error}"
            );
            fallback_error.clone()
        })?;
        if response.errcode != 0 {
            let errmsg = response.errmsg.unwrap_or_default();
            log::error!(
                "dingtalk http: api failed, path={}, errcode={}, errmsg={}",
                path,
                response.errcode,
                errmsg
            );
            return Err(DingtalkClientError::ApiFailed {
                operation: operation_label(fallback_error),
                errcode: response.errcode,
                errmsg,
            });
        }

        response.result.ok_or_else(|| {
            log::error!("dingtalk http: response missing result, path={path}");
            fallback_error
        })
    }
}

fn operation_label(error: DingtalkClientError) -> &'static str {
    match error {
        DingtalkClientError::ConfigMissing => "config",
        DingtalkClientError::LoginFailed => "login",
        DingtalkClientError::SyncFailed => "sync",
        DingtalkClientError::NotifyFailed => "notify",
        DingtalkClientError::ApiFailed { operation, .. } => operation,
    }
}

#[async_trait]
impl DingTalkClient for DingtalkHttpClient {
    async fn exchange_login_code(
        &self,
        code: &str,
    ) -> Result<DingTalkLoginIdentity, DingtalkClientError> {
        let result: LoginCodeResult = self
            .post_oapi(
                "/topapi/v2/user/getuserinfo",
                json!({ "code": code }),
                DingtalkClientError::LoginFailed,
            )
            .await?;

        Ok(DingTalkLoginIdentity {
            dingtalk_user_id: result.userid,
            union_id: result.unionid,
        })
    }

    async fn list_departments(&self) -> Result<Vec<DingTalkDepartment>, DingtalkClientError> {
        let mut departments = Vec::new();
        let mut queue = VecDeque::from([1_i64]);

        while let Some(parent_id) = queue.pop_front() {
            let child_departments: Vec<DepartmentResult> = self
                .post_oapi(
                    "/topapi/v2/department/listsub",
                    json!({ "dept_id": parent_id, "language": "zh_CN" }),
                    DingtalkClientError::SyncFailed,
                )
                .await?;

            for department in child_departments {
                queue.push_back(department.dept_id);
                departments.push(DingTalkDepartment {
                    dingtalk_dept_id: department.dept_id,
                    parent_dingtalk_dept_id: department.parent_id,
                    name: department.name,
                    order_no: department.order,
                });
            }
        }

        Ok(departments)
    }

    async fn list_users_by_department(
        &self,
        dingtalk_dept_id: i64,
    ) -> Result<Vec<DingTalkUser>, DingtalkClientError> {
        let mut users = Vec::new();
        let mut cursor = 0_i64;

        loop {
            let result: UserListResult = self
                .post_oapi(
                    "/topapi/v2/user/list",
                    json!({
                        "dept_id": dingtalk_dept_id,
                        "cursor": cursor,
                        "size": 100,
                        "language": "zh_CN"
                    }),
                    DingtalkClientError::SyncFailed,
                )
                .await?;

            users.extend(result.list.into_iter().map(|user| DingTalkUser {
                dingtalk_user_id: user.userid,
                union_id: user.unionid,
                name: user.name,
                avatar_url: user.avatar,
                mobile: user.mobile,
            }));

            if !result.has_more.unwrap_or(false) {
                break;
            }
            cursor = result.next_cursor.unwrap_or(cursor + 100);
        }

        Ok(users)
    }

    async fn send_work_notification(
        &self,
        receiver_dingtalk_user_id: &str,
        message: &str,
    ) -> Result<(), DingtalkClientError> {
        let _: serde_json::Value = self
            .post_oapi(
                "/topapi/message/corpconversation/asyncsend_v2",
                json!({
                    "agent_id": self.config.agent_id,
                    "userid_list": receiver_dingtalk_user_id,
                    "msg": {
                        "msgtype": "text",
                        "text": { "content": message }
                    }
                }),
                DingtalkClientError::NotifyFailed,
            )
            .await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    errcode: Option<i64>,
    errmsg: Option<String>,
    access_token: Option<String>,
    expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct DingtalkApiResponse<T> {
    errcode: i64,
    errmsg: Option<String>,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct LoginCodeResult {
    userid: String,
    unionid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DepartmentResult {
    #[serde(rename = "dept_id")]
    dept_id: i64,
    #[serde(rename = "parent_id")]
    parent_id: Option<i64>,
    name: String,
    order: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct UserListResult {
    #[serde(default)]
    list: Vec<UserResult>,
    has_more: Option<bool>,
    next_cursor: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct UserResult {
    userid: String,
    unionid: Option<String>,
    name: String,
    avatar: Option<String>,
    mobile: Option<String>,
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
    fail_sync: bool,
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

    pub fn with_sync_failure(self) -> Self {
        self.inner.lock().expect("mock dingtalk lock").fail_sync = true;
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
        let state = self.inner.lock().expect("mock dingtalk lock");
        if state.fail_sync {
            return Err(DingtalkClientError::SyncFailed);
        }
        Ok(state.departments.clone())
    }

    async fn list_users_by_department(
        &self,
        dingtalk_dept_id: i64,
    ) -> Result<Vec<DingTalkUser>, DingtalkClientError> {
        let state = self.inner.lock().expect("mock dingtalk lock");
        if state.fail_sync {
            return Err(DingtalkClientError::SyncFailed);
        }
        Ok(state
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
