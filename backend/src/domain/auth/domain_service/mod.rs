//! 认证领域服务
//!
//! 小封装用户登录、修改密码等业务逻辑

pub use common::AuthDomainService;
pub use common::PasswordResetDispatch;

mod common;
mod login;
mod password_reset;
mod profile;
#[cfg(test)]
mod tests;
