use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectStatus {
    Active,
    Completed,
    Paused,
    Archived,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct Project {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub owner_id: Option<Uuid>,
    pub description: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub status: ProjectStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateProject {
    pub code: String,
    pub name: String,
    pub owner_id: Option<Uuid>,
    pub description: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub status: ProjectStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateProject {
    pub name: String,
    pub owner_id: Option<Uuid>,
    pub description: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub status: ProjectStatus,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectRepositoryError {
    #[error("project not found")]
    NotFound,
    #[error("database operation failed")]
    Database,
}

#[async_trait]
pub trait ProjectRepository: Clone + Send + Sync + 'static {
    async fn list_projects(&self) -> Result<Vec<Project>, ProjectRepositoryError>;
    async fn create_project(
        &self,
        project: CreateProject,
    ) -> Result<Project, ProjectRepositoryError>;
    async fn update_project(
        &self,
        id: Uuid,
        project: UpdateProject,
    ) -> Result<Project, ProjectRepositoryError>;
    async fn archive_project(&self, id: Uuid) -> Result<Project, ProjectRepositoryError>;
}

#[derive(Debug, Clone, Default)]
pub struct MemoryProjectRepository {
    projects: Arc<Mutex<HashMap<Uuid, Project>>>,
}

#[async_trait]
impl ProjectRepository for MemoryProjectRepository {
    async fn list_projects(&self) -> Result<Vec<Project>, ProjectRepositoryError> {
        let mut projects = self
            .projects
            .lock()
            .expect("memory project repository lock")
            .values()
            .filter(|project| project.status != ProjectStatus::Archived)
            .cloned()
            .collect::<Vec<_>>();
        projects.sort_by(|left, right| left.code.cmp(&right.code));
        Ok(projects)
    }

    async fn create_project(
        &self,
        project: CreateProject,
    ) -> Result<Project, ProjectRepositoryError> {
        let stored = Project {
            id: Uuid::new_v4(),
            code: project.code,
            name: project.name,
            owner_id: project.owner_id,
            description: project.description,
            start_date: project.start_date,
            end_date: project.end_date,
            status: project.status,
        };
        self.projects
            .lock()
            .expect("memory project repository lock")
            .insert(stored.id, stored.clone());
        Ok(stored)
    }

    async fn update_project(
        &self,
        id: Uuid,
        project: UpdateProject,
    ) -> Result<Project, ProjectRepositoryError> {
        let mut projects = self
            .projects
            .lock()
            .expect("memory project repository lock");
        let Some(existing) = projects.get_mut(&id) else {
            return Err(ProjectRepositoryError::NotFound);
        };
        existing.name = project.name;
        existing.owner_id = project.owner_id;
        existing.description = project.description;
        existing.start_date = project.start_date;
        existing.end_date = project.end_date;
        existing.status = project.status;
        Ok(existing.clone())
    }

    async fn archive_project(&self, id: Uuid) -> Result<Project, ProjectRepositoryError> {
        let mut projects = self
            .projects
            .lock()
            .expect("memory project repository lock");
        let Some(existing) = projects.get_mut(&id) else {
            return Err(ProjectRepositoryError::NotFound);
        };
        existing.status = ProjectStatus::Archived;
        Ok(existing.clone())
    }
}

#[derive(Debug, Clone)]
pub struct SqlxProjectRepository {
    pool: PgPool,
}

impl SqlxProjectRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectRepository for SqlxProjectRepository {
    async fn list_projects(&self) -> Result<Vec<Project>, ProjectRepositoryError> {
        let rows = sqlx::query(
            "select id, code, name, owner_id, description, start_date, end_date, status
             from projects
             where archived_at is null
             order by code asc",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| ProjectRepositoryError::Database)?;

        rows.into_iter().map(row_to_project).collect()
    }

    async fn create_project(
        &self,
        project: CreateProject,
    ) -> Result<Project, ProjectRepositoryError> {
        let row = sqlx::query(
            "insert into projects (
                id, code, name, owner_id, description, start_date, end_date, status, updated_at
             )
             values ($1, $2, $3, $4, $5, $6, $7, $8, now())
             returning id, code, name, owner_id, description, start_date, end_date, status",
        )
        .bind(Uuid::new_v4())
        .bind(project.code)
        .bind(project.name)
        .bind(project.owner_id)
        .bind(project.description)
        .bind(project.start_date)
        .bind(project.end_date)
        .bind(project.status.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ProjectRepositoryError::Database)?;

        row_to_project(row)
    }

    async fn update_project(
        &self,
        id: Uuid,
        project: UpdateProject,
    ) -> Result<Project, ProjectRepositoryError> {
        let row = sqlx::query(
            "update projects set
                name = $2,
                owner_id = $3,
                description = $4,
                start_date = $5,
                end_date = $6,
                status = $7,
                updated_at = now()
             where id = $1 and archived_at is null
             returning id, code, name, owner_id, description, start_date, end_date, status",
        )
        .bind(id)
        .bind(project.name)
        .bind(project.owner_id)
        .bind(project.description)
        .bind(project.start_date)
        .bind(project.end_date)
        .bind(project.status.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| ProjectRepositoryError::Database)?;

        row.map(row_to_project)
            .transpose()?
            .ok_or(ProjectRepositoryError::NotFound)
    }

    async fn archive_project(&self, id: Uuid) -> Result<Project, ProjectRepositoryError> {
        let row = sqlx::query(
            "update projects set status = 'archived', archived_at = now(), updated_at = now()
             where id = $1 and archived_at is null
             returning id, code, name, owner_id, description, start_date, end_date, status",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| ProjectRepositoryError::Database)?;

        row.map(row_to_project)
            .transpose()?
            .ok_or(ProjectRepositoryError::NotFound)
    }
}

impl ProjectStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Paused => "paused",
            Self::Archived => "archived",
        }
    }
}

impl serde::Serialize for ProjectStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ProjectStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

impl FromStr for ProjectStatus {
    type Err = ProjectStatusParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            "paused" => Ok(Self::Paused),
            "archived" => Ok(Self::Archived),
            other => Err(ProjectStatusParseError(other.to_string())),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown project status: {0}")]
pub struct ProjectStatusParseError(String);

fn row_to_project(row: sqlx::postgres::PgRow) -> Result<Project, ProjectRepositoryError> {
    let status: String = row
        .try_get("status")
        .map_err(|_| ProjectRepositoryError::Database)?;
    Ok(Project {
        id: row
            .try_get("id")
            .map_err(|_| ProjectRepositoryError::Database)?,
        code: row
            .try_get("code")
            .map_err(|_| ProjectRepositoryError::Database)?,
        name: row
            .try_get("name")
            .map_err(|_| ProjectRepositoryError::Database)?,
        owner_id: row
            .try_get("owner_id")
            .map_err(|_| ProjectRepositoryError::Database)?,
        description: row
            .try_get("description")
            .map_err(|_| ProjectRepositoryError::Database)?,
        start_date: row
            .try_get("start_date")
            .map_err(|_| ProjectRepositoryError::Database)?,
        end_date: row
            .try_get("end_date")
            .map_err(|_| ProjectRepositoryError::Database)?,
        status: status
            .parse()
            .map_err(|_| ProjectRepositoryError::Database)?,
    })
}
