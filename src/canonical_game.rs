use std::{collections::HashMap, fmt::Display};

use crate::dyadic_rational_number::DyadicRationalNumber;

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct GameId(usize);

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Game {
    pub nus: Option<Nus>,
    pub options: Options,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Nus {
    pub number: DyadicRationalNumber,
    pub up_multiple: i32,
    pub nimber: u32,
}

impl Nus {
    fn integer(number: i64) -> Nus {
        Nus {
            number: DyadicRationalNumber::from(number),
            up_multiple: 0,
            nimber: 0,
        }
    }

    fn rational(number: DyadicRationalNumber) -> Self {
        Nus {
            number,
            up_multiple: 0,
            nimber: 0,
        }
    }
}

impl Display for Nus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.number == DyadicRationalNumber::from(0) && self.up_multiple == 0 && self.nimber == 0
        {
            write!(f, "0")?;
            return Ok(());
        }

        if self.number != DyadicRationalNumber::from(0) {
            write!(f, "{}", self.number)?;
        }

        if self.up_multiple == 1 {
            write!(f, "^")?;
        } else if self.up_multiple == -1 {
            write!(f, "v")?;
        } else if self.up_multiple > 0 {
            write!(f, "^{}", self.up_multiple)?;
        } else if self.up_multiple < 0 {
            write!(f, "v{}", self.up_multiple.abs())?;
        }

        if self.nimber == 1 {
            write!(f, "*")?;
        } else if self.nimber != 0 {
            write!(f, "*{}", self.nimber)?;
        }

        Ok(())
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Options {
    left: Vec<GameId>,
    right: Vec<GameId>,
}

#[derive(Debug)]
pub struct GameBackend {
    known_games: Vec<Game>,
    nus_index: HashMap<Nus, GameId>,
    options_index: HashMap<Options, GameId>,
}

impl GameBackend {
    /// Add new game to the index
    fn add_new_game(&mut self, game: Game) -> GameId {
        // Check if already present
        if let Some(id) = game.nus.clone().and_then(|nus| self.nus_index.get(&nus)) {
            return *id;
        }
        if let Some(id) = self.options_index.get(&game.options) {
            return *id;
        }

        // Current length is the next di
        let id = GameId(self.known_games.len());
        self.known_games.push(game.clone());

        // Populate indices
        game.nus.map(|nus| self.nus_index.insert(nus, id));
        self.options_index.insert(game.options, id);
        id
    }

    fn get_game_id_by_nus(&self, nus: &Nus) -> Option<GameId> {
        self.nus_index.get(nus).copied()
    }

    fn get_game_id_by_options(&self, options: &Options) -> Option<GameId> {
        self.options_index.get(options).cloned()
    }

    pub fn get_game(&self, game_id: GameId) -> Game {
        self.known_games[game_id.0].clone()
    }

    pub fn construct_nus(&mut self, nus: &Nus) -> GameId {
        let parity: u32 = (nus.up_multiple & 1) as u32;
        let sign = if nus.up_multiple >= 0 { 1 } else { -1 };
        let number_option = self.construct_rational(nus.number);
        let mut last_defined = nus.up_multiple;
        let last_defined_id;

        loop {
            if last_defined == 0 {
                last_defined_id = None;
                break;
            }
            let tmp_nus = Nus {
                number: nus.number,
                up_multiple: last_defined,
                nimber: nus.nimber ^ parity ^ ((last_defined & 1) as u32),
            };
            if let Some(id) = self.get_game_id_by_nus(&tmp_nus) {
                last_defined_id = Some(id);
                break;
            }
            last_defined -= sign;
        }

        let mut last_defined_id =
            last_defined_id.unwrap_or(self.construct_nimber(nus.number, nus.nimber ^ parity));

        let mut i = last_defined + sign;
        loop {
            if i == nus.up_multiple + sign {
                break;
            }

            let current_nimber = nus.nimber ^ parity ^ (i as u32 & 1);
            let new_nus = Nus {
                number: nus.number,
                up_multiple: i,
                nimber: current_nimber,
            };
            let new_options;

            if i == 1 && current_nimber == 1 {
                // special case for n^*
                let star_option = self.construct_nus(&Nus {
                    number: nus.number,
                    up_multiple: 0,
                    nimber: 1,
                });
                new_options = Options {
                    left: vec![number_option, star_option],
                    right: vec![number_option],
                };
            } else if i == -1 && current_nimber == 1 {
                // special case for nv*
                let star_option = self.construct_nus(&Nus {
                    number: nus.number,
                    up_multiple: 0,
                    nimber: 1,
                });
                new_options = Options {
                    left: vec![number_option],
                    right: vec![number_option, star_option],
                };
            } else if i > 0 {
                new_options = Options {
                    left: vec![number_option],
                    right: vec![last_defined_id],
                };
            } else {
                new_options = Options {
                    left: vec![last_defined_id],
                    right: vec![number_option],
                };
            }

            let new_game = Game {
                nus: Some(new_nus),
                options: new_options,
            };
            last_defined_id = self.add_new_game(new_game);
            i += sign;
        }

        last_defined_id
    }

    /// Create a game equal to a rational number
    pub fn construct_rational(&mut self, rational: DyadicRationalNumber) -> GameId {
        // Return id if already constructed
        let nus = Nus::rational(rational);
        if let Some(id) = self.get_game_id_by_nus(&nus) {
            return id;
        }

        // Recursion base case - rational is integer
        if let Some(int) = rational.to_integer() {
            return self.construct_integer(int);
        }

        // Recursively construct the game
        let left_option = self.construct_rational(rational.step(-1));
        let right_option = self.construct_rational(rational.step(1));
        let game = Game {
            nus: Some(nus),
            options: Options {
                left: vec![left_option],
                right: vec![right_option],
            },
        };

        self.add_new_game(game)
    }

    /// Create a game equal to a rational number plus a nimber
    pub fn construct_nimber(&mut self, rational: DyadicRationalNumber, nimber: u32) -> GameId {
        // Find last defined nimber smaller than the requested one
        let mut last_defined = nimber;
        let mut last_defined_id = None;
        loop {
            if last_defined == 0 {
                break;
            }

            let nus = Nus {
                number: rational,
                up_multiple: 0,
                nimber: last_defined,
            };
            if let Some(id) = self.get_game_id_by_nus(&nus) {
                last_defined_id = Some(id);
                break;
            }
            last_defined -= 1;
        }
        let last_defined = last_defined;

        // If no previous nimbers, then the first option is the number, i.e. [rational + *0]
        let mut last_defined_id = last_defined_id.unwrap_or(self.construct_rational(rational));

        for i in (last_defined + 1)..=nimber {
            let previous_nimber = self.get_game(last_defined_id);
            let nus = Nus {
                number: rational,
                up_multiple: 0,
                nimber: i,
            };

            let mut options = Options {
                left: previous_nimber.options.left,
                right: previous_nimber.options.right,
            };
            options.left.push(last_defined_id);
            options.right.push(last_defined_id);
            let game = Game {
                nus: Some(nus),
                options,
            };
            last_defined_id = self.add_new_game(game);
        }

        last_defined_id
    }

    /// Create a game equal to an integer number
    pub fn construct_integer(&mut self, number: i64) -> GameId {
        let sign = if number >= 0 { 1 } else { -1 };

        // Find last defined game equal to integer smaller/greater (see sign) to the 'number'
        let mut last_defined = number;
        let mut last_defined_id;
        loop {
            if let Some(gid) = self.get_game_id_by_nus(&Nus::integer(last_defined)) {
                last_defined_id = gid;
                break;
            }
            last_defined -= sign;
        }

        // Add numbers up to and including the requested one
        let mut i = last_defined + sign;
        loop {
            if i == number + sign {
                break;
            }

            let prev = self.get_game_id_by_nus(&Nus::integer(i - sign)).unwrap();
            let new_game = Game {
                nus: Some(Nus::integer(i)),
                options: Options {
                    left: (if sign > 0 { vec![prev] } else { vec![] }),
                    right: (if sign > 0 { vec![] } else { vec![prev] }),
                },
            };
            last_defined_id = self.add_new_game(new_game);
            i += sign;
        }
        last_defined_id
    }

    /// Initialize new game storage
    pub fn new() -> GameBackend {
        let mut res = GameBackend {
            known_games: Vec::new(),
            nus_index: HashMap::new(),
            options_index: HashMap::new(),
        };

        // Construct 0 by hand - special case, don't use construct_* here
        let zero = Game {
            nus: Some(Nus::integer(0)),
            options: Options {
                left: Vec::new(),
                right: Vec::new(),
            },
        };
        res.add_new_game(zero);

        res
    }
}

#[test]
fn constructs_integers() {
    let mut b = GameBackend::new();

    let eight_id = b.construct_integer(8);
    let eight = b.get_game(eight_id);
    assert_eq!(format!("{}", &eight.nus.unwrap()), "8".to_string());

    let minus_forty_two_id = b.construct_integer(-42);
    let minus_forty_two = b.get_game(minus_forty_two_id);
    assert_eq!(
        format!("{}", &minus_forty_two.nus.unwrap()),
        "-42".to_string()
    );
}

#[test]
fn constructs_rationals() {
    let mut b = GameBackend::new();

    let three_sixteenth_id = b.construct_rational(DyadicRationalNumber::new(3, 4));
    let three_sixteenth = b.get_game(three_sixteenth_id);
    assert_eq!(
        format!("{}", &three_sixteenth.nus.unwrap()),
        "3/16".to_string()
    );
}

#[test]
fn constructs_nimbers() {
    let mut b = GameBackend::new();

    let star_id = b.construct_nimber(DyadicRationalNumber::from(1), 4);
    let star = b.get_game(star_id);
    assert_eq!(format!("{}", &star.nus.unwrap()), "1*4".to_string());
}

#[test]
fn constructs_up() {
    let mut b = GameBackend::new();

    let up_id = b.construct_nus(&Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: 1,
        nimber: 0,
    });
    let up = b.get_game(up_id);
    assert_eq!(format!("{}", &up.nus.unwrap()), "^".to_string());

    let up_star_id = b.construct_nus(&Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: 1,
        nimber: 1,
    });
    let up_star = b.get_game(up_star_id);
    assert_eq!(format!("{}", &up_star.nus.unwrap()), "^*".to_string());

    let down_id = b.construct_nus(&Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: -3,
        nimber: 0,
    });
    let down = b.get_game(down_id);
    assert_eq!(format!("{}", &down.nus.unwrap()), "v3".to_string());
}
