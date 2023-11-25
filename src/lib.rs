pub mod client;
pub mod server;

#[derive(PartialEq, Eq)]
pub enum UserPermissionLevel {
    None,
    Mod,
    Owner,
}

pub struct User {
    username: Option<String>,
    permission_level: UserPermissionLevel,
}
