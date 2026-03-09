mod api;
mod media;

pub use api::create_routes;
pub(crate) use media::{serve_image, serve_thumbnail};
