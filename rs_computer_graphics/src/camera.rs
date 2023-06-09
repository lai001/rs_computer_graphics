use crate::rotator::Rotator;
use glam::{Vec3Swizzles, Vec4Swizzles};
use winit::event::ElementState;

pub trait CameraInputEventHandle {
    fn mouse_motion_handle(camera: &mut Camera, delta: (f64, f64), is_cursor_visible: bool);
    fn keyboard_input_handle(
        camera: &mut Camera,
        virtual_key_code: &winit::event::VirtualKeyCode,
        element_state: &winit::event::ElementState,
        is_cursor_visible: bool,
    );
}

pub struct DefaultCameraInputEventHandle {}

impl CameraInputEventHandle for DefaultCameraInputEventHandle {
    fn mouse_motion_handle(camera: &mut Camera, delta: (f64, f64), is_cursor_visible: bool) {
        if is_cursor_visible == false {
            let speed_x = 0.0_f32;
            let speed_y = 0.01_f32;
            let dx: f32;
            if delta.0.is_sign_negative() {
                dx = 1.0;
            } else {
                dx = -1.0;
            }
            let dy: f32;
            if delta.1.is_sign_negative() {
                dy = 1.0;
            } else {
                dy = -1.0;
            }
            camera.add_world_rotation_with_relative_rotator(Rotator {
                yaw: dx * speed_x,
                roll: 0.0,
                pitch: dy * speed_y,
            });
        }
    }

    fn keyboard_input_handle(
        camera: &mut Camera,
        virtual_key_code: &winit::event::VirtualKeyCode,
        element_state: &winit::event::ElementState,
        is_cursor_visible: bool,
    ) {
        let speed = 0.05_f32;
        if virtual_key_code == &winit::event::VirtualKeyCode::W
            && element_state == &ElementState::Pressed
            && is_cursor_visible == false
        {
            camera.add_world_location(glam::Vec3 {
                x: 0.0,
                y: 0.0,
                z: -1.0 * speed,
            });
        }
        if virtual_key_code == &winit::event::VirtualKeyCode::A
            && element_state == &ElementState::Pressed
            && is_cursor_visible == false
        {
            camera.add_world_location(glam::Vec3 {
                x: -1.0 * speed,
                y: 0.0,
                z: 0.0,
            });
        }
        if virtual_key_code == &winit::event::VirtualKeyCode::S
            && element_state == &ElementState::Pressed
            && is_cursor_visible == false
        {
            camera.add_world_location(glam::Vec3 {
                x: 0.0,
                y: 0.0,
                z: 1.0 * speed,
            });
        }
        if virtual_key_code == &winit::event::VirtualKeyCode::D
            && element_state == &ElementState::Pressed
            && is_cursor_visible == false
        {
            camera.add_world_location(glam::Vec3 {
                x: 1.0 * speed,
                y: 0.0,
                z: 0.0,
            });
        }
        if virtual_key_code == &winit::event::VirtualKeyCode::Q
            && element_state == &ElementState::Pressed
            && is_cursor_visible == false
        {
            camera.add_world_location(glam::Vec3 {
                x: 0.0,
                y: 1.0 * speed,
                z: 0.0,
            });
        }
        if virtual_key_code == &winit::event::VirtualKeyCode::E
            && element_state == &ElementState::Pressed
            && is_cursor_visible == false
        {
            camera.add_world_location(glam::Vec3 {
                x: 0.0,
                y: -1.0 * speed,
                z: 0.0,
            });
        }
    }
}

pub struct Camera {
    world_location: glam::Vec3,
    up_vector: glam::Vec3,
    forward_vector: glam::Vec3,
    fov_y_radians: f32,
    aspect_ratio: f32,
    z_near: f32,
    z_far: f32,
    projection_matrix: glam::Mat4,
    view_matrix: glam::Mat4,
    rotator: Rotator,
}

impl Camera {
    fn new(
        world_location: glam::Vec3,
        forward_vector: glam::Vec3,
        up_vector: glam::Vec3,
        fov_y_radians: f32,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
    ) -> Camera {
        let projection_matrix =
            glam::Mat4::perspective_rh(fov_y_radians, aspect_ratio, z_near, z_far);
        let view_matrix = glam::Mat4::look_to_rh(world_location, forward_vector, up_vector);
        Camera {
            world_location,
            up_vector,
            forward_vector,
            fov_y_radians,
            aspect_ratio,
            z_near,
            z_far,
            projection_matrix,
            view_matrix,
            rotator: Rotator {
                yaw: 0.0,
                roll: 0.0,
                pitch: 0.0,
            },
        }
    }

    pub fn default(window_width: u32, window_height: u32) -> Camera {
        Self::new(
            glam::Vec3::new(0.0, 0.0, 0.0),
            glam::Vec3::new(0.0, 0.0, -1.0),
            glam::Vec3::new(0.0, 1.0, 0.0),
            39.6_f32.to_radians(),
            window_width as f32 / window_height as f32,
            0.01,
            1000.0,
        )
    }

    pub fn set_fov_y_radians(&mut self, fov_y_radians: f32) {
        self.fov_y_radians = fov_y_radians;
        self.update_projection_matrix();
    }

    pub fn set_z_near(&mut self, z_near: f32) {
        self.z_near = z_near;
        self.update_projection_matrix();
    }

    pub fn set_z_far(&mut self, z_far: f32) {
        self.z_far = z_far;
        self.update_projection_matrix();
    }

    pub fn add_world_location(&mut self, world_location: glam::Vec3) {
        self.world_location = self.world_location + world_location;
        self.update_view_matrix();
    }

    pub fn add_world_rotation_with_relative_rotator(&mut self, rotator: Rotator) {}

    fn update_projection_matrix(&mut self) {
        self.projection_matrix = glam::Mat4::perspective_rh(
            self.fov_y_radians,
            self.aspect_ratio,
            self.z_near,
            self.z_far,
        );
    }

    fn update_view_matrix(&mut self) {
        self.view_matrix =
            glam::Mat4::look_to_rh(self.world_location, self.forward_vector, self.up_vector);
    }

    pub fn get_view_matrix(&self) -> glam::Mat4 {
        return self.view_matrix;
    }

    pub fn get_projection_matrix(&self) -> glam::Mat4 {
        return self.projection_matrix;
    }

    pub fn get_world_location(&self) -> glam::Vec3 {
        self.world_location
    }
}
