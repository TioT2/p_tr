use std::sync::Arc;

use math::{Ext2u, Vec2f, Vec3f};

use crate::render::shape;

pub mod timer;
pub mod input;
pub mod math;
pub mod render;

struct Camera {
    pub location: Vec3f,
    pub at: Vec3f,

    pub direction: Vec3f,
    pub right: Vec3f,
    pub up: Vec3f,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            location: Vec3f::new(0.0, 0.0, 1.0),
            at: Vec3f::new(0.0, 0.0, 0.0),
            direction: Vec3f::new(0.0, 0.0, -1.0),
            right: Vec3f::new(1.0, 0.0, 0.0),
            up: Vec3f::new(0.0, 1.0, 0.0),
        }
    }

    pub fn set(&mut self, location: Vec3f, at: Vec3f, approx_up: Vec3f) {
        self.direction = (at - location).normalized();
        self.right = Vec3f::cross(self.direction, approx_up).normalized();
        self.up = Vec3f::cross(self.right, self.direction).normalized();
        self.location = location;
        self.at = at;
    }
}

struct System {
    window: Arc<winit::window::Window>,
    render: render::Render,
    timer: timer::Timer,
    input: input::Input,
    camera: Camera,
}

impl System {
    /// Generate 'Cornell box' shape
    fn gen_cornell_box() -> Box<dyn shape::Shape> {
        let mut scene = shape::Scene::new();

        // Sphere
        scene.add_shape(Box::new(shape::Sphere::new(
            Vec3f::new(2.0, 3.0, -2.0),
            2.0,
            shape::Material {
                base_color: Vec3f::new(0.30, 0.47, 0.80),
                emission: Vec3f::new(0.0, 0.0, 0.0),
                metallic: 0.2,
                roughness: 0.8,
            }
        )));

        // Light source
        scene.add_shape(Box::new({
            let center = Vec3f::new(0.0, 11.9, 0.0);
            let size = 2.0;
            let material = shape::Material {
                emission: Vec3f::new(1.0, 1.0, 1.0) * 100.0,
                base_color: Vec3f::new(1.0, 1.0, 1.0),
                metallic: 0.0,
                roughness: 1.0,
            };

            shape::Aabb::new(center - size, center + size, material)
        }));

        // Bottom plane
        scene.add_shape(Box::new(shape::Quad::new(
            Vec3f::new(-5.0, 0.0, -5.0),
            Vec3f::new(0.0, 0.0, 1.0) / 30.0,
            Vec3f::new(1.0, 0.0, 0.0) / 10.0,
            shape::Material {
                emission: Vec3f::new(0.0, 0.0, 0.0),
                base_color: Vec3f::new(1.0, 1.0, 1.0),
                metallic: 0.0,
                roughness: 1.0,
            }
        )));

        // Left plane
        scene.add_shape(Box::new(shape::Quad::new(
            Vec3f::new(-5.0, 0.0, -5.0),
            Vec3f::new(0.0, 1.0, 0.0) / 10.0,
            Vec3f::new(0.0, 0.0, 1.0) / 30.0,
            shape::Material {
                emission: Vec3f::new(0.0, 0.0, 0.0),
                base_color: Vec3f::new(0.0, 1.0, 0.0),
                metallic: 0.0,
                roughness: 1.0,
            }
        )));

        // Right plane
        scene.add_shape(Box::new(shape::Quad::new(
            Vec3f::new(5.0, 0.0, -5.0),
            Vec3f::new(0.0, 0.0, 1.0) / 30.0,
            Vec3f::new(0.0, 1.0, 0.0) / 10.0,
            shape::Material {
                emission: Vec3f::new(0.0, 0.0, 0.0),
                base_color: Vec3f::new(1.0, 0.0, 0.0),
                metallic: 0.0,
                roughness: 1.0,
            }
        )));

        // Front plane
        scene.add_shape(Box::new(shape::Quad::new(
            Vec3f::new(-5.0, 0.0, 25.0),
            Vec3f::new(0.0, 1.0, 0.0) / 10.0,
            Vec3f::new(1.0, 0.0, 0.0) / 10.0,
            shape::Material {
                emission: Vec3f::new(0.0, 0.0, 0.0),
                base_color: Vec3f::new(1.0, 1.0, 1.0),
                metallic: 0.0,
                roughness: 1.0,
            }
        )));

        // Back plane
        scene.add_shape(Box::new(shape::Quad::new(
            Vec3f::new(-5.0, 0.0, -5.0),
            Vec3f::new(1.0, 0.0, 0.0) / 10.0,
            Vec3f::new(0.0, 1.0, 0.0) / 10.0,
            shape::Material {
                emission: Vec3f::new(0.0, 0.0, 0.0),
                base_color: Vec3f::new(1.0, 1.0, 1.0),
                metallic: 0.95,
                roughness: 0.05,
            }
        )));

        // Top plane
        scene.add_shape(Box::new(shape::Quad::new(
            Vec3f::new(-5.0, 10.0, -5.0),
            Vec3f::new(1.0, 0.0, 0.0) / 10.0,
            Vec3f::new(0.0, 0.0, 1.0) / 30.0,
            shape::Material {
                emission: Vec3f::new(0.0, 0.0, 0.0),
                base_color: Vec3f::new(1.0, 1.0, 1.0),
                metallic: 0.0,
                roughness: 1.0,
            }
        )));

        Box::new(scene)
    }

