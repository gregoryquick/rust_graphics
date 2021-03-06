///Pipeline for generating terrain texture
pub struct TextureGenerationPipeline {
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    pub render_pipeline: wgpu::RenderPipeline,
}

impl TextureGenerationPipeline {
    pub fn new(device: &wgpu::Device, texture_size: u32, vertex_desc: wgpu::VertexBufferLayout) -> Self {
        //Create Texture
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

        //Load shaders
        let vs_src = include_str!("shaders/texture_gen/shader.vert");
        let fs_src = include_str!("shaders/texture_gen/shader.frag");
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
              buffers: &[vertex_desc],
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
        
        TextureGenerationPipeline {
            texture,
            texture_view,
            render_pipeline,
        }
    }
}

///Pipeline for drawing to window
pub struct WindowDisplayPipeline {
    pub size: winit::dpi::PhysicalSize<u32>,
    pub surface: wgpu::Surface,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub render_pipeline: wgpu::RenderPipeline,
}

use winit::{
    window::Window,
};

impl WindowDisplayPipeline {
     pub fn new(device: &wgpu::Device, instance: &wgpu::Instance, window: &Window, vertex_desc: wgpu::VertexBufferLayout) -> Self {
        //Get window hooks and information
        let size = window.inner_size();
        let surface = unsafe { instance.create_surface(window) };

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
            label: Some("Display Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Display Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
              module: &vs_module,
              entry_point: "main",
              buffers: &[vertex_desc],
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

        //Return
        WindowDisplayPipeline{
            size,
            surface,
            sc_desc,
            swap_chain,
            render_pipeline,
        }
     }
}
