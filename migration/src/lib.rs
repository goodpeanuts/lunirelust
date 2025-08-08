pub use sea_orm_migration::prelude::*;

mod m20250807_073903_create_users_table;
mod m20250807_073910_create_devices_table;
mod m20250807_073914_create_uploaded_files_table;
mod m20250807_073919_create_user_auth_table;
mod m20250807_074248_seed_initial_data;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250807_073903_create_users_table::Migration),
            Box::new(m20250807_073910_create_devices_table::Migration),
            Box::new(m20250807_073914_create_uploaded_files_table::Migration),
            Box::new(m20250807_073919_create_user_auth_table::Migration),
            Box::new(m20250807_074248_seed_initial_data::Migration),
        ]
    }
}
