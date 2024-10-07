mod reader;
mod camera;
mod text;
//mod winfont;

use std::default;

use graphics::{data::Vertex, text::GlyphAtlas, vector::Vector3, AppSkeleton};
use wgpu::util::DeviceExt;
use winit::event::{WindowEvent, KeyboardInput};

use gcode::GCommand;
use camera::*;

//extern crate directwrite;
//use directwrite::font_collection::FontCollection;
//use directwrite::enums::{FontStretch, FontStyle, FontWeight, InformationalStringId};

struct BoundTextLine {
    bind_group: wgpu::BindGroup,
    len: usize,
}
/*
impl BoundTextLine {
    fn bind(text_line: &str) -> BoundTextLine {
        let text_storage = text::Buffer::from(text_line);
        let mut text_uniform = text::Metadata::from(&text_storage);
        text_uniform.pixel_size = [40; 2];
        text_uniform.pixel_position = [20; 2];
        text_uniform.kerning = 100;

        let text_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Text Metadata Buffer"),
                contents: bytemuck::cast_slice(&[text_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let text_storage_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Text Storage Buffer"),
                contents: text_storage.pack_glyphs(),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }
        );

        BoundTextLine {
            bind_group: device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    layout: &self.text_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.screen_metadata_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: text_uniform_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: text_storage_buffer.as_entire_binding(),
                        },
                    ],
                    label: Some("text_bind_group"),
                }
            ),
            len: text_line.len(),
        }
    }
}
    */

struct NamedPipeline<'a> {
    name: &'a str,
    pipeline: wgpu::RenderPipeline,
    // store some information about what bind groups im using and index they are at
    // this is necessary in the render function to tell the encoder the right bind groups
}

impl NamedPipeline<'_> {
    fn named_for<'a>(name: &'a str, app: FigBuilder<'a>) -> PipelineBuilder<'a> {
        // device: &'a wgpu::Device
        PipelineBuilder {
            app,
            name,
            shader: None,
            pipeline_layout: None,
            vertex_buffer_layout: Some(Vertex::desc()),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            front_face_format: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
        }
    }
}

struct PipelineBuilder<'a> {
    //device: &'a wgpu::Device,
    app: FigBuilder<'a>,
    name: &'a str,
    shader: Option<wgpu::ShaderModule>,
    pipeline_layout: Option<wgpu::PipelineLayout>,
    vertex_buffer_layout: Option<wgpu::VertexBufferLayout<'static>>,
    primitive_topology: wgpu::PrimitiveTopology,
    front_face_format: wgpu::FrontFace,
    cull_mode: Option<wgpu::Face>,
    polygon_mode: wgpu::PolygonMode,
}

