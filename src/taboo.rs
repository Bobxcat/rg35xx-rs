use std::{collections::HashMap, time::Instant};

use palette::LinSrgb;
use rand::prelude::*;

use crate::app::{App, Button};

static WORDS: &str = include_str!("../assets/words.csv");

#[derive(Clone)]
struct Card {
    word: String,
    taboo: Vec<String>,
}

struct GameState {
    num_players: usize,
    deck: Vec<Card>,
    discards: Vec<Card>,
    won_cards: Vec<Vec<Card>>,
    all_cards: Vec<Card>,
}

impl GameState {
    fn new(num_players: usize) -> Self {
        let mut lines = WORDS
            .split('\n')
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|line| line.split(',').map(|word| word.trim()))
            .map(|mut words| {
                let word = words.next().unwrap().to_string();
                let taboo = words.map(|word| word.to_string()).collect::<Vec<_>>();
                Card { word, taboo }
            })
            .collect::<Vec<_>>();

        println!("Found {} words", lines.len());
        let mut cards = HashMap::new();
        for card in lines.drain(..) {
            if cards.contains_key(&card.word) {
                println!("Found duplicate {}!", card.word);
            } else {
                cards.insert(card.word.clone(), card);
            }
        }
        let mut lines = cards.into_values().collect::<Vec<_>>();
        println!("{} words after removing duplicates", lines.len());

        let mut rng = rand::thread_rng();
        lines.shuffle(&mut rng);

        Self {
            num_players,
            deck: lines.clone(),
            discards: vec![],
            won_cards: (0..num_players).map(|_| vec![]).collect::<Vec<_>>(),
            all_cards: lines,
        }
    }

    fn deck_size(&self) -> usize {
        self.deck.len() + self.discards.len()
    }

    fn draw_card(&mut self) -> Card {
        let mut rng = rand::thread_rng();
        if self.deck.is_empty() {
            self.discards.shuffle(&mut rng);
            std::mem::swap(&mut self.deck, &mut self.discards);
        }
        if self.deck.is_empty() {
            // Refill the deck
            self.deck = self.all_cards.clone();
            self.deck.shuffle(&mut rng);
        }
        self.deck.pop().unwrap()
    }
}

enum TurnState {
    ReadyingUp,
    Playing {
        start_time: Instant,
        card: Card,
        results: Vec<(Card, bool)>,
    },
    TurnEnded {
        results: Vec<(Card, bool)>,
        showing: usize,
    },
}

pub enum TabooApp {
    Menu {
        players: usize,
    },
    InGame {
        game: GameState,
        turn: TurnState,
        current_turn: usize,
    },
}

impl Default for TabooApp {
    fn default() -> Self {
        Self::Menu { players: 2 }
    }
}

fn render_card(frame: &mut crate::app::Frame, card: &Card, x: usize, y: usize) {
    frame.text(
        "fonts/Ubuntu-B.ttf",
        x,
        y,
        48.0,
        LinSrgb::new(255, 255, 255),
        &card.word,
    );
    for (i, taboo) in card.taboo.iter().enumerate() {
        frame.text(
            "fonts/Ubuntu-B.ttf",
            x,
            y + 35 + i * 40,
            36.0,
            LinSrgb::new(255, 0, 0),
            taboo,
        );
    }
}

