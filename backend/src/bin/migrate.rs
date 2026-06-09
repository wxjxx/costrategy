use costrategy_backend::config::AppConfig;
use sqlx::migrate::Migrator;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[tokio::main]
async fn main() {
    let config = match AppConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("config failed: {error}");
            std::process::exit(1);
        }
    };

    let pool = match sqlx::PgPool::connect(&config.database.url()).await {
        Ok(pool) => pool,
        Err(_) => {
            eprintln!("postgres failed: connection check failed");
            std::process::exit(1);
        }
    };

    if let Err(error) = MIGRATOR.run(&pool).await {
        eprintln!("migration failed: {error}");
        std::process::exit(1);
    }

    println!("migrations ok");
}
