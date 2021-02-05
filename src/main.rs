#[cfg(target_os = "macos")]
extern crate gfx_backend_metal as backend;


extern crate gfx_hal;
extern crate winit;

use std::mem::ManuallyDrop;

use gfx_hal::{
    device::Device,
    window::{Extent2D, PresentationSurface, Surface},
    queue::QueueFamily,
    Instance,
};

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder,Window},
    //window::Fullscreen
};

use shaderc::ShaderKind;

fn main() {

    let event_loop = EventLoop::new();

    const WINDOW_SIZE: [u32; 2] = [1024, 633];

    let monitor = event_loop.primary_monitor().unwrap();
    //Do the stuff to deal with different pixel densities
    let (logical_window_size, physical_window_size) = {
        use winit::dpi::{LogicalSize, PhysicalSize};
        
        let dpi = monitor.scale_factor();
        let logical: LogicalSize<u32> = WINDOW_SIZE.into();
        let physical: PhysicalSize<u32> = logical.to_physical(dpi);

        (logical, physical)
    };

    //println!("{}, {}", monitor.size().width, monitor.size().height);
    
    let mut surface_extent = Extent2D {
        width: physical_window_size.width,
        height: physical_window_size.height,
    };

    let window = WindowBuilder::new()
        .with_title("test_window")
        .with_inner_size(logical_window_size)
        //.with_fullscreen(Some(Fullscreen::Borderless(Some(monitor))))
        //.with_decorations(true)
        .build(&event_loop)
        .expect("Could not create a window!");
    
    //Get drawing surface and adapter    
    let (instance, surface, adapter) = get_hooks(&window);

    //Get actual gpu object, queue for it, command pool
    let (device, mut queue_group) = get_device(&surface,&adapter); //{

    //Make command pool to make comand buffers to send to gpu
    let (command_pool, mut command_buffer) = unsafe {
        use gfx_hal::command::Level;
        use gfx_hal::pool::{CommandPool, CommandPoolCreateFlags};

        let mut command_pool = device
            .create_command_pool(queue_group.family, CommandPoolCreateFlags::empty())
            .expect("Out of memory");

        let command_buffer = command_pool.allocate_one(Level::Primary);

        (command_pool, command_buffer)
    };

    //Get colour formating for screen
    let surface_color_format = {
        use gfx_hal::format::{ChannelType, Format};

        let supported_formats = surface
            .supported_formats(&adapter.physical_device)
            .unwrap_or(vec![]);

        let default_format = *supported_formats.get(0).unwrap_or(&Format::Rgba8Srgb);

        supported_formats
            .into_iter()
            .find(|format| format.base_format().1 == ChannelType::Srgb)
            .unwrap_or(default_format)
    };
    
    //Actual render code
    let render_pass = {
        use gfx_hal::image::Layout;
        use gfx_hal::pass::{
            Attachment, AttachmentLoadOp, AttachmentOps, AttachmentStoreOp, SubpassDesc,
        };
        
        //How to colour everything in. What is an "Attachment"?
        let color_attachment = Attachment {
            format: Some(surface_color_format),
            samples: 1,
            ops: AttachmentOps::new(AttachmentLoadOp::Clear, AttachmentStoreOp::Store),
            stencil_ops: AttachmentOps::DONT_CARE,
            layouts: Layout::Undefined..Layout::Present,
        };
        
        //What exactly is a subpass? This makes that aparently?
        let subpass = SubpassDesc {
            colors: &[(0, Layout::ColorAttachmentOptimal)],
            depth_stencil: None,
            inputs: &[],
            resolves: &[],
            preserves: &[],
        };
        
        //Use the colour technique then apply subpass?
        unsafe {
            device
                .create_render_pass(&[color_attachment], &[subpass], &[])
                .expect("Out of memory")
        }
    };

    //Structure for data pushed to shader
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    struct PushConstants {
        color: [f32; 4],
        pos: [f32; 2],
        scale: [f32; 2],
    }


    //Empty pipeline layout
    let pipeline_layout = unsafe {
        use gfx_hal::pso::ShaderStageFlags;

        let push_constant_bytes = std::mem::size_of::<PushConstants>() as u32;

        device
            .create_pipeline_layout(&[], &[(ShaderStageFlags::VERTEX, 0..push_constant_bytes)])
            .expect("Out of memory")
    };
        
    //Get shaders
    let vertex_shader = include_str!("shaders/triangle.vert");
    let fragment_shader = include_str!("shaders/triangle.frag");

    //Compile GLSL shader source to SPIR-V.
    fn compile_shader(glsl: &str, shader_kind: ShaderKind) -> Vec<u32> {
        let mut compiler = shaderc::Compiler::new().unwrap();
        let compiled_shader = compiler
            .compile_into_spirv(glsl, shader_kind, "unnamed", "main", None)
            .expect("Failed to compile shader");
        compiled_shader.as_binary().to_vec()
    }

    //Function to create pipeline with shaders
    //Pipeline created is only for the render pass it is made for?
    unsafe fn make_pipeline<B: gfx_hal::Backend>(
        device: &B::Device,
        render_pass: &B::RenderPass,
        pipeline_layout: &B::PipelineLayout,
        vertex_shader: &str,
        fragment_shader: &str,
    ) -> B::GraphicsPipeline {
        use gfx_hal::pass::Subpass;
        use gfx_hal::pso::{
            BlendState, ColorBlendDesc, ColorMask, EntryPoint, Face, GraphicsPipelineDesc,
            InputAssemblerDesc, Primitive, PrimitiveAssemblerDesc, Rasterizer, Specialization,
        };
        //Create shader modules
        let vertex_shader_module = device
            .create_shader_module(&compile_shader(vertex_shader, ShaderKind::Vertex))
            .expect("Failed to create vertex shader module");

        let fragment_shader_module = device
            .create_shader_module(&compile_shader(fragment_shader, ShaderKind::Fragment))
            .expect("Failed to create fragment shader module");

        //Peparing specific values to use with the shader modules
        let (vs_entry, fs_entry) = (
            EntryPoint {
                entry: "main",
                module: &vertex_shader_module,
                specialization: Specialization::default(),
            },
            EntryPoint {
                entry: "main",
                module: &fragment_shader_module,
                specialization: Specialization::default(),
            },
        );
        //Specify how (vertex?) shaders are used
        let primitive_assembler = PrimitiveAssemblerDesc::Vertex {
            buffers: &[],
            attributes: &[],
            input_assembler: InputAssemblerDesc::new(Primitive::TriangleList),
            vertex: vs_entry,
            tessellation: None,
            geometry: None,
        };
        //Create pipline to begin assembly
        let mut pipeline_desc = GraphicsPipelineDesc::new(
            primitive_assembler,
            Rasterizer {
                cull_face: Face::BACK,
                ..Rasterizer::FILL
            },
            Some(fs_entry),
            pipeline_layout,
            Subpass {
                index: 0,
                main_pass: render_pass,
            },
        );

        //Add color use info to pipline
        pipeline_desc.blender.targets.push(ColorBlendDesc {
            mask: ColorMask::ALL,
            blend: Some(BlendState::ALPHA),
        });
        let pipeline = device
            .create_graphics_pipeline(&pipeline_desc, None)
            .expect("Failed to create graphics pipeline");

        //Have all the piplines I want from them so destroy shader modules
        device.destroy_shader_module(vertex_shader_module);
        device.destroy_shader_module(fragment_shader_module);

        pipeline
    };

    //Create pipeline
    let pipeline = unsafe {
        make_pipeline::<backend::Backend>(
            &device,
            &render_pass,
            &pipeline_layout,
            vertex_shader,
            fragment_shader,
        )
    };
    
    //CPU-GPU syncronozation
    let submission_complete_fence = device.create_fence(true).expect("Out of memory");
    let rendering_complete_semaphore = device.create_semaphore().expect("Out of memory");


    let mut resource_holder: ResourceHolder<backend::Backend> =
        ResourceHolder(ManuallyDrop::new(Resources {
            instance,
            surface,
            device,
            command_pool,
            render_passes: vec![render_pass],
            pipeline_layouts: vec![pipeline_layout],
            pipelines: vec![pipeline],
            submission_complete_fence,
            rendering_complete_semaphore
        }));
    
    //Make and start window event loop
    let mut should_configure_swapchain = true;
    let start_time = std::time::Instant::now();
    event_loop.run(move |event, _, control_flow| {
        
        *control_flow = ControlFlow::Poll;
        
        match event{
            winit::event::Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    println!("Close button pressed, goodbye");
                     *control_flow = ControlFlow::Exit
                },
                WindowEvent::Resized(dims) => {
                    //println!("Resizing to {} by {}", dims.width, dims.height);
                    surface_extent = Extent2D {
                        width: dims.width,
                        height: dims.height,
                    };
                    should_configure_swapchain = true;
                },
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    surface_extent = Extent2D {
                        width: new_inner_size.width,
                        height: new_inner_size.height,
                    };
                    should_configure_swapchain = true;
                },
                _ => ()
            },
            //Logic part, for now just redraw
            Event::MainEventsCleared => {
                window.request_redraw()
            },
            // Here's where we'll perform our rendering.
            Event::RedrawRequested(_) => {
                let res: &mut Resources<_> = &mut resource_holder.0;
                let render_pass = &res.render_passes[0];
                let pipeline_layout = &res.pipeline_layouts[0];
                let pipeline = &res.pipelines[0];
                
                unsafe {
                    use gfx_hal::pool::CommandPool;

                    // We refuse to wait more than a second, to avoid hanging.
                    let render_timeout_ns = 1_000_000_000;

                    // Graphics commands may execute asynchronously, so to
                    // ensure we're finished rendering the previous frame
                    // before starting this new one, we wait here for the
                    // rendering to signal the `submission_complete_fence` from
                    // the previous frame.
                    //
                    // This may not be the most efficient option - say if you
                    // wanted to render more than one frame simulatneously
                    // - but for our example, it simplifies things.
                    res.device
                        .wait_for_fence(&res.submission_complete_fence, render_timeout_ns)
                        .expect("Out of memory or device lost");

                    // Once the fence has been signalled, we must reset it
                    res.device
                        .reset_fence(&res.submission_complete_fence)
                        .expect("Out of memory");

                    // This clears out the previous frame's command buffer and
                    // returns it to the pool for use this frame.
                    res.command_pool.reset(false);
                }

                //Reconfigure swapchain
                if should_configure_swapchain {
                    use gfx_hal::window::SwapchainConfig;

                    let caps = res.surface.capabilities(&adapter.physical_device);

                    // We pass our `surface_extent` as a desired default, but
                    // it may return us a different value, depending on what it
                    // supports.
                    let mut swapchain_config =
                        SwapchainConfig::from_caps(&caps, surface_color_format, surface_extent);

                    // If our device supports having 3 images in our swapchain,
                    // then we want to use that.
                    //
                    // This seems to fix some fullscreen slowdown on macOS.
                    if caps.image_count.contains(&3) {
                        swapchain_config.image_count = 3;
                    }

                    // In case the surface returned an extent different from
                    // the size we requested, we update our value.
                    surface_extent = swapchain_config.extent;

                    unsafe {
                        res.surface
                            .configure_swapchain(&res.device, swapchain_config)
                            .expect("Failed to configure swapchain");
                    };

                    should_configure_swapchain = false;
                }

                //Load surface swapchain
                let surface_image = unsafe {
                    // We refuse to wait more than a second, to avoid hanging.
                    let acquire_timeout_ns = 1_000_000_000;

                    match res.surface.acquire_image(acquire_timeout_ns) {
                        Ok((image, _)) => image,
                        Err(_) => {
                            should_configure_swapchain = true;
                            return;
                        }
                    }
                };

                //Load images to framebuffer
                let framebuffer = unsafe {
                    use std::borrow::Borrow;

                    use gfx_hal::image::Extent;

                    res.device
                        .create_framebuffer(
                            render_pass,
                            vec![surface_image.borrow()],
                            Extent {
                                width: surface_extent.width,
                                height: surface_extent.height,
                                depth: 1,
                            },
                        )
                        .unwrap()
                };

                //Make viewport
                let viewport = {
                    use gfx_hal::pso::{Rect, Viewport};

                    Viewport {
                        rect: Rect {
                            x: 0,
                            y: 0,
                            w: surface_extent.width as i16,
                            h: surface_extent.height as i16,
                        },
                        depth: 0.0..1.0,
                    }
                };

                // This anim oscillates smoothly between 0.0 and 1.0.
                let anim = start_time.elapsed().as_secs_f32().sin() * 0.5 + 0.5;
                let triangles = &[
                    // Small <-> big animated triangle
                    PushConstants {
                        color: [1.0, 1.0, 1.0, 1.0],
                        pos: [0.0, 0.0],
                        scale: [0.33 + anim * 0.66, 0.33 + anim * 0.66],
                    },
                ];

                /// Returns a view of a struct as a slice of `u32`s.
                unsafe fn push_constant_bytes<T>(push_constants: &T) -> &[u32] {
                    let size_in_bytes = std::mem::size_of::<T>();
                    let size_in_u32s = size_in_bytes / std::mem::size_of::<u32>();
                    let start_ptr = push_constants as *const T as *const u32;
                    std::slice::from_raw_parts(start_ptr, size_in_u32s)
                }           
                
                //Use command buffer
                unsafe {
                    use gfx_hal::command::{
                        ClearColor, ClearValue, CommandBuffer, CommandBufferFlags, SubpassContents,
                    };

                    //Start command buffer.
                    command_buffer.begin_primary(CommandBufferFlags::ONE_TIME_SUBMIT);

                    //Specify which part of surface is used
                    command_buffer.set_viewports(0, &[viewport.clone()]);
                    command_buffer.set_scissors(0, &[viewport.rect]);

                    //Remderpass
                    command_buffer.begin_render_pass(
                        render_pass,
                        &framebuffer,
                        viewport.rect,
                        &[ClearValue {
                            color: ClearColor {
                                float32: [0.0, 0.0, 0.0, 1.0],
                            },
                        }],
                        SubpassContents::Inline,
                    );

                    //Set pipeline in use
                    command_buffer.bind_graphics_pipeline(pipeline);
                    
                    //Draw each triangle in triangles
                    for triangle in triangles {
                        use gfx_hal::pso::ShaderStageFlags;
                        
                        //Push vertex shader constants
                        command_buffer.push_graphics_constants(
                            pipeline_layout,
                            ShaderStageFlags::VERTEX,
                            0,
                            push_constant_bytes(triangle),
                        );
                         //Draw vertices 0-2 in instance 0
                        command_buffer.draw(0..3, 0..1);
                    }

                    //Finish renderpass
                    command_buffer.end_render_pass();
                    //Close command buffer
                    command_buffer.finish();
                }

                //Submit command buffer for use
                unsafe {
                    use gfx_hal::queue::{CommandQueue, Submission};

                    //Make submission
                    let submission = Submission {
                        command_buffers: vec![&command_buffer],
                        wait_semaphores: None,
                        signal_semaphores: vec![&res.rendering_complete_semaphore],
                    };

                    //Submit to queue
                    queue_group.queues[0].submit(submission, Some(&res.submission_complete_fence));

                    //Display compute result
                    let result = queue_group.queues[0].present(
                        &mut res.surface,
                        surface_image,
                        Some(&res.rendering_complete_semaphore),
                    );

                    //If presenting failed reconfigure swapchain
                    should_configure_swapchain |= result.is_err();

                    //Destroy this frames framebuffer
                    res.device.destroy_framebuffer(framebuffer);
                }
            },
            _ => ()
        }
    });
}

