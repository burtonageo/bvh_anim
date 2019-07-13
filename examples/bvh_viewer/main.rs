#![allow(unused)]

mod arcball_camera;

use arcball_camera::ArcBall;
use bvh_anim::{Bvh, JointData};
use gl::{self, types::*};
use glutin::{
    dpi::LogicalSize, ContextBuilder, DeviceEvent, ElementState, Event, EventsLoop, KeyboardInput,
    MouseButton, Touch, TouchPhase, VirtualKeyCode, WindowBuilder, WindowEvent,
};
use nalgebra::{Matrix4, Point2, Point3, Vector3};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Timer {
    last_update: Instant,
    interval: Duration,
}

impl Timer {
    fn new(interval: Duration) -> Self {
        Timer {
            last_update: Instant::now(),
            interval,
        }
    }

    /// If this method returns `true`, then the timer
    /// fired during this tick.
    fn tick(&mut self, delta: &Duration) -> bool {
        self.interval += *delta;

        let curr_time = self.last_update + *delta;
        let next_update = self.last_update + self.interval;

        if next_update <= curr_time {
            self.last_update = Instant::now();
            true
        } else {
            false
        }
    }
}

struct AnimationPlayer {
    bvh: Bvh,
    timer: Timer,
    bones: Vec<Bone>,
    current_frame: usize,
    does_loop: bool,
}

impl AnimationPlayer {
    fn new(bvh: Bvh) -> Self {
        let frame_time = *bvh.frame_time();
        AnimationPlayer {
            bvh,
            timer: Timer::new(frame_time),
            bones: vec![],
            current_frame: 0,
            does_loop: false,
        }
    }

    fn tick(&mut self, delta: &Duration) {
        if self.timer.tick(delta) {
            self.anim_callback();
        }
    }

    fn calculate_joints_fk(&mut self) {
        let base_mat = Matrix4::<f32>::identity();
    }

    fn anim_callback(&mut self) {}
}

#[derive(Debug, PartialEq)]
struct Bone {
    position: Point3<f32>,
    direction: Vector3<f32>,
    length: f32,
    shader_program: GLuint,
    vbo: GLint,
    vao: GLint,
    ebo: GLint,
    model_matrix_uniform_loc: GLint,
    view_matrix_uniform_loc: GLint,
    projection_matrix_uniform_loc: GLint,
}

impl Bone {
    fn new(position: Point3<f32>, direction: Vector3<f32>, length: f32) -> Self {
        macro_rules! c {
            ($s:literal) => {
                unsafe {
                    ::std::ffi::CStr::from_bytes_with_nul_unchecked(concat!($s, "\0").as_bytes())
                }
            };
        }

        let vert_src = c!(r#"#version 330

        layout(location = 0) in vec3 position;

        layout(location = 0) out vec4 f_position;
        layout(location = 1) out vec4 f_color;

        uniform mat4 model_matrix;
        uniform mat4 view_matrix;
        uniform mat4 projection_matrix;

        void main() {
            mat4 mvp = projection_matrix * view_matrix * model_matrix;

            f_position = mvp * vec4(position, 1.0);
            f_color = vec4(0.7, 0.7, 0.7, 1.0);

            gl_Position = f_position;
        }

        "#);

        let frag_src = c!(r#"#version 330

        layout(location = 0) in vec4 f_position;
        layout(location = 1) in vec4 f_color;

        layout(location = 0) out vec4 target_color;

        void main() {
            target_color = f_color;
        }

        "#);

        let shader_program = 0;

        let model_matrix_uniform_loc =
            unsafe { gl::GetUniformLocation(shader_program, c!("model_matrix").as_ptr()) };

        let view_matrix_uniform_loc =
            unsafe { gl::GetUniformLocation(shader_program, c!("view_matrix").as_ptr()) };

        let projection_matrix_uniform_loc =
            unsafe { gl::GetUniformLocation(shader_program, c!("projection_matrix").as_ptr()) };

        let cuboid_verts = &[
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
        ];

        let cuboid_inds = &[0u16];

        let mut buffers = [0, 0];
        unsafe {
            gl::GenBuffers(buffers.len() as _, buffers.as_mut_ptr() as *mut _);
        }

        let [vbo, ebo] = buffers;

        Bone {
            position,
            direction,
            length,
            shader_program,
            vbo,
            vao: 0,
            ebo,
            model_matrix_uniform_loc,
            view_matrix_uniform_loc,
            projection_matrix_uniform_loc,
        }
    }

    fn render(&self) {
        unsafe {
            gl::UseProgram(self.shader_program);

            gl::UseProgram(0);
        }
    }
}

impl Drop for Bone {
    fn drop(&mut self) {
        unsafe {
            let mut buffers = [self.vbo, self.ebo];
            gl::DeleteBuffers(buffers.len() as _, buffers.as_mut_ptr() as *mut _);
            gl::DeleteVertexArrays(1, (&mut self.vao) as *mut _ as *mut _);

            gl::UseProgram(0);
            gl::DeleteProgram(self.shader_program);
        }
    }
}

fn main() {
    let mut events_loop = EventsLoop::new();
    let win_builder = WindowBuilder::new()
        .with_title("Bvh Viewer")
        .with_dimensions(LogicalSize::new(1024.0, 768.0));

    let context = ContextBuilder::new()
        .build_windowed(win_builder, &events_loop)
        .expect("Could not create OpenGL Context");

    let context = unsafe {
        context
            .make_current()
            .expect("Could not make the OpenGL context current")
    };

    unsafe {
        gl::load_with(|s| context.get_proc_address(s) as _);

        gl::ClearColor(0.1, 0.0, 0.4, 1.0);
    }

    let mut is_running = true;
    let mut prev_time = Instant::now();
    let mut arcball = ArcBall::new();
    let mut is_mouse_down = false;
    let mut prev_touch_pos = Point2::<f32>::origin();
    let mut loaded_skeletons: Vec<Bvh> = vec![];

    while is_running {
        let curr_time = Instant::now();
        let dt = curr_time - prev_time;
        prev_time = curr_time;

        events_loop.poll_events(|ev| match ev {
            Event::DeviceEvent { ref event, .. } => match event {
                DeviceEvent::Key(KeyboardInput {
                    virtual_keycode: Some(VirtualKeyCode::Escape),
                    ..
                }) => {
                    is_running = false;
                }
                DeviceEvent::MouseMotion {
                    delta: (ref mx, ref my),
                }
                    if is_mouse_down =>
                {
                    arcball.on_mouse_move(*mx as f32, *my as f32);
                }
                _ => {}
            },
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                    is_running = false;
                }
                WindowEvent::MouseInput {
                    ref state,
                    button: MouseButton::Left,
                    ..
                } => match state {
                    ElementState::Pressed => is_mouse_down = true,
                    ElementState::Released => is_mouse_down = false,
                },
                WindowEvent::Touch(ref t) => {
                    match t.phase {
                        TouchPhase::Started => {
                            is_mouse_down = true;
                            // Set the last touch position to the t.position
                        }
                        TouchPhase::Ended | TouchPhase::Cancelled => is_mouse_down = true,
                        TouchPhase::Moved => {
                            // Get the touch position
                            // Calculate the touch delta
                            // Modify the arcball using the delta info
                            // Set the last touch position to the current touch position.
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        });

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        // Tick the skeleton using the delta time
        // Render the skeleton.

        context.swap_buffers().expect("Could not swap buffers");
    }
}
