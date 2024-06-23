use std::rc::Rc;

use crate::math::{Ext2f, Ext2u, Vec3f};

pub struct CameraDescriptor {
    pub location: Vec3f,
    pub at: Vec3f,
    pub dir: Vec3f,
    pub right: Vec3f,
    pub up: Vec3f,
    pub projection_extent: Ext2f,
    pub near: f32,
}

#[repr(packed)]
#[allow(unused)]
struct CameraData {
    location: Vec3f,
    _pad0: f32,
    dir: Vec3f,
    near: f32,
    right: Vec3f,
    projection_width: f32,
    up: Vec3f,
    projection_height: f32,
}

#[derive(Default)]
#[repr(packed)]
#[allow(unused)]
struct SystemData {
    resolution: Ext2f,
    time: f32,
    static_frame_index: u32,
    texel_size: Ext2f,
}

pub struct Kernel<'t> {
    surface: wgpu::Surface<'t>,
    queue: wgpu::Queue,
    device: wgpu::Device,
}

struct Collector {
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

pub struct Render<'t> {
    kernel: Rc<Kernel<'t>>,
    surface_configuration: wgpu::SurfaceConfiguration,

    camera_buffer: wgpu::Buffer,
    system_buffer: wgpu::Buffer,
    static_frame_index: u32,

    collector_bind_group_layout: wgpu::BindGroupLayout,
    render_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,

    place_pipeline: wgpu::RenderPipeline,
    collectors: [Collector; 2],
}

impl<'t> Render<'t> {
    fn create_collectors<const N: usize>(device: &wgpu::Device, bind_group_layout: &wgpu::BindGroupLayout, extent: Ext2u) -> [Collector; N] {
        let collector_target_texture = device.create_texture(&wgpu::TextureDescriptor {
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            label: None,
            mip_level_count: 1,
            sample_count: 1,
            size: wgpu::Extent3d {
                width: extent.w,
                height: extent.h,
                depth_or_array_layers: N as u32,
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgpu::TextureFormat::Rgba32Float],
        });

        let build_collector = |index: usize| {
            let view = collector_target_texture.create_view(&wgpu::TextureViewDescriptor {
                array_layer_count: Some(1),
                aspect: wgpu::TextureAspect::All,
                base_array_layer: index as u32,
                base_mip_level: 0,
                dimension: Some(wgpu::TextureViewDimension::D2),
                format: Some(wgpu::TextureFormat::Rgba32Float),
                label: None,
                mip_level_count: None,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                }],
                label: None,
                layout: &bind_group_layout,
            });

