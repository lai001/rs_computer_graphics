use std::sync::Arc;

use wgpu::{TextureFormat, TextureViewDimension};

use crate::default_textures::DefaultTextures;

pub struct Material {
    diffuse_texture: Arc<Option<wgpu::Texture>>,
    specular_texture: Arc<Option<wgpu::Texture>>,
    physical_texture: Arc<Option<wgpu::Texture>>,
    page_table_texture: Arc<Option<wgpu::Texture>>,
}

impl Material {
    pub fn default() -> Material {
        Material {
            diffuse_texture: Arc::new(None),
            specular_texture: Arc::new(None),
            physical_texture: Arc::new(None),
            page_table_texture: Arc::new(None),
        }
    }

    pub fn new(
        diffuse_texture: Arc<Option<wgpu::Texture>>,
        specular_texture: Arc<Option<wgpu::Texture>>,
    ) -> Material {
        Material {
            diffuse_texture,
            specular_texture,
            physical_texture: Arc::new(None),
            page_table_texture: Arc::new(None),
        }
    }

    pub fn get_diffuse_texture(&self) -> Arc<Option<wgpu::Texture>> {
        self.diffuse_texture.clone()
    }

    pub fn set_diffuse_texture(&mut self, diffuse_texture: Arc<Option<wgpu::Texture>>) {
        self.diffuse_texture = diffuse_texture;
    }

    pub fn get_diffuse_texture_view(&self) -> wgpu::TextureView {
        match self.diffuse_texture.clone().as_ref() {
            Some(texture) => texture.create_view(&wgpu::TextureViewDescriptor::default()),
            None => DefaultTextures::default()
                .lock()
                .unwrap()
                .get_black_texture_view(),
        }
    }

    pub fn get_specular_texture(&self) -> Arc<Option<wgpu::Texture>> {
        self.specular_texture.clone()
    }

    pub fn set_specular_texture(&mut self, specular_texture: Arc<Option<wgpu::Texture>>) {
        self.specular_texture = specular_texture;
    }

    pub fn get_specular_texture_view(&self) -> wgpu::TextureView {
        match self.specular_texture.clone().as_ref() {
            Some(texture) => texture.create_view(&wgpu::TextureViewDescriptor::default()),
            None => DefaultTextures::default()
                .lock()
                .unwrap()
                .get_black_texture_view(),
        }
    }

    pub fn get_physical_texture(&self) -> Arc<Option<wgpu::Texture>> {
        self.physical_texture.clone()
    }

    pub fn set_physical_texture(&mut self, physical_texture: Arc<Option<wgpu::Texture>>) {
        self.physical_texture = physical_texture;
    }

    pub fn get_physical_texture_view(&self) -> wgpu::TextureView {
        match self.physical_texture.clone().as_ref() {
            Some(texture) => texture.create_view(&wgpu::TextureViewDescriptor::default()),
            None => DefaultTextures::default()
                .lock()
                .unwrap()
                .get_black_texture_view(),
        }
    }

    pub fn get_page_table_texture(&self) -> Arc<Option<wgpu::Texture>> {
        self.page_table_texture.clone()
    }

    pub fn set_page_table_texture(&mut self, page_table_texture: Arc<Option<wgpu::Texture>>) {
        self.page_table_texture = page_table_texture;
    }

    pub fn get_page_table_texture_view(&self) -> wgpu::TextureView {
        let texture_view_descriptor = wgpu::TextureViewDescriptor {
            label: Some("page table"),
            format: Some(TextureFormat::Rgba8Uint),
            dimension: Some(TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(1),
        };
        match self.page_table_texture.clone().as_ref() {
            Some(texture) => texture.create_view(&texture_view_descriptor),
            None => {
                DefaultTextures::default()
                    .lock()
                    .unwrap()
                    .get_black_texture_view()
                // panic!()
            }
        }
    }
}
