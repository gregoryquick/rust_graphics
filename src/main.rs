use winit::{
    event::*,
    event_loop::{EventLoop, ControlFlow},
    window::{Window, WindowBuilder},
};

mod pipelines;

fn main() {
    use futures::executor::block_on;
    //block_on(run());
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .unwrap();

    let mut frame_pipeline = block_on(WindowDisplayPipeline::new(&window));

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput { input, .. } => match input {
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        _ => {},
                    },
                    WindowEvent::Resized(physical_size) => {
                        frame_pipeline.resize(*physical_size);
                    },
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        frame_pipeline.resize(**new_inner_size);
                    },
                    _ => {},
                }
            },
            Event::RedrawRequested(_) => {
                match frame_pipeline.draw() {
                    Ok(_) => {},
                    //Recreate the swap_chain if lost
                    Err(wgpu::SwapChainError::Lost) => frame_pipeline.resize(frame_pipeline.size),
                    //Quit if out of memory
                    Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("{:?}", e),
                }
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            },
            _ => {},
        }
    });

    println!("Run complete")
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

//Adds function to get information for reading this struct from a vertex buffer
impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float2,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [-1.00, 1.00, 0.0], tex_coords: [0.0, 0.0,] },
    Vertex { position: [-1.00, -1.00, 0.0], tex_coords: [0.0, 1.0] },
    Vertex { position: [1.00, -1.00, 0.0], tex_coords: [1.0, 1.0] },
    Vertex { position: [1.00, 1.00, 0.0], tex_coords: [1.0, 0.0] },
];

const INDICES: &[u16] = &[
    0, 1, 2,
    2, 3, 0,
];

async fn run() {
    //Create instance
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

    //Create adapter, device, and queue
    let adapter = instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
        },
    ).await.unwrap();
    let (device, queue) = adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::default(),
        },
        None, // Trace path
    ).await.unwrap();

    let texture_size = 512u32;

    //Make everytinh needed for rendering
    let texture_generation_info = pipelines::TextureGenerationPipeline::new(&device, texture_size, Vertex::desc());   

    //Create buffer for getting data out of gpu
    let u32_size = std::mem::size_of::<u32>() as u32;
    let output_buffer_size = (u32_size * texture_size * texture_size) as wgpu::BufferAddress;
    let output_buffer_desc = wgpu::BufferDescriptor {
        label: None,
        size: output_buffer_size,
        usage: wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::MAP_READ,
        mapped_at_creation: false,
    };
    let output_buffer = device.create_buffer(&output_buffer_desc);
    
    //Create vertex buffer
    use wgpu::util::DeviceExt;
    let vertex_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsage::VERTEX,
        }
    );
    
    //Create index buffer
    let index_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsage::INDEX,
        }
    );
    let num_indices = INDICES.len() as u32;
    
    //Get an encoder to build comand buffer to give to gpu
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });
    
    //Create the render pass (Mutably borrows encoder)
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
            attachment: &texture_generation_info.texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.1,
                    g: 0.1,
                    b: 0.1,
                    a: 1.0,
                }),
                store: true,
            },
        }],
        depth_stencil_attachment: None,
    });

    //Set pipline as active
    render_pass.set_pipeline(&texture_generation_info.render_pipeline);
    
    //Read from all of vertex buffer into slot 0
    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    //Read the index buffer into slot 0?
    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    //draw vertices 0-(self.num_indices-1) with instance 0
    render_pass.draw_indexed(0..num_indices, 0, 0..1);

    //Drop that encoder borrow
    drop(render_pass);

    //Copy data from texture to output buffer
    encoder.copy_texture_to_buffer(
        wgpu::TextureCopyView {
            texture: &texture_generation_info.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        }, 
        wgpu::BufferCopyView {
            buffer: &output_buffer,
            layout: wgpu::TextureDataLayout {
                offset: 0u64,
                bytes_per_row: u32_size * texture_size,
                rows_per_image: texture_size,
            },
        }, 
        wgpu::Extent3d {
            width: texture_size,
            height: texture_size,
            depth: 1,
        },
    );


    //Finish command buffer and submit to gpu queue
    queue.submit(std::iter::once(encoder.finish()));

    //Poll for processed data
    let output_buffer_slice =  output_buffer.slice(..);
    output_buffer_slice.map_async(wgpu::MapMode::Read);
    device.poll(wgpu::Maintain::Wait);

    //Get the processed data
    let result = output_buffer_slice.get_mapped_range();
    let data = result.get(..).unwrap();

    //Make image
    use image::{ImageBuffer, Rgba};
    let image_buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(texture_size, texture_size, data).unwrap();
    
    //Save image
    image_buffer.save("output/image.png").unwrap();
}

struct WindowDisplayPipeline {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

impl WindowDisplayPipeline {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        //Create adapter, device, and queue
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
            },
        ).await.unwrap();
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None, // Trace path
        ).await.unwrap();
        
        //Create swapchain
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        //Load shaders
        let vs_src = include_str!("shaders/window_display/shader.vert");
        let fs_src = include_str!("shaders/window_display/shader.frag");
        let mut compiler = shaderc::Compiler::new().unwrap();
        let vs_spirv = compiler.compile_into_spirv(vs_src, shaderc::ShaderKind::Vertex, "shader.vert", "main", None).unwrap();
        let fs_spirv = compiler.compile_into_spirv(fs_src, shaderc::ShaderKind::Fragment, "shader.frag", "main", None).unwrap();
        let vs_module_desc = wgpu::ShaderModuleDescriptor{
            label: None,
            source: wgpu::util::make_spirv(&vs_spirv.as_binary_u8()),
            flags: wgpu::ShaderFlags::empty(),
        };
        let fs_module_desc = wgpu::ShaderModuleDescriptor{
            label: None,
            source: wgpu::util::make_spirv(&fs_spirv.as_binary_u8()),
            flags: wgpu::ShaderFlags::empty(),
        };
        let vs_module = device.create_shader_module(&vs_module_desc);
        let fs_module = device.create_shader_module(&fs_module_desc);

        //Create pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
              module: &vs_module,
              entry_point: "main",
              buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module:&fs_module,
                entry_point: "main",
                targets: &[
                    wgpu::ColorTargetState {
                        format: sc_desc.format,
                        color_blend: wgpu::BlendState::REPLACE,
                        alpha_blend: wgpu::BlendState::REPLACE,
                        write_mask: wgpu::ColorWrite::ALL,
                    },
                ],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                polygon_mode: wgpu::PolygonMode::Fill,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        //Create vertex buffer
        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsage::VERTEX,
            }
        );
    
        //Create index buffer
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsage::INDEX,
            }
        );
        let num_indices = INDICES.len() as u32;

        WindowDisplayPipeline {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn draw(&mut self) -> Result<(), wgpu::SwapChainError> {
        //Get current frame
        let frame = self.swap_chain.get_current_frame()?.output;

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Frame Encoder"),
        });

        //Create the render pass (Mutably borrows encoder)
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.1,
                        b: 0.1,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        //Set pipline as active
        render_pass.set_pipeline(&self.render_pipeline);
        //Create vertex buffer in slot 0
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        //Load index buffer
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        //Draw using slot 0
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

        //Drop the encoder borrow
        drop(render_pass);

        //Finish and submit commands
        self.queue.submit(std::iter::once(encoder.finish()));

        //Return ok
        Ok(())
    }
}

