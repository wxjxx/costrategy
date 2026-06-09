use crate::auth::UserRole;
use crate::dingtalk::{DingTalkClient, DingtalkClientError};
use crate::error::{ApiErrorCode, AppError};
use crate::users::{
    NewDepartment, NewUser, SyncLogRecord, SyncUserOutcome, UserRepository, UserStatus,
};
use std::collections::HashSet;

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct DingtalkSyncResult {
    pub created_users: usize,
    pub updated_users: usize,
    pub disabled_users: usize,
}

#[derive(Debug, Clone)]
pub struct DingtalkSyncService<C, R> {
    dingtalk: C,
    users: R,
}

impl<C, R> DingtalkSyncService<C, R>
where
    C: DingTalkClient,
    R: UserRepository,
{
    pub fn new(dingtalk: C, users: R) -> Self {
        Self { dingtalk, users }
    }

    pub async fn sync_contacts(&self) -> Result<DingtalkSyncResult, AppError> {
        log::info!("dingtalk sync: started");
        match self.try_sync_contacts().await {
            Ok(result) => {
                log::info!(
                    "dingtalk sync: completed, created_users={}, updated_users={}, disabled_users={}",
                    result.created_users,
                    result.updated_users,
                    result.disabled_users
                );
                Ok(result)
            }
            Err(error) => {
                log::error!(
                    "dingtalk sync: failed before completion, error_code={:?}, status={}",
                    error.code(),
                    error.status_code()
                );
                self.record_failed_sync_log(&error).await?;
                Err(error)
            }
        }
    }

    async fn try_sync_contacts(&self) -> Result<DingtalkSyncResult, AppError> {
        log::info!("dingtalk sync: listing departments from dingtalk");
        let departments = self.dingtalk.list_departments().await.map_err(|error| {
            log::error!("dingtalk sync: failed to list departments: {error}");
            dingtalk_sync_error_to_app(error)
        })?;
        log::info!(
            "dingtalk sync: listed departments, department_count={}",
            departments.len()
        );

        let mut seen_users = HashSet::new();
        let mut created_users = 0;
        let mut updated_users = 0;

        for department in departments {
            let department_id = department.dingtalk_dept_id;
            log::info!(
                "dingtalk sync: upserting department, dingtalk_dept_id={}",
                department_id
            );
            self.users
                .upsert_department(NewDepartment {
                    dingtalk_dept_id: department_id,
                    parent_dingtalk_dept_id: department.parent_dingtalk_dept_id,
                    name: department.name,
                    order_no: department.order_no,
                })
                .await
                .map_err(|error| {
                    log::error!(
                        "dingtalk sync: failed to upsert department, dingtalk_dept_id={}: {error}",
                        department_id
                    );
                    AppError::internal(ApiErrorCode::DatabaseError)
                })?;

            log::info!(
                "dingtalk sync: listing users by department, dingtalk_dept_id={}",
                department_id
            );
            let dingtalk_users = self
                .dingtalk
                .list_users_by_department(department_id)
                .await
                .map_err(|error| {
                    log::error!(
                        "dingtalk sync: failed to list users by department, dingtalk_dept_id={}: {error}",
                        department_id
                    );
                    dingtalk_sync_error_to_app(error)
                })?;
            log::info!(
                "dingtalk sync: listed users by department, dingtalk_dept_id={}, user_count={}",
                department_id,
                dingtalk_users.len()
            );

            let mut department_user_ids = Vec::with_capacity(dingtalk_users.len());
            let mut department_new_users = 0;
            let mut department_updated_users = 0;
            for dingtalk_user in dingtalk_users {
                department_user_ids.push(dingtalk_user.dingtalk_user_id.clone());

                let dingtalk_user_id = dingtalk_user.dingtalk_user_id.clone();
                if seen_users.insert(dingtalk_user_id.clone()) {
                    match self
                        .users
                        .upsert_synced_user(NewUser {
                            dingtalk_user_id: dingtalk_user.dingtalk_user_id,
                            union_id: dingtalk_user.union_id,
                            name: dingtalk_user.name,
                            avatar_url: dingtalk_user.avatar_url,
                            mobile: dingtalk_user.mobile,
                            role: UserRole::Employee,
                            status: UserStatus::Active,
                        })
                        .await
                        .map_err(|error| {
                            log::error!(
                                "dingtalk sync: failed to upsert user, dingtalk_dept_id={}, dingtalk_user_id={}: {error}",
                                department_id,
                                dingtalk_user_id
                            );
                            AppError::internal(ApiErrorCode::DatabaseError)
                        })?
                    {
                        SyncUserOutcome::Created => {
                            created_users += 1;
                            department_new_users += 1;
                        }
                        SyncUserOutcome::Updated => {
                            updated_users += 1;
                            department_updated_users += 1;
                        }
                        SyncUserOutcome::Unchanged => {}
                    }
                }
            }

            log::info!(
                "dingtalk sync: replacing department users, dingtalk_dept_id={}, relation_count={}",
                department_id,
                department_user_ids.len()
            );
            self.users
                .replace_department_users(department_id, &department_user_ids)
                .await
                .map_err(|error| {
                    log::error!(
                        "dingtalk sync: failed to replace department users, dingtalk_dept_id={}: {error}",
                        department_id
                    );
                    AppError::internal(ApiErrorCode::DatabaseError)
                })?;
            log::info!(
                "dingtalk sync: department completed, dingtalk_dept_id={}, created_users={}, updated_users={}, seen_user_total={}",
                department_id,
                department_new_users,
                department_updated_users,
                seen_users.len()
            );
        }

        log::info!(
            "dingtalk sync: disabling users missing from sync, active_user_count={}",
            seen_users.len()
        );
        let disabled_users = self
            .users
            .disable_users_missing_from_sync(&seen_users)
            .await
            .map_err(|error| {
                log::error!("dingtalk sync: failed to disable missing users: {error}");
                AppError::internal(ApiErrorCode::DatabaseError)
            })?;
        log::info!(
            "dingtalk sync: disabled missing users, disabled_users={}",
            disabled_users
        );

        log::info!("dingtalk sync: recording success sync log");
        self.users
            .record_sync_log(SyncLogRecord {
                status: "success".to_string(),
                created_users,
                updated_users,
                disabled_users,
                failure_reason: None,
            })
            .await
            .map_err(|error| {
                log::error!("dingtalk sync: failed to record success sync log: {error}");
                AppError::internal(ApiErrorCode::DatabaseError)
            })?;

        Ok(DingtalkSyncResult {
            created_users,
            updated_users,
            disabled_users,
        })
    }

    async fn record_failed_sync_log(&self, error: &AppError) -> Result<(), AppError> {
        let failure_reason = match error.code() {
            ApiErrorCode::DingtalkConfigMissing => "dingtalk config missing",
            ApiErrorCode::DingtalkSyncFailed => "dingtalk sync failed",
            ApiErrorCode::DatabaseError => "database operation failed",
            _ => "sync failed",
        };

        log::info!(
            "dingtalk sync: recording failed sync log, failure_reason={}",
            failure_reason
        );
        self.users
            .record_sync_log(SyncLogRecord {
                status: "failed".to_string(),
                created_users: 0,
                updated_users: 0,
                disabled_users: 0,
                failure_reason: Some(failure_reason.to_string()),
            })
            .await
            .map_err(|error| {
                log::error!("dingtalk sync: failed to record failed sync log: {error}");
                AppError::internal(ApiErrorCode::DatabaseError)
            })
    }
}

fn dingtalk_sync_error_to_app(error: DingtalkClientError) -> AppError {
    match error {
        DingtalkClientError::ConfigMissing => {
            AppError::internal(ApiErrorCode::DingtalkConfigMissing)
        }
        _ => AppError::internal(ApiErrorCode::DingtalkSyncFailed),
    }
}
