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
        let identity =
            self.dingtalk
                .exchange_login_code(code)
                .await
                .map_err(|error| match error {
                    DingtalkClientError::ConfigMissing => {
                        AppError::internal(ApiErrorCode::DingtalkConfigMissing)
                    }
                    _ => AppError::new(
                        StatusCode::UNAUTHORIZED,
                        ApiErrorCode::AuthDingtalkLoginFailed,
                    ),
                })?;

        let Some(user) = self
            .users
            .find_by_dingtalk_user_id(&identity.dingtalk_user_id)
            .await
            .map_err(|_| AppError::internal(ApiErrorCode::DatabaseError))?
        else {
            return Err(AppError::forbidden(ApiErrorCode::AuthUserNotSynced));
        };

        if user.status == UserStatus::Disabled {
            return Err(AppError::forbidden(ApiErrorCode::AuthUserDisabled));
        }
        let departments = self
            .users
            .list_user_departments(user.id)
            .await
            .map_err(|_| AppError::internal(ApiErrorCode::DatabaseError))?;

        Ok(CurrentUser {
            id: user.id,
            name: user.name,
            role: user.role,
            departments,
            permissions: user.role.permission_codes(),
        })
    }
}
