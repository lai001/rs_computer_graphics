use crate::thread_pool::ThreadPool;
use crate::{error::Result, handle::HandleManager};
use lazy_static::lazy_static;
use rs_artifact::{
    artifact::ArtifactReader, resource_type::EResourceType, shader_source_code::ShaderSourceCode,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

struct LoadResult {
    key: String,
    image: image::ImageResult<image::DynamicImage>,
}

struct STResourceManager {
    image_sync_cache: moka::sync::Cache<String, Arc<image::DynamicImage>>,
    artifact_reader: Option<ArtifactReader>,
    handle_manager: HandleManager,
}

impl STResourceManager {
    fn new() -> STResourceManager {
        STResourceManager {
            image_sync_cache: moka::sync::Cache::new(1000),
            artifact_reader: None,
            handle_manager: HandleManager::new(),
        }
    }

    fn get_shader_source_code(&mut self, url: &url::Url) -> Result<ShaderSourceCode> {
        if let Some(reader) = self.artifact_reader.as_mut() {
            let shader = reader.get_resource::<rs_artifact::shader_source_code::ShaderSourceCode>(
                url,
                Some(EResourceType::ShaderSourceCode),
            );
            match shader {
                Ok(shader) => Ok(shader),
                Err(err) => return Err(crate::error::Error::Artifact(err, None)),
            }
        } else {
            return Err(crate::error::Error::ArtifactReaderNotSet);
        }
    }

    fn get_all_shader_source_codes(&mut self) -> Vec<ShaderSourceCode> {
        let mut codes: Vec<ShaderSourceCode> = vec![];
        if let Some(reader) = self.artifact_reader.as_mut() {
            for (url, resource_info) in reader.get_artifact_file_header().resource_map.clone() {
                if resource_info.resource_type == EResourceType::ShaderSourceCode {
                    let shader = reader
                        .get_resource::<rs_artifact::shader_source_code::ShaderSourceCode>(
                            &url,
                            Some(EResourceType::ShaderSourceCode),
                        )
                        .expect("Never");
                    codes.push(shader);
                }
            }
        }
        return codes;
    }

    fn set_artifact_reader(&mut self, reader: Option<ArtifactReader>) {
        self.artifact_reader = reader;
    }

    fn cache_image(&self, key: &str, image: Arc<image::DynamicImage>) {
        log::trace!("Cache image, key: {key}");
        self.image_sync_cache.insert(key.to_string(), image);
    }

    fn get_cache_image(&self, key: &str) -> Option<Arc<image::DynamicImage>> {
        self.image_sync_cache.get(key)
    }

    fn get_cache_or_load_image(&self, key: &str, path: &str) -> Option<Arc<image::DynamicImage>> {
        if !self.image_sync_cache.contains_key(key) {
            self.load_image_from_disk_and_cache(key, path);
        }
        self.image_sync_cache.get(key)
    }

    fn load_image_from_disk_and_cache(&self, key: &str, path: &str) {
        let image = image::open(path);
        match image {
            Ok(image) => {
                self.cache_image(key, Arc::new(image));
            }
            Err(error) => {
                log::warn!("Load image failed, {}", error);
            }
        }
    }

    fn load_images_from_disk_and_cache_parallel(&self, dic: HashMap<&str, &str>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut count = dic.len();
        for (key, path) in dic {
            ThreadPool::global().spawn({
                let path = path.to_string();
                let key = key.to_string();
                let sender = sender.clone();
                move || {
                    let _ = sender.send(LoadResult {
                        key,
                        image: image::open(path),
                    });
                }
            });
        }
        while count > 0 {
            match receiver.recv() {
                Ok(result) => {
                    match result.image {
                        Ok(image) => self.cache_image(&result.key, Arc::new(image)),
                        Err(error) => log::warn!("{error}"),
                    }
                    count -= 1;
                }
                Err(error) => {
                    log::warn!("{}", error);
                }
            }
        }
    }

    fn next_texture(&mut self) -> crate::handle::TextureHandle {
        self.handle_manager.next_texture()
    }

    fn next_ui_texture(&mut self) -> crate::handle::EGUITextureHandle {
        self.handle_manager.next_ui_texture()
    }

    fn next_buffer(&mut self) -> crate::handle::BufferHandle {
        self.handle_manager.next_buffer()
    }
}

#[derive(Clone)]
pub struct ResourceManager {
    inner: Arc<Mutex<STResourceManager>>,
}

impl ResourceManager {
    pub fn new() -> ResourceManager {
        ResourceManager {
            inner: Arc::new(Mutex::new(STResourceManager::new())),
        }
    }

    pub fn default() -> ResourceManager {
        GLOBAL_RESOURCE_MANAGER.clone()
    }

    pub fn cache_image(&mut self, key: &str, image: Arc<image::DynamicImage>) {
        self.inner.lock().unwrap().cache_image(key, image);
    }

    pub fn get_cache_image(&self, key: &str) -> Option<Arc<image::DynamicImage>> {
        self.inner.lock().unwrap().get_cache_image(key)
    }

    pub fn get_cache_or_load_image(
        &self,
        key: &str,
        path: &str,
    ) -> Option<Arc<image::DynamicImage>> {
        self.inner
            .lock()
            .unwrap()
            .get_cache_or_load_image(key, path)
    }

    pub fn load_image_from_disk_and_cache(&self, key: &str, path: &str) {
        self.inner
            .lock()
            .unwrap()
            .load_image_from_disk_and_cache(key, path);
    }

    pub fn load_images_from_disk_and_cache_parallel(&self, dic: HashMap<&str, &str>) {
        self.inner
            .lock()
            .unwrap()
            .load_images_from_disk_and_cache_parallel(dic);
    }

    pub fn set_artifact_reader(&mut self, reader: Option<ArtifactReader>) {
        self.inner.lock().unwrap().set_artifact_reader(reader);
    }

    pub fn get_shader_source_code(&mut self, url: &url::Url) -> Result<ShaderSourceCode> {
        self.inner.lock().unwrap().get_shader_source_code(url)
    }

    pub fn get_all_shader_source_codes(&mut self) -> Vec<ShaderSourceCode> {
        self.inner.lock().unwrap().get_all_shader_source_codes()
    }

    pub fn next_texture(&mut self) -> crate::handle::TextureHandle {
        self.inner.lock().unwrap().next_texture()
    }

    pub fn next_ui_texture(&mut self) -> crate::handle::EGUITextureHandle {
        self.inner.lock().unwrap().next_ui_texture()
    }

    pub fn next_buffer(&mut self) -> crate::handle::BufferHandle {
        self.inner.lock().unwrap().next_buffer()
    }
}

lazy_static! {
    static ref GLOBAL_RESOURCE_MANAGER: ResourceManager = ResourceManager::new();
}
