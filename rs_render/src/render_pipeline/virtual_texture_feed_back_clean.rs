use crate::{
    base_render_pipeline::{BaseRenderPipeline, ColorAttachment},
    base_render_pipeline_pool::BaseRenderPipelineBuilder,
    global_shaders::{
        global_shader::GlobalShader, virtual_texture_clean::VirtualTextureCleanShader,
    },
    gpu_vertex_buffer::{Draw, EDrawCallType, GpuVertexBufferImp},
    shader_library::ShaderLibrary,
};
use wgpu::*;

pub struct VirtualTextureFeedBackClearPipeline {
    base_render_pipeline: BaseRenderPipeline,
}

impl VirtualTextureFeedBackClearPipeline {
    pub fn new(
        device: &Device,
        shader_library: &ShaderLibrary,
        texture_format: &TextureFormat,
    ) -> VirtualTextureFeedBackClearPipeline {
        let mut builder = BaseRenderPipelineBuilder::default();
        builder.targets = vec![Some(texture_format.clone().into())];
        builder.shader_name = VirtualTextureCleanShader {}.get_name();
        builder.depth_stencil = Some(DepthStencilState {
            depth_compare: CompareFunction::Always,
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        });
        builder.primitive = Some(PrimitiveState {
            cull_mode: None,
            ..Default::default()
        });
        let base_render_pipeline = BaseRenderPipeline::new(device, shader_library, builder);

        VirtualTextureFeedBackClearPipeline {
            base_render_pipeline,
        }
    }

    pub fn draw(
        &self,
        device: &Device,
        queue: &Queue,
        output_view: &TextureView,
        depth_view: &TextureView,
    ) {
        self.base_render_pipeline.draw_resources(
            device,
            queue,
            vec![],
            &vec![GpuVertexBufferImp {
                vertex_buffers: &vec![],
                vertex_count: 6,
                index_buffer: None,
                index_count: None,
                draw_type: EDrawCallType::Draw(Draw { instances: 0..1 }),
            }],
            &[ColorAttachment {
                color_ops: Some(Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                }),
                view: output_view,
                resolve_target: None,
            }],
            Some(Operations {
                load: LoadOp::Clear(1.0),
                store: StoreOp::Store,
            }),
            Some(Operations {
                load: LoadOp::Clear(0),
                store: StoreOp::Store,
            }),
            Some(depth_view),
            None,
            None,
        );
    }
}
