use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use palette::LinSrgb;
use rand::{distributions::Uniform, prelude::*};

use crate::app::{App, Button};

const BLOCK_WIDTH: usize = 10;
const BLOCK_HEIGHT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn is_opposite(&self, other: Self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match (self, other) {
            (Self::Up, Self::Down)
            | (Self::Down, Self::Up)
            | (Self::Left, Self::Right)
            | (Self::Right, Self::Left) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Point(usize, usize);

impl Point {
    fn in_wrapped_direction(&self, direction: Direction, width: usize, height: usize) -> Self {
        let (dx, dy) = match direction {
            Direction::Down => (0, 1),
            Direction::Up => (0, -1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        };

        let new_x = if dx < 0 && self.0 == 0 {
            width - 1
        } else if dx > 0 && self.0 == width - 1 {
            0
        } else {
            self.0.checked_add_signed(dx).unwrap()
        };
        let new_y = if dy < 0 && self.1 == 0 {
            height - 1
        } else if dy > 0 && self.1 == height - 1 {
            0
        } else {
            self.1.checked_add_signed(dy).unwrap()
        };

        Self(new_x, new_y)
    }
}

struct State {
    dead: bool,
    width: usize,
    height: usize,
    snake: Vec<Point>,
    foods: HashSet<Point>,
    direction: Direction,
    new_direction: Direction,
}

impl Default for State {
    fn default() -> Self {
        Self {
            dead: false,
            width: 640 / BLOCK_WIDTH,
            height: 480 / BLOCK_HEIGHT,
            snake: vec![Point(32, 24), Point(31, 24)],
            foods: HashSet::new(),
            direction: Direction::Right,
            new_direction: Direction::Right,
        }
    }
}

impl State {
    fn step(&mut self) {
        if self.dead {
            return;
        }

        if self.direction.is_opposite(self.new_direction) {
            self.new_direction = self.direction;
        }
        self.direction = self.new_direction;

        // Move the snake in the given direction
        let head = self.snake[0];
        let new_head = head.in_wrapped_direction(self.direction, self.width, self.height);

        // Check that the snake didn't hit itself
        if self.snake.contains(&new_head) {
            self.dead = true;
            return;
        }

        self.snake.insert(0, new_head);

        // Eat any foods at the new head
        if self.foods.remove(&new_head) {
            // Don't pop the snake's tail
        } else {
            self.snake.pop();
        }

        // Spawn foods as needed
        if self.foods.len() < 2 {
            let mut rng = rand::thread_rng();
            for _ in 0..5 {
                // Pick a point, check that it isn't already a food or a snake
                let wdist = Uniform::from(0..self.width);
                let hdist = Uniform::from(0..self.height);
                let p = Point(wdist.sample(&mut rng), hdist.sample(&mut rng));
                if !self.snake.contains(&p) && !self.foods.contains(&p) {
                    self.foods.insert(p);
                    break;
                }
            }
        }
    }
}

pub struct SnakeApp {
    state: Option<State>,
    last_step: Instant,
    difficulty: u32,
}

impl Default for SnakeApp {
    fn default() -> Self {
        Self {
            state: None,
            last_step: Instant::now(),
            difficulty: 5,
        }
    }
}

impl App for SnakeApp {
    fn update(&mut self, input: &crate::app::Input, frame: &mut crate::app::Frame) {
        frame.fill_rect(0, 0, frame.width(), frame.height(), LinSrgb::new(0, 0, 0));

        if let Some(state) = self.state.as_mut() {
            // Handle any input
            for (button, dir) in [
                (Button::PovDown, Direction::Down),
                (Button::PovUp, Direction::Up),
                (Button::PovLeft, Direction::Left),
                (Button::PovRight, Direction::Right),
            ] {
                if input.pressed(button) {
                    state.new_direction = dir;
                }
            }

            // Step, if time has elapsed
            if self.last_step.elapsed() > Duration::from_millis(1000 / self.difficulty as u64) {
                state.step();
                self.last_step = Instant::now();
            }

            // Render the snake
            frame.fill_rect(
                state.snake[0].0 * BLOCK_WIDTH,
                state.snake[0].1 * BLOCK_HEIGHT,
                BLOCK_WIDTH,
                BLOCK_HEIGHT,
                LinSrgb::new(255, 0, 0),
            );
            for body in state.snake[1..].iter() {
                frame.fill_rect(
                    body.0 * BLOCK_WIDTH,
                    body.1 * BLOCK_HEIGHT,
                    BLOCK_WIDTH,
                    BLOCK_HEIGHT,
                    LinSrgb::new(0, 255, 0),
                );
            }

            // Render the foods
            for food in state.foods.iter() {
                frame.fill_rect(
                    food.0 * BLOCK_WIDTH,
                    food.1 * BLOCK_HEIGHT,
                    BLOCK_WIDTH,
                    BLOCK_HEIGHT,
                    LinSrgb::new(0, 0, 255),
                );
            }

            if state.dead {
                frame.text(
                    "fonts/Ubuntu-B.ttf",
                    50,
                    50,
                    18.0,
                    LinSrgb::new(255, 0, 0),
                    &format!("GAME OVER - Score: {}", state.snake.len()),
                );

                if input.just_pressed(Button::MenuR) || input.just_pressed(Button::MenuL) {
                    self.state = None;
                }
            }
        } else {
            frame.text(
                "fonts/Ubuntu-B.ttf",
                50,
                50,
                18.0,
                LinSrgb::new(255, 0, 0),
                "Press START",
            );
            frame.text(
                "fonts/Ubuntu-B.ttf",
                50,
                70,
                18.0,
                LinSrgb::new(255, 0, 0),
                &format!("Difficulty: {}", self.difficulty),
            );

            if input.just_pressed(Button::MenuR) {
                self.state = Some(State::default());
                self.last_step = Instant::now();
            }
            if input.just_pressed(Button::PovUp) {
                self.difficulty += 1;
                if self.difficulty > 1000 {
                    self.difficulty = 1000;
                }
            }
            if input.just_pressed(Button::PovDown) {
                self.difficulty -= 1;
                if self.difficulty < 1 {
                    self.difficulty = 1;
                }
            }
        }
    }
}
