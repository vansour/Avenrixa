#![allow(unused_imports)]
#![allow(dead_code)]
//! 图片领域模块

pub mod repository;

// 重新导出现有类型
pub use crate::models::{Image, Paginated, CursorPaginated, PaginationParams};
pub use crate::models::{DeleteRequest, RestoreRequest, RenameRequest, SetExpiryRequest, ApproveRequest, DuplicateRequest, EditImageRequest, EditImageResponse};
pub use crate::models::{FilterParams, WatermarkParams};
pub use crate::image_processor::ImageProcessor;
pub use repository::{ImageRepository, CategoryRepository, PostgresImageRepository, PostgresCategoryRepository};
