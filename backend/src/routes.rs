use crate::app_state::AppState;
use crate::auth::{CurrentUser, DingtalkAuthService, Permission, UserRole, SESSION_COOKIE_NAME};
use crate::config::AppConfig;
use crate::dingtalk::{DingTalkClient, DingtalkSyncService};
use crate::error::{ApiErrorCode, AppError};
use crate::notifications::{
    NotificationRepository, NotificationRepositoryError, NotificationType, TaskNotificationService,
};
use crate::projects::{
    CreateProject, ProjectRepository, ProjectRepositoryError, ProjectStatus, UpdateProject,
};
use crate::settings::{
    build_settings_response, validate_updates, SettingsRepository, SettingsRepositoryError,
    SettingsUpdate,
};
use crate::storage::{AttachmentStorage, StorageError};
use crate::tasks::{
    CreateTask, CreateTaskAttachment, CreateTaskComment, TaskFilter, TaskPriority, TaskRepository,
    TaskRepositoryError, TaskSort, TaskStatus, UpdateTask,
};
use crate::users::{NewUser, UserRepository, UserRepositoryError, UserStatus};
use actix_multipart::Multipart;
use actix_web::cookie::time::Duration;
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MAX_ATTACHMENT_SIZE: usize = 20 * 1024 * 1024;
const MAX_AVATAR_URL_LENGTH: usize = 700_000;

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
        .route(
            "/api/auth/admin-token/login",
            web::post().to(admin_token_login::<C, R>),
        )
        .route("/api/auth/logout", web::post().to(logout::<C, R>))
        .route("/api/me", web::get().to(me::<C, R>))
        .route("/api/me/avatar", web::patch().to(update_my_avatar::<C, R>))
        .route(
            "/api/dingtalk/sync",
            web::post().to(sync_dingtalk_contacts::<C, R>),
        )
        .route(
            "/api/dingtalk/sync-logs",
            web::get().to(list_dingtalk_sync_logs::<C, R>),
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
            "/api/projects/{project_id}",
            web::delete().to(delete_project::<C, R, P>),
        )
        .route(
            "/api/projects/{project_id}/archive",
            web::post().to(archive_project::<C, R, P>),
        );
}

pub fn configure_user_routes<C, R>(config: &mut web::ServiceConfig)
where
    C: DingTalkClient,
    R: UserRepository,
{
    config
        .route("/api/users", web::get().to(list_users::<C, R>))
        .route(
            "/api/users/{user_id}/role",
            web::patch().to(update_user_role::<C, R>),
        )
        .route(
            "/api/users/{user_id}/status",
            web::patch().to(update_user_status::<C, R>),
        );
}

