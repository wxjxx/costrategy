use crate::app_state::AppState;
use crate::auth::{CurrentUser, DingtalkAuthService, Permission, SESSION_COOKIE_NAME};
use crate::dingtalk::{DingTalkClient, DingtalkSyncService};
use crate::error::{ApiErrorCode, AppError};
use crate::notifications::{NotificationRepository, TaskNotificationService};
use crate::projects::{
    CreateProject, ProjectRepository, ProjectRepositoryError, ProjectStatus, UpdateProject,
};
use crate::storage::{AttachmentStorage, StorageError};
use crate::tasks::{
    CreateTask, CreateTaskAttachment, CreateTaskComment, TaskFilter, TaskPriority, TaskRepository,
    TaskRepositoryError, TaskStatus, UpdateTask,
};
use crate::users::UserRepository;
use actix_multipart::Multipart;
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MAX_ATTACHMENT_SIZE: usize = 20 * 1024 * 1024;

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(health);
    crate::swagger::configure(config);
}

pub fn configure_app<C, R>(config: &mut web::ServiceConfig)
where
    C: DingTalkClient,
    R: UserRepository,
{
    configure(config);
    config
        .route("/api/auth/dingtalk/login", web::post().to(login::<C, R>))
        .route("/api/me", web::get().to(me::<C, R>))
        .route(
            "/api/dingtalk/sync",
            web::post().to(sync_dingtalk_contacts::<C, R>),
        );
}

pub fn configure_project_routes<C, R, P>(config: &mut web::ServiceConfig)
where
    C: DingTalkClient,
    R: UserRepository,
    P: ProjectRepository,
{
    config
        .route("/api/projects", web::get().to(list_projects::<C, R, P>))
        .route("/api/projects", web::post().to(create_project::<C, R, P>))
        .route(
            "/api/projects/{project_id}",
            web::put().to(update_project::<C, R, P>),
        )
        .route(
            "/api/projects/{project_id}/archive",
            web::post().to(archive_project::<C, R, P>),
        );
}

pub fn configure_task_routes<C, R, T, N>(config: &mut web::ServiceConfig)
where
    C: DingTalkClient,
    R: UserRepository,
    T: TaskRepository,
    N: NotificationRepository,
{
    config
        .route("/api/tasks", web::get().to(list_tasks::<C, R, T>))
        .route("/api/tasks", web::post().to(create_task::<C, R, T, N>))
        .route(
            "/api/tasks/{task_id}",
            web::get().to(get_task_detail::<C, R, T>),
        )
        .route(
            "/api/tasks/{task_id}",
            web::put().to(update_task::<C, R, T, N>),
        )
        .route(
            "/api/tasks/{task_id}/status",
            web::patch().to(update_task_status::<C, R, T>),
        )
        .route(
            "/api/tasks/{task_id}/archive",
            web::post().to(archive_task::<C, R, T>),
        )
        .route(
            "/api/tasks/{task_id}/comments",
            web::post().to(create_task_comment::<C, R, T>),
        );
}

pub fn configure_attachment_routes<C, R, T, S>(config: &mut web::ServiceConfig)
where
    C: DingTalkClient,
    R: UserRepository,
    T: TaskRepository,
    S: AttachmentStorage,
{
    config
        .route(
            "/api/tasks/{task_id}/attachments",
            web::post().to(upload_task_attachment::<C, R, T, S>),
        )
        .route(
            "/api/tasks/{task_id}/attachments/{attachment_id}/download",
            web::get().to(download_task_attachment::<C, R, T, S>),
        )
        .route(
            "/api/tasks/{task_id}/attachments/{attachment_id}",
            web::delete().to(delete_task_attachment::<C, R, T, S>),
        );
}

