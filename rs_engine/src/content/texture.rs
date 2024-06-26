use crate::{build_asset_url, url_extension::UrlExtension, ASSET_SCHEME};
#[cfg(feature = "editor")]
use anyhow::{anyhow, Context, Result};
use rs_artifact::{asset::Asset, resource_type::EResourceType};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextureFile {
    pub url: url::Url,
    pub image_reference: Option<url::Url>,
    pub is_virtual_texture: bool,
    pub virtual_image_reference: Option<String>,
}

impl Asset for TextureFile {
    fn get_url(&self) -> url::Url {
        self.url.clone()
    }

    fn get_resource_type(&self) -> EResourceType {
        EResourceType::Content(rs_artifact::content_type::EContentType::Texture)
    }
}

impl TextureFile {
    pub fn new(url: url::Url) -> Self {
        Self {
            url,
            image_reference: None,
            is_virtual_texture: false,
            virtual_image_reference: None,
        }
    }

    pub fn get_image_reference_path(&self) -> Option<PathBuf> {
        self.image_reference.clone().map(|x| {
            PathBuf::from_str(
                x.to_string()
                    .strip_prefix(&format!("{}://", ASSET_SCHEME))
                    .unwrap(),
            )
            .unwrap()
        })
    }

    pub fn set_image_reference_path(&mut self, image_reference_path: impl AsRef<Path>) {
        let path = image_reference_path.as_ref();
        self.image_reference = build_asset_url(path.to_str().unwrap()).ok();
    }

    pub fn get_name(&self) -> String {
        self.url.get_name_in_editor()
    }

    #[cfg(feature = "editor")]
    pub fn is_virtual_image_cache_vaild<P: AsRef<Path>>(
        &self,
        virtual_cache_dir: P,
        endian_type: Option<rs_artifact::EEndianType>,
    ) -> anyhow::Result<()> {
        if !self.is_virtual_texture {
            return Err(anyhow!("Is not a virtual texture"));
        }
        let virtual_image_reference = &self
            .virtual_image_reference
            .clone()
            .ok_or(anyhow!("Property virtual_image_reference is not set."))?;
        let abs_path = virtual_cache_dir.as_ref().join(virtual_image_reference);
        if !abs_path.exists() || !abs_path.is_file() {
            return Err(anyhow!(
                "{:?} is not exists or not a file.",
                virtual_image_reference
            ));
        }
        let decode_result = rs_artifact::virtual_texture::image::decode_from_path(
            virtual_image_reference,
            endian_type,
        );
        Ok(decode_result.map(|_| ())?)
    }

    #[cfg(feature = "editor")]
    pub fn create_virtual_texture_cache<P: AsRef<Path>>(
        &mut self,
        asset_folder: P,
        output: P,
        endian_type: Option<rs_artifact::EEndianType>,
        tile_size: u32,
    ) -> anyhow::Result<()> {
        // self.is_virtual_image_cache_vaild(endian_type)?;
        let image_reference = self
            .get_image_reference_path()
            .clone()
            .ok_or(anyhow!("image_reference is null."))?;
        let create_result = create_virtual_texture_cache_file(
            asset_folder.as_ref().join(image_reference),
            output.as_ref().to_path_buf(),
            endian_type,
            tile_size,
        );

        // if create_result.is_ok() {
        //     self.virtual_image_reference = Some(output.as_ref().to_path_buf());
        // }
        create_result
    }

    #[cfg(feature = "editor")]
    pub fn get_pref_virtual_cache_name<P: AsRef<Path>>(&self, asset_folder: P) -> Result<String> {
        let Some(image_reference) = &self.get_image_reference_path() else {
            return Err(anyhow!("image_reference is null."));
        };
        let mut hasher = <md5::Md5 as md5::Digest>::new();
        let data = std::fs::read(asset_folder.as_ref().join(image_reference))
            .context(format!("Failed to read from {:?}", image_reference))?;
        md5::Digest::update(&mut hasher, data);
        let result = md5::Digest::finalize(hasher);
        let result = result.to_ascii_lowercase();
        let result = result
            .iter()
            .fold("".to_string(), |acc, x| format!("{acc}{:x?}", x));
        Ok(result)
    }

    #[cfg(all(debug_assertions, feature = "editor"))]
    fn _decode_virtual_texture_to_dir<P: AsRef<Path>>(
        path: P,
        out_dir: P,
        endian_type: Option<rs_artifact::EEndianType>,
    ) {
        let mut image_file =
            rs_artifact::virtual_texture::image::decode_from_path(path, endian_type).unwrap();
        let tile_map = image_file.get_tile_map().to_vec();
        for (level, image_infos) in tile_map.iter().enumerate() {
            for (index, _) in image_infos.iter() {
                let image = image_file
                    .get_dynamic_image(&rs_artifact::virtual_texture::image::TileIndex {
                        x: index.x,
                        y: index.y,
                        mipmap_level: level as u32,
                    })
                    .unwrap();
                let out_dir = out_dir.as_ref();
                let _ = std::fs::create_dir(out_dir);
                let _ = image.save_with_format(
                    out_dir.join(format!("{}_{}_{}.png", level, index.x, index.y)),
                    image::ImageFormat::Png,
                );
            }
        }
    }
}

#[cfg(feature = "editor")]
pub fn create_virtual_texture_cache_file<P: AsRef<Path>>(
    file_path: P,
    output: P,
    endian_type: Option<rs_artifact::EEndianType>,
    tile_size: u32,
) -> anyhow::Result<()> {
    assert!(tile_size.is_power_of_two());
    let image = image::open(file_path.as_ref())
        .context(format!("Can not open file {:?}", file_path.as_ref()))?;

    if image.width() % tile_size != 0
        || image.height() % tile_size != 0
        || image.width().min(image.height()) < tile_size
    {
        return Err(anyhow!("Size is not correct."));
    }
    let mut tiles: Vec<std::collections::HashMap<glam::UVec2, image::DynamicImage>> = Vec::new();

    let mut lod_images =
        crate::mipmap_generator::MipmapGenerator::generate_from_image_cpu(image, None, None);
    let mut lod_sizes: Vec<glam::UVec2> = Vec::new();

    for image in lod_images.iter_mut() {
        lod_sizes.push(glam::uvec2(image.width(), image.height()));
        let mut images: std::collections::HashMap<glam::UVec2, image::DynamicImage> =
            std::collections::HashMap::new();
        for x in 0..image.width() / tile_size {
            for y in 0..image.height() / tile_size {
                let sub_image = image::DynamicImage::ImageRgba8(
                    image::GenericImage::sub_image(
                        image,
                        x * tile_size,
                        y * tile_size,
                        tile_size,
                        tile_size,
                    )
                    .to_image(),
                );
                images.insert(glam::uvec2(x, y), sub_image);
            }
        }
        if !images.is_empty() {
            tiles.push(images);
        }
    }

    Ok(rs_artifact::virtual_texture::image::encode_to_file(
        output,
        endian_type,
        lod_sizes,
        tiles,
    )?)
}