impl<'a> PipelineBuilder<'a> {
    fn with_shader(mut self, source: wgpu::ShaderSource) -> Self {
        let device = &self.app.skeleton.device;
        self.shader = Some(device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(format!("{} shader", self.name).as_str()),
            source: source.into(),
        }));
        self
    }

    fn with_vertex_buffer_layout(mut self, vertex_buffer_layout: wgpu::VertexBufferLayout<'static>) -> Self {
        self.vertex_buffer_layout = Some(vertex_buffer_layout);
        self
    }

    fn with_primitive_topology(mut self, primitive_topology: wgpu::PrimitiveTopology) -> Self {
        self.primitive_topology = primitive_topology;
        self
    }

    fn with_front_face_format(mut self, front_face_format: wgpu::FrontFace) -> Self {
        self.front_face_format = front_face_format;
        self
    }

    fn with_cull_mode(mut self, cull_mode: Option<wgpu::Face>) -> Self {
        self.cull_mode = cull_mode;
        self
    }

    fn with_polygon_mode(mut self, polygon_mode: wgpu::PolygonMode) -> Self {
        self.polygon_mode = polygon_mode;
        self
    }

    fn layout_for_camera3d(mut self) -> Self {
        let camera = self.app.camera3d.as_ref().expect("cannot layout camera3d: no camera3d in use!");
        let device = &self.app.skeleton.device;
        assert!(self.pipeline_layout.is_none(), "cannot use camera3d, pipeline layout is already defined! (hint: did you already call a layout_for_*() function?)");
        self.pipeline_layout = Some(device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(format!("{} render pipeline layout", self.name).as_str()),
            bind_group_layouts: &[
                &camera.bind_group_layout,
            ],
            push_constant_ranges: &[],
        }));
        self
    }

    fn layout_for_text2d(mut self) -> Self {
        let text = self.app.text2d.as_ref().expect("cannot layout text2d: no text2d in use!");
        let device = &self.app.skeleton.device;
        assert!(self.pipeline_layout.is_none(), "cannot use camera3d, pipeline layout is already defined! (hint: did you already call a layout_for_*() function?)");
        self.pipeline_layout = Some(device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(format!("{} render pipeline layout", self.name).as_str()),
            bind_group_layouts: &[
                &text.bind_group_layout,
                &text.glyph_atlas.glyph_bind_group_layout,
            ],
            push_constant_ranges: &[],
        }));
        self
    }

    fn use_custom_bind_group_layouts(mut self, layouts: &[&wgpu::BindGroupLayout]) -> Self {
        let device = &self.app.skeleton.device;
        self.pipeline_layout = Some(device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(format!("{} render pipeline layout", self.name).as_str()),
            bind_group_layouts: layouts,
            push_constant_ranges: &[],
        }));
        self
    }

    fn build(self) -> (FigBuilder<'a>, NamedPipeline<'a>) {
        let device = &self.app.skeleton.device;
        let config = &self.app.skeleton.config;
        let shader = &self.shader.expect(format!("cannot build {} pipeline without shader!", self.name).as_str());
        let vertex_buffer_layout = self.vertex_buffer_layout.expect(format!("cannot build {} pipeline without vertex buffer layout!", self.name).as_str());
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(format!("{} render pipeline", self.name).as_str()),
            layout: self.pipeline_layout.as_ref(),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers: &[
                    vertex_buffer_layout
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: self.primitive_topology,
                strip_index_format: None,
                front_face: self.front_face_format,
                cull_mode: self.cull_mode,
                polygon_mode: self.polygon_mode,
                // needs Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // needs Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        (
            self.app,
            NamedPipeline {
                name: self.name,
                pipeline: render_pipeline,
            }
        )
    }
}



// holds mutable elements that need to be maintained to ensure the app's state is consistent
struct AppState {
    vertex_buffer: wgpu::Buffer,
    vertex_count: usize,

    //controller: CameraController,
}

//#[derive(Default)]
struct FigBuilder<'a> {
    skeleton: graphics::AppSkeleton,
    state: Option<AppState>,

    camera3d: Option<camera::Camera3D<CameraLegacy>>,
    text2d: Option<graphics::text::Text2D>,

    named_pipelines: Vec<NamedPipeline<'a>>,
}

impl<'a> FigBuilder<'a> {
    fn state(mut self, state: AppState) -> Self {
        self.state = Some(state);
        self
    }

    fn add_pipeline(mut self, pipeline: NamedPipeline<'a>) -> Self {
        // you need a skeleton and a camera3d (or text2d) to even create a NamedPipeline so i dont need
        // to explicitly check.
        //let skeleton = self.skeleton.as_ref().expect("cannot add pipeline with no skeleton!");
        //let camera = self.camera3d.as_ref().expect("cannot add pipeline with no camera!");
        self.named_pipelines.push(pipeline);
        self
    }

