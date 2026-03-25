<div align="center">

# Verden 🌐🎨

</div>

This software is part of a project for the Web Programming course at UNICT.

---

Configuration to set up before starting:

1. Create a PostgreSQL database.
2. Generate a good secret string for JWT.
3. [ Optional ] Create a Sentry project

Variables with values as example:

```
JWT_SECRET=foobar
DATABASE_URL=postgres://user:password@localhost:5432/verden
PAGE_LIMIT=20
SAVE_FILE_BASE_PATH="./uploads"
UPLOADS_ENDPOINT="/uploads"
RUST_LOG=verden=debug,tower_http=debug
ALLOWED_HOST=localhost:3000
SENTRY_DSN=.... # Optional

# LDAP (optional)
LDAP_ENABLED=false
LDAP_URL=ldaps://ldap.example.com:636
LDAP_BASE_DN=ou=people,dc=example,dc=com
LDAP_BIND_DN=cn=readonly,dc=example,dc=com
LDAP_BIND_PASSWORD=changeme
LDAP_USER_FILTER=(uid={username})
LDAP_USERNAME_ATTR=uid
LDAP_NAME_ATTR=cn
LDAP_EMAIL_ATTR=mail
LDAP_MEMBEROF_ATTR=memberOf
LDAP_ADMIN_GROUP_DN=cn=verden-admins,ou=groups,dc=example,dc=com
```

When LDAP_ENABLED=true and LDAP variables are configured, `/v1/auth/login` authenticates against LDAP and
creates a local user row on first login. In this mode `/v1/auth/signup` is disabled.
LDAP credentials are never stored in the local database.
When `LDAP_ADMIN_GROUP_DN` is configured, users in that LDAP group are mapped as admin (`is_staff=true`).
Role mapping is synchronized at each LDAP login.

# Deploy

This is a guide for a good deploy on a [Dokku](https://dokku.me) server, which
deploys Verden on port 9090.

Dockerfile defines a `DATABASE_URL` argument by default cause `sqlx` dependence
but, if you define the environment variabile, it will use the one you defined on
`dokku config`.

1. Log into the server and create a new app
   ```
   dokku apps:create verden-api
   ```
2. Create the database and link it to the app. `DATABASE_URL` automatically set
   ```
   dokku postgres:create verden-api # Database has got the same app name
   dokku postgres:link verden-api verden-api
   ```
3. Create a storage where uploads will be located
   ```
   mkdir -p /var/lib/dokku/data/storage/verden-api/uploads/
   dokku storage:mount verden-api /var/lib/dokku/data/storage/verden-api/uploads:/storage/uploads
   ```
4. Set config vars
   ```
   dokku config:set verden-api JWT_SECRET=foobar
   dokku config:set verden-api PAGE_LIMIT=20
   dokku config:set verden-api SAVE_FILE_BASE_PATH=/storage/uploads
   dokku config:set verden-api UPLOADS_ENDPOINT=/uploads
   dokku config:set verden-api RUST_LOG=verden=debug,tower_http=debug
   dokku config:set verdena-pi ALLOWED_HOST=0.0.0.0:9090
   dokku config:set verdena-pi SENTRY_DSN=https://example@example.ingest.sentry.io/42
   ```
5. Fix ports for HTTP
   ```
   dokku proxy:ports-add verden-api http:80:9090
   dokku proxy:ports-remove verden-api http:9090:9090
   ```
6. Add a remote and push this code
   ```
   git remote add dokku dokku_user@your_server:verden-api
   git push dokku main
   ```
7. Install [Let's Encrypt](https://github.com/dokku/dokku-letsencrypt)
   ```
   sudo dokku plugin:install https://github.com/dokku/dokku-letsencrypt.git
   dokku config:set --no-restart verden-api DOKKU_LETSENCRYPT_EMAIL=your_email
   dokku letsencrypt:enable verden-api
   ```
8. Log in the app and run migrate
   ```
   dokku enter verden-api
   sqlx migrate run
   ```
9. Enjoy Verden at `https://verden-api.<your-server>`
