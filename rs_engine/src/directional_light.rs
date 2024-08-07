use crate::drawable::{CustomDrawObject, EDrawObjectType};
use crate::engine::Engine;
use rs_core_minimal::frustum::Frustum;
use rs_core_minimal::misc::get_orthographic_frustum;
use rs_render::command::{DrawObject, EBindingResource};
use rs_render::constants;
use rs_render::renderer::{EBuiltinPipelineType, EPipelineType};
use rs_render::vertex_data_type::mesh_vertex::MeshVertex3;
use serde::{Deserialize, Serialize};

pub struct Runtime {
    draw_object: EDrawObjectType,
    constants_handle: crate::handle::BufferHandle,
    constants: constants::Constants,
}

#[derive(Serialize, Deserialize)]
pub struct DirectionalLight {
    eye: glam::Vec3,
    light_projection: glam::Mat4,
    light_view: glam::Mat4,
    transformation: glam::Mat4,
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
    #[serde(skip)]
    runtime: Option<Runtime>,
}

impl DirectionalLight {
    pub fn get_interactive_transformation(&mut self) -> &mut glam::Mat4 {
        &mut self.transformation
    }

    pub fn new(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> DirectionalLight {
        let light_projection = glam::Mat4::orthographic_rh(left, right, bottom, top, near, far);
        let up = glam::Vec3::Y;
        let dir = glam::Vec3::NEG_Z;
        let eye = glam::Vec3::ZERO;
        let light_view = glam::Mat4::look_to_rh(eye, dir, up);
        let transformation = glam::Mat4::IDENTITY;
        DirectionalLight {
            light_projection,
            left,
            right,
            bottom,
            top,
            near,
            far,
            light_view,
            eye,
            transformation,
            runtime: None,
        }
    }

    pub fn initialize(&mut self, engine: &mut Engine) {
        let frustum = get_orthographic_frustum(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
        );

        let (draw_object, constants_handle) = Self::make_draw_object(engine, &frustum);
        let runtime = Runtime {
            draw_object: EDrawObjectType::Custom(CustomDrawObject {
                draw_object,
                window_id: engine.get_main_window_id(),
            }),
            constants_handle,
            constants: constants::Constants::default(),
        };
        self.runtime = Some(runtime);
    }

    fn make_draw_object(
        engine: &mut Engine,
        frustum: &Frustum,
    ) -> (DrawObject, crate::handle::BufferHandle) {
        let lines = frustum.make_lines();
        let mut v1 = lines[0..4]
            .iter()
            .flat_map(|x| {
                vec![
                    MeshVertex3 {
                        position: x.p_0,
                        vertex_color: glam::vec4(0.0, 1.0, 0.0, 1.0),
                    },
                    MeshVertex3 {
                        position: x.p_1,
                        vertex_color: glam::vec4(0.0, 1.0, 0.0, 1.0),
                    },
                ]
            })
            .collect::<Vec<MeshVertex3>>();
        let mut v2 = lines[4..8]
            .iter()
            .flat_map(|x| {
                vec![
                    MeshVertex3 {
                        position: x.p_0,
                        vertex_color: glam::vec4(1.0, 0.0, 0.0, 1.0),
                    },
                    MeshVertex3 {
                        position: x.p_1,
                        vertex_color: glam::vec4(1.0, 0.0, 0.0, 1.0),
                    },
                ]
            })
            .collect::<Vec<MeshVertex3>>();

        let mut v3 = lines[8..]
            .iter()
            .flat_map(|x| {
                vec![
                    MeshVertex3 {
                        position: x.p_0,
                        vertex_color: glam::vec4(0.0, 1.0, 0.0, 1.0),
                    },
                    MeshVertex3 {
                        position: x.p_1,
                        vertex_color: glam::vec4(1.0, 0.0, 0.0, 1.0),
                    },
                ]
            })
            .collect::<Vec<MeshVertex3>>();

        let mut vertex: Vec<MeshVertex3> = vec![];
        vertex.append(&mut v1);
        vertex.append(&mut v2);
        vertex.append(&mut v3);

        let vertex_count = vertex.len();
        let vertex_buffer_handle =
            engine.create_vertex_buffer(&vertex, Some(format!("rs.VertexBuffer")));
        let constants_handle = engine.create_constants_buffer(
            &vec![constants::Constants::default()],
            Some(format!("rs.Constants")),
        );
        (
            DrawObject::new(
                0,
                vec![*vertex_buffer_handle],
                vertex_count as u32,
                EPipelineType::Builtin(EBuiltinPipelineType::Primitive),
                None,
                None,
                vec![
                    vec![EBindingResource::Constants(
                        *engine.get_global_constants_handle(),
                    )],
                    vec![EBindingResource::Constants(*constants_handle)],
                ],
            ),
            constants_handle,
        )
    }

    pub fn get_draw_objects(&self) -> Vec<&crate::drawable::EDrawObjectType> {
        self.runtime
            .as_ref()
            .map(|x| vec![&x.draw_object])
            .unwrap_or(vec![])
    }

    pub fn update_clip(&mut self, near: f32, far: f32, engine: &mut Engine) {
        self.near = near;
        self.far = far;
        self.update(engine);
    }

    pub fn update_view_rect(
        &mut self,
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        engine: &mut Engine,
    ) {
        self.left = left;
        self.right = right;
        self.bottom = bottom;
        self.top = top;
        self.update(engine);
    }

    pub fn update(&mut self, engine: &mut Engine) {
        self.light_projection = glam::Mat4::orthographic_rh(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
        );

        if let Some(runtime) = self.runtime.as_mut() {
            runtime.constants.model = self.transformation;
            engine.update_buffer(
                runtime.constants_handle.clone(),
                rs_foundation::cast_any_as_u8_slice(&runtime.constants),
            );
        }
    }

    pub fn get_light_projection(&self) -> &glam::Mat4 {
        &self.light_projection
    }

    pub fn get_light_view(&mut self) -> &mut glam::Mat4 {
        &mut self.light_view
    }

    pub fn get_light_space_matrix(&mut self) -> glam::Mat4 {
        let up = glam::Vec3::Y;
        let dir = glam::Vec3::NEG_Z;
        self.light_view = self.transformation.inverse() * glam::Mat4::look_to_rh(self.eye, dir, up);
        self.light_projection * self.light_view
    }
}
