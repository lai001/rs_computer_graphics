use crate::{
    base_render_pipeline::{BaseRenderPipeline, ColorAttachment},
    base_render_pipeline_pool::{BaseRenderPipelineBuilder, BaseRenderPipelinePool},
    global_shaders::skeleton_shading::{SkeletonShadingShader, NUM_MAX_BONE},
    gpu_vertex_buffer::GpuVertexBufferImp,
    shader_library::ShaderLibrary,
    vertex_data_type::mesh_vertex::{MeshVertex0, MeshVertex1, MeshVertex2},
    view_mode::EViewModeType,
    VertexBufferType,
};
use std::sync::Arc;
use type_layout::TypeLayout;
use wgpu::*;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Constants {
    pub model: glam::Mat4,
    pub diffuse_texture_size: glam::Vec2,
    pub diffuse_texture_max_lod: u32,
    pub is_virtual_diffuse_texture: u32,
    pub specular_texture_size: glam::Vec2,
    pub specular_texture_max_lod: u32,
    pub is_virtual_specular_texture: u32,
    pub id: u32,
    _pad8_0: u32,
    _pad8_1: u32,
    _pad8_2: u32,
    pub bones: [glam::Mat4; 255],
}

impl Default for Constants {
    fn default() -> Self {
        Self {
            model: Default::default(),
            diffuse_texture_size: Default::default(),
            diffuse_texture_max_lod: Default::default(),
            is_virtual_diffuse_texture: Default::default(),
            specular_texture_size: Default::default(),
            specular_texture_max_lod: Default::default(),
            is_virtual_specular_texture: Default::default(),
            bones: [glam::Mat4::IDENTITY; NUM_MAX_BONE],
            id: Default::default(),
            _pad8_0: Default::default(),
            _pad8_1: Default::default(),
            _pad8_2: Default::default(),
        }
    }
}

pub struct SkinMeshShadingPipeline {
    base_render_pipeline: Arc<BaseRenderPipeline>,
    builder: BaseRenderPipelineBuilder,
}

impl SkinMeshShadingPipeline {
    pub fn new(
        device: &Device,
        shader_library: &ShaderLibrary,
        texture_format: &TextureFormat,
        pool: &mut BaseRenderPipelinePool,
    ) -> SkinMeshShadingPipeline {
        let builder = BaseRenderPipelineBuilder::default()
            .set_shader(&SkeletonShadingShader {})
            .set_targets(vec![Some(texture_format.clone().into())])
            .set_depth_stencil(Some(DepthStencilState {
                depth_compare: CompareFunction::Less,
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }))
            .set_vertex_buffer_type(Some(VertexBufferType::Interleaved(vec![
                MeshVertex0::type_layout(),
                MeshVertex1::type_layout(),
                MeshVertex2::type_layout(),
            ])))
            .set_primitive(Some(PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                ..Default::default()
            }));
        let base_render_pipeline =
            pool.get(device, shader_library, &SkeletonShadingShader {}, &builder);

        SkinMeshShadingPipeline {
            base_render_pipeline,
            builder,
        }
    }

    pub fn draw(
        &self,
        device: &Device,
        queue: &Queue,
        output_view: &TextureView,
        depth_view: &TextureView,
        mesh_buffers: &[GpuVertexBufferImp],
        binding_resource: Vec<Vec<BindingResource<'_>>>,
    ) {
        self.base_render_pipeline.draw_resources2(
            device,
            queue,
            binding_resource,
            mesh_buffers,
            &[ColorAttachment {
                color_ops: None,
                view: output_view,
                resolve_target: None,
            }],
            None,
            None,
            Some(depth_view),
        );
    }

    pub fn set_view_mode(
        &mut self,
        view_mode: EViewModeType,
        device: &Device,
        shader_library: &ShaderLibrary,
        pool: &mut BaseRenderPipelinePool,
    ) {
        match view_mode {
            EViewModeType::Wireframe => {
                self.builder = self.builder.clone().set_primitive(Some(PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    polygon_mode: PolygonMode::Line,
                    ..Default::default()
                }));
            }
            EViewModeType::Lit => {
                self.builder = self.builder.clone().set_primitive(Some(PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    polygon_mode: PolygonMode::Fill,
                    ..Default::default()
                }));
            }
            EViewModeType::Unlit => todo!(),
        }

        self.base_render_pipeline = pool.get(
            device,
            shader_library,
            &SkeletonShadingShader {},
            &self.builder,
        );
    }
}