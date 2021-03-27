#![feature(array_map)]
mod state;
mod render;

use futures::executor::block_on;
use winit::event::*;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Fullscreen, WindowBuilder};

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let monitor = event_loop.primary_monitor().unwrap();
    let video_mode = monitor.video_modes().next().unwrap();

    let window = WindowBuilder::new()
        .with_visible(true)
        .with_title("Window")
        .build(&event_loop)
        .unwrap();

    //window.set_cursor_visible(true);
    
    let mut render = block_on(render::Render::new(&window, &video_mode));

    let mut state = state::State{
        boids: Vec::new(),
    };
    let initial_boids: &[state::Boid] = &[
        state::Boid{
            position: cgmath::Vector3::new(0.0, 0.0, 0.0),
            rotation: {
                let rotation_angle: f32 = 0.0;
                let eval: f32 = rotation_angle/2.0;
                cgmath::Quaternion::new(eval.cos(),0.0,0.0,eval.sin())
            }
        },
    ];

    state.boids.extend(initial_boids);

    //Event loop!
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
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
                    _ => {},
                }
            },
            Event::RedrawRequested(_) => {
                render.draw(&state);
            },
            _ => {},
        }
    });
}