impl App for TabooApp {
    fn update(&mut self, input: &crate::app::Input, frame: &mut crate::app::Frame) {
        frame.fill_rect(0, 0, frame.width(), frame.height(), LinSrgb::new(0, 0, 0));

        match self {
            Self::Menu { players } => {
                frame.text(
                    "fonts/Ubuntu-B.ttf",
                    50,
                    50,
                    18.0,
                    LinSrgb::new(255, 0, 0),
                    &format!("Number of players/teams: {}", *players),
                );
                frame.text(
                    "fonts/Ubuntu-B.ttf",
                    50,
                    70,
                    18.0,
                    LinSrgb::new(255, 0, 0),
                    "Press START",
                );
                if input.just_pressed(Button::PovUp) {
                    *players += 1;
                }
                if input.just_pressed(Button::PovDown) {
                    *players -= 1;
                    if *players < 2 {
                        *players = 2;
                    }
                }
                if input.just_pressed(Button::MenuR) {
                    *self = Self::InGame {
                        game: GameState::new(*players),
                        turn: TurnState::ReadyingUp,
                        current_turn: 0,
                    };
                }
            }
            Self::InGame {
                game,
                turn,
                current_turn,
            } => match turn {
                TurnState::ReadyingUp => {
                    frame.text(
                        "fonts/Ubuntu-B.ttf",
                        50,
                        50,
                        18.0,
                        LinSrgb::new(255, 255, 255),
                        &format!("{} cards in deck", game.deck_size()),
                    );
                    frame.text(
                        "fonts/Ubuntu-B.ttf",
                        50,
                        70,
                        18.0,
                        LinSrgb::new(255, 255, 255),
                        &format!("Team {}: Press A to start", current_turn),
                    );
                    frame.text(
                        "fonts/Ubuntu-B.ttf",
                        50,
                        90,
                        18.0,
                        LinSrgb::new(255, 255, 255),
                        "B to finish game",
                    );
                    for team in 0..game.num_players {
                        frame.text(
                            "fonts/Ubuntu-B.ttf",
                            350,
                            50 + team * 20,
                            24.0,
                            if team == *current_turn {
                                LinSrgb::new(255, 255, 255)
                            } else {
                                LinSrgb::new(255, 0, 0)
                            },
                            &format!("Team {}: {}", team, game.won_cards[team].len()),
                        );
                    }
                    if input.just_pressed(Button::ActionA) {
                        *turn = TurnState::Playing {
                            start_time: Instant::now(),
                            card: game.draw_card(),
                            results: vec![],
                        };
                    }
                    if input.just_pressed(Button::ActionB) {
                        *self = Self::Menu {
                            players: game.num_players,
                        };
                    }
                }
                TurnState::Playing {
                    start_time,
                    card,
                    results,
                } => {
                    let remaining = 60.0 - start_time.elapsed().as_secs_f32();
                    frame.text(
                        "fonts/Ubuntu-B.ttf",
                        50,
                        50,
                        48.0,
                        LinSrgb::new(255, 255, 255),
                        &format!(
                            "{:.1}s ({})",
                            remaining,
                            results.iter().filter(|(_, x)| *x).count()
                        ),
                    );
                    render_card(frame, card, 100, 140);
                    frame.text(
                        "fonts/Ubuntu-B.ttf",
                        50,
                        430,
                        18.0,
                        LinSrgb::new(255, 255, 255),
                        "B discard, A got card",
                    );
                    if remaining < 0.0 || input.just_pressed(Button::MenuR) {
                        let mut cards2 = vec![];
                        std::mem::swap(&mut cards2, results);
                        let showing = cards2.len() - 1;
                        *turn = TurnState::TurnEnded {
                            results: cards2,
                            showing,
                        };
                    } else if input.just_pressed(Button::ActionA) {
                        // Guessed the card
                        let mut next_card = game.draw_card();
                        std::mem::swap(&mut next_card, card);
                        results.push((next_card, true));
                    } else if input.just_pressed(Button::ActionB) {
                        // Give up/fail the card
                        let mut next_card = game.draw_card();
                        std::mem::swap(&mut next_card, card);
                        results.push((next_card, false));
                    }
                }
                TurnState::TurnEnded { results, showing } => {
                    frame.text(
                        "fonts/Ubuntu-B.ttf",
                        50,
                        50,
                        48.0,
                        LinSrgb::new(255, 255, 255),
                        &format!(
                            "Team {} got {} cards",
                            current_turn,
                            results.iter().filter(|(_, x)| *x).count(),
                        ),
                    );
                    frame.text(
                        "fonts/Ubuntu-B.ttf",
                        50,
                        100,
                        48.0,
                        LinSrgb::new(255, 255, 255),
                        &format!(
                            "(discarded {})",
                            results.iter().filter(|(_, x)| !*x).count(),
                        ),
                    );
                    frame.text(
                        "fonts/Ubuntu-B.ttf",
                        100,
                        150,
                        48.0,
                        LinSrgb::new(255, 255, 255),
                        if results[*showing].1 {
                            "Got"
                        } else {
                            "Discarded"
                        },
                    );
                    render_card(frame, &results[*showing].0, 100, 190);
                    frame.text(
                        "fonts/Ubuntu-B.ttf",
                        50,
                        430,
                        18.0,
                        LinSrgb::new(255, 255, 255),
                        "POV change cards, A to continue",
                    );

                    if input.just_pressed(Button::PovRight) {
                        *showing += 1;
                        if *showing >= results.len() {
                            *showing = results.len() - 1;
                        }
                    }
                    if input.just_pressed(Button::PovLeft) {
                        *showing = showing.saturating_sub(1);
                    }
                    if input.just_pressed(Button::ActionA) {
                        for (card, won) in results.drain(..) {
                            if won {
                                game.won_cards[*current_turn].push(card);
                            } else {
                                game.discards.push(card);
                            }
                        }
                        *current_turn += 1;
                        if *current_turn >= game.num_players {
                            *current_turn = 0;
                        }
                        *turn = TurnState::ReadyingUp;
                    }
                }
            },
        }
    }
}
