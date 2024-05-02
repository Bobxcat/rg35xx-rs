use std::num::NonZeroU32;
use std::rc::Rc;
use winit::event_loop::EventLoopBuilder;
use winit::platform::wayland::EventLoopBuilderExtWayland;
use winit::window::{Window, WindowId};
use winit::{application::ApplicationHandler, keyboard::PhysicalKey};
use winit::{
    event::ElementState,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
};
use winit::{event::WindowEvent, keyboard::KeyCode};

use crate::app::{Button, Frame, Input};

struct App<A> {
    app: A,
    frame_data: Vec<u8>,
    input: Input,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
}

impl<A: crate::app::App> ApplicationHandler for App<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Rc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        let context = softbuffer::Context::new(window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

        self.window = Some(window);
        self.surface = Some(surface);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

                // Update app
                let mut frame = Frame {
                    width: 640,
                    height: 480,
                    bytespp: 4,
                    data: &mut self.frame_data,
                };
                self.app.update(&self.input, &mut frame);
                self.input.update();

                // Draw.
                let window = self.window.as_ref().unwrap();
                let surface = self.surface.as_mut().unwrap();
                let (width, height) = {
                    let size = window.inner_size();
                    (size.width, size.height)
                };
                surface
                    .resize(
                        NonZeroU32::new(width).unwrap(),
                        NonZeroU32::new(height).unwrap(),
                    )
                    .unwrap();

                let mut buffer = surface.buffer_mut().unwrap();
                for index in 0..(width * height) {
                    let y = index / width;
                    let x = index % width;
                    //let red = x % 255;
                    //let green = y % 255;
                    //let blue = (x * y) % 255;
                    if y < 480 && x < 640 {
                        let poffset = (y * 640 * 4 + x * 4) as usize;
                        let red = self.frame_data[poffset] as u32;
                        let green = self.frame_data[poffset + 1] as u32;
                        let blue = self.frame_data[poffset + 2] as u32;

                        buffer[index as usize] = blue | (green << 8) | (red << 16);
                    } else {
                        buffer[index as usize] = 0;
                    }
                }

                buffer.present().unwrap();

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(button) = match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyA) => Some(Button::PovLeft),
                    PhysicalKey::Code(KeyCode::KeyS) => Some(Button::PovDown),
                    PhysicalKey::Code(KeyCode::KeyD) => Some(Button::PovRight),
                    PhysicalKey::Code(KeyCode::KeyW) => Some(Button::PovUp),
                    PhysicalKey::Code(KeyCode::Numpad4) => Some(Button::ActionV),
                    PhysicalKey::Code(KeyCode::Numpad2) => Some(Button::ActionB),
                    PhysicalKey::Code(KeyCode::Numpad6) => Some(Button::ActionA),
                    PhysicalKey::Code(KeyCode::Numpad8) => Some(Button::ActionH),
                    PhysicalKey::Code(KeyCode::Space) => Some(Button::BumperL),
                    PhysicalKey::Code(KeyCode::Numpad0) => Some(Button::BumperR),
                    PhysicalKey::Code(KeyCode::Period) => Some(Button::MenuR),
                    PhysicalKey::Code(KeyCode::Comma) => Some(Button::MenuL),
                    _ => None,
                } {
                    self.input
                        .event(button, event.state == ElementState::Pressed);
                }
            }
            _ => (),
        }
    }
}

pub fn run_app(app: impl crate::app::App) {
    let event_loop = EventLoop::new().unwrap();

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop.set_control_flow(ControlFlow::Poll);

    // ControlFlow::Wait pauses the event loop if no events are available to process.
    // This is ideal for non-game applications that only update in response to user
    // input, and uses significantly less power/CPU time than ControlFlow::Poll.
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App {
        app,
        frame_data: vec![0; 640 * 480 * 4],
        input: Input::default(),
        window: None,
        surface: None,
    };
    event_loop.run_app(&mut app).unwrap();
}

/// Run this app using `Wayland`, and allows running the event loop on a separate thread during simulation
pub fn run_app_wayland(app: impl crate::app::App) {
    let event_loop = EventLoopBuilder::default()
        .with_wayland()
        .with_any_thread(true)
        .build()
        .expect("Could not build event loop");

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        app,
        frame_data: vec![0; 640 * 480 * 4],
        input: Input::default(),
        window: None,
        surface: None,
    };
    event_loop.run_app(&mut app).unwrap();
}
