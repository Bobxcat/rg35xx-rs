use enum_iterator::{all, Sequence};
use enum_map::{Enum, EnumMap};
use include_dir::{include_dir, Dir};
use palette::LinSrgb;
use rusttype::{point, Scale};

#[derive(Default, Clone, Copy)]
pub struct ButtonState {
    pressed: bool,
    previous: bool,
}

impl ButtonState {
    fn pressed(&self) -> bool {
        self.pressed
    }

    fn just_pressed(&self) -> bool {
        self.pressed && !self.previous
    }

    fn just_released(&self) -> bool {
        !self.pressed && self.previous
    }

    fn just_changed(&self) -> bool {
        self.pressed != self.previous
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Sequence)]
pub enum Button {
    PovDown,
    PovUp,
    PovLeft,
    PovRight,
    BumperL,
    BumperR,
    MenuL,
    MenuR,
    ActionH,
    ActionV,
    ActionB,
    ActionA,
}

#[derive(Default, Clone)]
pub struct Input {
    buttons: EnumMap<Button, ButtonState>,
}

impl Input {
    pub fn update(&mut self) {
        for button in all::<Button>() {
            self.buttons[button].previous = self.buttons[button].pressed;
        }
    }

    pub fn event(&mut self, button: Button, value: bool) {
        self.buttons[button].pressed = value;
    }

    pub fn pressed(&self, button: Button) -> bool {
        self.buttons[button].pressed()
    }

    pub fn just_pressed(&self, button: Button) -> bool {
        self.buttons[button].just_pressed()
    }

    pub fn just_released(&self, button: Button) -> bool {
        self.buttons[button].just_released()
    }

    pub fn just_changed(&self, button: Button) -> bool {
        self.buttons[button].just_changed()
    }
}

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

pub struct Frame<'a> {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) bytespp: usize,
    pub data: &'a mut [u8],
}

impl<'a> Frame<'a> {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn bytespp(&self) -> usize {
        self.bytespp
    }

    pub fn put_pixel(&mut self, x: usize, y: usize, color: LinSrgb<u8>) {
        if x >= self.width || y >= self.height {
            return;
        }
        let pixel_offset = y * self.width * self.bytespp + x * self.bytespp;
        if self.bytespp == 4 {
            self.data[pixel_offset] = color.red;
            self.data[pixel_offset + 1] = color.green;
            self.data[pixel_offset + 2] = color.blue;
        } else if self.bytespp == 2 {
            // Format is 5-6-5 (probably)
            let d = ((color.red as u16 >> 3) << 11)
                | ((color.green as u16 >> 2) << 5)
                | (color.blue as u16 >> 3);
            let parts = d.to_le_bytes();
            self.data[pixel_offset] = parts[0];
            self.data[pixel_offset + 1] = parts[1];
        } else {
            panic!("Unknown bytespp {}", self.bytespp);
        }
    }

    pub fn fill_rect(
        &mut self,
        startx: usize,
        starty: usize,
        width: usize,
        height: usize,
        color: LinSrgb<u8>,
    ) {
        //assert!(startx + width < self.width);
        //assert!(starty + height < self.height);
        for y in starty..starty + height {
            for x in startx..startx + width {
                self.put_pixel(x, y, color);
            }
        }
    }

    pub fn text(
        &mut self,
        font: &str,
        x: usize,
        y: usize,
        fontsize: f32,
        color: LinSrgb<u8>,
        s: &str,
    ) {
        let file = ASSETS.get_file(font).unwrap();
        let font = rusttype::Font::try_from_bytes(file.contents()).unwrap();
        //let fontsize = fontsize;
        //let pixel_height = fontsize.ceil() as usize;

        let offset = point(x as f32, y as f32);
        let scale = Scale {
            x: fontsize,
            y: fontsize,
        };

        let glyphs = font.layout(s, scale, offset).collect::<Vec<_>>();

        //self.fill_rect(0, 0, frame.width(), frame.height(), LinSrgb::new(0, 0, 0));

        for g in glyphs.iter() {
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw(|x, y, v| {
                    let x = x as i32 + bb.min.x;
                    let y = y as i32 + bb.min.y;
                    let c = color.into_format::<f32>();
                    let (r, g, b) = c.into_components();
                    let c = LinSrgb::new(r * v, g * v, b * v);
                    self.put_pixel(
                        x as usize,
                        y as usize,
                        c.into_format(),
                        //LinSrgb::new((v * 255.0) as u8, 0, 0),
                    );
                });
            }
        }
    }
}

pub trait App {
    fn update(&mut self, input: &Input, frame: &mut Frame);
}
