use crate::{
    auth::ldap,
    errors::AppError,
    auth::models::{AuthBody, Claims, LoginCredentials, SignUpForm},
    user::models::{User, UserList},
    routes::JsonCreate,
};
use axum::{routing::post, Json, Router};
use rand::{distributions::Alphanumeric, Rng};

/// Create routes for `/v1/auth/` namespace
pub fn create_route() -> Router {
    Router::new()
        .route("/login", post(make_login))
        .route("/signup", post(signup))
}

/// Make login. Check if a user with the email and password passed in request body exists into the
/// database
async fn make_login(Json(payload): Json<LoginCredentials>) -> Result<Json<AuthBody>, AppError> {
    if ldap::is_enabled() {
        let identity = ldap::authenticate(&payload.username, &payload.password)
            .await?
            .ok_or(AppError::Unauthorized)?;
        let user = get_or_create_ldap_user(identity).await?;

        let claims = Claims::new(user.id);
        let token = claims.get_token()?;
        return Ok(Json(AuthBody::new(token)));
    }

    let user = User::new(
        String::new(),
        String::new(),
        payload.username,
        payload.password,
    );
    match User::find(user).await {
        Ok(user) => {
            let claims = Claims::new(user.id);
            let token = claims.get_token()?;
            Ok(Json(AuthBody::new(token)))
        }
        Err(_) => Err(AppError::NotFound("User not found".to_string())),
    }
}

/// Create a new user
async fn signup(Json(payload): Json<SignUpForm>) -> Result<JsonCreate<AuthBody>, AppError> {
    if ldap::is_enabled() {
        return Err(AppError::BadRequest(
            "Signup locale disabilitata quando LDAP e' attivo".to_string(),
        ));
    }

    if payload.password1 != payload.password2 {
        return Err(AppError::BadRequest(
            "The inserted passwords do not match".to_string(),
        ));
    }

    if User::email_has_taken(&payload.email).await? {
        return Err(AppError::BadRequest(
            "An user with this email already exists".to_string(),
        ));
    }

    if User::username_has_taken(&payload.username).await? {
        return Err(AppError::BadRequest(
            "An user with this username already exists".to_string(),
        ));
    }

    let user = User::new(
        payload.name,
        payload.email,
        payload.username,
        payload.password1,
    );
    let user = User::create(user).await?;

    let claims = Claims::new(user.id);
    let token = claims.get_token()?;
    Ok(JsonCreate(AuthBody::new(token)))
}

async fn get_or_create_ldap_user(identity: ldap::LdapIdentity) -> Result<UserList, AppError> {
    let username = identity.username;
    let name = identity.name;
    let mut email = identity.email;
    let is_staff = identity.is_staff;

    if let Ok(user) = User::find_by_username(&username).await {
        if user.is_staff.unwrap_or(false) != is_staff {
            return User::set_staff_by_username(&username, is_staff).await;
        }

        return Ok(user);
    }

    if User::email_has_taken(&email).await? {
        email = format!("{}+ldap@ldap.local", username);
    }

    // Never persist LDAP credentials: we store a local, random non-usable placeholder.
    let password = generate_unusable_local_password();

    let user = User::new(name, email, username.clone(), password);
    let user = User::create(user).await?;

    if is_staff {
        return User::set_staff_by_username(&username, true).await;
    }

    Ok(user)
}

fn generate_unusable_local_password() -> String {
    let random: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    format!("ldap-disabled-{}", random)
}