            Collector { view, bind_group }
        };

        std::array::from_fn(build_collector)
    }

    pub fn new(window: impl wgpu::WindowHandle + 't, surface_ext: Ext2u) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor  {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });

        let surface = instance.create_surface(window).ok()?;

        let adapter = futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))?;

        let (device, queue) = futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
        }, None)).ok()?;

        let surface_format = {
            let caps = surface.get_capabilities(&adapter);
            *caps.formats.iter().find(|f| f.is_srgb() && f.has_color_aspect() && f.components() == 4).unwrap_or(&caps.formats[0])
        };
        // Setup surface
        let surface_configuration = wgpu::SurfaceConfiguration {
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            desired_maximum_frame_latency: 3,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            format: surface_format,
            width: surface_ext.w,
            height: surface_ext.h,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: vec![surface_format]
        };
        surface.configure(&device, &surface_configuration);

        let collector_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                count: None,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                visibility: wgpu::ShaderStages::FRAGMENT,
            }],
            label: None,
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera UBO"),
            mapped_at_creation: false,
            size: std::mem::size_of::<CameraData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        });

        let system_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("System UBO"),
            mapped_at_creation: false,
            size: std::mem::size_of::<SystemData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        });

        let render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        has_dynamic_offset: false,
                        min_binding_size: Some(std::num::NonZeroU64::try_from(std::mem::size_of::<CameraData>() as u64).unwrap()),
                        ty: wgpu::BufferBindingType::Uniform
                    },
                    visibility: wgpu::ShaderStages::FRAGMENT,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        has_dynamic_offset: false,
                        min_binding_size: Some(std::num::NonZeroU64::try_from(std::mem::size_of::<SystemData>() as u64).unwrap()),
                        ty: wgpu::BufferBindingType::Uniform
                    },
                    visibility: wgpu::ShaderStages::FRAGMENT,
                }
            ],
        });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &camera_buffer,
                        offset: 0,
                        size: None,
                    })
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &system_buffer,
                        offset: 0,
                        size: None,
                    })
                },
            ],
            label: None,
            layout: &render_bind_group_layout,
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&render_bind_group_layout, &collector_bind_group_layout],
            ..Default::default()
        });

        let render_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shaders/render.wgsl")))
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Main pipeline"),
            depth_stencil: None,
            fragment: Some(wgpu::FragmentState {
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                entry_point: "fs_main",
                module: &render_shader_module,
                targets: &[Some(wgpu::ColorTargetState {
                    blend: None,
                    format: wgpu::TextureFormat::Rgba32Float,
                    write_mask: wgpu::ColorWrites::ALL,
                })]
            }),
            layout: Some(&render_pipeline_layout),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            vertex: wgpu::VertexState {
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                entry_point: "vs_main",
                module: &render_shader_module,
            }
        });

        let place_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Place Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shaders/place.wgsl")))
        });

        let place_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&render_bind_group_layout, &collector_bind_group_layout],
            ..Default::default()
        });

        let place_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            depth_stencil: None,
            fragment: Some(wgpu::FragmentState {
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                entry_point: "fs_main",
                module: &place_shader_module,
                targets: &[Some(wgpu::ColorTargetState {
                    blend: None,
                    format: surface_format,
                    write_mask: wgpu::ColorWrites::ALL,
                })]
            }),
            label: None,
            layout: Some(&place_pipeline_layout),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            vertex: wgpu::VertexState {
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                entry_point: "vs_main",
                module: &place_shader_module,
            }
        });

        Some(Self {
            collectors: Self::create_collectors(&device, &collector_bind_group_layout, surface_ext),
            kernel: Rc::new(Kernel {
                device,
                queue,
                surface,
            }),
            render_bind_group,
            camera_buffer,
            system_buffer,
            render_pipeline,
            place_pipeline,
            static_frame_index: 0,
            collector_bind_group_layout,
            surface_configuration,
        })
    }

    /// Render resize function
    pub fn resize(&mut self, new_extent: Ext2u) {
        self.static_frame_index = 0;
        self.collectors = Self::create_collectors(&self.kernel.device, &self.collector_bind_group_layout, new_extent.clone());
        self.surface_configuration.width = new_extent.w;
        self.surface_configuration.height = new_extent.h;
        self.kernel.surface.configure(&self.kernel.device, &self.surface_configuration);
    } // fn resize

    pub fn set_camera(&mut self, camera_data: &CameraDescriptor) {
        self.kernel.queue.write_buffer(&self.camera_buffer, 0, unsafe {
            std::slice::from_raw_parts(std::mem::transmute(&CameraData {
                _pad0: 0.0,
                dir: camera_data.dir,
                location: camera_data.location,
                near: camera_data.near,
                projection_height: camera_data.projection_extent.h,
                projection_width: camera_data.projection_extent.w,
                right: camera_data.right,
                up: camera_data.up,
            }), std::mem::size_of::<CameraData>())
        });
        self.static_frame_index = 0;
    } // fn set_camera

    pub fn render(&mut self) {
        let image = match self.kernel.surface.get_current_texture() {
            Ok(v) => v,
            Err(_) => return,
        };
        let image_view = image.texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.kernel.queue.write_buffer(&self.system_buffer, 0, unsafe {
            let s = image.texture.size();
            let resolution = Ext2f::new(s.width as f32, s.height as f32);
            let texel_size = Ext2f::new(1.0 / resolution.w, 1.0 / resolution.h);
            std::slice::from_raw_parts(std::mem::transmute(&SystemData {
                resolution,
                texel_size,
                time: std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).map(|v| {
                    (v.as_millis() & 0xFFFFFF) as f32 / 1000.0
                }).unwrap_or(0.0),
                static_frame_index: self.static_frame_index,
                ..Default::default()
            }), std::mem::size_of::<SystemData>())
        });

        let mut encoder = self.kernel.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());


        let read_collector = &self.collectors[self.static_frame_index as usize & 1];
        let target_collector = &self.collectors[(self.static_frame_index + 1) as usize & 1];

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
                view: &target_collector.view,
            })],
            ..Default::default()
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.render_bind_group, &[]);
        render_pass.set_bind_group(1, &read_collector.bind_group, &[]);
        render_pass.draw(0..4, 0..1);

        drop(render_pass);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
                view: &image_view,
            })],
            ..Default::default()
        });

        render_pass.set_pipeline(&self.place_pipeline);
        render_pass.set_bind_group(0, &self.render_bind_group, &[]);
        render_pass.set_bind_group(1, &target_collector.bind_group, &[]);
        render_pass.draw(0..4, 0..1);

        drop(render_pass);

        self.kernel.queue.submit([encoder.finish()]);
        image.present();

        self.static_frame_index += 1;
    }
}
