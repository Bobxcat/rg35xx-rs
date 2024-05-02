use clap::Parser;
use enum_iterator::all;
use palette::LinSrgb;

use rg35xx::app::{App, Buttons, Frame, Input};

struct InputTestApp;

impl App for InputTestApp {
    fn update(&mut self, input: &Input, frame: &mut Frame) {
        frame.fill_rect(32, 32, 32, 32, LinSrgb::new(255, 255, 0));
        for (i, button) in all::<Buttons>().enumerate() {
            if input.just_pressed(button) {
                frame.fill_rect(0, i * 32, 32, 32, LinSrgb::new(255, 0, 0));
            }
            if input.just_released(button) {
                frame.fill_rect(0, i * 32, 32, 32, LinSrgb::new(0, 0, 0));
            }
        }
    }
}

#[derive(Parser)]
struct Args {
    #[arg(long)]
    sim: bool,
}

fn main() {
    let args = Args::parse();

    let app = rg35xx::make_menu();
    if args.sim {
        rg35xx::sim::run_app(app);
    } else {
        rg35xx::rg35xx::run_app(app);
    }
}
