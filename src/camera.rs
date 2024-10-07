use cgmath::EuclideanSpace;
use glam::{Mat4, Vec3};
use graphics::vector::Vector3;
use winit::event::{WindowEvent, KeyboardInput, ElementState, VirtualKeyCode};

//use graphics::vector::{Mat4, Vector3};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);


// move to fox-graphics eventually?
pub struct Camera3D<C: Camera> {
    pub camera: C,
    pub uniform: CameraUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub controller: CameraController,
}

pub enum CameraType {
    FPSCamera {
        eye: Vector3<f32>,
        target: Vector3<f32>,
        up: Vector3<f32>,
        aspect: f32,
        fovy: f32,
        znear: f32,
        zfar: f32,
    },
    OrbitCamera {
        eye: Vec3,
        target: Vec3,
        up: Vec3,
        distance: f32,
        pitch: f32,
        yaw: f32,
        //bounds: OrbitCameraBounds,
        aspect: f32,
        fovy: f32,
        znear: f32,
        zfar: f32,
    },
}

trait Camera {
    fn build_view_projection_matrix(&self) -> Mat4;
}

pub struct CameraLegacy {
    pub eye: Vector3<f32>,
    pub target: Vector3<f32>,
    pub up: Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera for CameraLegacy {
    fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(
            Vec3::new(self.eye.x, self.eye.y, self.eye.z),
            Vec3::new(self.target.x, self.target.y, self.target.z),
            Vec3::new(self.up.x, self.up.y, self.up.z)
        );
        let proj = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
        proj * view
    }
}

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &CameraLegacy) {
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
    }
}

pub struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}

impl CameraController {
    pub fn default() -> Self {
        Self::new(10.0)
    }
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    pub fn process_events(&self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state,
                    virtual_keycode: Some(keycode),
                    ..
                },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::Up => {
                        //self.is_forward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::Left => {
                        //self.is_left_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::Down => {
                        //self.is_backward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::Right => {
                        //self.is_right_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_camera(&self, camera: &mut CameraLegacy) {
        /*
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Prevents glitching when camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the fowrard/backward is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and eye so 
            // that it doesn't change. The eye therefore still 
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
        */
    }
}
