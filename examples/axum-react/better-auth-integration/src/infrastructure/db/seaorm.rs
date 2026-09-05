use better_auth::seaorm::sea_orm::{Database, DatabaseConnection};

/// Establishes a database connection.
pub async fn connect(database_url: &str) -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    let db = Database::connect(database_url).await?;
    Ok(db)
}
