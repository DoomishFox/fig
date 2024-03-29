mod reader;
mod camera;
mod text;
mod winfont;

use graphics::data::Vertex;
use wgpu::util::DeviceExt;

use gcode::GCommand;
use camera::*;

extern crate directwrite;
//use directwrite::font_collection::FontCollection;
//use directwrite::enums::{FontStretch, FontStyle, FontWeight, InformationalStringId};



struct Fig {
    glyph_bind_group: wgpu::BindGroup,

    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,

    text_vertex_buffer: wgpu::Buffer,
    text_pipeline: wgpu::RenderPipeline,
    text_buffer: text::Buffer,
    text_bind_group: wgpu::BindGroup,

    screen_metadata_buffer: wgpu::Buffer,
}

impl Fig {
    fn init(
        state: &graphics::AppSkeleton,
        vertices: Vec::<Vertex>,
    ) -> Self {

        let screen_uniform_buffer = state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Screen Metadata Buffer"),
                contents: bytemuck::cast_slice(state.screen_size.size()),
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

        let atlas_texture = state.device.create_texture(
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

        state.queue.write_texture(
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
        let atlas_sampler = state.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let glyph_bind_group_layout = state.device.create_bind_group_layout(
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

        let glyph_bind_group = state.device.create_bind_group(
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
        let camera = Camera {
            // position the camera one unit up and 2 units back
            // +z is out of the screen
            eye: (0.0, 500.0, 500.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: state.config.width as f32 / state.config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 1000.0,
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout = state.device.create_bind_group_layout(
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

        let camera_bind_group = state.device.create_bind_group(
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
        let shader = state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let vertex_buffer = state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices.as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let render_pipeline = state.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    format: state.config.format,
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
        let text_storage = text::Buffer::from("Hello World");
        let mut text_uniform = text::Metadata::from(&text_storage);
        text_uniform.pixel_size = [30; 2];
        text_uniform.pixel_position = [20; 2];
        text_uniform.kerning = 100;

        let text_uniform_buffer = state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Text Metadata Buffer"),
                contents: bytemuck::cast_slice(&[text_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let text_storage_buffer = state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Text Storage Buffer"),
                contents: text_storage.pack_glyphs(),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }
        );

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

        let text_bind_group_layout = state.device.create_bind_group_layout(
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

        let text_bind_group = state.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &text_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: screen_uniform_buffer.as_entire_binding(),
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
        );
        // ====== END TEXT BIND GROUP ======

        // ====== TEXT PIPELINE ======
        let text_shader = state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("textshader.wgsl").into()),
        });

        let text_pipeline_layout =
            state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
        let text_vertex_buffer = state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Text Vertex Buffer"),
                contents: bytemuck::cast_slice(text_vertices.as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let text_pipeline = state.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    format: state.config.format,
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
            text_buffer: text_storage,
            text_bind_group,

            screen_metadata_buffer: screen_uniform_buffer,
        }
    }
}

impl graphics::Application for Fig {
    fn required_features() -> wgpu::Features {
        wgpu::Features::POLYGON_MODE_LINE
    }

    fn input(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    fn update(&mut self, queue: &wgpu::Queue) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
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

        // geometry pass
        // nested so that we release the mutable borrow of encoder before calling encoder.finish()
        {
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
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.vertex_count, 0..1);    
        }

        // text pass
        if self.text_buffer.len() > 0 {
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

            text_pass.set_pipeline(&self.text_pipeline);
            text_pass.set_bind_group(0, &self.text_bind_group, &[]);
            text_pass.set_bind_group(1, &self.glyph_bind_group, &[]);
            text_pass.set_vertex_buffer(0, self.text_vertex_buffer.slice(..));
            text_pass.draw(0..6, 0..self.text_buffer.len() as u32);
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
    let app = Fig::init(&skeleton, vertices);

    graphics::run::<Fig>(app, skeleton);
}
