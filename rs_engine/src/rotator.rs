#[derive(Clone, Copy, Debug)]
pub struct Rotator {
    pub yaw: f32,
    pub roll: f32,
    pub pitch: f32,
}

impl Rotator {
    pub fn zero() -> Rotator {
        Rotator {
            yaw: 0.0,
            roll: 0.0,
            pitch: 0.0,
        }
    }

    pub fn to_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_euler(glam::EulerRot::XYZ, self.pitch, self.yaw, self.roll)
    }

    pub fn to_radians(&self) -> Self {
        Rotator {
            yaw: self.yaw.to_radians(),
            roll: self.roll.to_radians(),
            pitch: self.pitch.to_radians(),
        }
    }

    pub fn to_degrees(&self) -> Self {
        Rotator {
            yaw: self.yaw.to_degrees(),
            roll: self.roll.to_degrees(),
            pitch: self.pitch.to_degrees(),
        }
    }

    pub fn to_forward_vector(&self) -> glam::Vec3 {
        let mut forward_vector = glam::Vec3::ZERO;
        let pitch = self.pitch;
        forward_vector.x = pitch.cos() * self.yaw.cos();
        forward_vector.y = pitch.sin();
        forward_vector.z = pitch.cos() * self.yaw.sin();
        forward_vector
    }

    pub fn from_forward_vector(forward_vector: glam::Vec3) -> Rotator {
        let pitch = (-forward_vector.y).asin();
        let yaw = forward_vector.z.atan2(forward_vector.x);
        Rotator {
            yaw,
            roll: 0.0,
            pitch,
        }
    }
}
