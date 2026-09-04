//! Better Auth Schema Definitions
//!
//! Defines the database entities for authentication using SeaORM and Better Auth's AuthEntity derive.

use better_auth::seaorm::sea_orm;
use better_auth::seaorm::sea_orm::entity::prelude::*;
use better_auth::seaorm::sea_orm::{ConnectionTrait, Schema};
use better_auth::{
    seaorm::{AuthEntity, DatabaseConnection},
    AuthSchema,
};

// ===== User Entity =====

pub mod user {
    use super::*;

    #[derive(Clone, Debug, serde::Serialize, DeriveEntityModel, AuthEntity)]
    #[auth(role = "user")]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: Option<String>,
        pub email: Option<String>,
        pub email_verified: bool,
        pub image: Option<String>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ===== Session Entity =====

pub mod session {
    use super::*;

    #[derive(Clone, Debug, serde::Serialize, DeriveEntityModel, AuthEntity)]
    #[auth(role = "session")]
    #[sea_orm(table_name = "sessions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub expires_at: DateTimeUtc,
        pub token: String,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
        pub ip_address: Option<String>,
        pub user_agent: Option<String>,
        pub user_id: String,
        pub active: bool,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ===== Account Entity (OAuth) =====

pub mod account {
    use super::*;

    #[derive(Clone, Debug, serde::Serialize, DeriveEntityModel, AuthEntity)]
    #[auth(role = "account")]
    #[sea_orm(table_name = "accounts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub account_id: String,
        pub provider_id: String,
        pub user_id: String,
        pub access_token: Option<String>,
        pub refresh_token: Option<String>,
        pub id_token: Option<String>,
        pub access_token_expires_at: Option<DateTimeUtc>,
        pub refresh_token_expires_at: Option<DateTimeUtc>,
        pub scope: Option<String>,
        pub password: Option<String>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ===== Verification Entity =====

pub mod verification {
    use super::*;

    #[derive(Clone, Debug, serde::Serialize, DeriveEntityModel, AuthEntity)]
    #[auth(role = "verification")]
    #[sea_orm(table_name = "verifications")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub identifier: String,
        pub value: String,
        pub expires_at: DateTimeUtc,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ===== Auth Schema =====

/// Application auth schema that ties all entities together
pub struct AppAuthSchema;

impl AuthSchema for AppAuthSchema {
    type User = crate::auth_schema::user::Model;
    type Session = crate::auth_schema::session::Model;
    type Account = crate::auth_schema::account::Model;
    type Verification = crate::auth_schema::verification::Model;
}

/// Run database migrations for auth tables
pub async fn run_app_migrations(database: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    let schema = Schema::new(database.get_database_backend());
    for statement in [
        schema
            .create_table_from_entity(user::Entity)
            .if_not_exists()
            .to_owned(),
        schema
            .create_table_from_entity(session::Entity)
            .if_not_exists()
            .to_owned(),
        schema
            .create_table_from_entity(account::Entity)
            .if_not_exists()
            .to_owned(),
        schema
            .create_table_from_entity(verification::Entity)
            .if_not_exists()
            .to_owned(),
    ] {
        let _ = database.execute(&statement).await?;
    }
    Ok(())
}
