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

Dockerfile includes FreeIPA CA installation at build time using `IPA_CA_URL`
build arg (default: `https://ipa.lab.students.cs.unibo.it/ipa/config/ca.crt`).

1. Create app and database service
   ```
   dokku apps:create verden2
   dokku postgres:create verden2-db
   dokku postgres:link verden2-db verden2
   ```

2. Create a storage where uploads will be located
   ```
   mkdir -p /var/lib/dokku/data/storage/verden2/uploads/
   dokku storage:mount verden2 /var/lib/dokku/data/storage/verden2/uploads:/storage/uploads
   ```

3. Set required config vars
   ```
   dokku config:set verden2 JWT_SECRET=foobar
   dokku config:set verden2 PAGE_LIMIT=20
   dokku config:set verden2 SAVE_FILE_BASE_PATH=/storage/uploads
   dokku config:set verden2 UPLOADS_ENDPOINT=/uploads
   dokku config:set verden2 RUST_LOG=verden=debug,tower_http=debug
   dokku config:set verden2 ALLOWED_HOST=0.0.0.0:9090
   dokku config:set verden2 SENTRY_DSN=https://example@example.ingest.sentry.io/42
   ```

4. Set LDAP vars (optional)
   ```
   dokku config:set verden2 LDAP_ENABLED=true
   dokku config:set verden2 LDAP_URL=ldaps://ipa.lab.students.cs.unibo.it:636
   dokku config:set verden2 LDAP_BASE_DN=cn=accounts,dc=lab,dc=students,dc=cs,dc=unibo,dc=it
   dokku config:set verden2 LDAP_BIND_DN='uid=radius,cn=users,cn=accounts,dc=lab,dc=students,dc=cs,dc=unibo,dc=it'
   dokku config:set verden2 LDAP_BIND_PASSWORD='changeme'
   dokku config:set verden2 LDAP_USER_FILTER='(uid={username})'
   dokku config:set verden2 LDAP_USERNAME_ATTR=uid
   dokku config:set verden2 LDAP_NAME_ATTR=cn
   dokku config:set verden2 LDAP_EMAIL_ATTR=mail
   dokku config:set verden2 LDAP_MEMBEROF_ATTR=memberOf
   dokku config:set verden2 LDAP_ADMIN_GROUP_DN='cn=verden-admins,cn=groups,cn=accounts,dc=lab,dc=students,dc=cs,dc=unibo,dc=it'
   ```

   Full command in one shot:
   ```
   dokku config:set verden2 LDAP_ENABLED=true LDAP_URL=ldaps://ipa.lab.students.cs.unibo.it:636 LDAP_BASE_DN=cn=accounts,dc=lab,dc=students,dc=cs,dc=unibo,dc=it LDAP_BIND_DN='uid=radius,cn=users,cn=accounts,dc=lab,dc=students,dc=cs,dc=unibo,dc=it' LDAP_BIND_PASSWORD='changeme' LDAP_USER_FILTER='(uid={username})' LDAP_USERNAME_ATTR=uid LDAP_NAME_ATTR=cn LDAP_EMAIL_ATTR=mail LDAP_MEMBEROF_ATTR=memberOf LDAP_ADMIN_GROUP_DN='cn=verden-admins,cn=groups,cn=accounts,dc=lab,dc=students,dc=cs,dc=unibo,dc=it'
   ```

   Disable LDAP and use local auth again:
   ```
   dokku config:set verden2 LDAP_ENABLED=false
   ```

   Check effective LDAP config:
   ```
   dokku config:get verden2 LDAP_ENABLED
   dokku config:get verden2 LDAP_URL
   dokku config:get verden2 LDAP_BASE_DN
   dokku config:get verden2 LDAP_BIND_DN
   ```

5. Set build arg for FreeIPA CA (optional override)
   ```
   dokku docker-options:add verden2 build '--build-arg IPA_CA_URL=https://ipa.lab.students.cs.unibo.it/ipa/config/ca.crt'
   ```

6. Configure domain and ports
   ```
   dokku domains:set verden2 api.a3dm.lab.students.cs.unibo.it
   dokku ports:clear verden2
   dokku ports:add verden2 http:9090:9090
   dokku ports:add verden2 https:443:9090
   ```

7. Add a remote and push this code
   ```
   git remote add dokku dokku@your_server:verden2
   git push dokku main
   ```

8. Run database migrations
   ```
   dokku run verden2 sqlx migrate run
   ```

9. Install [Let's Encrypt](https://github.com/dokku/dokku-letsencrypt)
   ```
   sudo dokku plugin:install https://github.com/dokku/dokku-letsencrypt.git
   dokku config:set --no-restart verden2 DOKKU_LETSENCRYPT_EMAIL=your_email
   dokku letsencrypt:enable verden2
   ```

10. Enjoy Verden at `https://api.a3dm.lab.students.cs.unibo.it`
