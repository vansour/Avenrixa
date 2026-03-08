#![allow(unused_imports)]
//! 存储基础设施

pub mod file_queue;
pub mod image_processor;

// 导出基础设施层的图片处理器
pub use image_processor::{FilterParams, ImageProcessor, WatermarkParams};

// 导出文件队列
pub use file_queue::{FileSaveQueue, FileSaveResult, FileSaveTask};
