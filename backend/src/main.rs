use actix_web::{web, App, HttpServer};
use costrategy_backend::app_state::AppState;
use costrategy_backend::auth::SessionStore;
use costrategy_backend::config::AppConfig;
use costrategy_backend::dingtalk::ConfiguredDingTalkClient;
use costrategy_backend::logging::{init_logging, request_logger};
use costrategy_backend::notifications::{
    start_due_tomorrow_scheduler, start_overdue_scheduler, ReminderNotificationService,
    SqlxNotificationRepository,
};
use costrategy_backend::projects::SqlxProjectRepository;
use costrategy_backend::settings::SqlxSettingsRepository;
use costrategy_backend::storage::RustfsAttachmentStorage;
use costrategy_backend::tasks::SqlxTaskRepository;
use costrategy_backend::users::SqlxUserRepository;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logging();
    log::info!("starting costrategy backend service");

    let config = AppConfig::from_env().map_err(|error| std::io::Error::other(error.to_string()))?;
    log::info!("loaded backend configuration");

    let pool = sqlx::PgPool::connect(&config.database.url())
        .await
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    log::info!("connected to postgres");

    let dingtalk_configured = config.dingtalk.is_some();
    let dingtalk = ConfiguredDingTalkClient::from_config(config.dingtalk.clone());
    let users = SqlxUserRepository::new(pool.clone());
    let app_state = AppState::new(dingtalk.clone(), users.clone(), SessionStore::default());
    let projects = SqlxProjectRepository::new(pool.clone());
    let tasks = SqlxTaskRepository::new(pool.clone());
    let notifications = SqlxNotificationRepository::new(pool.clone());
    let settings = SqlxSettingsRepository::new(pool);
    let storage = RustfsAttachmentStorage::from_config(&config.rustfs)
        .await
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let _background_schedulers = dingtalk_configured.then(|| {
        let reminder_service = ReminderNotificationService::new(
            dingtalk.clone(),
            users.clone(),
            notifications.clone(),
            tasks.clone(),
        );
        (
            start_due_tomorrow_scheduler(reminder_service.clone()),
            start_overdue_scheduler(reminder_service),
        )
    });
    if dingtalk_configured {
        log::info!("started notification background schedulers; dingtalk contacts sync is manual");
    } else {
        log::info!("dingtalk configuration is not set; background schedulers are disabled");
    }

    let bind_address = ("127.0.0.1", 8080);
    log::info!(
        "starting http server on {}:{}",
        bind_address.0,
        bind_address.1
    );

    HttpServer::new(move || {
        App::new()
            .wrap(request_logger())
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::new(projects.clone()))
            .app_data(web::Data::new(tasks.clone()))
            .app_data(web::Data::new(notifications.clone()))
            .app_data(web::Data::new(storage.clone()))
            .app_data(web::Data::new(settings.clone()))
            .app_data(web::Data::new(config.clone()))
            .configure(
                costrategy_backend::routes::configure_app::<
                    ConfiguredDingTalkClient,
                    SqlxUserRepository,
                >,
            )
            .configure(
                costrategy_backend::routes::configure_project_routes::<
                    ConfiguredDingTalkClient,
                    SqlxUserRepository,
                    SqlxProjectRepository,
                >,
            )
            .configure(
                costrategy_backend::routes::configure_user_routes::<
                    ConfiguredDingTalkClient,
                    SqlxUserRepository,
                >,
            )
            .configure(
                costrategy_backend::routes::configure_settings_routes::<
                    ConfiguredDingTalkClient,
                    SqlxUserRepository,
                    SqlxSettingsRepository,
                >,
            )
            .configure(
                costrategy_backend::routes::configure_task_routes::<
                    ConfiguredDingTalkClient,
                    SqlxUserRepository,
                    SqlxTaskRepository,
                    SqlxNotificationRepository,
                >,
            )
            .configure(
                costrategy_backend::routes::configure_attachment_routes::<
                    ConfiguredDingTalkClient,
                    SqlxUserRepository,
                    SqlxTaskRepository,
                    RustfsAttachmentStorage,
                >,
            )
            .configure(
                costrategy_backend::routes::configure_notification_routes::<
                    ConfiguredDingTalkClient,
                    SqlxUserRepository,
                    SqlxNotificationRepository,
                >,
            )
    })
    .bind(bind_address)?
    .run()
    .await
}
