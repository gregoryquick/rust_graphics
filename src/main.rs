fn main() {
    use futures::executor::block_on;
    block_on(run());
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

//Adds function to get information for reading this struct from a vertex buffer
impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
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
    //Create adapter, device, and queue
    let adapter = wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
            compatible_surface: None,
        },
        wgpu::BackendBit::PRIMARY,
    ).await.unwrap();
    let (device, queue) = adapter.request_device(&Default::default()).await;

    //Create texture
    let texture_size = 256u32;
    let texture_desc = wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: texture_size,
            height: texture_size,
            depth: 1,
        },
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsage::COPY_SRC | wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        label: None,
    };
    let texture = device.create_texture(&texture_desc);
    let texture_view = texture.create_default_view();

    //Create buffer for image read/write
    let u32_size = std::mem::size_of::<u32>() as u32;
    let output_buffer_size = (u32_size * texture_size * texture_size) as wgpu::BufferAddress;
    let output_buffer_desc = wgpu::BufferDescriptor {
        size: output_buffer_size,
        usage: wgpu::BufferUsage::COPY_DST
            // this tells wpgu that we want to read this buffer from the cpu
            | wgpu::BufferUsage::MAP_READ,
        label: None,
    };
    let output_buffer = device.create_buffer(&output_buffer_desc);

    //Load shaders
    let vs_src = include_str!("shaders/shader.vert");
    let fs_src = include_str!("shaders/shader.frag");
    let mut compiler = shaderc::Compiler::new().unwrap();
    let vs_spirv = compiler.compile_into_spirv(vs_src, shaderc::ShaderKind::Vertex, "shader.vert", "main", None).unwrap();
    let fs_spirv = compiler.compile_into_spirv(fs_src, shaderc::ShaderKind::Fragment, "shader.frag", "main", None).unwrap();
    let vs_module = device.create_shader_module(wgpu::util::make_spirv(&vs_spirv.as_binary_u8()));
    let fs_module = device.create_shader_module(wgpu::util::make_spirv(&fs_spirv.as_binary_u8()));

    //Create render pipeline
    let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            //Determine how to rasterize
            rasterization_state: Some(
                wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
                clamp_depth: false,
            }),
            //How to procces colors
            color_states: &[
                    wgpu::ColorStateDescriptor {
                    format: texture_desc.format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                },
            ],
            //What are my primatives
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            //Not using depth or stencill buffer
            depth_stencil_state: None,
            //Vertex buffer info
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[Vertex::desc()],
            },
            //Not using multisampling
            sample_count: 1,
            //Use all samples
            sample_mask: !0,
            //No anti-aliasing stuff
            alpha_to_coverage_enabled: false,
        });

    //Create vertex and index buffers
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(VERTICES),
        usage: wgpu::BufferUsage::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsage::INDEX,
        });

    //Figure out number of indices
    let num_indices = INDICES.len() as u32;
    
    //Encoder
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: None,
    });

    //Create the render pass
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
            attachment: &texture_view,
            resolve_target: None,
            load_op: wgpu::LoadOp::Clear,
            store_op: wgpu::StoreOp::Store,
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
    render_pass.set_pipeline(&render_pipeline);
    //Read from all of vertex buffer into slot 0
    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    //Get the index buffer
    render_pass.set_index_buffer(index_buffer.slice(..));
    //draw vertices 0-(self.num_indices-1) with instance 0
    render_pass.draw_indexed(0..num_indices, 0, 0..1);

    //Drop that encoder borrow
    drop(render_pass);

    encoder.copy_texture_to_buffer(
        wgpu::TextureCopyView {
            texture: &texture,
            mip_level: 0,
            array_layer: 0,
            origin: wgpu::Origin3d::ZERO,
        }, 
        wgpu::BufferCopyView {
            buffer: &output_buffer,
            offset: 0,
            bytes_per_row: u32_size * texture_size,
            rows_per_image: texture_size,
        }, 
        texture_desc.size,
    );

    //Finish command buffer and submit to gpu queue
    queue.submit(&[encoder.finish()]);

    //Read from output buffer then move on
    let mapping = output_buffer.map_read(0, output_buffer_size);
    device.poll(wgpu::Maintain::Wait);
    let result = mapping.await.unwrap();
    let data = result.as_slice();

    //Format as image
    use image::{ImageBuffer, Rgba};
    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
        texture_size,
        texture_size,
        data,
    ).unwrap();

    //Save image
    buffer.save("image.png").unwrap();
}

