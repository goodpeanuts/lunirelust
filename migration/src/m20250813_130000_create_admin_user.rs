use sea_orm_migration::prelude::*;

use crate::m20250807_073903_create_users_table::Users;
use crate::m20250807_073919_create_user_auth_table::UserAuth;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Insert admin user record with ON CONFLICT DO NOTHING
        let insert_user = Query::insert()
            .into_table(Users::Table)
            .columns([
                Users::Id,
                Users::Username,
                Users::Email,
                Users::CreatedBy,
                Users::CreatedAt,
                Users::ModifiedBy,
                Users::ModifiedAt,
            ])
            .values_panic([
                "00000000-0000-0000-0000-000000000000".into(),
                "admin".into(),
                "admin@luna.local".into(),
                Option::<String>::None.into(),
                Expr::current_timestamp().into(),
                Option::<String>::None.into(),
                Expr::current_timestamp().into(),
            ])
            .to_owned();

        // Execute with ON CONFLICT DO NOTHING
        let result = db
            .execute(db.get_database_backend().build(&insert_user))
            .await;
        match result {
            Ok(_) => {
                println!("Admin user created successfully");
            }
            Err(e) => {
                // If user already exists (unique constraint violation), just log and continue
                if e.to_string().contains("duplicate key")
                    || e.to_string().contains("UNIQUE constraint")
                {
                    println!("Admin user already exists, skipping creation");
                } else {
                    return Err(e);
                }
            }
        }

        // Insert admin user authentication record with ON CONFLICT DO NOTHING
        // Password: admin123 (using argon2id hash)
        let insert_auth = Query::insert()
            .into_table(UserAuth::Table)
            .columns([
                UserAuth::UserId,
                UserAuth::PasswordHash,
                UserAuth::CreatedAt,
                UserAuth::ModifiedAt,
            ])
            .values_panic([
                "00000000-0000-0000-0000-000000000000".into(),
                "$argon2i$v=19$m=16,t=2,p=1$ZnlsdXN0ang$SvneP1vCPivkdEe//CSlLg".into(),
                Expr::current_timestamp().into(),
                Expr::current_timestamp().into(),
            ])
            .to_owned();

        // Execute with error handling for duplicates
        let result = db
            .execute(db.get_database_backend().build(&insert_auth))
            .await;
        match result {
            Ok(_) => {
                println!("Admin user authentication created successfully");
            }
            Err(e) => {
                // If auth record already exists, just log and continue
                if e.to_string().contains("duplicate key")
                    || e.to_string().contains("UNIQUE constraint")
                {
                    println!("Admin user authentication already exists, skipping creation");
                } else {
                    return Err(e);
                }
            }
        }

        println!("管理员用户初始化完成");
        println!("用户名: admin");
        println!("密码: admin123");
        println!("邮箱: admin@luna.local");
        println!("请在首次登录后更改默认密码");

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Remove admin user authentication
        let delete_auth = Query::delete()
            .from_table(UserAuth::Table)
            .and_where(Expr::col(UserAuth::UserId).eq("00000000-0000-0000-0000-000000000000"))
            .to_owned();

        db.execute(db.get_database_backend().build(&delete_auth))
            .await?;

        // Remove admin user
        let delete_user = Query::delete()
            .from_table(Users::Table)
            .and_where(Expr::col(Users::Id).eq("00000000-0000-0000-0000-000000000000"))
            .to_owned();

        db.execute(db.get_database_backend().build(&delete_user))
            .await?;

        Ok(())
    }
}
