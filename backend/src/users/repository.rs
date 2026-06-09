use crate::auth::UserRole;
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatus {
    Active,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    pub dingtalk_user_id: String,
    pub union_id: Option<String>,
    pub name: String,
    pub avatar_url: Option<String>,
    pub mobile: Option<String>,
    pub role: UserRole,
    pub status: UserStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUser {
    pub dingtalk_user_id: String,
    pub union_id: Option<String>,
    pub name: String,
    pub avatar_url: Option<String>,
    pub mobile: Option<String>,
    pub role: UserRole,
    pub status: UserStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDepartment {
    pub dingtalk_dept_id: i64,
    pub parent_dingtalk_dept_id: Option<i64>,
    pub name: String,
    pub order_no: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepartmentRecord {
    pub id: Uuid,
    pub dingtalk_dept_id: i64,
    pub parent_dingtalk_dept_id: Option<i64>,
    pub name: String,
    pub order_no: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncUserOutcome {
    Created,
    Updated,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncLogRecord {
    pub status: String,
    pub created_users: usize,
    pub updated_users: usize,
    pub disabled_users: usize,
    pub failure_reason: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum UserRepositoryError {
    #[error("database operation failed")]
    Database,
}

#[async_trait]
pub trait UserRepository: Clone + Send + Sync + 'static {
    async fn get_user(&self, id: Uuid) -> Result<Option<User>, UserRepositoryError>;

    async fn find_by_dingtalk_user_id(
        &self,
        dingtalk_user_id: &str,
    ) -> Result<Option<User>, UserRepositoryError>;

    async fn upsert_department(
        &self,
        department: NewDepartment,
    ) -> Result<DepartmentRecord, UserRepositoryError>;

    async fn replace_department_users(
        &self,
        dingtalk_dept_id: i64,
        dingtalk_user_ids: &[String],
    ) -> Result<(), UserRepositoryError>;

    async fn upsert_synced_user(
        &self,
        user: NewUser,
    ) -> Result<SyncUserOutcome, UserRepositoryError>;

    async fn disable_users_missing_from_sync(
        &self,
        active_dingtalk_user_ids: &HashSet<String>,
    ) -> Result<usize, UserRepositoryError>;

    async fn record_sync_log(&self, log: SyncLogRecord) -> Result<(), UserRepositoryError>;
}

#[derive(Debug, Clone, Default)]
pub struct MemoryUserRepository {
    inner: Arc<Mutex<MemoryState>>,
}

#[derive(Debug, Default)]
struct MemoryState {
    users_by_dingtalk_id: HashMap<String, User>,
    departments_by_dingtalk_id: HashMap<i64, DepartmentRecord>,
    department_users: HashSet<(i64, String)>,
    sync_logs: Vec<SyncLogRecord>,
}

impl MemoryUserRepository {
    pub async fn insert_user(&self, user: NewUser) -> User {
        let stored = User {
            id: Uuid::new_v4(),
            dingtalk_user_id: user.dingtalk_user_id,
            union_id: user.union_id,
            name: user.name,
            avatar_url: user.avatar_url,
            mobile: user.mobile,
            role: user.role,
            status: user.status,
        };

        self.inner
            .lock()
            .expect("memory repository lock")
            .users_by_dingtalk_id
            .insert(stored.dingtalk_user_id.clone(), stored.clone());

        stored
    }

    pub async fn find_by_dingtalk_user_id(&self, dingtalk_user_id: &str) -> Option<User> {
        self.inner
            .lock()
            .expect("memory repository lock")
            .users_by_dingtalk_id
            .get(dingtalk_user_id)
            .cloned()
    }

    pub async fn department_count(&self) -> usize {
        self.inner
            .lock()
            .expect("memory repository lock")
            .departments_by_dingtalk_id
            .len()
    }

    pub async fn department_user_count(&self) -> usize {
        self.inner
            .lock()
            .expect("memory repository lock")
            .department_users
            .len()
    }

    pub async fn sync_logs(&self) -> Vec<SyncLogRecord> {
        self.inner
            .lock()
            .expect("memory repository lock")
            .sync_logs
            .clone()
    }
}

#[derive(Debug, Clone)]
pub struct SqlxUserRepository {
    pool: PgPool,
}

impl SqlxUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for MemoryUserRepository {
    async fn get_user(&self, id: Uuid) -> Result<Option<User>, UserRepositoryError> {
        Ok(self
            .inner
            .lock()
            .expect("memory repository lock")
            .users_by_dingtalk_id
            .values()
            .find(|user| user.id == id)
            .cloned())
    }

    async fn find_by_dingtalk_user_id(
        &self,
        dingtalk_user_id: &str,
    ) -> Result<Option<User>, UserRepositoryError> {
        Ok(self.find_by_dingtalk_user_id(dingtalk_user_id).await)
    }

    async fn upsert_department(
        &self,
        department: NewDepartment,
    ) -> Result<DepartmentRecord, UserRepositoryError> {
        let mut state = self.inner.lock().expect("memory repository lock");
        let stored = state
            .departments_by_dingtalk_id
            .entry(department.dingtalk_dept_id)
            .or_insert_with(|| DepartmentRecord {
                id: Uuid::new_v4(),
                dingtalk_dept_id: department.dingtalk_dept_id,
                parent_dingtalk_dept_id: department.parent_dingtalk_dept_id,
                name: department.name.clone(),
                order_no: department.order_no,
            });

        stored.parent_dingtalk_dept_id = department.parent_dingtalk_dept_id;
        stored.name = department.name;
        stored.order_no = department.order_no;

        Ok(stored.clone())
    }

    async fn replace_department_users(
        &self,
        dingtalk_dept_id: i64,
        dingtalk_user_ids: &[String],
    ) -> Result<(), UserRepositoryError> {
        let mut state = self.inner.lock().expect("memory repository lock");
        state
            .department_users
            .retain(|(dept_id, _)| *dept_id != dingtalk_dept_id);
        state.department_users.extend(
            dingtalk_user_ids
                .iter()
                .cloned()
                .map(|user_id| (dingtalk_dept_id, user_id)),
        );
        Ok(())
    }

    async fn upsert_synced_user(
        &self,
        user: NewUser,
    ) -> Result<SyncUserOutcome, UserRepositoryError> {
        let mut state = self.inner.lock().expect("memory repository lock");
        let Some(existing) = state.users_by_dingtalk_id.get_mut(&user.dingtalk_user_id) else {
            state.users_by_dingtalk_id.insert(
                user.dingtalk_user_id.clone(),
                User {
                    id: Uuid::new_v4(),
                    dingtalk_user_id: user.dingtalk_user_id,
                    union_id: user.union_id,
                    name: user.name,
                    avatar_url: user.avatar_url,
                    mobile: user.mobile,
                    role: user.role,
                    status: user.status,
                },
            );
            return Ok(SyncUserOutcome::Created);
        };

        let changed = existing.union_id != user.union_id
            || existing.name != user.name
            || existing.avatar_url != user.avatar_url
            || existing.mobile != user.mobile
            || existing.status != UserStatus::Active;

        existing.union_id = user.union_id;
        existing.name = user.name;
        existing.avatar_url = user.avatar_url;
        existing.mobile = user.mobile;
        existing.status = UserStatus::Active;

        if changed {
            Ok(SyncUserOutcome::Updated)
        } else {
            Ok(SyncUserOutcome::Unchanged)
        }
    }

    async fn disable_users_missing_from_sync(
        &self,
        active_dingtalk_user_ids: &HashSet<String>,
    ) -> Result<usize, UserRepositoryError> {
        let mut state = self.inner.lock().expect("memory repository lock");
        let mut disabled = 0;
        for user in state.users_by_dingtalk_id.values_mut() {
            if !active_dingtalk_user_ids.contains(&user.dingtalk_user_id)
                && user.status != UserStatus::Disabled
            {
                user.status = UserStatus::Disabled;
                disabled += 1;
            }
        }
        Ok(disabled)
    }

    async fn record_sync_log(&self, log: SyncLogRecord) -> Result<(), UserRepositoryError> {
        self.inner
            .lock()
            .expect("memory repository lock")
            .sync_logs
            .push(log);
        Ok(())
    }
}

#[async_trait]
impl UserRepository for SqlxUserRepository {
    async fn get_user(&self, id: Uuid) -> Result<Option<User>, UserRepositoryError> {
        let row = sqlx::query(
            "select id, dingtalk_user_id, union_id, name, avatar_url, mobile, role, status
             from users
             where id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| UserRepositoryError::Database)?;

        row.map(row_to_user).transpose()
    }

    async fn find_by_dingtalk_user_id(
        &self,
        dingtalk_user_id: &str,
    ) -> Result<Option<User>, UserRepositoryError> {
        let row = sqlx::query(
            "select id, dingtalk_user_id, union_id, name, avatar_url, mobile, role, status
             from users
             where dingtalk_user_id = $1",
        )
        .bind(dingtalk_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| UserRepositoryError::Database)?;

        row.map(row_to_user).transpose()
    }

    async fn upsert_department(
        &self,
        department: NewDepartment,
    ) -> Result<DepartmentRecord, UserRepositoryError> {
        let row = sqlx::query(
            "insert into departments (
                id, dingtalk_dept_id, parent_dingtalk_dept_id, name, order_no, updated_at
             )
             values ($1, $2, $3, $4, $5, now())
             on conflict (dingtalk_dept_id) do update set
                parent_dingtalk_dept_id = excluded.parent_dingtalk_dept_id,
                name = excluded.name,
                order_no = excluded.order_no,
                updated_at = now()
             returning id, dingtalk_dept_id, parent_dingtalk_dept_id, name, order_no",
        )
        .bind(Uuid::new_v4())
        .bind(department.dingtalk_dept_id)
        .bind(department.parent_dingtalk_dept_id)
        .bind(department.name)
        .bind(department.order_no)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| UserRepositoryError::Database)?;

        Ok(DepartmentRecord {
            id: row
                .try_get("id")
                .map_err(|_| UserRepositoryError::Database)?,
            dingtalk_dept_id: row
                .try_get("dingtalk_dept_id")
                .map_err(|_| UserRepositoryError::Database)?,
            parent_dingtalk_dept_id: row
                .try_get("parent_dingtalk_dept_id")
                .map_err(|_| UserRepositoryError::Database)?,
            name: row
                .try_get("name")
                .map_err(|_| UserRepositoryError::Database)?,
            order_no: row
                .try_get("order_no")
                .map_err(|_| UserRepositoryError::Database)?,
        })
    }

    async fn replace_department_users(
        &self,
        dingtalk_dept_id: i64,
        dingtalk_user_ids: &[String],
    ) -> Result<(), UserRepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| UserRepositoryError::Database)?;
        let department_id: Uuid =
            sqlx::query_scalar("select id from departments where dingtalk_dept_id = $1")
                .bind(dingtalk_dept_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| UserRepositoryError::Database)?;

        sqlx::query("delete from department_users where department_id = $1")
            .bind(department_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| UserRepositoryError::Database)?;

        for dingtalk_user_id in dingtalk_user_ids {
            let user_id: Option<Uuid> =
                sqlx::query_scalar("select id from users where dingtalk_user_id = $1")
                    .bind(dingtalk_user_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|_| UserRepositoryError::Database)?;
            if let Some(user_id) = user_id {
                sqlx::query(
                    "insert into department_users (department_id, user_id)
                     values ($1, $2)
                     on conflict do nothing",
                )
                .bind(department_id)
                .bind(user_id)
                .execute(&mut *tx)
                .await
                .map_err(|_| UserRepositoryError::Database)?;
            }
        }

        tx.commit().await.map_err(|_| UserRepositoryError::Database)
    }

    async fn upsert_synced_user(
        &self,
        user: NewUser,
    ) -> Result<SyncUserOutcome, UserRepositoryError> {
        let existing = self
            .find_by_dingtalk_user_id(&user.dingtalk_user_id)
            .await?;
        let Some(existing) = existing else {
            sqlx::query(
                "insert into users (
                    id, dingtalk_user_id, union_id, name, avatar_url, mobile, role, status,
                    last_synced_at, updated_at
                 )
                 values ($1, $2, $3, $4, $5, $6, $7, $8, now(), now())",
            )
            .bind(Uuid::new_v4())
            .bind(user.dingtalk_user_id)
            .bind(user.union_id)
            .bind(user.name)
            .bind(user.avatar_url)
            .bind(user.mobile)
            .bind(user.role.as_str())
            .bind(user.status.as_str())
            .execute(&self.pool)
            .await
            .map_err(|_| UserRepositoryError::Database)?;
            return Ok(SyncUserOutcome::Created);
        };

        let changed = existing.union_id != user.union_id
            || existing.name != user.name
            || existing.avatar_url != user.avatar_url
            || existing.mobile != user.mobile
            || existing.status != UserStatus::Active;

        sqlx::query(
            "update users set
                union_id = $2,
                name = $3,
                avatar_url = $4,
                mobile = $5,
                status = 'active',
                last_synced_at = now(),
                updated_at = now()
             where dingtalk_user_id = $1",
        )
        .bind(user.dingtalk_user_id)
        .bind(user.union_id)
        .bind(user.name)
        .bind(user.avatar_url)
        .bind(user.mobile)
        .execute(&self.pool)
        .await
        .map_err(|_| UserRepositoryError::Database)?;

        if changed {
            Ok(SyncUserOutcome::Updated)
        } else {
            Ok(SyncUserOutcome::Unchanged)
        }
    }

    async fn disable_users_missing_from_sync(
        &self,
        active_dingtalk_user_ids: &HashSet<String>,
    ) -> Result<usize, UserRepositoryError> {
        let active_ids = active_dingtalk_user_ids.iter().cloned().collect::<Vec<_>>();
        let result = sqlx::query(
            "update users set status = 'disabled', updated_at = now()
             where status <> 'disabled'
               and not (dingtalk_user_id = any($1))",
        )
        .bind(active_ids)
        .execute(&self.pool)
        .await
        .map_err(|_| UserRepositoryError::Database)?;

        usize::try_from(result.rows_affected()).map_err(|_| UserRepositoryError::Database)
    }

    async fn record_sync_log(&self, log: SyncLogRecord) -> Result<(), UserRepositoryError> {
        sqlx::query(
            "insert into dingtalk_sync_logs (
                id, started_at, finished_at, status, created_users, updated_users,
                disabled_users, failure_reason
             )
             values ($1, now(), now(), $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(log.status)
        .bind(i32::try_from(log.created_users).map_err(|_| UserRepositoryError::Database)?)
        .bind(i32::try_from(log.updated_users).map_err(|_| UserRepositoryError::Database)?)
        .bind(i32::try_from(log.disabled_users).map_err(|_| UserRepositoryError::Database)?)
        .bind(log.failure_reason)
        .execute(&self.pool)
        .await
        .map_err(|_| UserRepositoryError::Database)?;

        Ok(())
    }
}

impl UserStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
        }
    }
}