    fn add_camera3d(mut self, camera: camera::CameraLegacy, controller: camera::CameraController) -> Self {
        //let skeleton = self.skeleton.as_ref().expect("cannot add camera3d without skeleton!");
        let camera = camera::CameraLegacy {
            aspect: self.skeleton.config.width as f32 / self.skeleton.config.height as f32,
            ..camera
        };

        let mut uniform = camera::CameraUniform::new();
        uniform.update_view_proj(&camera);

        let buffer = self.skeleton.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let bind_group_layout = self.skeleton.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                ],
                label: Some("camera_bind_group_layout"),
            }
        );

        let bind_group = self.skeleton.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }
                ],
                label: Some("camera_bind_group"),
            }
        );
        
        self.camera3d = Some(camera::Camera3D {
            camera,
            uniform,
            buffer,
            bind_group_layout,
            bind_group,
            controller,
        });
        self
    }

    fn add_text2d(mut self, glyph_atlas: GlyphAtlas) -> Self {
        //let skeleton = self.skeleton.as_ref().expect("cannot add text2d without skeleton!");
        let text_vertices = vec![
            Vertex::at(0.0,0.0,0.0),
            Vertex::at(1.0,1.0,0.0),
            Vertex::at(0.0,1.0,0.0),
            Vertex::at(1.0,0.0,0.0),
            Vertex::at(1.0,1.0,0.0),
            Vertex::at(0.0,0.0,0.0),
        ];
        let vertex_buffer = self.skeleton.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Text Vertex Buffer"),
                contents: bytemuck::cast_slice(text_vertices.as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let bind_group_layout = self.skeleton.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry { // screen metadata
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry { // text metadata
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer  {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None
                        },
                        count: None,
                    }
                ],
                label: Some("text_bind_group_layout"),
            }
        );
        let screen_uniform_buffer = self.skeleton.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Screen Metadata Buffer"),
                contents: bytemuck::cast_slice(self.skeleton.screen_size.size()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );
        self.text2d = Some(graphics::text::Text2D {
            vertex_buffer,
            bind_group_layout,
            glyph_atlas,
            screen_uniform_buffer,
        });
        self
    }

    fn build(self) -> (AppSkeleton, Fig<'a>) {
        (
            self.skeleton,
            Fig {
                state: self.state.expect("failed to build: no app state!"),

                camera3d: self.camera3d,
                text2d: self.text2d,

                named_pipelines: self.named_pipelines,
            }
        )
    }
}

struct Fig<'a> {
    // owns the skeleton and should then ensure all the lifetimes are correct
    //skeleton: graphics::AppSkeleton,
    state: AppState,

    camera3d: Option<camera::Camera3D<CameraLegacy>>,
    text2d: Option<graphics::text::Text2D>,

    named_pipelines: Vec<NamedPipeline<'a>>,
}

impl<'a> Fig<'_> {
    //fn builder() -> FigBuilder<'a> {
    //    FigBuilder::default()
    //}
    fn for_skeleton(skeleton: graphics::AppSkeleton) -> FigBuilder<'a> {
        FigBuilder {
            skeleton: skeleton,
            state: None,
            camera3d: None,
            text2d: None,
            named_pipelines: Vec::<NamedPipeline>::default(),
        }
    }

    // create bind group from uniform buffers?
    /*
    fn bind_text_line(&self, device: &wgpu::Device, text: text::TextLine) -> BoundTextLine {
        let text_storage = text::Buffer::from(text.as_str());
        let mut text_uniform = text::Metadata::from(&text_storage);
        text_uniform.pixel_size = [40; 2];
        text_uniform.pixel_position = [20; 2];
        text_uniform.kerning = 100;

        let text_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Text Metadata Buffer"),
                contents: bytemuck::cast_slice(&[text_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let text_storage_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Text Storage Buffer"),
                contents: text_storage.pack_glyphs(),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }
        );

        BoundTextLine {
            bind_group: device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    layout: &self.text_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.screen_metadata_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: text_uniform_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: text_storage_buffer.as_entire_binding(),
                        },
                    ],
                    label: Some("text_bind_group"),
                }
            ),
            len: text.len(),
        }
    }
    */
}

struct FigLegacy {
    glyph_bind_group: wgpu::BindGroup,

    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    camera: CameraLegacy,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,

