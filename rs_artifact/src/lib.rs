pub mod artifact;
pub mod asset;
pub mod endian;
pub mod error;
pub mod file_header;
pub mod node_anim;
pub mod skin_mesh;
pub mod image;
#[cfg(target_os = "android")]
pub mod java_input_stream;
pub mod level;
pub mod mesh_vertex;
pub mod mesh_vertex_visitor;
pub mod property_value_type;
pub mod resource_info;
pub mod resource_type;
pub mod shader_source_code;
pub mod static_mesh;
pub mod type_expected;
pub mod virtual_texture;

pub use endian::EEndianType;

pub fn default_url() -> &'static url::Url {
    static URL: std::sync::OnceLock<url::Url> = std::sync::OnceLock::new();
    URL.get_or_init(|| url::Url::parse("rs://").unwrap())
}