    pub fn new(window: winit::window::Window) -> Self {
        let window_size = window.inner_size();
        let window = Arc::new(window);

        let mut result = Self {
            render: render::Render::new(window.clone(), Ext2u::new(window_size.width, window_size.height)).unwrap(),
            window,
            timer: timer::Timer::new(),
            input: input::Input::new(),
            camera: Camera::new(),
        };

        result.render.set_traced_shape(Self::gen_cornell_box().as_ref());

        result.camera.set(
            Vec3f::new(-3.2, 2.8, 0.3),
            Vec3f::new(-2.4, 2.4, -0.1),
            Vec3f::new(0.0, 1.0, 0.0)
        );

        result.update_render_camera();
        result
    }

    fn update_render_camera(&mut self) {
        self.render.set_camera(&render::CameraDescriptor {
            forward: self.camera.direction,
            location: self.camera.location,
            near_plane: 1.0,
            projection_size: {
                let size = self.window.inner_size();

                Vec2f::new(size.width as f32, size.height as f32) / u32::min(size.width, size.height) as f32
            },
            right: self.camera.right,
            up: self.camera.up,
        });
    }

    fn on_window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if self.window.id() != window_id {
            return;
        }

        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } => {
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    self.input.on_key_change(code, event.state == winit::event::ElementState::Pressed);
                }
            }
            winit::event::WindowEvent::Resized(new_extent) => {
                self.render.resize(Ext2u::new(new_extent.width, new_extent.height));
                self.update_render_camera();
            }
            winit::event::WindowEvent::RedrawRequested => {
                self.timer.response();
                let timer_state = self.timer.get_state();
                let input_state = self.input.get_state();

                if input_state.is_key_clicked(input::KeyCode::F11) {
                    self.window.set_fullscreen(if self.window.fullscreen().is_some() {
                        None
                    } else {
                        Some(winit::window::Fullscreen::Borderless(None))
                    });
                }

                // Update camera and so on
                let camera_update_required = 'camera_control: {
                    let move_axis = Vec3f::new(
                        (input_state.is_key_pressed(input::KeyCode::KeyD) as i32 - input_state.is_key_pressed(input::KeyCode::KeyA) as i32) as f32,
                        (input_state.is_key_pressed(input::KeyCode::KeyR) as i32 - input_state.is_key_pressed(input::KeyCode::KeyF) as i32) as f32,
                        (input_state.is_key_pressed(input::KeyCode::KeyW) as i32 - input_state.is_key_pressed(input::KeyCode::KeyS) as i32) as f32,
                    );
                    let rotate_axis = Vec2f::new(
                      (input_state.is_key_pressed(input::KeyCode::ArrowRight) as i32 - input_state.is_key_pressed(input::KeyCode::ArrowLeft) as i32) as f32,
                      (input_state.is_key_pressed(input::KeyCode::ArrowDown) as i32 - input_state.is_key_pressed(input::KeyCode::ArrowUp) as i32) as f32,
                    );

                    if move_axis.length() <= 0.01 && rotate_axis.length() <= 0.01 {
                        break 'camera_control false;
                    }

                    let movement_delta = (
                        self.camera.right     * move_axis.x +
                        self.camera.up        * move_axis.y +
                        self.camera.direction * move_axis.z
                    ) * timer_state.get_delta_time() as f32 * 8.0;

                    let mut azimuth = self.camera.direction.y.acos();
                    let mut elevator = self.camera.direction.z.signum() * (
                        self.camera.direction.x / (
                            self.camera.direction.x * self.camera.direction.x +
                            self.camera.direction.z * self.camera.direction.z
                        ).sqrt()
                    ).acos();

                    elevator += rotate_axis.x * timer_state.get_delta_time() as f32 * 2.0;
                    azimuth += rotate_axis.y * timer_state.get_delta_time() as f32 * 2.0;

                    azimuth = azimuth.clamp(0.01, std::f32::consts::PI - 0.01);

                    let new_direction = Vec3f{
                        x: azimuth.sin() * elevator.cos(),
                        y: azimuth.cos(),
                        z: azimuth.sin() * elevator.sin()
                    };

                    self.camera.set(self.camera.location + movement_delta, self.camera.location + movement_delta + new_direction, Vec3f {x: 0.0, y: 1.0, z: 0.0});
                    true
                };

                if input_state.is_key_pressed(input::KeyCode::Space) {
                    self.render.reset_frame();
                }

                unsafe {
                    static mut T: Option<std::time::Instant> = None;

                    if let Some(time) = T {
                        let now = std::time::Instant::now();
                        let delta = now.duration_since(time);

                        if delta.as_secs_f32() > 1.0 {
                            T = Some(now);
                            println!("{}", timer_state.get_fps());
                        }
                    } else {
                        T = Some(std::time::Instant::now());
                    }
                }

                self.input.clear_changed();

                if camera_update_required {
                    self.update_render_camera();
                }
                self.render.render();
            }
            _ => {}
        }
    }
}

struct Application {
    system: Option<System>,
}

impl Application {
    pub fn new() -> Self {
        Self { system: None }
    }
}

impl winit::application::ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Ok(window) = event_loop.create_window(winit::window::WindowAttributes::default()
            .with_title("PathTRacing")
            .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
        ) {
            self.system = Some(System::new(window));
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let Some(system) = self.system.as_ref() else {
            return;
        };

        system.window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(system) = self.system.as_mut() else {
            return
        };

        system.on_window_event(event_loop, window_id, event);
    }
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut application = Application::new();

    event_loop.run_app(&mut application).unwrap();
}
