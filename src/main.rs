mod reader;
mod camera;

use graphics::data::Vertex;
use wgpu::util::DeviceExt;

use gcode::GCommand;
use camera::*;



struct Fig {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,
}

impl Fig {
    fn init(
        state: &graphics::AppSkeleton,
        vertices: Vec::<Vertex>,
    ) -> Self {
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

        Self {
            render_pipeline,
            vertex_buffer,
            vertex_count: vertices.len() as u32,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller: CameraController::new(5.0),
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
    
        // submit will accept anything that implements IntoIter
        queue.submit(std::iter::once(encoder.finish()));
        output.present();
    
        Ok(())
    }
}

fn main() {
    let skeleton = pollster::block_on(graphics::build::<Fig>("fig"));

    // open gcode file
    let mut reader = reader::BufferedReader::open("test2.gcode")
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
