use crate::{asset::Asset, resource_type::EResourceType};
use rs_render_types::MaterialOptions;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct TextureBinding {
    pub group: usize,
    pub binding: usize,
    pub texture_url: url::Url,
}

impl TextureBinding {
    pub fn get_texture_bind_name(&self) -> String {
        format!("_texture_{}_{}", self.group, self.binding)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct MaterialInfo {
    pub map_textures: HashSet<TextureBinding>,
    pub virtual_textures: HashSet<url::Url>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Material {
    pub url: url::Url,
    pub code: HashMap<MaterialOptions, String>,
    pub material_info: HashMap<MaterialOptions, MaterialInfo>,
}

impl Asset for Material {
    fn get_url(&self) -> url::Url {
        self.url.clone()
    }

    fn get_resource_type(&self) -> EResourceType {
        EResourceType::Material
    }
}