pub fn configure_settings_routes<C, R, S>(config: &mut web::ServiceConfig)
where
    C: DingTalkClient,
    R: UserRepository,
    S: SettingsRepository,
{
    config
        .route("/api/settings", web::get().to(get_settings::<C, R, S>))
        .route("/api/settings", web::put().to(update_settings::<C, R, S>));
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
            "/api/tasks/{task_id}",
            web::delete().to(delete_task::<C, R, T>),
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
            web::post().to(create_task_comment::<C, R, T, N>),
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
            "/api/rich-text/images",
            web::post().to(upload_rich_text_image::<C, R, S>),
        )
        .route(
            "/api/rich-text/images/{file_name}",
            web::get().to(download_rich_text_image::<C, R, S>),
        )
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
    config
        .route(
            "/api/notification-records",
            web::get().to(list_notification_records::<C, R, N>),
        )
        .route(
            "/api/my-notifications",
            web::get().to(list_my_notifications::<C, R, N>),
        )
        .route(
            "/api/my-notifications/{notification_id}/read",
            web::patch().to(mark_my_notification_read::<C, R, N>),
        )
        .route(
            "/api/notification-rules",
            web::get().to(list_notification_rules::<C, R, N>),
        )
        .route(
            "/api/notification-rules/{rule_type}",
            web::patch().to(update_notification_rule::<C, R, N>),
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

#[derive(Debug, Deserialize)]
struct AdminTokenLoginRequest {
    token: String,
}

#[derive(Debug, Deserialize)]
struct UpdateMyAvatarRequest {
    avatar_url: Option<String>,
}

async fn login<C, R>(
    state: web::Data<AppState<C, R>>,
    payload: web::Json<DingtalkLoginRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
{
    log::info!("dingtalk auth route: login request received");
    let service = DingtalkAuthService::new(state.dingtalk.clone(), state.users.clone());
    let current_user = service.login_with_code(&payload.code).await?;
    log::info!(
        "dingtalk auth route: creating backend session, user_id={}, role={:?}",
        current_user.id,
        current_user.role
    );
    let session_token = state.sessions.create(current_user.clone());
    let cookie = Cookie::build(SESSION_COOKIE_NAME, session_token)
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .finish();

    log::info!(
        "dingtalk auth route: login response ready, user_id={}",
        current_user.id
    );
    Ok(HttpResponse::Ok().cookie(cookie).json(current_user))
}

async fn admin_token_login<C, R>(
    state: web::Data<AppState<C, R>>,
    config: web::Data<AppConfig>,
    payload: web::Json<AdminTokenLoginRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
{
    let Some(configured_token) = config.admin_auth_token.as_deref() else {
        log::warn!("admin token auth route: token login attempted but token is not configured");
        return Err(AppError::new(
            actix_web::http::StatusCode::UNAUTHORIZED,
            ApiErrorCode::AuthNotLogin,
        ));
    };

    if payload.token != configured_token {
        log::warn!("admin token auth route: invalid token login attempt");
        return Err(AppError::new(
            actix_web::http::StatusCode::UNAUTHORIZED,
            ApiErrorCode::AuthNotLogin,
        ));
    }

    log::info!("admin token auth route: token accepted, creating bootstrap admin session");
    let admin_dingtalk_user_id = "__admin_token__";
    let existing_admin = state
        .users
        .find_by_dingtalk_user_id(admin_dingtalk_user_id)
        .await
        .map_err(|error| {
            log::error!("admin token auth route: failed to load existing bootstrap admin: {error}");
            AppError::internal(ApiErrorCode::DatabaseError)
        })?;
    state
        .users
        .upsert_synced_user(NewUser {
            dingtalk_user_id: admin_dingtalk_user_id.to_string(),
            union_id: None,
            name: "系统管理员".to_string(),
            avatar_url: existing_admin.and_then(|user| user.avatar_url),
            mobile: None,
            role: crate::auth::UserRole::Admin,
            status: UserStatus::Active,
        })
        .await
        .map_err(|error| {
            log::error!("admin token auth route: failed to upsert bootstrap admin: {error}");
            AppError::internal(ApiErrorCode::DatabaseError)
        })?;

    let user = state
        .users
        .find_by_dingtalk_user_id(admin_dingtalk_user_id)
        .await
        .map_err(|error| {
            log::error!("admin token auth route: failed to load bootstrap admin: {error}");
            AppError::internal(ApiErrorCode::DatabaseError)
        })?
        .ok_or_else(|| AppError::internal(ApiErrorCode::DatabaseError))?;
    let current_user = CurrentUser {
        id: user.id,
        name: user.name,
        avatar_url: user.avatar_url,
        role: user.role,
        departments: Vec::new(),
        permissions: user.role.permission_codes(),
    };
    let session_token = state.sessions.create(current_user.clone());
    let cookie = Cookie::build(SESSION_COOKIE_NAME, session_token)
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .finish();

    log::info!(
        "admin token auth route: login response ready, user_id={}",
        current_user.id
    );
    Ok(HttpResponse::Ok().cookie(cookie).json(current_user))
}

async fn logout<C, R>(
    state: web::Data<AppState<C, R>>,
    request: HttpRequest,
) -> Result<HttpResponse, AppError>
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

    if state.sessions.remove(cookie.value()).is_none() {
        return Err(AppError::new(
            actix_web::http::StatusCode::UNAUTHORIZED,
            ApiErrorCode::AuthNotLogin,
        ));
    }

    let cleared_cookie = Cookie::build(SESSION_COOKIE_NAME, "")
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(Duration::seconds(0))
        .finish();

    Ok(HttpResponse::Ok().cookie(cleared_cookie).finish())
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

async fn update_my_avatar<C, R>(
    state: web::Data<AppState<C, R>>,
    request: HttpRequest,
    payload: web::Json<UpdateMyAvatarRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
{
    let current_user = require_current_user(&state, &request)?;
    let avatar_url = normalize_avatar_url(payload.avatar_url.as_deref())?;
    let user = state
        .users
        .update_user_avatar(current_user.id, avatar_url)
        .await
        .map_err(user_error_to_app)?;
    let departments = state
        .users
        .list_user_departments(user.id)
        .await
        .map_err(user_error_to_app)?;
    let refreshed_user = CurrentUser {
        id: user.id,
        name: user.name,
        avatar_url: user.avatar_url,
        role: user.role,
        departments,
        permissions: user.role.permission_codes(),
    };

    if let Some(cookie) = request.cookie(SESSION_COOKIE_NAME) {
        let _ = state
            .sessions
            .replace(cookie.value(), refreshed_user.clone());
    }

    Ok(HttpResponse::Ok().json(refreshed_user))
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
    log::info!(
        "dingtalk sync route: request received, user_id={}, role={:?}",
        current_user.id,
        current_user.role
    );
    if !current_user.role.has(Permission::RunDingtalkSync) {
        log::warn!(
            "dingtalk sync route: permission denied, user_id={}, role={:?}",
            current_user.id,
            current_user.role
        );
        return Err(AppError::forbidden(ApiErrorCode::AuthForbidden));
    }

    log::info!(
        "dingtalk sync route: permission granted, starting service, user_id={}",
        current_user.id
    );
    let service = DingtalkSyncService::new(state.dingtalk.clone(), state.users.clone());
    let result = service.sync_contacts().await?;
    log::info!(
        "dingtalk sync route: service completed, user_id={}, created_users={}, updated_users={}, disabled_users={}",
        current_user.id,
        result.created_users,
        result.updated_users,
        result.disabled_users
    );
    Ok(HttpResponse::Ok().json(result))
}

async fn list_dingtalk_sync_logs<C, R>(
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

    Ok(HttpResponse::Ok().json(
        state
            .users
            .list_sync_logs()
            .await
            .map_err(user_error_to_app)?,
    ))
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

fn normalize_avatar_url(avatar_url: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(raw_avatar_url) = avatar_url else {
        return Ok(None);
    };
    let avatar_url = raw_avatar_url.trim();
    if avatar_url.is_empty() {
        return Ok(None);
    }
    if avatar_url.len() > MAX_AVATAR_URL_LENGTH {
        return Err(AppError::bad_request(ApiErrorCode::ValidationFailed));
    }
    if avatar_url.starts_with("https://")
        || avatar_url.starts_with("http://")
        || avatar_url.starts_with("data:image/")
    {
        return Ok(Some(avatar_url.to_string()));
    }

    Err(AppError::bad_request(ApiErrorCode::ValidationFailed))
}

#[derive(Debug, serde::Deserialize)]
struct CreateProjectRequest {
    code: Option<String>,
    name: String,
    owner_id: Option<Uuid>,
    description: Option<String>,
    start_date: Option<chrono::NaiveDate>,
    end_date: Option<chrono::NaiveDate>,
    status: ProjectStatus,
}

#[derive(Debug, serde::Deserialize)]
struct UpdateProjectRequest {
    name: String,
    owner_id: Option<Uuid>,
    description: Option<String>,
    start_date: Option<chrono::NaiveDate>,
    end_date: Option<chrono::NaiveDate>,
    status: ProjectStatus,
}

#[derive(Debug, serde::Deserialize)]
struct UpdateUserRoleRequest {
    role: crate::auth::UserRole,
}

#[derive(Debug, serde::Deserialize)]
struct UpdateUserStatusRequest {
    status: UserStatus,
}

#[derive(Debug, serde::Deserialize)]
struct SettingsUpdateRequest {
    settings: Vec<SettingsUpdateItemRequest>,
}

#[derive(Debug, serde::Deserialize)]
struct SettingsUpdateItemRequest {
    key: String,
    value: String,
}

async fn list_users<C, R>(
    state: web::Data<AppState<C, R>>,
    request: HttpRequest,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
{
    require_current_user(&state, &request)?;
    Ok(HttpResponse::Ok().json(state.users.list_users().await.map_err(user_error_to_app)?))
}

async fn update_user_role<C, R>(
    state: web::Data<AppState<C, R>>,
    request: HttpRequest,
    path: web::Path<Uuid>,
    payload: web::Json<UpdateUserRoleRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
{
    require_admin(&state, &request, Permission::ManageUsers)?;
    Ok(HttpResponse::Ok().json(
        state
            .users
            .update_user_role(path.into_inner(), payload.role)
            .await
            .map_err(user_error_to_app)?,
    ))
}

async fn update_user_status<C, R>(
    state: web::Data<AppState<C, R>>,
    request: HttpRequest,
    path: web::Path<Uuid>,
    payload: web::Json<UpdateUserStatusRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
{
    require_admin(&state, &request, Permission::ManageUsers)?;
    Ok(HttpResponse::Ok().json(
        state
            .users
            .update_user_status(path.into_inner(), payload.status)
            .await
            .map_err(user_error_to_app)?,
    ))
}

async fn get_settings<C, R, S>(
    state: web::Data<AppState<C, R>>,
    settings: web::Data<S>,
    config: web::Data<AppConfig>,
    request: HttpRequest,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    S: SettingsRepository,
{
    require_admin(&state, &request, Permission::ManageSystemSettings)?;
    let stored = settings
        .list_settings()
        .await
        .map_err(settings_error_to_app)?;
    Ok(HttpResponse::Ok().json(build_settings_response(stored, &config)))
}

async fn update_settings<C, R, S>(
    state: web::Data<AppState<C, R>>,
    settings: web::Data<S>,
    config: web::Data<AppConfig>,
    request: HttpRequest,
    payload: web::Json<SettingsUpdateRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    S: SettingsRepository,
{
    let current_user = require_admin(&state, &request, Permission::ManageSystemSettings)?;
    let updates = payload
        .settings
        .iter()
        .map(|item| SettingsUpdate {
            key: item.key.clone(),
            value: item.value.clone(),
            updated_by: current_user.id,
        })
        .collect::<Vec<_>>();
    validate_updates(&updates).map_err(settings_error_to_app)?;
    let stored = settings
        .upsert_settings(updates)
        .await
        .map_err(settings_error_to_app)?;

    Ok(HttpResponse::Ok().json(build_settings_response(stored, &config)))
}

fn require_admin<C, R>(
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
                code: payload
                    .code
                    .as_deref()
                    .map(str::trim)
                    .filter(|code| !code.is_empty())
                    .map(str::to_string),
                name: payload.name.trim().to_string(),
                owner_id: payload.owner_id,
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
                    owner_id: payload.owner_id,
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

async fn delete_project<C, R, P>(
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
    let current_user = require_current_user(&state, &request)?;
    if current_user.role != UserRole::Admin {
        return Err(AppError::forbidden(ApiErrorCode::AuthForbidden));
    }
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

fn user_error_to_app(error: UserRepositoryError) -> AppError {
    match error {
        UserRepositoryError::NotFound => AppError::new(
            actix_web::http::StatusCode::NOT_FOUND,
            ApiErrorCode::ResourceNotFound,
        ),
        UserRepositoryError::Database => AppError::internal(ApiErrorCode::DatabaseError),
    }
}

fn notification_error_to_app(error: NotificationRepositoryError) -> AppError {
    match error {
        NotificationRepositoryError::Validation => {
            AppError::bad_request(ApiErrorCode::ValidationFailed)
        }
        NotificationRepositoryError::Database => AppError::internal(ApiErrorCode::DatabaseError),
    }
}

fn settings_error_to_app(error: SettingsRepositoryError) -> AppError {
    match error {
        SettingsRepositoryError::Validation => {
            AppError::bad_request(ApiErrorCode::ValidationFailed)
        }
        SettingsRepositoryError::Database => AppError::internal(ApiErrorCode::DatabaseError),
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
    include_archived: Option<bool>,
    sort: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct CreateTaskRequest {
    project_id: uuid::Uuid,
    title: String,
    assignee_id: uuid::Uuid,
    assignee_ids: Option<Vec<uuid::Uuid>>,
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
    assignee_ids: Option<Vec<uuid::Uuid>>,
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

#[derive(Serialize)]
struct RichTextImageUploadResponse {
    url: String,
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
    let sort = query
        .sort
        .as_deref()
        .unwrap_or(TaskSort::default().as_str())
        .parse::<TaskSort>()
        .map_err(|_| AppError::bad_request(ApiErrorCode::ValidationFailed))?;
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
                include_archived: query.include_archived.unwrap_or(false),
                sort,
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
    let mut detail = tasks
        .get_task_detail(path.into_inner())
        .await
        .map_err(task_error_to_app)?;
    if detail.task.creator_name.is_none() {
        detail.task.creator_name = state
            .users
            .get_user(detail.task.creator_id)
            .await
            .map_err(user_error_to_app)?
            .map(|user| user.name);
    }
    Ok(HttpResponse::Ok().json(detail))
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

async fn upload_rich_text_image<C, R, S>(
    state: web::Data<AppState<C, R>>,
    storage: web::Data<S>,
    request: HttpRequest,
    payload: Multipart,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    S: AttachmentStorage,
{
    let current_user = require_current_user(&state, &request)?;
    if !current_user.role.has(Permission::UploadTaskAttachment) {
        return Err(AppError::forbidden(ApiErrorCode::AuthForbidden));
    }

    let uploaded = read_uploaded_image(payload).await?;
    let file_name = build_rich_text_image_file_name(&uploaded);
    let object_key = build_rich_text_image_object_key(&file_name);
    storage
        .put_object(&object_key, uploaded.bytes, uploaded.mime_type.as_deref())
        .await
        .map_err(storage_upload_error)?;

    Ok(HttpResponse::Ok().json(RichTextImageUploadResponse {
        url: format!("/api/rich-text/images/{file_name}"),
    }))
}

async fn download_rich_text_image<C, R, S>(
    state: web::Data<AppState<C, R>>,
    storage: web::Data<S>,
    request: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    S: AttachmentStorage,
{
    require_current_user(&state, &request)?;
    let object_key = build_rich_text_image_object_key(&sanitize_file_name(&path.into_inner()));
    let object = storage
        .get_object(&object_key)
        .await
        .map_err(storage_download_error)?;
    let mut response = HttpResponse::Ok();
    if let Some(mime_type) = object.mime_type {
        response.content_type(mime_type);
    }
    Ok(response.body(object.bytes))
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

async fn list_my_notifications<C, R, N>(
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
    Ok(HttpResponse::Ok().json(
        notifications
            .list_records_for_receiver(current_user.id)
            .await
            .map_err(|_| AppError::internal(ApiErrorCode::DatabaseError))?,
    ))
}

async fn mark_my_notification_read<C, R, N>(
    state: web::Data<AppState<C, R>>,
    notifications: web::Data<N>,
    request: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    N: NotificationRepository,
{
    let current_user = require_current_user(&state, &request)?;
    let Some(record) = notifications
        .mark_record_read(path.into_inner(), current_user.id)
        .await
        .map_err(|_| AppError::internal(ApiErrorCode::DatabaseError))?
    else {
        return Err(AppError::new(
            actix_web::http::StatusCode::NOT_FOUND,
            ApiErrorCode::ValidationFailed,
        ));
    };
    Ok(HttpResponse::Ok().json(record))
}

#[derive(Debug, serde::Deserialize)]
struct UpdateNotificationRuleRequest {
    enabled: bool,
}

async fn list_notification_rules<C, R, N>(
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
    if !current_user.role.has(Permission::ManageSystemSettings) {
        return Err(AppError::forbidden(ApiErrorCode::AuthForbidden));
    }

    Ok(HttpResponse::Ok().json(
        notifications
            .list_rules()
            .await
            .map_err(notification_error_to_app)?,
    ))
}

async fn update_notification_rule<C, R, N>(
    state: web::Data<AppState<C, R>>,
    notifications: web::Data<N>,
    request: HttpRequest,
    path: web::Path<String>,
    payload: web::Json<UpdateNotificationRuleRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    N: NotificationRepository,
{
    let current_user = require_current_user(&state, &request)?;
    if !current_user.role.has(Permission::ManageSystemSettings) {
        return Err(AppError::forbidden(ApiErrorCode::AuthForbidden));
    }
    let rule_type = path
        .as_str()
        .parse::<NotificationType>()
        .map_err(|_| AppError::bad_request(ApiErrorCode::ValidationFailed))?;

    Ok(HttpResponse::Ok().json(
        notifications
            .update_rule(rule_type, payload.enabled, current_user.id)
            .await
            .map_err(notification_error_to_app)?,
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
            assignee_ids: payload
                .assignee_ids
                .clone()
                .unwrap_or_else(|| vec![payload.assignee_id]),
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

async fn create_task_comment<C, R, T, N>(
    state: web::Data<AppState<C, R>>,
    tasks: web::Data<T>,
    notifications: web::Data<N>,
    request: HttpRequest,
    path: web::Path<uuid::Uuid>,
    payload: web::Json<CreateTaskCommentRequest>,
) -> Result<HttpResponse, AppError>
where
    C: DingTalkClient,
    R: UserRepository,
    T: TaskRepository,
    N: NotificationRepository,
{
    let current_user = require_current_user(&state, &request)?;
    let task_id = path.into_inner();
    let comment = tasks
        .create_comment(CreateTaskComment {
            task_id,
            author_id: current_user.id,
            author_name: Some(current_user.name),
            content: payload.content.clone(),
        })
        .await
        .map_err(task_error_to_app)?;

    let task = tasks.get_task(task_id).await.map_err(task_error_to_app)?;
    TaskNotificationService::new(
        state.dingtalk.clone(),
        state.users.clone(),
        notifications.get_ref().clone(),
    )
    .notify_task_commented(&task, current_user.id, &comment.content)
    .await?;

    Ok(HttpResponse::Ok().json(comment))
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
                assignee_ids: payload
                    .assignee_ids
                    .clone()
                    .unwrap_or_else(|| vec![payload.assignee_id]),
                status: payload.status,
                priority: payload.priority,
                start_date: payload.start_date,
                due_date: payload.due_date,
                description_json: payload.description_json.clone(),
            },
        )
        .await
        .map_err(task_error_to_app)?;

    if previous.assignees != updated.assignees {
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
        && !task
            .assignees
            .iter()
            .any(|assignee| assignee.id == current_user.id)
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

async fn delete_task<C, R, T>(
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
    let current_user = require_current_user(&state, &request)?;
    let task_id = path.into_inner();
    let task = tasks.get_task(task_id).await.map_err(task_error_to_app)?;
    let can_delete =
        task.creator_id == current_user.id || current_user.role.has(Permission::ArchiveTask);
    if !can_delete {
        return Err(AppError::forbidden(ApiErrorCode::AuthForbidden));
    }

    Ok(HttpResponse::Ok().json(
        tasks
            .archive_task(task_id, current_user.id)
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
        if is_svg_upload(&file_name, mime_type.as_deref()) {
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

async fn read_uploaded_image(payload: Multipart) -> Result<UploadedAttachment, AppError> {
    let uploaded = read_uploaded_attachment(payload).await?;
    let is_image = uploaded
        .mime_type
        .as_deref()
        .is_some_and(|mime_type| mime_type.starts_with("image/"));
    if is_image {
        Ok(uploaded)
    } else {
        Err(AppError::bad_request(ApiErrorCode::ValidationFailed))
    }
}

fn build_attachment_object_key(task_id: Uuid, file_name: &str) -> String {
    format!("tasks/{task_id}/attachments/{}-{file_name}", Uuid::new_v4())
}

fn build_rich_text_image_object_key(file_name: &str) -> String {
    format!("rich-text/images/{file_name}")
}

fn build_rich_text_image_file_name(uploaded: &UploadedAttachment) -> String {
    let extension = uploaded
        .file_name
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .filter(|extension| {
            !extension.is_empty()
                && extension.len() <= 10
                && extension
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        })
        .map(str::to_ascii_lowercase)
        .or_else(|| image_extension_from_mime(uploaded.mime_type.as_deref()).map(str::to_string))
        .unwrap_or_else(|| "png".to_string());
    format!("{}.{}", Uuid::new_v4(), extension)
}

fn image_extension_from_mime(mime_type: Option<&str>) -> Option<&'static str> {
    match mime_type? {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        _ => None,
    }
}

fn is_svg_upload(file_name: &str, mime_type: Option<&str>) -> bool {
    let is_svg_mime = mime_type
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("image/svg+xml"));
    let is_svg_extension = file_name
        .rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("svg"));
    is_svg_mime || is_svg_extension
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
