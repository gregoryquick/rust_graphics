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