struct Resources<B: gfx_hal::Backend> {
        instance: B::Instance,
        surface: B::Surface,
        device: B::Device,
        command_pool: B::CommandPool,
        render_passes: Vec<B::RenderPass>,
        pipeline_layouts: Vec<B::PipelineLayout>,
        pipelines: Vec<B::GraphicsPipeline>,
        submission_complete_fence: B::Fence,
        rendering_complete_semaphore: B::Semaphore,
    }

//Need this stuff for some reason or another
struct ResourceHolder<B: gfx_hal::Backend>(ManuallyDrop<Resources<B>>);

//Properly destroying the struct
impl<B: gfx_hal::Backend> Drop for ResourceHolder<B> {
    fn drop(&mut self) {
        unsafe {
            // We are moving the `Resources` out of the struct...
            let Resources {
                instance,
                mut surface,
                device,
                command_pool,
                render_passes,
                pipeline_layouts,
                pipelines,
                submission_complete_fence,
                rendering_complete_semaphore,
            } = ManuallyDrop::take(&mut self.0);

            // ... and destroying them individually:
            device.destroy_semaphore(rendering_complete_semaphore);
            device.destroy_fence(submission_complete_fence);
            for pipeline in pipelines {
                device.destroy_graphics_pipeline(pipeline);
            }
            for pipeline_layout in pipeline_layouts {
                device.destroy_pipeline_layout(pipeline_layout);
            }
            for render_pass in render_passes {
                device.destroy_render_pass(render_pass);
            }
            device.destroy_command_pool(command_pool);
            surface.unconfigure_swapchain(&device);
            instance.destroy_surface(surface);
        }
    }
}

fn get_hooks(window: &Window) -> (backend::Instance, backend::Surface, gfx_hal::adapter::Adapter<backend::Backend>) {
    let instance = backend::Instance::create("test_window", 1).unwrap();
    let surface = unsafe{
        instance.create_surface(window).unwrap()
    };
    let adapter = instance.enumerate_adapters().remove(0);
    (instance, surface, adapter)
}

fn get_device(surface: &backend::Surface, adapter: &gfx_hal::adapter::Adapter<backend::Backend>) -> (backend::Device, gfx_hal::queue::QueueGroup<backend::Backend>) {
    let queue_family = adapter
        .queue_families
        .iter()
        .find(|family| {
            surface.supports_queue_family(family) && family.queue_type().supports_graphics()
        })
        .expect("No compatible queue family found");

    let mut gpu = unsafe {
        use gfx_hal::adapter::PhysicalDevice;

        adapter
            .physical_device
            .open(&[(queue_family, &[1.0])], gfx_hal::Features::empty())
            .expect("Failed to open device")
    };

    (gpu.device, gpu.queue_groups.pop().unwrap())
}
