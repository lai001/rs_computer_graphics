use crate::{file_manager::get_engine_root_dir, frustum::Frustum};
use glam::Vec4Swizzles;

pub fn calculate_max_mips(length: u32) -> u32 {
    32 - length.leading_zeros()
    // let mut mipmap_level: u32 = 1;
    // let mut length = length;
    // while length > 4 {
    //     length /= 2;
    //     mipmap_level += 1;
    // }
    // return mipmap_level;
}

pub fn calculate_mipmap_level_sizes(length: u32) -> Vec<u32> {
    let mut sizes = Vec::new();
    let mut length = length;
    while length > 0 {
        sizes.push(length);
        length /= 2;
    }
    sizes
}

pub fn get_mip_level_size(length: u32, level: u32) -> u32 {
    u32::max(1, length >> level)
}

#[cfg(feature = "editor")]
pub fn is_run_from_ide() -> bool {
    let vars = std::env::vars().filter(|x| x.0 == "VSCODE_HANDLES_UNCAUGHT_ERRORS".to_string());
    vars.count() != 0
}

#[cfg(feature = "editor")]
pub fn is_dev_mode() -> bool {
    let is_cargo_exist = get_engine_root_dir().join(".cargo").exists();
    let is_xmake_exist = get_engine_root_dir().join(".xmake").exists();
    let is_vscode_exist = get_engine_root_dir().join(".vscode").exists();
    is_run_from_ide() || is_cargo_exist || is_xmake_exist || is_vscode_exist
}

pub fn get_md5_from_string(text: &str) -> String {
    let mut hasher = <md5::Md5 as md5::Digest>::new();
    md5::digest::Update::update(&mut hasher, text.as_bytes());
    let result = md5::Digest::finalize(hasher);
    let result = result.to_ascii_lowercase();
    let result = result
        .iter()
        .fold("".to_string(), |acc, x| format!("{acc}{:x?}", x));
    result
}

fn transform_coordinates(p: glam::Vec3, m: glam::Mat4) -> glam::Vec3 {
    let p = glam::vec4(p.x, p.y, p.z, 1.0);
    (m * p).xyz()
}

pub fn get_orthographic_frustum(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
) -> Frustum {
    let projection = glam::Mat4::orthographic_rh(left, right, bottom, top, near, far);
    let inv_projection = projection.inverse();

    let min = glam::vec3(left, bottom, near);
    let max = glam::vec3(right, top, far);
    let n_0 = glam::vec3(max.x, max.y, min.z);
    let n_1 = glam::vec3(max.x, min.y, min.z);
    let n_2 = glam::vec3(min.x, min.y, min.z);
    let n_3 = glam::vec3(min.x, max.y, min.z);

    let near_0 = transform_coordinates(n_0, inv_projection);
    let near_1 = transform_coordinates(n_1, inv_projection);
    let near_2 = transform_coordinates(n_2, inv_projection);
    let near_3 = transform_coordinates(n_3, inv_projection);

    let f_0 = glam::vec3(max.x, max.y, max.z);
    let f_1 = glam::vec3(max.x, min.y, max.z);
    let f_2 = glam::vec3(min.x, min.y, max.z);
    let f_3 = glam::vec3(min.x, max.y, max.z);

    let far_0 = transform_coordinates(f_0, inv_projection);
    let far_1 = transform_coordinates(f_1, inv_projection);
    let far_2 = transform_coordinates(f_2, inv_projection);
    let far_3 = transform_coordinates(f_3, inv_projection);

    Frustum {
        near_0,
        near_1,
        near_2,
        near_3,
        far_0,
        far_1,
        far_2,
        far_3,
    }
}
