pub mod actor;
pub mod camera;
#[cfg(not(target_os = "android"))]
pub mod camera_input_event_handle;
pub mod console_cmd;
pub mod content;
pub mod drawable;
pub mod engine;
pub mod error;
pub mod file_type;
pub mod frame_sync;
pub mod handle;
pub mod input_mode;
pub mod logger;
pub mod mesh_buffer;
pub mod mipmap_generator;
pub mod plugin;
pub mod plugin_context;
pub mod property;
pub mod render_thread_mode;
pub mod resource_manager;
pub mod rotator;
pub mod scene_node;
pub mod static_virtual_texture_source;
pub mod sync;
pub mod thread_pool;
pub mod url_extension;

pub const ASSET_SCHEME: &str = "asset";
pub const CONTENT_SCHEME: &str = "content";

pub fn build_asset_url(name: impl AsRef<str>) -> Result<url::Url, url::ParseError> {
    url::Url::parse(&format!("{}://asset/{}", ASSET_SCHEME, name.as_ref()))
}

pub fn build_content_file_url(name: impl AsRef<str>) -> Result<url::Url, url::ParseError> {
    url::Url::parse(&format!("{}://Content/{}", CONTENT_SCHEME, name.as_ref()))
}
