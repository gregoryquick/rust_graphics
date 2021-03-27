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
    use std::f32::consts::PI;
    let initial_boids: &[state::Boid] = &[
        state::Boid{
            position: cgmath::Vector3::new(0.0, 0.5, 0.0),
            velocity: cgmath::Vector3::new(0.0, -1.0, 0.0),
            rotation: {
                let rotation_angle: f32 = PI;
                let eval: f32 = rotation_angle/2.0;
                cgmath::Quaternion::new(eval.cos(),0.0,0.0,eval.sin())
            },
            angular_velocity: {
                cgmath::Quaternion::new(0.0,0.0,0.0,0.0)
            },
        },
        state::Boid{
            position: cgmath::Vector3::new(-0.3, 0.2, 0.0),
            velocity: cgmath::Vector3::new(0.0, 0.0, 0.0),
            rotation: {
                let rotation_angle: f32 = -PI/2.0;
                let eval: f32 = rotation_angle/2.0;
                cgmath::Quaternion::new(eval.cos(),0.0,0.0,eval.sin())
            },
            angular_velocity: {
                cgmath::Quaternion::new(0.0,0.0,0.0,1.0)
            },
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
                let delta_t: f32 = 0.001;
                //println!("Stepped {}", delta_t);
                state::State::update(&mut state, &delta_t);
                render.draw(&state);
                window.request_redraw();
            },
            _ => {},
        }
    });
}
