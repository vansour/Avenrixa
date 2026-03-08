//! 图片处理模块
//!
//! 此模块现在作为基础设施层图片处理模块的重新导出点

// 重新导出基础设施层的图片处理类型
pub use crate::infrastructure::storage::{ImageProcessor, FilterParams, WatermarkParams};
