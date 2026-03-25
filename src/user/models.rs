use crate::{
    config::CONFIG,
    db::get_client,
    errors::AppError,
    model::models::{Model, ModelUser},
};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, NoneAsEmptyString};
use sqlx::Row;
use validator::Validate;

/// User model
#[derive(Deserialize, Serialize, Validate)]
pub struct User {
    id: i32,
    name: String,
    #[validate(length(min = 4, message = "Can not be empty"))]
    email: String,
    #[validate(length(min = 2, message = "Can not be empty"))]
    username: String,
    #[validate(length(min = 8, message = "Must be min 8 chars length"))]
    password: String,
    is_staff: Option<bool>,
    avatar: Option<String>,
}

/// Paylod used for user editing
#[derive(Deserialize)]
pub struct UserEdit {
    pub name: String,
    pub email: String,
    pub username: String,
    pub is_staff: Option<bool>,
}

/// Response used to print a user (or a users list)
#[serde_as]
#[derive(Deserialize, Serialize, sqlx::FromRow, Validate)]
pub struct UserList {
    pub id: i32,
    pub name: String,
    #[validate(length(min = 4, message = "Can not be empty"))]
    pub email: String,
    #[validate(length(min = 2, message = "Can not be empty"))]
    pub username: String,
    pub is_staff: Option<bool>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub avatar: Option<String>,
}

impl User {
    /// By default an user has id = 0. It is not created yet
    pub fn new(name: String, email: String, username: String, password: String) -> Self {
        Self {
            id: 0,
            name,
            email,
            username,
            password,
            is_staff: Some(false),
            avatar: None,
        }
    }

    /// Create a new user from the model using a SHA256 crypted password
    pub async fn create(user: User) -> Result<UserList, AppError> {
        let pool = unsafe { get_client() };

        user.validate()
            .map_err(|error| AppError::BadRequest(error.to_string()))?;

        let crypted_password = sha256::digest(user.password);

        let rec: UserList = sqlx::query_as(
            r#"
                INSERT INTO users (name, email, username, password)
                VALUES ( $1, $2, $3, $4)
                RETURNING id, name, email, username, is_staff, avatar
            "#,
        )
        .bind(user.name)
        .bind(user.email)
        .bind(user.username)
        .bind(crypted_password)
        .fetch_one(pool)
        .await?;

        Ok(rec)
    }

    /// Find a user using the model. It used for login
    pub async fn find(user: User) -> Result<UserList, AppError> {
        let pool = unsafe { get_client() };

        let crypted_password = sha256::digest(user.password);

        let rec: UserList = sqlx::query_as(
            r#"
                SELECT id, name, email, username, is_staff, avatar FROM "users"
                WHERE username = $1 AND password = $2
            "#,
        )
        .bind(user.username)
        .bind(crypted_password)
        .fetch_one(pool)
        .await?;

        Ok(rec)
    }

    /// Find an user by username without password check.
    pub async fn find_by_username(username: &str) -> Result<UserList, AppError> {
        let pool = unsafe { get_client() };

        let rec: UserList = sqlx::query_as(
            r#"
                SELECT id, name, email, username, is_staff, avatar FROM "users"
                WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_one(pool)
        .await?;

        Ok(rec)
    }

    /// Update staff flag by username and returns updated user.
    pub async fn set_staff_by_username(username: &str, is_staff: bool) -> Result<UserList, AppError> {
        let pool = unsafe { get_client() };

        let rec: UserList = sqlx::query_as(
            r#"
                UPDATE users SET is_staff = $1
                WHERE username = $2
                RETURNING id, name, email, username, is_staff, avatar
            "#,
        )
        .bind(is_staff)
        .bind(username)
        .fetch_one(pool)
        .await?;

        Ok(rec)
    }

    /// Returns the user with id = `user_id`
    pub async fn find_by_id(user_id: i32) -> Result<UserList, AppError> {
        let pool = unsafe { get_client() };

        let rec: UserList = sqlx::query_as(
            r#"
                SELECT id, name, email, username, is_staff, avatar FROM "users"
                WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        Ok(rec)
    }

    /// List all users
    pub async fn list(page: i64) -> Result<Vec<UserList>, AppError> {
        let pool = unsafe { get_client() };
        let rows: Vec<UserList> = sqlx::query_as(
            r#"SELECT id, name, email, username, is_staff, avatar FROM users
            ORDER BY id DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(CONFIG.page_limit)
        .bind(CONFIG.page_limit * page)
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    /// Return the number of users.
    pub async fn count() -> Result<i64, AppError> {
        let pool = unsafe { get_client() };
        let cursor = sqlx::query(r#"SELECT COUNT(id) as count FROM users"#)
            .fetch_one(pool)
            .await?;

        let count: i64 = cursor.try_get(0).unwrap();
        Ok(count)
    }

    /// Prevent the "uniquess" Postgres fields check. Check if username has been taken
    pub async fn username_has_taken(username: &String) -> Result<bool, AppError> {
        let pool = unsafe { get_client() };
        let cursor = sqlx::query(
            r#"
                SELECT COUNT(id) as count FROM users WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_one(pool)
        .await?;

        let count: i64 = cursor.try_get(0).unwrap();

        Ok(count > 0)
    }

    /// Prevent the "uniquess" Postgres fields check. Check if email has been taken
    pub async fn email_has_taken(email: &String) -> Result<bool, AppError> {
        let pool = unsafe { get_client() };
        let cursor = sqlx::query(
            r#"
                SELECT COUNT(id) as count FROM users WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_one(pool)
        .await?;

        let count: i64 = cursor.try_get(0).unwrap();

        Ok(count > 0)
    }
}

impl UserList {
    /// Edit an user avatar
    pub async fn edit_avatar(&mut self, avatar: Option<String>) -> Result<(), AppError> {
        let pool = unsafe { get_client() };
        sqlx::query(
            r#"
            UPDATE users SET avatar = $1 WHERE id = $2
            "#,
        )
        .bind(&avatar)
        .bind(self.id)
        .execute(pool)
        .await?;

        self.avatar = avatar;

        Ok(())
    }

    /// Edit an user
    pub async fn edit(&mut self, payload: UserEdit) -> Result<(), AppError> {
        let pool = unsafe { get_client() };

        // Make assignments before the `sqlx::query()` so to perform validation.
        // If the `AppError::BadRequest` is raised, the query (and then the update) will be skipped
        self.name = payload.name.clone();
        self.username = payload.username.clone();
        self.email = payload.email.clone();
        self.is_staff = payload.is_staff;

        self.validate()
            .map_err(|error| AppError::BadRequest(error.to_string()))?;

        sqlx::query(
            r#"
            UPDATE users SET name = $1, username = $2, email = $3, is_staff = $4 WHERE id = $5
            "#,
        )
        .bind(&payload.name)
        .bind(&payload.username)
        .bind(&payload.email)
        .bind(payload.is_staff.unwrap_or_default())
        .bind(self.id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get all models created by an user
    pub async fn get_models(&self, page: i64) -> Result<Vec<ModelUser>, AppError> {
        Model::list_from_author(page, self.id).await
    }

    /// Returns the number of models for an user
    pub async fn count_models(&self) -> Result<i64, AppError> {
        Model::count_filter_by_author(self.id).await
    }
}
