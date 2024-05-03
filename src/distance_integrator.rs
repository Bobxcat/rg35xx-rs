use std::{collections::HashMap, ops::Div, time::Instant};

use palette::LinSrgb;
use rand::prelude::*;

use crate::app::{App, Button};

struct ButtonHoldIncrementer {
    last_update: Instant,
    pressed_time: f64,
    previously_pressed: bool,
    time_to_increment: f64,
    increment_amount: u32,
}

impl Default for ButtonHoldIncrementer {
    fn default() -> Self {
        Self {
            last_update: Instant::now(),
            pressed_time: 0.0,
            previously_pressed: false,
            time_to_increment: 0.0,
            increment_amount: 5,
        }
    }
}

impl ButtonHoldIncrementer {
    /// Returns how much to increment by
    fn update(&mut self, pressed: bool, positive: bool, value: &mut u32) {
        let elapsed = self.last_update.elapsed().as_secs_f64();
        self.last_update = Instant::now();
        if pressed {
            self.pressed_time += elapsed;
            if !self.previously_pressed {
                self.previously_pressed = true;
                self.time_to_increment = 0.5;
                if positive {
                    *value += 1;
                } else {
                    *value = value.saturating_sub(1);
                }
            } else if self.time_to_increment > 0.0 {
                self.time_to_increment -= elapsed;
            } else {
                // Time to increment!
                self.time_to_increment = 0.3;

                // Round to the next multiple of five
                if positive {
                    *value += 1;
                } else {
                    *value = value.saturating_sub(1);
                }
                while *value % self.increment_amount != 0 {
                    if positive {
                        *value += 1;
                    } else {
                        *value = value.saturating_sub(1);
                    }
                }

                if self.pressed_time > 6.0 {
                    self.increment_amount = 25;
                } else if self.pressed_time > 2.0 {
                    self.increment_amount = 10;
                }
            }
        } else {
            self.previously_pressed = false;
            self.pressed_time = 0.0;
            self.increment_amount = 5;
        }
    }
}

enum Unit {
    Imperial,
    Metric,
}

impl Unit {
    fn distance_conversion_factor(&self) -> f64 {
        match self {
            Self::Imperial => 0.000539957,
            Self::Metric => 0.001,
        }
    }

    fn distance_to_display(&self, raw: f64) -> f64 {
        raw * self.distance_conversion_factor()
    }

    fn display_to_distance(&self, display: f64) -> f64 {
        display / self.distance_conversion_factor()
    }

    fn distance_unit(&self) -> &'static str {
        match self {
            Self::Imperial => "nm",
            Self::Metric => "km",
        }
    }

    /// Converting from m/s to <dist>/hr
    fn speed_conversion_factor(&self) -> f64 {
        self.distance_conversion_factor() * 3600.0
    }

    fn speed_to_display(&self, raw: f64) -> f64 {
        raw * self.speed_conversion_factor()
    }

    fn speed_from_display(&self, display: f64) -> f64 {
        display / self.speed_conversion_factor()
    }

    fn speed_unit(&self) -> &'static str {
        match self {
            Self::Imperial => "mph",
            Self::Metric => "km/h",
        }
    }

    /// Converting an altitude value to meters
    fn altitude_conversion_factor(&self) -> f64 {
        match self {
            Self::Imperial => 0.3048,
            Self::Metric => 1.0,
        }
    }

    fn altitude_unit(&self) -> &'static str {
        match self {
            Self::Imperial => "ft",
            Self::Metric => "m",
        }
    }
}

pub struct DistanceIntegrator {
    last_update: Instant,
    time: f64,
    distance: f64,
    speed: f64,
    display_speed: u32,
    crosswind: f64,
    crosswind_bearing: f64,

    /// This altitude is in thousands of feet
    altitude: u32,

    /// The struct's internal values are meters, seconds, and meters/second, but
    /// this sets how it's displayed
    unit: Unit,

    speed_incrementer: ButtonHoldIncrementer,
    speed_decrementer: ButtonHoldIncrementer,
}

impl Default for DistanceIntegrator {
    fn default() -> Self {
        Self {
            last_update: Instant::now(),
            time: 0.0,
            distance: 0.0,
            speed: 0.0,
            display_speed: 200,
            crosswind: 0.0,
            crosswind_bearing: 0.0,
            altitude: 0,
            unit: Unit::Imperial,
            speed_incrementer: Default::default(),
            speed_decrementer: Default::default(),
        }
    }
}

