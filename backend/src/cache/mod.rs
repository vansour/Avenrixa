mod hash;
mod image;
mod store;
#[cfg(test)]
mod tests;

pub use redis::AsyncCommands as CacheCommands;
pub use redis::RedisError as CacheBackendError;
pub use redis::aio::ConnectionLike as CacheConnectionLike;

pub type CacheConnection = redis::aio::ConnectionManager;

pub use hash::HashCache;
pub use image::ImageCache;
pub use store::Cache;
