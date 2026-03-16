# Other API Endpoints

## Authentication

### Login

```
POST /api/auth/login
Content-Type: application/json

{ "email": "user@example.com", "password": "secret" }
```

Returns a session cookie.

### Signup

```
POST /api/auth/signup
Content-Type: application/json

{ "email": "new@example.com", "password": "secret", "full_name": "New User" }
```

### Logout

```
POST /api/auth/logout
```

### Session Info

```
GET /api/session
```

```json
{ "user": "Administrator", "roles": ["Administrator", "All"] }
```

## DocType Meta

```
GET /api/doctype/{name}
```

Returns the full DocType definition (fields, permissions, naming rule) with customizations applied. Used by the frontend to render dynamic forms.

## Whitelisted Methods

```
POST /api/method/{app}.{method_name}
Content-Type: application/json

{ "param1": "value1" }
```

Executes `apps/{app}/api/{method_name}.rhai` and returns the result.

## Activity Timeline

```
GET /api/activity/{doctype}/{name}?limit=50
```

Returns the audit trail for a document (created, updated, submitted, cancelled, comments).

```
POST /api/activity/{doctype}/{name}/comment
Content-Type: application/json

{ "content": "Please review this" }
```

## User Settings

```
GET /api/settings/{key}
PUT /api/settings/{key}
```

Per-user key-value storage (saved views, workspace layout, sidebar preferences).

## Role Permissions

```
GET /api/role-permission/{doctype}          # Get default + override permissions
PUT /api/role-permission/{doctype}          # Save permission overrides
DELETE /api/role-permission/{doctype}       # Reset to defaults
GET /api/role-permission-by-role/{role}     # Get all permissions for a role
```

## Health Check

```
GET /api/health
```

Returns `ok`.
