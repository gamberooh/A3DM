use crate::{config::CONFIG, errors::AppError};
use ldap3::{LdapConnAsync, Scope, SearchEntry};

pub struct LdapIdentity {
    pub username: String,
    pub name: String,
    pub email: String,
    pub is_staff: bool,
}

struct LdapSettings {
    url: String,
    base_dn: String,
    bind_dn: Option<String>,
    bind_password: Option<String>,
    user_filter: String,
    username_attr: String,
    name_attr: String,
    email_attr: String,
    memberof_attr: String,
    admin_group_dn: Option<String>,
}

impl LdapSettings {
    fn from_config() -> Option<Self> {
        if !CONFIG.ldap_enabled.unwrap_or(false) {
            return None;
        }

        let url = CONFIG.ldap_url.as_ref()?.trim().to_string();
        let base_dn = CONFIG.ldap_base_dn.as_ref()?.trim().to_string();

        if url.is_empty() || base_dn.is_empty() {
            return None;
        }

        Some(Self {
            url,
            base_dn,
            bind_dn: CONFIG
                .ldap_bind_dn
                .as_ref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            bind_password: CONFIG
                .ldap_bind_password
                .as_ref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            user_filter: CONFIG
                .ldap_user_filter
                .clone()
                .unwrap_or_else(|| "(uid={username})".to_string()),
            username_attr: CONFIG
                .ldap_username_attr
                .clone()
                .unwrap_or_else(|| "uid".to_string()),
            name_attr: CONFIG
                .ldap_name_attr
                .clone()
                .unwrap_or_else(|| "cn".to_string()),
            email_attr: CONFIG
                .ldap_email_attr
                .clone()
                .unwrap_or_else(|| "mail".to_string()),
            memberof_attr: CONFIG
                .ldap_memberof_attr
                .clone()
                .unwrap_or_else(|| "memberOf".to_string()),
            admin_group_dn: CONFIG
                .ldap_admin_group_dn
                .as_ref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        })
    }
}

pub fn is_enabled() -> bool {
    LdapSettings::from_config().is_some()
}

pub async fn authenticate(username: &str, password: &str) -> Result<Option<LdapIdentity>, AppError> {
    let settings = match LdapSettings::from_config() {
        Some(settings) => settings,
        None => return Ok(None),
    };

    if password.trim().is_empty() {
        return Err(AppError::Unauthorized);
    }

    let (conn, mut ldap) = LdapConnAsync::new(&settings.url)
        .await
        .map_err(|error| {
            tracing::error!("LDAP connection error: {:?}", error);
            AppError::Unauthorized
        })?;
    ldap3::drive!(conn);

    if let (Some(bind_dn), Some(bind_password)) = (&settings.bind_dn, &settings.bind_password) {
        ldap.simple_bind(bind_dn, bind_password)
            .await
            .map_err(|error| {
                tracing::error!("LDAP service bind error: {:?}", error);
                AppError::Unauthorized
            })?
            .success()
            .map_err(|error| {
                tracing::error!("LDAP service bind not successful: {:?}", error);
                AppError::Unauthorized
            })?;
    }

    let escaped_username = ldap3::ldap_escape(username);
    let filter = settings
        .user_filter
        .replace("{username}", escaped_username.as_ref());

    let (entries, _result) = ldap
        .search(
            &settings.base_dn,
            Scope::Subtree,
            &filter,
            vec![
                settings.username_attr.as_str(),
                settings.name_attr.as_str(),
                settings.email_attr.as_str(),
                settings.memberof_attr.as_str(),
            ],
        )
        .await
        .map_err(|error| {
            tracing::error!("LDAP search error: {:?}", error);
            AppError::Unauthorized
        })?
        .success()
        .map_err(|error| {
            tracing::error!("LDAP search not successful: {:?}", error);
            AppError::Unauthorized
        })?;

    if entries.len() != 1 {
        let _ = ldap.unbind().await;
        return Err(AppError::Unauthorized);
    }

    let entry = SearchEntry::construct(entries.into_iter().next().unwrap());

    ldap.simple_bind(&entry.dn, password)
        .await
        .map_err(|error| {
            tracing::error!("LDAP user bind error: {:?}", error);
            AppError::Unauthorized
        })?
        .success()
        .map_err(|error| {
            tracing::error!("LDAP user bind not successful: {:?}", error);
            AppError::Unauthorized
        })?;

    let ldap_username = first_attr(&entry, &settings.username_attr)
        .unwrap_or_else(|| username.to_string());
    let ldap_name = first_attr(&entry, &settings.name_attr)
        .unwrap_or_else(|| ldap_username.clone());
    let ldap_email = first_attr(&entry, &settings.email_attr)
        .unwrap_or_else(|| format!("{}@ldap.local", ldap_username));
    let is_staff = has_group_membership(
        &entry,
        &settings.memberof_attr,
        settings.admin_group_dn.as_deref(),
    );

    let _ = ldap.unbind().await;

    Ok(Some(LdapIdentity {
        username: ldap_username,
        name: ldap_name,
        email: ldap_email,
        is_staff,
    }))
}

fn first_attr(entry: &SearchEntry, attr: &str) -> Option<String> {
    entry.attrs.get(attr).and_then(|values| values.first()).cloned()
}

fn has_group_membership(entry: &SearchEntry, memberof_attr: &str, admin_group_dn: Option<&str>) -> bool {
    let admin_group_dn = match admin_group_dn {
        Some(value) => value.trim().to_lowercase(),
        None => return false,
    };

    entry
        .attrs
        .get(memberof_attr)
        .map(|values| values.iter().any(|group| group.trim().to_lowercase() == admin_group_dn))
        .unwrap_or(false)
}
