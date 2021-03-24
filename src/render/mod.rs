mod buffer;

use winit::monitor::VideoMode;
use winit::window::Window;

use crate::state;

use buffer::*;

#[allow(dead_code)]
pub struct Render {
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    staging_belt: wgpu::util::StagingBelt,
}

impl Render {
    #[allow(dead_code)]
    pub fn width(&self) -> f32 {
        self.sc_desc.width as f32
    }

    #[allow(dead_code)]
    pub fn height(&self) -> f32 {
        self.sc_desc.height as f32
    }

    pub async fn new(window: &Window, video_mode: &VideoMode) -> Self {
        //gpu handle
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
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
        
        let size = video_mode.size();

        //Create swap chain
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: adapter.get_swap_chain_preferred_format(&surface),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        

        //Load shader binaries
        let vs_src = include_str!("shaders/shader.vert");
        let fs_src = include_str!("shaders/shader.frag");
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
        
        //Create pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Display Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let pipeline = create_render_pipeline(
            &device,
            &render_pipeline_layout,
            sc_desc.format,
            &[Vertex::DESC],
            vs_module_desc,
            fs_module_desc,
        );

        //Create buffers
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: Vertex::SIZE * 4 * 3,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: U32_SIZE * 6 * 3,
            usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        //Used for loading data from cpu to gpu buffers
        let staging_belt = wgpu::util::StagingBelt::new(1024);

        //Return
        Self {
            surface,
            adapter,
            device,
            queue,
            sc_desc,
            swap_chain,
            pipeline,
            vertex_buffer,
            index_buffer,
            staging_belt,
        }
    }

    pub fn draw(&mut self, state: &state::State) {
        //Create encoder for frame
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Frame Encoder"),
        });

        //TODO Load state data into buffers
        let num_indices = {
            let (stg_vertex, stg_index, num_indices) = BoidBufferBuilder::new().push_boid().build(&self.device);
            //stg_vertex.copy_to_buffer(&mut encoder, &self.vertex_buffer);

            let VERTICES: &[Vertex] = &[
                Vertex {position: [0.000, 0.086].into()},
                Vertex {position: [-0.100, -0.086].into()},
                Vertex {position: [0.100, -0.086].into()},
            ];
            
            use wgpu::util::{BufferInitDescriptor, DeviceExt};
            self.vertex_buffer = self.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(VERTICES),
                    usage: wgpu::BufferUsage::VERTEX,
                }
            );            

            let INDICES: &[u16] = &[0, 1, 2];
            
            //stg_index.copy_to_buffer(&mut encoder, &self.index_buffer);
            self.index_buffer = self.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(INDICES),
                    usage: wgpu::BufferUsage::INDEX,
                }
            );
            
            //num_indices
            3
        };

        match self.swap_chain.get_current_frame() {
            Ok(frame) => {
                //Create the render pass (Mutably borrows encoder)
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.output.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.1,
                                b: 0.1,
                                a: 1.0,
                            }),store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });

                //Draw from index buffer
                //let num_indices = 0;
                if num_indices != 0 {
                    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.set_pipeline(&self.pipeline);
                    render_pass.draw_indexed(0..num_indices, 0, 0..1);
                }
                
                //Encoder borrow is gone now!
                drop(render_pass);

                //Add command buffer to queue!
                self.queue.submit(std::iter::once(encoder.finish()));
            },
            Err(wgpu::SwapChainError::Outdated) => {
                self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
            },
            Err(e) => {
                eprintln!("Error: {}", e);
            },
        }
    }
}


fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    vs_module_desc: wgpu::ShaderModuleDescriptor,
    fs_module_desc: wgpu::ShaderModuleDescriptor,
) -> wgpu::RenderPipeline {
    //Load shader Modules
    let vs_module = device.create_shader_module(&vs_module_desc);
    let fs_module = device.create_shader_module(&fs_module_desc);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: "main",
            buffers: vertex_layouts,
        },
        fragment: Some(wgpu::FragmentState {
            module: &fs_module,
            entry_point: "main",
            targets: &[wgpu::ColorTargetState {
                format: color_format,
                color_blend: wgpu::BlendState::REPLACE,
                alpha_blend: wgpu::BlendState::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::Back,
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
    })
}
