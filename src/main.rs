mod reader;
mod text;
//mod winfont;

use std::ops::{Deref, DerefMut};

use glam::Vec3;
use graphics::vector::Vector3;

use graphics::app::{App, AppBuilder, AppSkeleton, Application};
use graphics::data::{Vertex, ScreenSize};
use graphics::text::{GlyphAtlas, Text2D};
use graphics::pipeline::{NamedPipeline, PipelineBuilder};
use graphics::camera::{fps_camera::CameraLegacy, CameraController};
use wgpu::util::DeviceExt;
use winit::event::{WindowEvent, KeyboardInput};

use gcode::GCommand;

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



// holds mutable elements that need to be maintained to ensure the app's state is consistent
struct AppState {
    vertex_buffer: wgpu::Buffer,
    vertex_count: usize,

    //controller: CameraController,
}

struct Fig<'a, S>(App<'a, S>);

impl<'a, S> Deref for Fig<'a, S> {
    type Target = App<'a, S>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, S> DerefMut for Fig<'a, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

//impl graphics::Application for FigLegacy {
impl graphics::app::Application for Fig<'static, AppState> {
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
    let skeleton = pollster::block_on(graphics::build::<Fig<AppState>>("fig"));
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
        eye: Vec3 { x: 0.0, y: 500.0, z: 500.0 },
        // have it look at the origin
        target: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
        // which way is "up"
        up: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
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

    let app = App::for_skeleton(skeleton)
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

    // tbh i think i can add named pipelines after i build() the appbuilder.
    // its just pushing it onto a vector that gets iterated through during render, shouldnt be an issue
    let (skeleton, app) = app
        .add_pipeline(wireframe_pipeline)
        .add_pipeline(text_pipeline)
        .build();

    graphics::run::<Fig<AppState>>(Fig(app), skeleton);
}
