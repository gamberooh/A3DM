use crate::errors::AppError;
use axum::{
    async_trait,
    extract::{FromRequest, RequestParts, TypedHeader},
    headers::{authorization::Bearer, Authorization},
};
use crate::user::models::User;
use chrono::{Duration, Local};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

/// Claims struct
#[derive(Serialize, Deserialize)]
pub struct Claims {
    /// ID from the user model
    pub user_id: i32,
    /// User token version used to invalidate old JWTs on logout.
    pub token_version: i32,
    /// Expiration timestamp
    exp: usize,
}

/// Body used as response to login
#[derive(Serialize)]
pub struct AuthBody {
    /// Access token string
    access_token: String,
    /// "Bearer" string
    token_type: String,
}

/// Payload used for login
#[derive(Deserialize)]
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
}

/// Paylod used for user creation
#[derive(Deserialize)]
pub struct SignUpForm {
    pub name: String,
    pub email: String,
    pub username: String,
    pub password1: String,
    pub password2: String,
}

static KEYS: Lazy<Keys> = Lazy::new(|| {
    let secret = &crate::config::CONFIG.jwt_secret;
    Keys::new(secret.as_bytes())
});

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

impl Claims {
    /// Create a new Claim using the `user_id` and the current timestamp + 2 days
    pub fn new(user_id: i32, token_version: i32) -> Self {
        let expiration = Local::now() + Duration::days(1);

        Self {
            user_id,
            token_version,
            exp: expiration.timestamp() as usize,
        }
    }

    /// Returns the token as a string. If a token is not encoded, raises an
    /// `AppError::TokenCreation`
    pub fn get_token(&self) -> Result<String, AppError> {
        let token = encode(&Header::default(), &self, &KEYS.encoding)
            .map_err(|_| AppError::TokenCreation)?;

        Ok(token)
    }
}

impl AuthBody {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            token_type: "Bearer".to_string(),
        }
    }
}

/// Parse a request to get the Authorization header and then decode it checking its validation
#[async_trait]
impl<B> FromRequest<B> for Claims
where
    B: Send,
{
    type Rejection = AppError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request(req)
                .await
                .map_err(|_| AppError::InvalidToken)?;
        // Decode the user data
        let token_data = decode::<Claims>(bearer.token(), &KEYS.decoding, &Validation::default())
            .map_err(|_| AppError::InvalidToken)?;

        let now = Local::now().timestamp() as usize;

        if token_data.claims.exp < now {
            return Err(AppError::InvalidToken);
        }

        let current_token_version = User::token_version(token_data.claims.user_id)
            .await
            .map_err(|_| AppError::InvalidToken)?;

        if token_data.claims.token_version != current_token_version {
            return Err(AppError::InvalidToken);
        }

        Ok(token_data.claims)
    }
}
