fn main() {
    use futures::executor::block_on;
    block_on(run());
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

    //Create Texture
    let texture_size = 256u32;
    let texture_desc = wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: texture_size,
            height: texture_size,
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsage::COPY_SRC | wgpu::TextureUsage::RENDER_ATTACHMENT,
        label: None,
    };
    let texture = device.create_texture(&texture_desc);
    let texture_view_desc = wgpu::TextureViewDescriptor{
        base_mip_level: 0,
        level_count:None,
        array_layer_count: None,
        base_array_layer: 0,
        dimension: None,
        format: None,
        aspect: wgpu::TextureAspect::All,
        label: None,
    };
    let texture_view = texture.create_view(&texture_view_desc);
    
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

    //Load shaders
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
    let vs_module = device.create_shader_module(&vs_module_desc);
    let fs_module = device.create_shader_module(&fs_module_desc);

    //Create render pipeline
    let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
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
                    format: texture_desc.format,
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
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsage::VERTEX,
        });
    
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
            attachment: &texture_view,
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
    render_pass.set_pipeline(&render_pipeline);
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
            texture: &texture,
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
        texture_desc.size,
    );


    //Finish command buffer and submit to gpu queue
    queue.submit(std::iter::once(encoder.finish()));

    //Poll for processed data
    let bufferSlice =  output_buffer.slice(..);
    bufferSlice.map_async(wgpu::MapMode::Read);
    device.poll(wgpu::Maintain::Wait);

    //Get the processed data
    let result = bufferSlice.get_mapped_range();
    let data = result.get(..).unwrap();

    //Make image
    use image::{ImageBuffer, Rgba};
    let image_buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(texture_size, texture_size, data).unwrap();
    
    //Save image
    image_buffer.save("image.png").unwrap();
}

