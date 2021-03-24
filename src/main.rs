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

    let state = state::State{
        boids: Vec::new(),
    };

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
