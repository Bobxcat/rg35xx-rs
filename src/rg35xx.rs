use std::{
    sync::mpsc::{channel, Sender},
    time::Duration,
};

use framebuffer::Framebuffer;
use pasts::Loop;
use std::task::Poll::{self, Pending};
use stick::{Controller, Event, Listener};

use crate::app::{App, Buttons, Frame, Input};

type Exit = usize;

struct State {
    listener: Listener,
    controllers: Vec<Controller>,
    rumble: (f32, f32),
    channel: Sender<Event>,
}

impl State {
    fn connect(&mut self, controller: Controller) -> Poll<Exit> {
        println!(
            "Connected p{}, id: {:016X}, name: {}",
            self.controllers.len() + 1,
            controller.id(),
            controller.name(),
        );
        self.controllers.push(controller);
        Pending
    }

    fn event(&mut self, id: usize, event: Event) -> Poll<Exit> {
        let player = id + 1;
        println!("p{}: {}", player, event);
        self.channel.send(event).unwrap();
        match event {
            Event::Disconnect => {
                self.controllers.swap_remove(id);
            }
            Event::MenuR(true) => {
                self.controllers[id].rumble(1.0);
            } // return Ready(player),
            Event::ActionA(pressed) => {
                self.controllers[id].rumble(f32::from(u8::from(pressed)));
            }
            Event::ActionB(pressed) => {
                self.controllers[id].rumble(0.5 * f32::from(u8::from(pressed)));
            }
            Event::BumperL(pressed) => {
                self.rumble.0 = f32::from(u8::from(pressed));
                self.controllers[id].rumble(self.rumble);
            }
            Event::BumperR(pressed) => {
                self.rumble.1 = f32::from(u8::from(pressed));
                self.controllers[id].rumble(self.rumble);
            }
            _ => {}
        }
        Pending
    }
}

async fn event_loop(sender: Sender<Event>) {
    let mut state = State {
        listener: Listener::default(),
        controllers: Vec::new(),
        rumble: (0.0, 0.0),
        channel: sender,
    };

    let player_id = Loop::new(&mut state)
        .when(|s| &mut s.listener, State::connect)
        .poll(|s| &mut s.controllers, State::event)
        .await;

    println!("p{} ended the session", player_id);
}

pub fn run_app(mut app: impl App) {
    let mut framebuffer = Framebuffer::new("/dev/fb0").unwrap();

    let width = framebuffer.var_screen_info.xres as usize;
    let height = framebuffer.var_screen_info.yres as usize;
    let line_length = framebuffer.fix_screen_info.line_length;
    let bytespp = framebuffer.var_screen_info.bits_per_pixel as usize / 8;

    /*println!(
        "w={} h={} line_length={} bytespp={}",
        w, h, line_length, bytespp
    );*/
    println!("{:#?}", framebuffer.var_screen_info);
    println!("{:#?}", framebuffer.fix_screen_info);

    let is_double_buffered =
        framebuffer.var_screen_info.yres_virtual != framebuffer.var_screen_info.yres;

    let (button_tx, button_rx) = channel();
    std::thread::spawn(|| {
        pasts::block_on(event_loop(button_tx));
    });

    let mut input_state = Input::default();
    let mut is_high_frame = false;
    let mut frame_data = vec![0; width * height * bytespp];
    let mut frame = Frame {
        width,
        height,
        bytespp,
        data: &mut frame_data,
    };
    loop {
        // Handle the input buttons
        input_state.update();
        let mut exit_set = false;
        while let Ok(event) = button_rx.try_recv() {
            if let Event::Exit(exit) = event {
                if exit {
                    exit_set = true;
                    break;
                }
            }
            if let Some((button, value)) = match event {
                Event::BumperL(v) => Some((Buttons::BumperL, v)),
                Event::BumperR(v) => Some((Buttons::BumperR, v)),
                Event::PovDown(v) => Some((Buttons::PovDown, v)),
                Event::PovUp(v) => Some((Buttons::PovUp, v)),
                Event::PovLeft(v) => Some((Buttons::PovLeft, v)),
                Event::PovRight(v) => Some((Buttons::PovRight, v)),
                Event::MenuL(v) => Some((Buttons::MenuL, v)),
                Event::MenuR(v) => Some((Buttons::MenuR, v)),
                Event::ActionH(v) => Some((Buttons::ActionH, v)),
                Event::ActionV(v) => Some((Buttons::ActionV, v)),
                Event::ActionB(v) => Some((Buttons::ActionB, v)),
                Event::ActionA(v) => Some((Buttons::ActionA, v)),
                // JoyZ and CamZ are the two triggers, but they're joysticks
                _ => None,
            } {
                input_state.event(button, value);
            }
        }
        if exit_set {
            break;
        }

        // Update the active app
        app.update(&input_state, &mut frame);

        // Write out the frame to the inactive buffer
        let yoffset = if is_high_frame && is_double_buffered {
            480
        } else {
            0
        };
        framebuffer.write_frame_offset(frame.data, 640 * yoffset * 4);

        if is_double_buffered {
            // Flip the active buffers
            let mut var_info = Framebuffer::get_var_screeninfo(&framebuffer.device).unwrap();
            var_info.yoffset = yoffset as u32;
            Framebuffer::put_var_screeninfo(&framebuffer.device, &var_info).unwrap();
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}
