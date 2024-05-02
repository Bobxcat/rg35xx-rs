use palette::LinSrgb;

use crate::app::{App, Button};

struct MenuEntry {
    name: String,
    builder: Box<dyn Fn() -> Box<dyn App>>,
}

#[derive(Default)]
pub struct MenuApp {
    apps: Vec<MenuEntry>,
    current_app: usize,
    app: Option<Box<dyn App>>,
}

impl MenuApp {
    pub fn register_app<A: 'static + Default + App, S: Into<String>>(&mut self, name: S) {
        self.apps.push(MenuEntry {
            name: name.into(),
            builder: Box::new(|| Box::<A>::default()),
        });
    }
}

impl App for MenuApp {
    fn update(&mut self, input: &crate::app::Input, frame: &mut crate::app::Frame) {
        if let Some(app) = self.app.as_mut() {
            app.update(input, frame);
            return;
        }

        frame.fill_rect(0, 0, frame.width(), frame.height(), LinSrgb::new(0, 0, 0));

        for (i, app) in self.apps.iter().enumerate() {
            frame.text(
                "fonts/Ubuntu-B.ttf",
                200,
                50 + i * 40,
                36.0,
                if i == self.current_app {
                    LinSrgb::new(255, 255, 255)
                } else {
                    LinSrgb::new(255, 0, 0)
                },
                &app.name,
            );
        }
        if input.just_pressed(Button::PovUp) {
            self.current_app = self.current_app.saturating_sub(1);
        }
        if input.just_pressed(Button::PovDown) {
            self.current_app = (self.current_app + 1).min(self.apps.len() - 1);
        }
        if input.just_pressed(Button::ActionA) {
            // Start the app...
            self.app = Some((self.apps[self.current_app].builder)());
        }
    }
}
