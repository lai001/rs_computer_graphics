use rs_foundation::id_generator::IDGenerator;
use std::{ops::Deref, sync::Arc};

pub struct HandleManager {
    texture_idgenerator: IDGenerator,
    virtual_texture_idgenerator: IDGenerator,
    buffer_idgenerator: IDGenerator,
    gui_texture_idgenerator: IDGenerator,
    sampler_idgenerator: IDGenerator,
    material_render_pipeline_idgenerator: IDGenerator,
}

impl HandleManager {
    pub fn new() -> HandleManager {
        HandleManager {
            texture_idgenerator: IDGenerator::new(),
            virtual_texture_idgenerator: IDGenerator::new(),
            buffer_idgenerator: IDGenerator::new(),
            gui_texture_idgenerator: IDGenerator::new(),
            sampler_idgenerator: IDGenerator::new(),
            material_render_pipeline_idgenerator: IDGenerator::new(),
        }
    }

    pub fn next_material_render_pipeline(&mut self) -> MaterialRenderPipelineHandle {
        let new_id = self.material_render_pipeline_idgenerator.get_next_id();
        MaterialRenderPipelineHandle {
            inner: Arc::new(new_id),
        }
    }

    pub fn next_texture(&mut self) -> TextureHandle {
        let new_id = self.texture_idgenerator.get_next_id();
        TextureHandle {
            inner: Arc::new(new_id),
        }
    }

    pub fn next_virtual_texture(&mut self) -> TextureHandle {
        let new_id = self.virtual_texture_idgenerator.get_next_id();
        TextureHandle {
            inner: Arc::new(new_id),
        }
    }

    pub fn next_ui_texture(&mut self) -> EGUITextureHandle {
        let new_id = self.gui_texture_idgenerator.get_next_id();
        EGUITextureHandle {
            inner: Arc::new(new_id),
        }
    }

    pub fn next_buffer(&mut self) -> BufferHandle {
        let new_id = self.buffer_idgenerator.get_next_id();
        BufferHandle {
            inner: Arc::new(new_id),
        }
    }

    pub fn next_sampler(&mut self) -> SamplerHandle {
        let new_id = self.sampler_idgenerator.get_next_id();
        SamplerHandle {
            inner: Arc::new(new_id),
        }
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub struct TextureHandle {
    inner: Arc<u64>,
}

impl Deref for TextureHandle {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub struct EGUITextureHandle {
    inner: Arc<u64>,
}

impl Deref for EGUITextureHandle {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub struct BufferHandle {
    inner: Arc<u64>,
}

impl BufferHandle {
    pub fn strong_cout(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    pub fn only_self(&self) -> bool {
        self.strong_cout() <= 1
    }
}

impl Deref for BufferHandle {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub struct SamplerHandle {
    inner: Arc<u64>,
}

impl Deref for SamplerHandle {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub struct MaterialRenderPipelineHandle {
    inner: Arc<u64>,
}

impl Deref for MaterialRenderPipelineHandle {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
