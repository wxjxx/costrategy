use actix_web::{web, App, HttpServer};
use costrategy_backend::app_state::AppState;
use costrategy_backend::auth::SessionStore;
use costrategy_backend::config::AppConfig;
use costrategy_backend::dingtalk::MockDingTalkClient;
use costrategy_backend::notifications::SqlxNotificationRepository;
use costrategy_backend::projects::SqlxProjectRepository;
use costrategy_backend::storage::RustfsAttachmentStorage;
use costrategy_backend::tasks::SqlxTaskRepository;
use costrategy_backend::users::SqlxUserRepository;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = AppConfig::from_env().map_err(|error| std::io::Error::other(error.to_string()))?;
    let pool = sqlx::PgPool::connect(&config.database.url())
        .await
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let app_state = AppState::new(
        MockDingTalkClient::default(),
        SqlxUserRepository::new(pool.clone()),
        SessionStore::default(),
    );
    let projects = SqlxProjectRepository::new(pool.clone());
    let tasks = SqlxTaskRepository::new(pool.clone());
    let notifications = SqlxNotificationRepository::new(pool);
    let storage = RustfsAttachmentStorage::from_config(&config.rustfs)
        .await
        .map_err(|error| std::io::Error::other(error.to_string()))?;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::new(projects.clone()))
            .app_data(web::Data::new(tasks.clone()))
            .app_data(web::Data::new(notifications.clone()))
            .app_data(web::Data::new(storage.clone()))
            .configure(
                costrategy_backend::routes::configure_app::<MockDingTalkClient, SqlxUserRepository>,
            )
            .configure(
                costrategy_backend::routes::configure_project_routes::<
                    MockDingTalkClient,
                    SqlxUserRepository,
                    SqlxProjectRepository,
                >,
            )
            .configure(
                costrategy_backend::routes::configure_task_routes::<
                    MockDingTalkClient,
                    SqlxUserRepository,
                    SqlxTaskRepository,
                    SqlxNotificationRepository,
                >,
            )
            .configure(
                costrategy_backend::routes::configure_attachment_routes::<
                    MockDingTalkClient,
                    SqlxUserRepository,
                    SqlxTaskRepository,
                    RustfsAttachmentStorage,
                >,
            )
            .configure(
                costrategy_backend::routes::configure_notification_routes::<
                    MockDingTalkClient,
                    SqlxUserRepository,
                    SqlxNotificationRepository,
                >,
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
