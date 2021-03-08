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

    let mut frame_pipeline = block_on(PipelineManager::new(&window));

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
                    Err(wgpu::SwapChainError::Lost) => frame_pipeline.resize(frame_pipeline.display_pipeline.size),
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
    Vertex { position: [0.000, 0.086, 0.00], tex_coords: [0.5, 0.0,] },
    Vertex { position: [-0.100, -0.086, 0.00], tex_coords: [0.0, 1.0] },
    Vertex { position: [0.100, -0.086, 0.00], tex_coords: [1.0, 1.0] },
];

const INDICES: &[u16] = &[
    0, 1, 2,
];

///Enitity instance struct
struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)).into(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &[
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We don't have to do this in code though.
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float4,
                },
            ],
        }
    }
}


///For actualy using pipelines
struct PipelineManager {
    device: wgpu::Device,
    queue: wgpu::Queue,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    display_pipeline: pipelines::WindowDisplayPipeline,
}

impl PipelineManager {
    async fn new(window: &Window) -> Self {
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
            None,
        ).await.unwrap();

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


        //Create starting triangles
        let instances =  vec![
            Instance {
                position: cgmath::Vector3::new(0.0, 0.0, 0.0),
                rotation: {
                    use std::f32::consts::PI;
                    let rotation_angle: f32 = PI/2.0;
                    let eval: f32 = rotation_angle/2.0;
                    let w = eval.cos();
                    let B = eval.sin();
                    cgmath::Quaternion::new(w,0.0,0.0,B)
                },
            },
            Instance {
                position: cgmath::Vector3::new(-0.5, 0.0, 0.0),
                rotation: cgmath::Quaternion::new(1.0, 0.0, 0.0, 0.0),
            },
            Instance {
            position: cgmath::Vector3::new(0.0, 0.5, 0.0),
                rotation: {
                    use std::f32::consts::PI;
                    let rotation_angle: f32 = PI;
                    let eval: f32 = rotation_angle/2.0;
                    let w = eval.cos();
                    let B = eval.sin();
                    cgmath::Quaternion::new(w,0.0,0.0,B)
                },
            },

        ];

        //Create buffer for storing triangle instances
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsage::VERTEX,
            }
        );

        //Make pipeline for drawing on the window
        let display_pipeline = pipelines::WindowDisplayPipeline::new(&device, &instance, window, &[Vertex::desc(), InstanceRaw::desc()]);

        
        //Return
        PipelineManager {
            device,
            queue,
            vertex_buffer,
            index_buffer,
            num_indices,
            instances,
            instance_buffer,
            display_pipeline,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.display_pipeline.sc_desc.width = new_size.width;
        self.display_pipeline.sc_desc.height = new_size.height;
        self.display_pipeline.swap_chain = self.device.create_swap_chain(&self.display_pipeline.surface, &self.display_pipeline.sc_desc);
    }

    fn draw(&mut self) -> Result<(), wgpu::SwapChainError> {
        //Get current frame
        let frame = self.display_pipeline.swap_chain.get_current_frame()?.output;

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
        render_pass.set_pipeline(&self.display_pipeline.render_pipeline);
        //Create vertex buffer in slot 0
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        //Load index buffer
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        //Load index buffer
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        //Draw using slot 0
        render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as _);

        //Drop the encoder borrow
        drop(render_pass);

        //Finish and submit commands
        self.queue.submit(std::iter::once(encoder.finish()));

        //Return ok
        Ok(())
    }
}