    text_vertex_buffer: wgpu::Buffer,
    text_pipeline: wgpu::RenderPipeline,
    text_bind_group_layout: wgpu::BindGroupLayout,
    text_bind_groups: Vec<BoundTextLine>,

    screen_metadata_buffer: wgpu::Buffer,

    command_string: String,
}

impl FigLegacy {
    fn init(
        skeleton: &graphics::AppSkeleton,
        vertices: Vec::<Vertex>,
    ) -> Self {

        let screen_uniform_buffer = skeleton.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Screen Metadata Buffer"),
                contents: bytemuck::cast_slice(skeleton.screen_size.size()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        // ===== GLYPHS ======
        /*
        let factory = directwrite::Factory::new().unwrap();

        let collection = FontCollection::system_font_collection(&factory, false).unwrap();
        let lucidia_id = collection.find_family_by_name("Lucida Sans Typewriter").unwrap();
        let lucidia_family = collection.family(lucidia_id).unwrap();
        let lucidia_font = lucidia_family
            .first_matching_font(FontWeight::NORMAL, FontStretch::Normal, FontStyle::Normal)
            .unwrap();
        println!(
            "Font name: {:#?}",
            lucidia_font.informational_strings(InformationalStringId::FullName)
        );
        println!("Face name: {:#?}", lucidia_font.face_name());
        // this is from the directwrite crate, which is super cool, but also unfortunately
        // does not have glyph rasterization. i *could* add it, except cargo is refusing
        // to find any project metadata in the pulled git repo. fucking wonderful.
        */

        // i think im just going to use a font atlas or something for now
        // resources/glyphatlas.bin is a 5x12 font atlas in 8 bit single channel format

        let atlas_bytes: Vec<u8> = std::fs::read("src/resources/glyphatlas.bin")
            .unwrap().iter()
            .map(|v| match v { 0 => 0, _ => 255 })
            .collect();

        let atlas_size = wgpu::Extent3d {
            width: 160,
            height: 144,
            depth_or_array_layers: 1,
        };

        let atlas_texture = skeleton.device.create_texture(
            &wgpu::TextureDescriptor {
                size: atlas_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some("glyph_atlas"),
                view_formats: &[]
            }
        );

        skeleton.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(160 * 4),
                rows_per_image: Some(144),
            },
            atlas_size,
        );