impl App for DistanceIntegrator {
    fn update(&mut self, input: &crate::app::Input, frame: &mut crate::app::Frame) {
        frame.fill_rect(0, 0, frame.width(), frame.height(), LinSrgb::new(0, 0, 0));

        let elapsed = self.last_update.elapsed();
        self.last_update = Instant::now();
        let elapsed = elapsed.as_secs_f64();

        // Compute air density, see https://en.wikipedia.org/wiki/Density_of_air
        let h = Unit::Imperial.altitude_conversion_factor() * self.altitude as f64 * 1000.0;
        let temp_lapse_rate = 0.0065;
        let est_oat = 288.15 - h * temp_lapse_rate;
        let pressure =
            101325.0 * (1.0 - 0.0065 * h / 288.15).powf(9.80665 * 0.0289652 / (8.31446 * 0.0065));
        let rho = pressure * 0.0289652 / (8.31446 * est_oat);

        // Compute TAS, see https://aviation.stackexchange.com/questions/25801/how-do-you-convert-true-airspeed-to-indicated-airspeed
        let ki = 0.0;
        let a0 = 290.07;
        let mach = (self.speed - ki) / a0;
        let inner = mach * mach / 5.0 + 1.0;
        let tas = (2.0 * 101_325.0 / rho * (inner.powf(3.5) - 1.0)).sqrt();
        //println!("rho={rho} h={h} est_oat={est_oat} {} -> {tas}", self.speed);

        // For computing crosswind, the velocity forms a triangle.
        // The Hypotenuse side is the actual heading (and speed), theta is the crab angle.
        // The Opposite side is the crosswind component
        // The Adjacent is the actual ground speed
        let crosswind_component = self.crosswind_bearing.to_radians().sin() * self.crosswind; // Positive is right
        let headwind_component = self.crosswind_bearing.to_radians().cos() * self.crosswind;
        let crab = if self.speed > 0.1 && tas > crosswind_component {
            (crosswind_component / self.speed).asin().to_degrees()
        } else {
            0.0
        };
        let groundspeed = crab.to_radians().cos() * tas - headwind_component;

        self.time += elapsed;
        self.distance += groundspeed * elapsed;

        frame.text(
            "fonts/Ubuntu-B.ttf",
            20,
            70,
            72.0,
            LinSrgb::new(255, 255, 255),
            &format!(
                "{:.1} {} ({:.1}gs)",
                self.unit.distance_to_display(self.distance),
                self.unit.distance_unit(),
                // Distance in grid squares
                self.distance / 10_000.0,
            ),
        );
        let time_per_distance = if groundspeed > 1.0 {
            let reference_distance = self.unit.display_to_distance(10.0);
            reference_distance / groundspeed
        } else {
            0.0
        };
        frame.text(
            "fonts/Ubuntu-B.ttf",
            20,
            150,
            72.0,
            LinSrgb::new(255, 255, 255),
            &format!(
                "{:.0} {} ({}:{:02.0}/10)",
                self.unit.speed_to_display(self.speed),
                self.unit.speed_unit(),
                time_per_distance.div_euclid(60.0),
                time_per_distance.rem_euclid(60.0),
            ),
        );
        let minutes = self.time.div_euclid(60.0) as u32;
        let seconds = self.time.rem_euclid(60.0) as u32;
        let milliseconds = (self.time.rem_euclid(1.0) * 1000.0) as u32;
        frame.text(
            "fonts/Ubuntu-B.ttf",
            20,
            230,
            72.0,
            LinSrgb::new(255, 255, 255),
            &format!(
                "{:.1} k{} {:02}:{:02}.{:03}",
                self.altitude as f64 * 0.3048 / self.unit.altitude_conversion_factor(),
                self.unit.altitude_unit(),
                minutes,
                seconds,
                milliseconds
            ),
        );

        // Crosswind computer
        frame.text(
            "fonts/Ubuntu-B.ttf",
            20,
            310,
            72.0,
            LinSrgb::new(255, 255, 255),
            &format!(
                "{:.1}m/s @ {:.0}deg",
                self.crosswind, self.crosswind_bearing,
            ),
        );
        frame.text(
            "fonts/Ubuntu-B.ttf",
            20,
            390,
            72.0,
            LinSrgb::new(255, 255, 255),
            &format!("{:.1}deg crab", crab),
        );

        // Show how many distance we go during descent
        let secs_to_descend_1km = match self.unit {
            // 1kfpm standard imperial descent
            Unit::Imperial => 60.0 / 1.0,

            // 400m per minute standard metric descent
            Unit::Metric => 60.0 / 0.4,
        };
        let descent_distance = secs_to_descend_1km * groundspeed;
        frame.text(
            "fonts/Ubuntu-B.ttf",
            20,
            470,
            72.0,
            LinSrgb::new(255, 255, 255),
            &format!(
                "{:.1} {} per 1k{}",
                self.unit.distance_to_display(descent_distance),
                self.unit.distance_unit(),
                self.unit.altitude_unit(),
            ),
        );

        //let mut speed = self.unit.speed_to_display(self.speed) as u32;
        self.speed_incrementer.update(
            input.pressed(Button::ActionH),
            true,
            &mut self.display_speed,
        );
        self.speed_decrementer.update(
            input.pressed(Button::ActionB),
            false,
            &mut self.display_speed,
        );
        self.speed = self.unit.speed_from_display(self.display_speed as f64);

        if input.just_pressed(Button::BumperR) {
            self.distance = 0.0;
            self.time = 0.0;
        }
        if input.just_pressed(Button::MenuL) {
            self.unit = match self.unit {
                Unit::Imperial => Unit::Metric,
                Unit::Metric => Unit::Imperial,
            };
            self.display_speed = self.unit.speed_to_display(self.speed) as u32;
        }

        if input.pressed(Button::PovUp) {
            self.crosswind += elapsed * 2.0;
        }
        if input.pressed(Button::PovDown) {
            self.crosswind -= elapsed * 2.0;
            if self.crosswind < 0.0 {
                self.crosswind = 0.0;
            }
        }
        if input.pressed(Button::PovLeft) {
            self.crosswind_bearing -= elapsed * 30.0;
            if self.crosswind_bearing < -180.0 {
                self.crosswind_bearing += 360.0;
            }
        }
        if input.pressed(Button::PovRight) {
            self.crosswind_bearing += elapsed * 30.0;
            if self.crosswind_bearing > 180.0 {
                self.crosswind_bearing -= 360.0;
            }
        }
        if input.just_pressed(Button::ActionA) {
            self.altitude += 1;
        }
        if input.just_pressed(Button::ActionV) {
            self.altitude = self.altitude.saturating_sub(1);
        }
    }
}
