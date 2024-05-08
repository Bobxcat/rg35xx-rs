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
    teams: bool,
    deck: Vec<Card>,
    discards: Vec<Card>,
    won_cards: Vec<Vec<Card>>,
    all_cards: Vec<Card>,
}

impl GameState {
    fn new(num_players: usize, teams: bool) -> Self {
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
            } else if card.word.contains(' ') || card.word.contains('-') {
                println!("Found multi-word {}!", card.word);
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
            teams,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum CardResult {
    Won,
    Discarded,
    Timeout,
}

impl CardResult {
    fn won(&self) -> bool {
        matches!(self, Self::Won)
    }

    fn discarded(&self) -> bool {
        matches!(self, Self::Discarded)
    }
}

enum TurnState {
    ReadyingUp,
    Playing {
        start_time: Instant,
        card: Card,
        results: Vec<(Card, CardResult)>,
    },
    TurnEnded {
        results: Vec<(Card, CardResult)>,
        showing: usize,
    },
}

enum CurrentTurn {
    Team(usize),
    Player { asker: usize, askee: usize },
}

impl CurrentTurn {
    fn next(&mut self, num_players: usize) {
        match self {
            Self::Team(team) => {
                if *team + 1 >= num_players {
                    *team = 0;
                } else {
                    *team += 1;
                }
            }
            Self::Player { asker, askee } => {
                let delta = if *askee < *asker {
                    *askee + num_players - *asker
                } else {
                    *askee - *asker
                };
                //println!("{} {} {}", *asker, *askee, delta);
                *asker += 1;
                *askee += 1;
                if *askee >= num_players {
                    *askee = 0;
                }
                if *asker >= num_players {
                    *asker = 0;
                    if delta + 1 == num_players {
                        *askee = 1;
                    } else {
                        *askee = delta + 1;
                    }
                }
            }
        }
    }
}

pub enum TabooApp {
    Menu {
        players: usize,
        teams: bool,
    },
    InGame {
        game: GameState,
        turn: TurnState,
        current_turn: CurrentTurn,
    },
}

impl Default for TabooApp {
    fn default() -> Self {
        Self::Menu {
            players: 2,
            teams: true,
        }
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
            Self::Menu { players, teams } => {
                let mut ctx = frame.context();
                ctx.set_fontsize(18.0);
                ctx.set_color(LinSrgb::new(255, 0, 0));

                ctx.offset(50, 50);
                if *teams {
                    ctx.text(&format!("Number of teams: {}", *players));
                } else {
                    ctx.text(&format!("Number of individual players: {}", *players));
                }

                ctx.offset(0, 20);
                ctx.text("Press START");

                if input.just_pressed(Button::PovUp) {
                    *players += 1;
                }
                if input.just_pressed(Button::PovDown) {
                    *players -= 1;
                    if *players < 2 {
                        *players = 2;
                    }
                }
                if input.just_pressed(Button::MenuL) {
                    *teams = !*teams;
                }
                if input.just_pressed(Button::MenuR) {
                    *self = Self::InGame {
                        game: GameState::new(*players, *teams),
                        turn: TurnState::ReadyingUp,
                        current_turn: if *teams {
                            CurrentTurn::Team(0)
                        } else {
                            CurrentTurn::Player { asker: 0, askee: 1 }
                        },
                    };
                }
            }
            Self::InGame {
                game,
                turn,
                current_turn,
            } => match turn {
                TurnState::ReadyingUp => {
                    let mut ctx = frame.context();
                    ctx.set_fontsize(18.0);
                    ctx.text(&format!("{} cards in deck", game.deck_size()));
                    ctx.offset(0, 20);
                    match *current_turn {
                        CurrentTurn::Team(team) => {
                            ctx.text(&format!("Team {}: Press A to start", team));
                        }
                        CurrentTurn::Player { asker, askee } => {
                            ctx.text(&format!(
                                "Player {} asking {}: Press A to start",
                                asker, askee
                            ));
                        }
                    }
                    ctx.offset(0, 20);
                    ctx.text("B to finish game");

                    let mut ctx = frame.context();
                    ctx.set_fontsize(24.0);
                    ctx.offset(350, 50);
                    for team in 0..game.num_players {
                        let is_up = match *current_turn {
                            CurrentTurn::Team(active_team) => active_team == team,
                            CurrentTurn::Player { asker, askee } => team == asker || team == askee,
                        };
                        if is_up {
                            ctx.set_color(LinSrgb::new(255, 255, 255));
                        } else {
                            ctx.set_color(LinSrgb::new(255, 0, 0));
                        }
                        ctx.text(&format!("Team {}: {}", team, game.won_cards[team].len()));
                        ctx.offset(0, 20);
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
                            teams: game.teams,
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
                            results.iter().filter(|(_, x)| x.won()).count()
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
                        results.push((card.clone(), CardResult::Timeout));
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
                        results.push((next_card, CardResult::Won));
                    } else if input.just_pressed(Button::ActionB) {
                        // Give up/fail the card
                        let mut next_card = game.draw_card();
                        std::mem::swap(&mut next_card, card);
                        results.push((next_card, CardResult::Discarded));
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
                            "Got {} cards",
                            results.iter().filter(|(_, x)| x.won()).count(),
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
                            results.iter().filter(|(_, x)| x.discarded()).count(),
                        ),
                    );
                    frame.text(
                        "fonts/Ubuntu-B.ttf",
                        100,
                        150,
                        48.0,
                        LinSrgb::new(255, 255, 255),
                        match results[*showing].1 {
                            CardResult::Won => "Got",
                            CardResult::Discarded => "Discarded",
                            CardResult::Timeout => "Timed out",
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
                        for (card, card_result) in results.drain(..) {
                            if card_result.won() {
                                match *current_turn {
                                    CurrentTurn::Team(team) => game.won_cards[team].push(card),
                                    CurrentTurn::Player { asker, askee } => {
                                        game.won_cards[asker].push(card.clone());
                                        game.won_cards[askee].push(card);
                                    }
                                }
                            } else {
                                game.discards.push(card);
                            }
                        }
                        current_turn.next(game.num_players);
                        *turn = TurnState::ReadyingUp;
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_turn_increment() {
        let mut start = CurrentTurn::Player { asker: 0, askee: 1 };
        let expected = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            (0, 2),
            (1, 3),
            (2, 0),
            (3, 1),
            (0, 3),
            (1, 0),
            (2, 1),
            (3, 2),
        ];
        for (expected_asker, expected_askee) in expected {
            match start {
                CurrentTurn::Team(_) => panic!("Not a team game!"),
                CurrentTurn::Player { asker, askee } => {
                    assert_eq!(asker, expected_asker);
                    assert_eq!(askee, expected_askee);
                }
            }
            start.next(4);
        }
    }
}
