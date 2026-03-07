//! 文件保存队列模块
//!
//! 此模块现在作为基础设施层文件队列模块的重新导出点

// 重新导出基础设施层的文件队列类型
pub use crate::infrastructure::storage::{FileSaveQueue, FileSaveTask};
