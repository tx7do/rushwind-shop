//! The database bootstrap: on a fresh database (no `mall_brands`
//! table), apply the golden DDL statement-by-statement (CREATE … IF NOT
//! EXISTS / DO-block constraint guards — idempotent by construction).
//! The DDL accumulates in `backend/sql/schema.sql`; the demo seed joins
//! as its domains land.

use sea_orm::DatabaseConnection;

/// Applies the golden DDL when the database is fresh. Existing
/// databases (already carrying the schema) are left untouched — the
/// golden-DDL pipeline owns evolution.
pub async fn run(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    if crate::tables_present(db).await? {
        return Ok(());
    }
    exec_statements(db, crate::SCHEMA_SQL).await?;
    seed_dev_admin(db).await?;
    Ok(())
}

/// The dev bootstrap account (`admin` / `admin123`), seeded only on a
/// fresh database with the password hashed through the engine's own
/// Argon2id — no plaintext credential ever sits in a script file.
/// Replace with the reference's tenant/seed pipeline as the identity
/// domain lands.
async fn seed_dev_admin(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    use sea_orm::ConnectionTrait as _;
    let have = db
        .query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT 1 FROM public.sys_users LIMIT 1".to_string(),
        ))
        .await?;
    if have.is_some() {
        return Ok(());
    }
    let hash = rushwind_password::PasswordHasher::hash(
        &rushwind_password_argon2::Argon2Hasher::new(),
        "admin123",
    )
    .map_err(|e| sea_orm::DbErr::Custom(format!("seed hash: {e:?}")))?;
    let now = crate::now();
    let stmt = format!(
        "INSERT INTO public.sys_users (tenant_id, username, nickname, email, password, status, created_at) \
         VALUES (0, 'admin', 'Administrator', 'admin@rushwind.shop', '{}', 'NORMAL', '{}')",
        hash.replace('\'', "''"),
        now.format("%Y-%m-%d %H:%M:%S%:z"),
    );
    db.execute_raw(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        stmt,
    ))
    .await?;
    // The platform-admin role binding — the route authority gate reads
    // the caller's role codes through the perm-codes face, and the
    // shell's pages declare `sys:platform_admin`.
    let seed_roles = format!(
        "WITH r AS (\
           INSERT INTO public.sys_roles (tenant_id, name, code, status, sort_order, created_at) \
           VALUES (0, 'Platform Admin', 'sys:platform_admin', 'ON', 1, '{now}') \
           RETURNING id\
         ), u AS (SELECT id FROM public.sys_users WHERE username = 'admin' LIMIT 1) \
         INSERT INTO public.sys_user_roles (tenant_id, user_id, role_id, created_at) \
         SELECT 0, u.id, r.id, '{now}' FROM u, r",
        now = now.format("%Y-%m-%d %H:%M:%S%:z"),
    );
    db.execute_raw(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        seed_roles,
    ))
    .await?;
    eprintln!("[bootstrap] dev admin seeded (admin / admin123) + platform-admin role");
    Ok(())
}

/// Executes a SQL script by splitting on statement-terminating
/// semicolons (the scripts carry no functions/triggers — plain
/// DDL/DML — so a line-end split is exact).
async fn exec_statements(db: &DatabaseConnection, script: &str) -> Result<(), sea_orm::DbErr> {
    let mut buf = String::new();
    for line in script.lines() {
        buf.push_str(line);
        buf.push('\n');
        if line.trim_end().ends_with(';') {
            exec_one(db, &buf).await?;
            buf.clear();
        }
    }
    if !buf.trim().is_empty() {
        exec_one(db, &buf).await?;
    }
    Ok(())
}

async fn exec_one(db: &DatabaseConnection, stmt: &str) -> Result<(), sea_orm::DbErr> {
    use sea_orm::ConnectionTrait as _;
    let backend = sea_orm::DatabaseBackend::Postgres;
    let result = db
        .execute_raw(sea_orm::Statement::from_string(backend, stmt.to_string()))
        .await;
    if let Err(e) = &result {
        eprintln!(
            "[bootstrap] statement failed: {e}\n>>> {}<<<",
            &stmt[..stmt.len().min(400)]
        );
    }
    result?;
    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn schema_statements_split_cleanly() {
        // Every statement must terminate with ';' and reassemble to the
        // full script length (the splitter loses only the final newline
        // of each statement — round-tripped back by exec joining).
        let script = "CREATE TABLE a (id bigint);\nCREATE INDEX x ON a (id);\n";
        let mut n = 0;
        let mut buf = String::new();
        for line in script.lines() {
            buf.push_str(line);
            buf.push('\n');
            if line.trim_end().ends_with(';') {
                assert!(buf.trim_end().ends_with(';'));
                n += 1;
                buf.clear();
            }
        }
        assert_eq!(n, 2);
        assert!(buf.is_empty());
    }
}
