use std::sync::Arc;

use crate::math::{Ext2u, Vec2f, Vec3f};

/// Render camera descriptor
pub struct CameraDescriptor {
    /// Camrea origin
    pub location: Vec3f,

    /// Forward direction
    pub forward: Vec3f,

    /// Right direction
    pub right: Vec3f,

    /// Up direction
    pub up: Vec3f,

    /// Projection width/height
    pub projection_size: Vec2f,

    /// Near projection plane
    pub near_plane: f32,
}

#[repr(C)]
struct CameraBufferData {
    location: Vec3f,
    _pad0: f32,

    dir: Vec3f,
    near: f32,

    right: Vec3f,
    projection_width: f32,

    up: Vec3f,
    projection_height: f32,
}

#[repr(C)]
struct SystemBufferData {
    resolution: Vec2f,
    time: f32,
    static_frame_index: u32,
    resolution_scale: Vec2f,

    texel_size: Vec2f,
}

/// Convert structure into byte slice
unsafe fn into_byte_slice<'t, T>(item: &'t T) -> &'t [u8] {
    unsafe {
        std::slice::from_raw_parts(
            (item as *const T) as *const u8,
            std::mem::size_of::<T>()
        )
    }
}

/// Collector texture
struct CollectorTexture {
    /// Frame view
    view: wgpu::TextureView,

    /// Bind group corresponding to the texture
    bind_group: wgpu::BindGroup,
}

/// Renderer structure
pub struct Render {
    /// Destination surface
    surface: wgpu::Surface<'static>,

    /// Queue
    queue: wgpu::Queue,

    /// Device
    device: wgpu::Device,

    /// Configuration of the surface
    surface_configuration: wgpu::SurfaceConfiguration,

    /// Value to scale resolution by
    collector_extent_scale: u32,

    /// Renderer extent
    collector_extent: Ext2u,

    /// Buffer that holds camera data
    camera_buffer: wgpu::Buffer,

    /// Buffer that holds system data
    system_buffer: wgpu::Buffer,

    /// Index of the current static frame (e.g. amount of collected frames in target texture)
    static_frame_index: u32,

    /// Bind group layout for the collector
    collector_bind_group_layout: wgpu::BindGroupLayout,

    /// Bind group used in rendering process
    render_bind_group: wgpu::BindGroup,

    /// Pipeline used in rendering process
    render_pipeline: wgpu::RenderPipeline,

    /// Pipeline that maps collector contents on the screen
    place_pipeline: wgpu::RenderPipeline,

    /// Target textures
    collectors: [CollectorTexture; 2],

    /// Time of the rendering start
    render_start_time: std::time::Instant,

    /// Window holder
    _window_handle: Arc<dyn wgpu::WindowHandle>,
}

impl Render {
    fn create_collectors<const N: usize>(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        extent: Ext2u
    ) -> [CollectorTexture; N] {
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
                label: None,
                format: Some(wgpu::TextureFormat::Rgba32Float),
                dimension: Some(wgpu::TextureViewDimension::D2),
                usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: index as u32,
                array_layer_count: Some(1),
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                }],
                label: None,
                layout: &bind_group_layout,
            });

            CollectorTexture { view, bind_group }
        };

        std::array::from_fn(build_collector)
    }

    pub fn new(window: Arc<dyn wgpu::WindowHandle>, surface_ext: Ext2u) -> Option<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let surface = instance.create_surface(window.clone()).ok()?;

        let adapter = futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })).ok()?;

        let (device, queue) = futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })).ok()?;

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
            size: std::mem::size_of::<CameraBufferData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        });

        let system_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("System UBO"),
            mapped_at_creation: false,
            size: std::mem::size_of::<SystemBufferData>() as u64,
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
                        min_binding_size: Some(std::num::NonZeroU64::try_from(std::mem::size_of::<CameraBufferData>() as u64).unwrap()),
                        ty: wgpu::BufferBindingType::Uniform
                    },
                    visibility: wgpu::ShaderStages::FRAGMENT,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        has_dynamic_offset: false,
                        min_binding_size: Some(std::num::NonZeroU64::try_from(std::mem::size_of::<SystemBufferData>() as u64).unwrap()),
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
                entry_point: Some("fs_main"),
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
                entry_point: Some("vs_main"),
                module: &render_shader_module,
            },
            cache: None,
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
                entry_point: Some("fs_main"),
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
                entry_point: Some("vs_main"),
                module: &place_shader_module,
            },
            cache: None,
        });

        let collector_extent_scale = 2;
        let collector_extent = Ext2u::new(
            surface_configuration.width / collector_extent_scale,
            surface_configuration.height / collector_extent_scale
        );

        Some(Self {
            collector_extent_scale,
            collector_extent,
            collectors: Self::create_collectors(&device, &collector_bind_group_layout, collector_extent),
            device,
            queue,
            surface,
            render_bind_group,
            camera_buffer,
            system_buffer,
            render_pipeline,
            place_pipeline,
            static_frame_index: 0,
            collector_bind_group_layout,
            surface_configuration,
            render_start_time: std::time::Instant::now(),
            _window_handle: window,
        })
    }

    /// Render resize function
    pub fn resize(&mut self, new_extent: Ext2u) {
        self.static_frame_index = 0;

        self.collector_extent = Ext2u::new(
            new_extent.w / self.collector_extent_scale,
            new_extent.h / self.collector_extent_scale,
        );

        self.collectors = Self::create_collectors(
            &self.device,
            &self.collector_bind_group_layout,
            self.collector_extent
        );

        self.surface_configuration.width = new_extent.w;
        self.surface_configuration.height = new_extent.h;

        self.surface.configure(&self.device, &self.surface_configuration);
    }

    pub fn set_camera(&mut self, camera_data: &CameraDescriptor) {
        let buffer_camera_data = CameraBufferData {
            _pad0: 0.0,
            dir: camera_data.forward,
            location: camera_data.location,
            near: camera_data.near_plane,
            projection_width: camera_data.projection_size.x,
            projection_height: camera_data.projection_size.y,
            right: camera_data.right,
            up: camera_data.up,
        };

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            unsafe { into_byte_slice(&buffer_camera_data) }
        );
        self.static_frame_index = 0;
    }

    pub fn render(&mut self) {
        let image = match self.surface.get_current_texture() {
            Ok(v) => v,
            Err(_) => return,
        };
        let target_image_view = image.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // let target_image_size = image.texture.size();
        let resolution = Vec2f::new(self.collector_extent.w as f32, self.collector_extent.h as f32);
        let system_buffer_data = SystemBufferData {
            resolution,
            texel_size: resolution.map(f32::recip),
            time: std::time::Instant::now().duration_since(self.render_start_time).as_secs_f32(),
            resolution_scale: Vec2f::new(self.collector_extent_scale as f32, self.collector_extent_scale as f32),
            static_frame_index: self.static_frame_index,
        };

        // Update system buffer
        self.queue.write_buffer(&self.system_buffer, 0, unsafe {
            into_byte_slice(&system_buffer_data)
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

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
                depth_slice: None,
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
                view: &target_image_view,
                depth_slice: None,
            })],
            ..Default::default()
        });

        render_pass.set_pipeline(&self.place_pipeline);
        render_pass.set_bind_group(0, &self.render_bind_group, &[]);
        render_pass.set_bind_group(1, &target_collector.bind_group, &[]);
        render_pass.draw(0..4, 0..1);

        drop(render_pass);

        self.queue.submit([encoder.finish()]);
        image.present();

        self.static_frame_index += 1;
    }
}