impl FromStr for UserStatus {
    type Err = UserRepositoryError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "active" => Ok(Self::Active),
            "disabled" => Ok(Self::Disabled),
            _ => Err(UserRepositoryError::Database),
        }
    }
}

fn row_to_user(row: sqlx::postgres::PgRow) -> Result<User, UserRepositoryError> {
    let role: String = row
        .try_get("role")
        .map_err(|_| UserRepositoryError::Database)?;
    let status: String = row
        .try_get("status")
        .map_err(|_| UserRepositoryError::Database)?;

    Ok(User {
        id: row
            .try_get("id")
            .map_err(|_| UserRepositoryError::Database)?,
        dingtalk_user_id: row
            .try_get("dingtalk_user_id")
            .map_err(|_| UserRepositoryError::Database)?,
        union_id: row
            .try_get("union_id")
            .map_err(|_| UserRepositoryError::Database)?,
        name: row
            .try_get("name")
            .map_err(|_| UserRepositoryError::Database)?,
        avatar_url: row
            .try_get("avatar_url")
            .map_err(|_| UserRepositoryError::Database)?,
        mobile: row
            .try_get("mobile")
            .map_err(|_| UserRepositoryError::Database)?,
        role: role.parse().map_err(|_| UserRepositoryError::Database)?,
        status: status.parse()?,
    })
}
