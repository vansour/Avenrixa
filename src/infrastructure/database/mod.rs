#![allow(unused_imports)]
//! 数据库基础设施

// 重新导出数据库操作
pub use crate::db::{AppState, init_schema, create_default_admin, log_admin_credentials};
