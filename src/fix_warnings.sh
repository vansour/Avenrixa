#!/bin/bash
# 修复 src/domain/image/mod.rs - 移除未使用的 processor 导出
sed -i '/^pub use processor::ImageProcessor;$/d' src/domain/image/mod.rs
sed -i '/^pub use processor::FilterParams;$/d' src/domain/image/mod.rs
sed -i '/^pub use processor::WatermarkParams;$/d' src/domain/image/mod.rs

# 修复 src/domain/admin/mod.rs - 移除未使用的 AdminRepository 导出
sed -i '/^pub use repository::AdminRepository;$/d' src/domain/admin/mod.rs

# 修复 src/handlers/admin.rs - 移除未使用的导入
sed -i '/^use crate::domain::image::PostgresImageRepository;$/d' src/handlers/admin.rs
sed -i '/^use redis::AsyncCommands;$/d' src/handlers/admin.rs

# 修复 src/handlers/images.rs - 移除未使用的导入
sed -i 's/ImageService, ImageRepository, PostgresImageRepository, ImageProcessor, UploadRequest/ImageService, ImageRepository, PostgresImageRepository/' src/handlers/images.rs

# 修复 src/models.rs - 移除未使用的导入
sed -i '/^use uuid::Uuid;$/d' src/models.rs

# 修复 src/domain/admin/repository.rs - 移除未使用的 DateTime 导入
sed -i 's/use chrono::{DateTime, Utc};/use chrono::Utc;/' src/domain/admin/repository.rs