        let atlas_texture_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = skeleton.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let glyph_bind_group_layout = skeleton.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            //sample_type: wgpu::TextureSampleType::Uint,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("glyph_atlas_bind_group_layout"),
            }
        );

        let glyph_bind_group = skeleton.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &glyph_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&atlas_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                    }
                ],
                label: Some("glyph_atlas_bind_group"),
            }
        );        

        // ===== CAMERA ======
        let camera = CameraLegacy {
            // position the camera one unit up and 2 units back
            // +z is out of the screen
            eye: Vector3 { x: 0.0, y: 500.0, z: 500.0 },
            // have it look at the origin
            target: Vector3 { x: 0.0, y: 0.0, z: 0.0 },
            // which way is "up"
            up: Vector3 { x: 0.0, y: 1.0, z: 0.0 },
            aspect: skeleton.config.width as f32 / skeleton.config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 1000.0,
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = skeleton.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout = skeleton.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                ],
                label: Some("camera_bind_group_layout"),
            }
        );

        let camera_bind_group = skeleton.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &camera_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_buffer.as_entire_binding(),
                    }
                ],
                label: Some("camera_bind_group"),
            }
        );
        // ====== END CAMERA ======

        // ====== WIREFRAME PIPELINE ======
        let shader = skeleton.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            skeleton.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let vertex_buffer = skeleton.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices.as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let render_pipeline = skeleton.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    Vertex::desc()
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: skeleton.config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                // requires Features::POLYGON_MODE_LINE which iirc is not
                // available on wasm. i not sorry i hate the web anyway
                polygon_mode: wgpu::PolygonMode::Line,
                // needs Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // needs Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        // ====== END WIREFRAME PIPELINE ======

        // ====== TEXT BIND GROUP ======

        // for the glyph textures i think i either want to use onion textures
        // or the descriptor thing mentioned here:
        // http://chunkstories.xyz/blog/a-note-on-descriptor-indexing/
        // this reddit thread talks about some pitfalls of onion textures:
        // https://www.reddit.com/r/rust_gamedev/comments/hfaz9k/updating_bind_groups_in_webgpu_with_texture_arrays/
        
        // when it comes to actually generating the glyph textures i think can
        // start with using DirectWrite! this crate uses it and it looks like
        // theres some really nice features to it:
        // https://github.com/servo/font-kit/blob/master/src/loaders/directwrite.rs

        // combining these two things might actually let me generate the glyph
        // textures on the fly as i need them and just keep them cached which
        // might cut down on load times a hair

        // one thing to note is that im not sure if ill have multiple font
        // sizes yet. i might, but i also might not and keep it all uniform

        let text_bind_group_layout = skeleton.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry { // screen metadata
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry { // text metadata
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer  {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None
                        },
                        count: None,
                    }
                ],
                label: Some("text_bind_group_layout"),
            }
        );
        // ====== END TEXT BIND GROUP ======

        // ====== TEXT PIPELINE ======
        let text_shader = skeleton.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("textshader.wgsl").into()),
        });

        let text_pipeline_layout =
            skeleton.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Text Pipeline Layout"),
                bind_group_layouts: &[
                    &text_bind_group_layout,
                    &glyph_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        
        // this is definitelly defined in clockwise order but if
        // i do it ccw it gets culled. dunno ¯\_(ツ)_/¯
        let text_vertices = vec![
            Vertex::at(0.0,0.0,0.0),
            Vertex::at(1.0,1.0,0.0),
            Vertex::at(0.0,1.0,0.0),
            Vertex::at(1.0,0.0,0.0),
            Vertex::at(1.0,1.0,0.0),
            Vertex::at(0.0,0.0,0.0),
        ];
        let text_vertex_buffer = skeleton.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Text Vertex Buffer"),
                contents: bytemuck::cast_slice(text_vertices.as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let text_pipeline = skeleton.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Pipeline"),
            layout: Some(&text_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &text_shader,
                entry_point: "vs_main",
                buffers: &[
                    Vertex::desc()
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &text_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: skeleton.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                // needs Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // needs Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            glyph_bind_group,

            render_pipeline,
            vertex_buffer,
            vertex_count: vertices.len() as u32,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller: CameraController::new(5.0),

            text_vertex_buffer,
            text_pipeline,
            text_bind_group_layout,
            text_bind_groups: vec![],

            screen_metadata_buffer: screen_uniform_buffer,
            
            // command buffer inits to empty, we'll use len() to check for content
            command_string: String::from("p"),
        }
    }
}

//impl graphics::Application for FigLegacy {
impl graphics::Application for Fig<'static> {
    fn required_features() -> wgpu::Features {
        wgpu::Features::POLYGON_MODE_LINE
    }

    fn input(&mut self, event: &winit::event::WindowEvent) -> bool {
        if self.camera3d.as_mut().unwrap().controller.process_events(event) { return true }
        match event {
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state,
                    virtual_keycode: Some(keycode),
                    ..
                },
                ..
            } => {
                //let is_pressed = *state == ElementState::Pressed;
                // if *state == ElementState::Pressed {
                //     match keycode {
                //         event::VirtualKeyCode::A => { self.command_string = self.command_string.to_owned() + "a"; println!("{}", self.command_string); true },
                //         _ => false,
                //     }
                // } else { false }
                false
            }
            _ => false,
        }
    }

    fn update(&mut self, queue: &wgpu::Queue) {
        //self.camera3d.as_ref().unwrap().controller.update_camera(self.camera3d.as_mut().unwrap().camera);
        //self.camera3d.unwrap().uniform.update_view_proj(&self.camera3d.as_mut().unwrap().camera);
        //queue.write_buffer(&self.camera3d.as_mut().unwrap().buffer, 0, bytemuck::cast_slice(&[self.camera3d.as_mut().unwrap().uniform]));
    }

    fn render(
        &mut self,
        surface: &wgpu::Surface,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), wgpu::SurfaceError> {
        let output = surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // update text
        // if (self.command_string.len() > 0) {
        //     let text_storage = text::Buffer::from(&self.command_string.as_str());
        //     let mut text_uniform = text::Metadata::from(&text_storage);
        //     text_uniform.pixel_size = [40; 2];
        //     text_uniform.pixel_position = [20; 2];
        //     text_uniform.kerning = 100;
        //     self.text_bind_groups[0] = self.bind_text_line(device, text::TextLine::from(text_storage, text_uniform));
        // }

        for named_pipeline in &self.named_pipelines {
            match named_pipeline {
                NamedPipeline { name: "wireframe", .. } => {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[
                            // target of @location(0) in fragment shader
                            Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color {
                                        r: 0.0,
                                        g: 0.0,
                                        b: 0.0,
                                        a: 1.0,
                                    }),
                                    store: true,
                                },
                            })
                        ],
                        depth_stencil_attachment: None,
                    });
                    render_pass.set_pipeline(&named_pipeline.pipeline);
                    render_pass.set_bind_group(0, &self.camera3d.as_ref().unwrap().bind_group, &[]);
                    render_pass.set_vertex_buffer(0, self.state.vertex_buffer.slice(..));
                    render_pass.draw(0..self.state.vertex_count as u32, 0..1);
                },
                NamedPipeline { name: "text", .. } => {
                    /*
                    let text_storage = text::Buffer::from("aaaa");
                    let mut text_uniform = text::Metadata::from(&text_storage);
                    text_uniform.pixel_size = [40; 2];
                    text_uniform.pixel_position = [20; 2];
                    text_uniform.kerning = 100;
                    //app.text_bind_groups.push(app.bind_text_line(&skeleton.device, text::TextLine::from(text_storage, text_uniform)));
                    let mut text_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("text pass"),
                        color_attachments: &[
                            Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: true,
                                },
                            })
                        ],
                        depth_stencil_attachment: None,
                    });
                    
                    text_pass.set_pipeline(&named_pipeline.pipeline);
                    text_pass.set_bind_group(0, BoundTextLine::bind(&self.text2d.as_ref().unwrap().bind_group., &[]);
                    text_pass.set_bind_group(1, &self.glyph_bind_group, &[]);
                    text_pass.set_vertex_buffer(0, self.text_vertex_buffer.slice(..));
                    text_pass.draw(0..6, 0..binding.len as u32);
                    */

                    /*if self.text_bind_groups.len() > 0 {
                        let mut text_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Text Pass"),
                            color_attachments: &[
                                // target of @location(0) in fragment shader
                                Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: true,
                                    },
                                })
                            ],
                            depth_stencil_attachment: None,
                        });
                
                        text_pass.set_pipeline(&named_pipeline.pipeline);
                        for binding in &self.text_bind_groups {
                            text_pass.set_bind_group(0, &binding.bind_group, &[]);
                            text_pass.set_bind_group(1, &self.glyph_bind_group, &[]);
                            text_pass.set_vertex_buffer(0, self.text_vertex_buffer.slice(..));
                            text_pass.draw(0..6, 0..binding.len as u32);
                        }
                    }
                    */
                },
                _ => {},
            }
            
        }
    
        // submit will accept anything that implements IntoIter
        queue.submit(std::iter::once(encoder.finish()));
        output.present();
    
        Ok(())
    }
}

