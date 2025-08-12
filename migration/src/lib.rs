pub use sea_orm_migration::prelude::*;

mod m20250807_073903_create_users_table;
mod m20250807_073910_create_devices_table;
mod m20250807_073914_create_uploaded_files_table;
mod m20250807_073919_create_user_auth_table;
mod m20250807_074248_seed_initial_data;
mod m20250809_120000_create_basic_tables;
mod m20250809_121000_create_record_table;
mod m20250809_122000_create_junction_tables;
mod m20250809_123000_create_links_table;
mod m20250809_124000_seed_default_data;
mod m20250809_125000_create_tag_tables;
mod m20250813_130000_create_admin_user;

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
            Box::new(m20250809_120000_create_basic_tables::Migration),
            Box::new(m20250809_121000_create_record_table::Migration),
            Box::new(m20250809_122000_create_junction_tables::Migration),
            Box::new(m20250809_123000_create_links_table::Migration),
            Box::new(m20250809_125000_create_tag_tables::Migration),
            Box::new(m20250809_124000_seed_default_data::Migration),
            Box::new(m20250813_130000_create_admin_user::Migration),
        ]
    }
}
