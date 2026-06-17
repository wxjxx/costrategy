use crate::auth::UserRole;
use crate::dingtalk::{DingTalkClient, DingtalkClientError};
use crate::error::{ApiErrorCode, AppError};
use crate::users::{UserRepository, UserStatus};
use actix_web::http::StatusCode;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct CurrentUser {
    pub id: Uuid,
    pub name: String,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub departments: Vec<String>,
    pub permissions: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct DingtalkAuthService<C, R> {
    dingtalk: C,
    users: R,
}

impl<C, R> DingtalkAuthService<C, R>
where
    C: DingTalkClient,
    R: UserRepository,
{
    pub fn new(dingtalk: C, users: R) -> Self {
        Self { dingtalk, users }
    }

    pub async fn login_with_code(&self, code: &str) -> Result<CurrentUser, AppError> {
        log::info!("dingtalk auth: exchanging login code with dingtalk");
        let identity =
            self.dingtalk
                .exchange_login_code(code)
                .await
                .map_err(|error| match error {
                    DingtalkClientError::ConfigMissing => {
                        log::error!("dingtalk auth: dingtalk runtime config is missing");
                        AppError::internal(ApiErrorCode::DingtalkConfigMissing)
                    }
                    error => {
                        log::warn!("dingtalk auth: dingtalk login code exchange failed: {error}");
                        AppError::new(
                            StatusCode::UNAUTHORIZED,
                            ApiErrorCode::AuthDingtalkLoginFailed,
                        )
                    }
                })?;
        log::info!(
            "dingtalk auth: login identity received for dingtalk_user_id={}",
            identity.dingtalk_user_id
        );

        let Some(user) = self
            .users
            .find_by_dingtalk_user_id(&identity.dingtalk_user_id)
            .await
            .map_err(|error| {
                log::error!("dingtalk auth: failed to find synced user: {error}");
                AppError::internal(ApiErrorCode::DatabaseError)
            })?
        else {
            log::warn!(
                "dingtalk auth: dingtalk user is not synced, dingtalk_user_id={}",
                identity.dingtalk_user_id
            );
            return Err(AppError::forbidden(ApiErrorCode::AuthUserNotSynced));
        };

        if user.status == UserStatus::Disabled {
            log::warn!(
                "dingtalk auth: synced user is disabled, user_id={}, dingtalk_user_id={}",
                user.id,
                user.dingtalk_user_id
            );
            return Err(AppError::forbidden(ApiErrorCode::AuthUserDisabled));
        }
        let departments = self
            .users
            .list_user_departments(user.id)
            .await
            .map_err(|error| {
                log::error!(
                    "dingtalk auth: failed to load user departments, user_id={}: {error}",
                    user.id
                );
                AppError::internal(ApiErrorCode::DatabaseError)
            })?;

        log::info!(
            "dingtalk auth: user authenticated, user_id={}, role={:?}, department_count={}",
            user.id,
            user.role,
            departments.len()
        );
        Ok(CurrentUser {
            id: user.id,
            name: user.name,
            avatar_url: user.avatar_url,
            role: user.role,
            departments,
            permissions: user.role.permission_codes(),
        })
    }
}