pub fn configure_notification_routes<C, R, N>(config: &mut web::ServiceConfig)
where
    C: DingTalkClient,
    R: UserRepository,
    N: NotificationRepository,
{
    config.route(
        "/api/notification-records",
        web::get().to(list_notification_records::<C, R, N>),
    );
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[get("/api/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse { status: "ok" })
}

#[derive(Debug, Deserialize)]
struct DingtalkLoginRequest {
    code: String,
}

async fn login<C, R>(
    state: web::Data<AppState<C, R>>,
    payload: web::Json<DingtalkLoginRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
{
    let service = DingtalkAuthService::new(state.dingtalk.clone(), state.users.clone());
    let current_user = service.login_with_code(&payload.code).await?;
    let session_token = state.sessions.create(current_user.clone());
    let cookie = Cookie::build(SESSION_COOKIE_NAME, session_token)
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .finish();

    Ok(HttpResponse::Ok().cookie(cookie).json(current_user))
}

async fn me<C, R>(
    state: web::Data<AppState<C, R>>,
    request: HttpRequest,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
{
    let current_user = require_current_user(&state, &request)?;
    Ok(HttpResponse::Ok().json(current_user))
}

async fn sync_dingtalk_contacts<C, R>(
    state: web::Data<AppState<C, R>>,
    request: HttpRequest,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
{
    let current_user = require_current_user(&state, &request)?;
    if !current_user.role.has(Permission::RunDingtalkSync) {
        return Err(AppError::forbidden(ApiErrorCode::AuthForbidden));
    }

    let service = DingtalkSyncService::new(state.dingtalk.clone(), state.users.clone());
    Ok(HttpResponse::Ok().json(service.sync_contacts().await?))
}

fn require_current_user<C, R>(
    state: &web::Data<AppState<C, R>>,
    request: &HttpRequest,
) -> Result<CurrentUser, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
{
    let Some(cookie) = request.cookie(SESSION_COOKIE_NAME) else {
        return Err(AppError::new(
            actix_web::http::StatusCode::UNAUTHORIZED,
            ApiErrorCode::AuthNotLogin,
        ));
    };

    state.sessions.get(cookie.value()).ok_or_else(|| {
        AppError::new(
            actix_web::http::StatusCode::UNAUTHORIZED,
            ApiErrorCode::AuthNotLogin,
        )
    })
}

#[derive(Debug, serde::Deserialize)]
struct CreateProjectRequest {
    code: String,
    name: String,
    description: Option<String>,
    start_date: Option<chrono::NaiveDate>,
    end_date: Option<chrono::NaiveDate>,
    status: ProjectStatus,
}

#[derive(Debug, serde::Deserialize)]
struct UpdateProjectRequest {
    name: String,
    description: Option<String>,
    start_date: Option<chrono::NaiveDate>,
    end_date: Option<chrono::NaiveDate>,
    status: ProjectStatus,
}

async fn list_projects<C, R, P>(
    state: web::Data<AppState<C, R>>,
    projects: web::Data<P>,
    request: HttpRequest,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    P: ProjectRepository,
{
    require_current_user(&state, &request)?;
    Ok(HttpResponse::Ok().json(
        projects
            .list_projects()
            .await
            .map_err(project_error_to_app)?,
    ))
}

async fn create_project<C, R, P>(
    state: web::Data<AppState<C, R>>,
    projects: web::Data<P>,
    request: HttpRequest,
    payload: web::Json<CreateProjectRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    P: ProjectRepository,
{
    require_project_manager(&state, &request)?;
    Ok(HttpResponse::Ok().json(
        projects
            .create_project(CreateProject {
                code: payload.code.trim().to_string(),
                name: payload.name.trim().to_string(),
                description: payload.description.clone(),
                start_date: payload.start_date,
                end_date: payload.end_date,
                status: payload.status,
            })
            .await
            .map_err(project_error_to_app)?,
    ))
}

async fn update_project<C, R, P>(
    state: web::Data<AppState<C, R>>,
    projects: web::Data<P>,
    request: HttpRequest,
    path: web::Path<uuid::Uuid>,
    payload: web::Json<UpdateProjectRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    P: ProjectRepository,
{
    require_project_manager(&state, &request)?;
    Ok(HttpResponse::Ok().json(
        projects
            .update_project(
                path.into_inner(),
                UpdateProject {
                    name: payload.name.trim().to_string(),
                    description: payload.description.clone(),
                    start_date: payload.start_date,
                    end_date: payload.end_date,
                    status: payload.status,
                },
            )
            .await
            .map_err(project_error_to_app)?,
    ))
}

async fn archive_project<C, R, P>(
    state: web::Data<AppState<C, R>>,
    projects: web::Data<P>,
    request: HttpRequest,
    path: web::Path<uuid::Uuid>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    P: ProjectRepository,
{
    require_project_manager(&state, &request)?;
    Ok(HttpResponse::Ok().json(
        projects
            .archive_project(path.into_inner())
            .await
            .map_err(project_error_to_app)?,
    ))
}

fn require_project_manager<C, R>(
    state: &web::Data<AppState<C, R>>,
    request: &HttpRequest,
) -> Result<CurrentUser, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
{
    let current_user = require_current_user(state, request)?;
    if !current_user.role.has(Permission::ManageProjects) {
        return Err(AppError::forbidden(ApiErrorCode::AuthForbidden));
    }
    Ok(current_user)
}

fn project_error_to_app(error: ProjectRepositoryError) -> AppError {
    match error {
        ProjectRepositoryError::NotFound => AppError::new(
            actix_web::http::StatusCode::NOT_FOUND,
            ApiErrorCode::ResourceNotFound,
        ),
        ProjectRepositoryError::Database => AppError::internal(ApiErrorCode::DatabaseError),
    }
}

#[derive(Debug, serde::Deserialize)]
struct TaskQuery {
    keyword: Option<String>,
    project_id: Option<uuid::Uuid>,
    assignee_id: Option<uuid::Uuid>,
    status: Option<TaskStatus>,
    priority: Option<TaskPriority>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
}

#[derive(Debug, serde::Deserialize)]
struct CreateTaskRequest {
    project_id: uuid::Uuid,
    title: String,
    assignee_id: uuid::Uuid,
    status: TaskStatus,
    priority: TaskPriority,
    start_date: chrono::NaiveDate,
    due_date: chrono::NaiveDate,
    description_json: serde_json::Value,
}

#[derive(Debug, serde::Deserialize)]
struct UpdateTaskRequest {
    project_id: uuid::Uuid,
    title: String,
    assignee_id: uuid::Uuid,
    status: TaskStatus,
    priority: TaskPriority,
    start_date: chrono::NaiveDate,
    due_date: chrono::NaiveDate,
    description_json: serde_json::Value,
}

#[derive(Debug, serde::Deserialize)]
struct UpdateTaskStatusRequest {
    status: TaskStatus,
}

#[derive(Debug, serde::Deserialize)]
struct CreateTaskCommentRequest {
    content: String,
}

struct UploadedAttachment {
    file_name: String,
    mime_type: Option<String>,
    bytes: Vec<u8>,
}

async fn list_tasks<C, R, T>(
    state: web::Data<AppState<C, R>>,
    tasks: web::Data<T>,
    request: HttpRequest,
    query: web::Query<TaskQuery>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    T: TaskRepository,
{
    require_current_user(&state, &request)?;
    Ok(HttpResponse::Ok().json(
        tasks
            .list_tasks(TaskFilter {
                keyword: query.keyword.clone(),
                project_id: query.project_id,
                assignee_id: query.assignee_id,
                status: query.status,
                priority: query.priority,
                date_from: query.date_from,
                date_to: query.date_to,
            })
            .await
            .map_err(task_error_to_app)?,
    ))
}

async fn get_task_detail<C, R, T>(
    state: web::Data<AppState<C, R>>,
    tasks: web::Data<T>,
    request: HttpRequest,
    path: web::Path<uuid::Uuid>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    T: TaskRepository,
{
    require_current_user(&state, &request)?;
    Ok(HttpResponse::Ok().json(
        tasks
            .get_task_detail(path.into_inner())
            .await
            .map_err(task_error_to_app)?,
    ))
}

async fn upload_task_attachment<C, R, T, S>(
    state: web::Data<AppState<C, R>>,
    tasks: web::Data<T>,
    storage: web::Data<S>,
    request: HttpRequest,
    path: web::Path<Uuid>,
    payload: Multipart,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    T: TaskRepository,
    S: AttachmentStorage,
{
    let current_user = require_current_user(&state, &request)?;
    if !current_user.role.has(Permission::UploadTaskAttachment) {
        return Err(AppError::forbidden(ApiErrorCode::AuthForbidden));
    }

    let task_id = path.into_inner();
    tasks.get_task(task_id).await.map_err(task_error_to_app)?;
    let uploaded = read_uploaded_attachment(payload).await?;
    let object_key = build_attachment_object_key(task_id, &uploaded.file_name);
    storage
        .put_object(
            &object_key,
            uploaded.bytes.clone(),
            uploaded.mime_type.as_deref(),
        )
        .await
        .map_err(storage_upload_error)?;

    Ok(HttpResponse::Ok().json(
        tasks
            .create_attachment(CreateTaskAttachment {
                task_id,
                file_name: uploaded.file_name,
                file_size: uploaded.bytes.len() as i64,
                mime_type: uploaded.mime_type,
                bucket: storage.bucket().to_string(),
                object_key,
                uploader_id: current_user.id,
                uploader_name: Some(current_user.name),
            })
            .await
            .map_err(task_error_to_app)?,
    ))
}

async fn download_task_attachment<C, R, T, S>(
    state: web::Data<AppState<C, R>>,
    tasks: web::Data<T>,
    storage: web::Data<S>,
    request: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    T: TaskRepository,
    S: AttachmentStorage,
{
    require_current_user(&state, &request)?;
    let (task_id, attachment_id) = path.into_inner();
    let attachment = tasks
        .get_attachment(task_id, attachment_id)
        .await
        .map_err(task_error_to_app)?;
    let stored = storage
        .get_object(&attachment.object_key)
        .await
        .map_err(storage_download_error)?;
    let content_type = stored
        .mime_type
        .or(attachment.mime_type)
        .unwrap_or_else(|| "application/octet-stream".to_string());

    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .insert_header((
            "Content-Disposition",
            format!(
                "attachment; filename=\"{}\"",
                attachment.file_name.replace('"', "")
            ),
        ))
        .body(stored.bytes))
}

async fn delete_task_attachment<C, R, T, S>(
    state: web::Data<AppState<C, R>>,
    tasks: web::Data<T>,
    storage: web::Data<S>,
    request: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    T: TaskRepository,
    S: AttachmentStorage,
{
    let current_user = require_current_user(&state, &request)?;
    let (task_id, attachment_id) = path.into_inner();
    let attachment = tasks
        .get_attachment(task_id, attachment_id)
        .await
        .map_err(task_error_to_app)?;
    let can_delete_own = attachment.uploader_id == current_user.id
        && current_user.role.has(Permission::DeleteOwnAttachment);
    if !can_delete_own && !current_user.role.has(Permission::DeleteAnyAttachment) {
        return Err(AppError::forbidden(ApiErrorCode::AttachmentDeleteForbidden));
    }

    storage
        .delete_object(&attachment.object_key)
        .await
        .map_err(storage_delete_error)?;

    Ok(HttpResponse::Ok().json(
        tasks
            .delete_attachment(task_id, attachment_id, current_user.id)
            .await
            .map_err(task_error_to_app)?,
    ))
}

async fn list_notification_records<C, R, N>(
    state: web::Data<AppState<C, R>>,
    notifications: web::Data<N>,
    request: HttpRequest,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    N: NotificationRepository,
{
    let current_user = require_current_user(&state, &request)?;
    if !current_user.role.has(Permission::ViewNotificationRecords) {
        return Err(AppError::forbidden(ApiErrorCode::AuthForbidden));
    }

    Ok(HttpResponse::Ok().json(
        notifications
            .list_records()
            .await
            .map_err(|_| AppError::internal(ApiErrorCode::DatabaseError))?,
    ))
}

async fn create_task<C, R, T, N>(
    state: web::Data<AppState<C, R>>,
    tasks: web::Data<T>,
    notifications: web::Data<N>,
    request: HttpRequest,
    payload: web::Json<CreateTaskRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    T: TaskRepository,
    N: NotificationRepository,
{
    let current_user = require_task_manager(&state, &request, Permission::CreateTask)?;
    let task = tasks
        .create_task(CreateTask {
            project_id: payload.project_id,
            title: payload.title.trim().to_string(),
            assignee_id: payload.assignee_id,
            status: payload.status,
            priority: payload.priority,
            start_date: payload.start_date,
            due_date: payload.due_date,
            description_json: payload.description_json.clone(),
            creator_id: current_user.id,
        })
        .await
        .map_err(task_error_to_app)?;

    TaskNotificationService::new(
        state.dingtalk.clone(),
        state.users.clone(),
        notifications.get_ref().clone(),
    )
    .notify_task_assigned(&task)
    .await?;

    Ok(HttpResponse::Ok().json(task))
}

async fn create_task_comment<C, R, T>(
    state: web::Data<AppState<C, R>>,
    tasks: web::Data<T>,
    request: HttpRequest,
    path: web::Path<uuid::Uuid>,
    payload: web::Json<CreateTaskCommentRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    T: TaskRepository,
{
    let current_user = require_current_user(&state, &request)?;
    Ok(HttpResponse::Ok().json(
        tasks
            .create_comment(CreateTaskComment {
                task_id: path.into_inner(),
                author_id: current_user.id,
                author_name: Some(current_user.name),
                content: payload.content.clone(),
            })
            .await
            .map_err(task_error_to_app)?,
    ))
}

async fn update_task<C, R, T, N>(
    state: web::Data<AppState<C, R>>,
    tasks: web::Data<T>,
    notifications: web::Data<N>,
    request: HttpRequest,
    path: web::Path<uuid::Uuid>,
    payload: web::Json<UpdateTaskRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    T: TaskRepository,
    N: NotificationRepository,
{
    let current_user = require_task_manager(&state, &request, Permission::EditTaskCoreFields)?;
    let task_id = path.into_inner();
    let previous = tasks.get_task(task_id).await.map_err(task_error_to_app)?;
    let updated = tasks
        .update_task(
            task_id,
            current_user.id,
            UpdateTask {
                project_id: payload.project_id,
                title: payload.title.trim().to_string(),
                assignee_id: payload.assignee_id,
                status: payload.status,
                priority: payload.priority,
                start_date: payload.start_date,
                due_date: payload.due_date,
                description_json: payload.description_json.clone(),
            },
        )
        .await
        .map_err(task_error_to_app)?;

    if previous.assignee_id != updated.assignee_id {
        TaskNotificationService::new(
            state.dingtalk.clone(),
            state.users.clone(),
            notifications.get_ref().clone(),
        )
        .notify_assignee_changed(&updated)
        .await?;
    }

    Ok(HttpResponse::Ok().json(updated))
}

async fn update_task_status<C, R, T>(
    state: web::Data<AppState<C, R>>,
    tasks: web::Data<T>,
    request: HttpRequest,
    path: web::Path<uuid::Uuid>,
    payload: web::Json<UpdateTaskStatusRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    T: TaskRepository,
{
    let current_user = require_current_user(&state, &request)?;
    let task = tasks.get_task(*path).await.map_err(task_error_to_app)?;
    if !current_user.role.has(Permission::UpdateAnyTaskStatus)
        && task.assignee_id != current_user.id
    {
        return Err(AppError::forbidden(ApiErrorCode::TaskNotAssignee));
    }
    if !current_user.role.has(Permission::UpdateAnyTaskStatus)
        && !current_user.role.has(Permission::UpdateOwnTaskStatus)
    {
        return Err(AppError::forbidden(ApiErrorCode::AuthForbidden));
    }

    Ok(HttpResponse::Ok().json(
        tasks
            .update_task_status(path.into_inner(), current_user.id, payload.status)
            .await
            .map_err(task_error_to_app)?,
    ))
}

async fn archive_task<C, R, T>(
    state: web::Data<AppState<C, R>>,
    tasks: web::Data<T>,
    request: HttpRequest,
    path: web::Path<uuid::Uuid>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    T: TaskRepository,
{
    let current_user = require_task_manager(&state, &request, Permission::ArchiveTask)?;
    Ok(HttpResponse::Ok().json(
        tasks
            .archive_task(path.into_inner(), current_user.id)
            .await
            .map_err(task_error_to_app)?,
    ))
}

fn require_task_manager<C, R>(
    state: &web::Data<AppState<C, R>>,
    request: &HttpRequest,
    permission: Permission,
) -> Result<CurrentUser, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
{
    let current_user = require_current_user(state, request)?;
    if !current_user.role.has(permission) {
        return Err(AppError::forbidden(ApiErrorCode::AuthForbidden));
    }
    Ok(current_user)
}

fn task_error_to_app(error: TaskRepositoryError) -> AppError {
    match error {
        TaskRepositoryError::NotFound => AppError::new(
            actix_web::http::StatusCode::NOT_FOUND,
            ApiErrorCode::ResourceNotFound,
        ),
        TaskRepositoryError::InvalidStatusTransition => {
            AppError::bad_request(ApiErrorCode::TaskInvalidStatusTransition)
        }
        TaskRepositoryError::DateRangeInvalid => {
            AppError::bad_request(ApiErrorCode::TaskDateRangeInvalid)
        }
        TaskRepositoryError::Validation => AppError::bad_request(ApiErrorCode::ValidationFailed),
        TaskRepositoryError::Database => AppError::internal(ApiErrorCode::DatabaseError),
    }
}

async fn read_uploaded_attachment(mut payload: Multipart) -> Result<UploadedAttachment, AppError> {
    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|_| AppError::bad_request(ApiErrorCode::ValidationFailed))?;
        if field.name() != Some("file") {
            continue;
        }

        let file_name = field
            .content_disposition()
            .and_then(|disposition| disposition.get_filename())
            .map(sanitize_file_name)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| AppError::bad_request(ApiErrorCode::ValidationFailed))?;
        let mime_type = field.content_type().map(ToString::to_string);
        let mut bytes = Vec::new();
        while let Some(chunk) = field.next().await {
            let chunk = chunk.map_err(|_| AppError::bad_request(ApiErrorCode::ValidationFailed))?;
            bytes.extend_from_slice(&chunk);
            if bytes.len() > MAX_ATTACHMENT_SIZE {
                return Err(AppError::bad_request(ApiErrorCode::ValidationFailed));
            }
        }
        if bytes.is_empty() {
            return Err(AppError::bad_request(ApiErrorCode::ValidationFailed));
        }

        return Ok(UploadedAttachment {
            file_name,
            mime_type,
            bytes,
        });
    }

    Err(AppError::bad_request(ApiErrorCode::ValidationFailed))
}

fn build_attachment_object_key(task_id: Uuid, file_name: &str) -> String {
    format!("tasks/{task_id}/attachments/{}-{file_name}", Uuid::new_v4())
}

fn sanitize_file_name(file_name: &str) -> String {
    let basename = file_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("attachment")
        .trim();
    let sanitized = basename
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>();
    if sanitized.is_empty() {
        "attachment".to_string()
    } else {
        sanitized
    }
}

fn storage_upload_error(_error: StorageError) -> AppError {
    AppError::internal(ApiErrorCode::AttachmentUploadFailed)
}

fn storage_download_error(_error: StorageError) -> AppError {
    AppError::internal(ApiErrorCode::StorageDownloadFailed)
}

fn storage_delete_error(_error: StorageError) -> AppError {
    AppError::internal(ApiErrorCode::InternalError)
}