fn main() {
    let skeleton = pollster::block_on(graphics::build::<Fig>("fig"));
    println!("built window with size: {:?}", skeleton.screen_size);
    // open gcode file
    let mut reader = reader::BufferedReader::open("test1.gcode")
        .expect("Error opening file!");
    let mut buffer = String::new();

    // create vertex buffer
    let mut vertices = Vec::<Vertex>::new();
    let mut is_first = true;

    while let Some(line) = reader.read_line(&mut buffer) {
        //println!("{}", line.trim());
        // if we successfully lex a command parse it and match it to ones
        // we care about
        if let Ok(Some(fields)) = gcode::lexer::lex(line.as_str()) {
            match gcode::parser::parse(fields) {
                Ok(GCommand::G1 {
                    x,
                    y,
                    z,
                    ..
                }) => {
                    if is_first {
                        println!("print started at: {:?}, {:?}, {:?}", x, y, z);
                        vertices.push(Vertex::at(
                            x.unwrap_or(0.0),
                            y.unwrap_or(0.0),
                            z.unwrap_or(0.0),
                        ));
                        is_first = false;
                        continue;
                    }
                    vertices.push(Vertex::at(
                        x.unwrap_or(vertices.last().unwrap().x()),
                        y.unwrap_or(vertices.last().unwrap().y()),
                        z.unwrap_or(vertices.last().unwrap().z()),
                    ));
                },
                _ => {},
                
                //println!("={:?}", command);
            }
        }
        //println!("= {:?}", gcode::lexer::lex(line.as_str()));
    }
    //println!("{:?}", vertices);

    // initialize shaders and hook handlers
    //let mut app = FigLegacy::init(&skeleton, vertices);

    //let text_storage = text::Buffer::from(&app.command_string.as_str());
    //let mut text_uniform = text::Metadata::from(&text_storage);
    //text_uniform.pixel_size = [40; 2];
    //text_uniform.pixel_position = [20; 2];
    //text_uniform.kerning = 100;
    //app.text_bind_groups.push(app.bind_text_line(&skeleton.device, text::TextLine::from(text_storage, text_uniform)));

    let camera = CameraLegacy {
        // position the camera one unit up and 2 units back
        // +z is out of the screen
        eye: Vector3 { x: 0.0, y: 500.0, z: 500.0 },
        // have it look at the origin
        target: Vector3 { x: 0.0, y: 0.0, z: 0.0 },
        // which way is "up"
        up: Vector3 { x: 0.0, y: 1.0, z: 0.0 },
        aspect: skeleton.config.width as f32 / skeleton.config.height as f32,
        fovy: 45.0,
        znear: 0.1,
        zfar: 1000.0,
    };

    let glyph_atlas = skeleton.create_glyph_atlas("src/resources/glyphatlas.bin");

    let vertex_buffer = skeleton.device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices.as_slice()),
            usage: wgpu::BufferUsages::VERTEX,
        }
    );

    let state = AppState {
        vertex_buffer,
        vertex_count: vertices.len(),
    };

    let app = Fig::for_skeleton(skeleton)
        .state(state)
        .add_text2d(glyph_atlas)
        .add_camera3d(camera, CameraController::default());

    let (app, wireframe_pipeline) = NamedPipeline::named_for("wireframe", app)
        .layout_for_camera3d()
        .with_shader(wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()))
        //.with_vertex_buffer_layout(Vertex::desc()) //defaults to Vertex::desc()
        .with_primitive_topology(wgpu::PrimitiveTopology::LineList)
        // requires Features::POLYGON_MODE_LINE which iirc is not
        // available on wasm. i not sorry i hate the web anyway
        .with_polygon_mode(wgpu::PolygonMode::Line)
        .with_cull_mode(None)
        .build();
    
    let (app, text_pipeline) = NamedPipeline::named_for("text", app)
        .layout_for_text2d()
        .with_shader(wgpu::ShaderSource::Wgsl(include_str!("textshader.wgsl").into()))
        .build();

    let (skeleton, app) = app
        .add_pipeline(wireframe_pipeline)
        .add_pipeline(text_pipeline)
        .build();

    graphics::run::<Fig>(app, skeleton);
}
