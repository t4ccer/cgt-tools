use std::{
    cmp::{self, Ordering},
    fmt::{self, Display, Write},
    fs::File,
    io::{BufReader, Read},
    ops::Add,
    sync::RwLock,
};

use crate::{
    dyadic_rational_number::DyadicRationalNumber, nimber::Nimber, rational::Rational,
    rw_hash_map::RwHashMap, thermograph::Thermograph, trajectory::Trajectory,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GameId(usize);

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Game {
    pub nus: Option<Nus>,
    pub options: Options,
}

impl Game {
    /// Check if a game is a sum of numbers, ups, and stars
    pub fn is_number_up_star(&self) -> bool {
        self.nus.is_some()
    }

    /// Check if a game is only a number
    pub fn is_number(&self) -> bool {
        match &self.nus {
            None => false,
            Some(nus) => nus.is_number(),
        }
    }

    /// Check if a game is only a nimber
    pub fn is_nimber(&self) -> bool {
        match &self.nus {
            None => false,
            Some(nus) => nus.is_nimber(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Nus {
    pub number: DyadicRationalNumber,
    pub up_multiple: i32,
    pub nimber: Nimber,
}

impl Nus {
    pub fn integer(number: i64) -> Nus {
        Nus {
            number: DyadicRationalNumber::from(number),
            up_multiple: 0,
            nimber: Nimber::from(0),
        }
    }

    pub fn rational(number: DyadicRationalNumber) -> Self {
        Nus {
            number,
            up_multiple: 0,
            nimber: Nimber::from(0),
        }
    }

    pub fn is_number(&self) -> bool {
        self.up_multiple == 0 && self.nimber == Nimber::from(0)
    }

    pub fn is_nimber(&self) -> bool {
        self.number == DyadicRationalNumber::from(0) && self.up_multiple == 0
    }
}

impl Add for Nus {
    type Output = Nus;

    fn add(self, rhs: Nus) -> Nus {
        &self + &rhs
    }
}

impl Add for &Nus {
    type Output = Nus;

    fn add(self, rhs: &Nus) -> Nus {
        Nus {
            number: self.number + rhs.number,
            up_multiple: self.up_multiple + rhs.up_multiple,
            nimber: self.nimber + rhs.nimber,
        }
    }
}

impl Display for Nus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.number == DyadicRationalNumber::from(0)
            && self.up_multiple == 0
            && self.nimber == Nimber::from(0)
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

        if self.nimber == Nimber::from(1) {
            write!(f, "*")?;
        } else if self.nimber != Nimber::from(0) {
            write!(f, "*{}", self.nimber)?;
        }

        Ok(())
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Options {
    pub left: Vec<GameId>,
    pub right: Vec<GameId>,
}

impl Options {
    pub fn empty() -> Self {
        Options {
            left: Vec::new(),
            right: Vec::new(),
        }
    }

    pub fn eliminate_duplicates(&mut self) {
        self.left.sort();
        self.left.dedup();

        self.right.sort();
        self.right.dedup();
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct GameBackend {
    known_games: RwLock<Vec<Game>>,
    nus_index: RwHashMap<Nus, GameId>,
    options_index: RwHashMap<Options, GameId>,
    negative_index: RwHashMap<GameId, GameId>,
    birthday_index: RwHashMap<GameId, i64>,
    leq_index: RwHashMap<(GameId, GameId), bool>,
    add_index: RwHashMap<(GameId, GameId), GameId>,
    thermograph_index: RwHashMap<GameId, Thermograph>,
    pub zero_id: GameId,
    pub star_id: GameId,
    pub up_id: GameId,
    pub up_star_id: GameId,
    pub one_id: GameId,
    pub negative_one_id: GameId,
    pub two_id: GameId,
    pub negative_two_id: GameId,
}

fn cat_options<T>(options: &[Option<T>]) -> Vec<T>
where
    T: Copy,
{
    options.iter().flatten().copied().collect()
}

impl GameBackend {
    /// Add new game to the index
    fn add_new_game(&self, game: Game) -> GameId {
        // Locking here guarantees that no two threads will try to insert the same game
        let mut known_games = self.known_games.write().unwrap();

        // Check if already present
        if let Some(id) = game.nus.clone().and_then(|nus| self.nus_index.get(&nus)) {
            return id;
        }
        if let Some(id) = self.options_index.get(&game.options) {
            return id;
        }

        let id = GameId(known_games.len());
        known_games.push(game.clone());

        // Populate indices
        game.nus.map(|nus| self.nus_index.insert(nus, id));
        self.options_index.insert(game.options, id);
        id
    }

    fn get_game_id_by_nus(&self, nus: &Nus) -> Option<GameId> {
        self.nus_index.get(nus)
    }

    fn get_game_id_by_options(&self, options: &Options) -> Option<GameId> {
        self.options_index.get(options)
    }

    pub fn get_game(&self, game_id: GameId) -> Game {
        self.known_games.read().unwrap()[game_id.0].clone()
    }

    fn compare_number_parts(&self, gid: GameId, hid: GameId) -> i32 {
        let g = self.get_game(gid).nus.unwrap().number;
        let g_num = g.numerator();
        let g_den_exp = g.denominator_exponent();

        let h = self.get_game(hid).nus.unwrap().number;
        let h_num = h.numerator();
        let h_den_exp = h.denominator_exponent();

        let cmp: i64;

        if g_den_exp <= h_den_exp {
            cmp = ((g_num as i64) << (h_den_exp - g_den_exp)) - (h_num as i64);
        } else {
            cmp = (g_num as i64) - ((h_num as i64) << (g_den_exp - h_den_exp));
        }

        cmp.signum() as i32
    }

    /// Construct a negative of given game
    pub fn construct_negative(&self, game_id: GameId) -> GameId {
        // If game is a nus, just take the negative of components
        let game = self.get_game(game_id);
        if let Some(nus) = game.nus {
            let neg_nus = Nus {
                number: -nus.number,
                up_multiple: -nus.up_multiple,
                nimber: nus.nimber, // Nimber is it's own multiple
            };
            return self.construct_nus(&neg_nus);
        }

        if let Some(neg_id) = self.negative_index.get(&game_id) {
            return neg_id;
        }

        let new_left_options = game
            .options
            .left
            .iter()
            .map(|left_id| self.construct_negative(*left_id))
            .collect::<Vec<_>>();

        let new_right_options = game
            .options
            .right
            .iter()
            .map(|left_id| self.construct_negative(*left_id))
            .collect::<Vec<_>>();

        let neg_options = Options {
            left: new_left_options,
            right: new_right_options,
        };
        let neg_id = self.construct_from_canonical_options(neg_options);

        self.negative_index.insert(game_id, neg_id);

        neg_id
    }

    // TODO: Rename args
    /// Construct a sum of two games
    pub fn construct_sum(&self, gid: GameId, hid: GameId) -> GameId {
        let g = self.get_game(gid);
        let h = self.get_game(hid);

        if let (Some(g_nus), Some(h_nus)) = (&g.nus, &h.nus) {
            return self.construct_nus(&(g_nus + h_nus));
        }

        if let Some(result) = self.add_index.get(&(gid, hid)) {
            return result;
        }

        // We want to return { GL+H, G+HL | GR+H, G+HR }

        // By the number translation theorem

        let mut options = Options::empty();

        if !g.is_number() {
            for g_l in g.options.left {
                options.left.push(self.construct_sum(g_l, hid));
            }
            for g_r in g.options.right {
                options.right.push(self.construct_sum(g_r, hid));
            }
        }
        if !h.is_number() {
            for h_l in h.options.left {
                options.left.push(self.construct_sum(gid, h_l));
            }
            for h_r in h.options.right {
                options.right.push(self.construct_sum(gid, h_r));
            }
        }

        let result = self.construct_from_options(options);
        self.add_index.insert((gid, hid), result);
        self.add_index.insert((hid, gid), result);
        result
    }

    /// Calculate mex if possible. Assumes that input is sorted
    fn mex(&self, moves: &[GameId]) -> Option<u32> {
        let mut i = 0;
        let mut mex = 0;
        loop {
            if i >= moves.len() {
                break;
            }

            let game = self.get_game(moves[i]);
            if !game.is_nimber() {
                return None;
            }

            if game.nus.unwrap().nimber == Nimber::from(mex) {
                mex += 1;
            } else {
                // It's a nimber, but exceeds mex.  We've found the true
                // mex - *provided* everything that remains is a nimber.
                break;
            }
            i += 1;
        }

        for m in moves[i..].iter() {
            if !self.get_game(*m).is_nimber() {
                return None;
            }
        }

        Some(mex)
    }

    fn eliminate_dominated_options(
        &self,
        moves: &[GameId],
        eliminate_smaller_options: bool,
    ) -> Vec<GameId> {
        let mut options: Vec<Option<GameId>> = moves.iter().cloned().map(Some).collect();

        for i in 0..options.len() {
            let option_i = match options[i] {
                None => continue,
                Some(id) => id,
            };
            for j in 0..i {
                let option_j = match options[j] {
                    None => continue,
                    Some(id) => id,
                };

                if (eliminate_smaller_options && self.leq(option_i, option_j))
                    || (!eliminate_smaller_options && self.leq(option_j, option_i))
                {
                    options[i] = None;
                }
                if (eliminate_smaller_options && self.leq(option_j, option_i))
                    || (!eliminate_smaller_options && self.leq(option_i, option_j))
                {
                    options[j] = None;
                }
            }
        }

        cat_options(&options)
    }

    fn canonicalize_options(&self, options: Options) -> Options {
        let options = self.bypass_reversible_options_l(options);
        let options = self.bypass_reversible_options_r(options);

        let left = self.eliminate_dominated_options(&options.left, true);
        let right = self.eliminate_dominated_options(&options.right, false);

        let options = Options { left, right };
        options
    }

    /// Safe function to construct a game from a list of options
    pub fn construct_from_options(&self, mut options: Options) -> GameId {
        options.eliminate_duplicates();

        let left_mex = self.mex(&options.left);
        let right_mex = self.mex(&options.right);
        if let (Some(left_mex), Some(right_mex)) = (left_mex, right_mex) {
            if left_mex == right_mex {
                let nus = Nus {
                    number: DyadicRationalNumber::from(0),
                    up_multiple: 0,
                    nimber: Nimber::from(left_mex),
                };
                return self.construct_nus(&nus);
            }
        }

        options = self.canonicalize_options(options);

        self.construct_from_canonical_options(options)
    }

    /// Inputs MUST be equal length
    fn compare_opts(&self, gs: &[GameId], hs: &[GameId]) -> Ordering {
        debug_assert!(gs.len() == gs.len(), "Inputs are not equal length");
        for (g, h) in gs.iter().zip(hs) {
            let cmp = self.compare(*g, *h);
            match cmp {
                Ordering::Equal => continue,
                _ => return cmp,
            }
        }
        Ordering::Equal
    }

    fn birthday(&self, id: GameId) -> i64 {
        let game = self.get_game(id);
        if let Some(nus) = &game.nus {
            let den_exp = nus.number.denominator_exponent();
            let up_mag = nus.up_multiple.abs() as i64;
            let nimber = nus.nimber;

            let number_birthday: i64;

            let num_mag = nus.number.numerator().abs();
            number_birthday = if den_exp == 0 {
                num_mag
            } else {
                1 + (num_mag >> den_exp) as i64 + den_exp as i64
            };

            let up_star_birthday: i64 = if up_mag > 0 && nimber == Nimber::from(0) {
                up_mag + 1
            } else if (up_mag & 1) == 1 && nimber != Nimber::from(1) {
                up_mag + (nimber.get() as i64 ^ 1)
            } else {
                up_mag + nimber.get() as i64
            };
            return number_birthday + up_star_birthday;
        }

        if let Some(birthday) = self.birthday_index.get(&id) {
            return birthday;
        }

        let mut birthday: i64 = 0;
        for left_opt in game.options.left {
            birthday = cmp::max(birthday, self.birthday(left_opt) + 1);
        }
        for right_opt in game.options.right {
            birthday = cmp::max(birthday, self.birthday(right_opt) + 1);
        }
        self.birthday_index.insert(id, birthday);

        birthday
    }

    pub fn compare(&self, lhs: GameId, rhs: GameId) -> Ordering {
        if lhs == rhs {
            return Ordering::Equal;
        }

        let cmp: i64 = self.birthday(lhs) - self.birthday(rhs);
        if cmp != 0 {
            return Ord::cmp(&cmp, &0);
        }

        let l = self.get_game(lhs);
        let r = self.get_game(rhs);

        let cmp = l.options.left.len() as i32 - r.options.left.len() as i32;
        if cmp != 0 {
            return Ord::cmp(&cmp, &0);
        }

        let cmp = l.options.right.len() as i32 - r.options.right.len() as i32;
        if cmp != 0 {
            return Ord::cmp(&cmp, &0);
        }

        let cmp = self.compare_opts(&l.options.left, &r.options.left);
        if cmp != Ordering::Equal {
            return cmp;
        }

        let cmp = self.compare_opts(&l.options.right, &r.options.right);
        if cmp != Ordering::Equal {
            return cmp;
        }

        dbg!(lhs, rhs);
        panic!("compare: Unreachable")
    }

    pub fn leq(&self, lhs: GameId, rhs: GameId) -> bool {
        if lhs == rhs {
            return true;
        }

        let lhs_game = self.get_game(lhs);
        let rhs_game = self.get_game(rhs);

        if let Some(lhs_nus) = &lhs_game.nus {
            if let Some(rhs_nus) = &rhs_game.nus {
                let cmp = self.compare_number_parts(lhs, rhs);
                if cmp < 0 {
                    return true;
                } else if cmp > 0 {
                    return false;
                } else {
                    if lhs_nus.up_multiple < rhs_nus.up_multiple - 1 {
                        return true;
                    } else if lhs_nus.up_multiple < rhs_nus.up_multiple {
                        return (lhs_nus.nimber + rhs_nus.nimber) != Nimber::from(1);
                    } else {
                        return false;
                    }
                }
            }
        }

        if let Some(leq) = self.leq_index.get(&(lhs, rhs)) {
            return leq;
        }

        let mut leq = true;

        if !lhs_game.is_number() {
            for lhs_l in &lhs_game.options.left {
                if self.leq(rhs, *lhs_l) {
                    leq = false;
                    break;
                }
            }
        }

        if leq && !rhs_game.is_number() {
            for rhs_r in &rhs_game.options.right {
                if self.leq(*rhs_r, lhs) {
                    leq = false;
                    break;
                }
            }
        }

        self.leq_index.insert((lhs, rhs), leq);

        leq
    }

    /// Return false if `H <= GL` for some left option `GL` of `G` or `HR <= G` for some right
    /// option `HR` of `H`. Otherwise return true.
    fn leq_arrays(
        &self,
        game_id: GameId,
        left_options: &[Option<GameId>],
        right_options: &[Option<GameId>],
    ) -> bool {
        for r_opt in right_options {
            if let Some(r_opt) = r_opt {
                if self.leq(*r_opt, game_id) {
                    return false;
                }
            }
        }

        let game = self.get_game(game_id);
        for l_opt in game.options.left {
            if self.geq_arrays(l_opt, left_options, right_options) {
                return false;
            }
        }

        true
    }

    fn geq_arrays(
        &self,
        game_id: GameId,
        left_options: &[Option<GameId>],
        right_options: &[Option<GameId>],
    ) -> bool {
        for l_opt in left_options {
            if let Some(l_opt) = l_opt {
                if self.leq(game_id, *l_opt) {
                    return false;
                }
            }
        }

        let game = self.get_game(game_id);
        for r_opt in game.options.right {
            if self.leq_arrays(r_opt, left_options, right_options) {
                return false;
            }
        }

        true
    }

    // TODO: Write it in "Rust way"
    fn bypass_reversible_options_l(&self, options: Options) -> Options {
        let mut i: i64 = 0;

        let mut left_options: Vec<Option<GameId>> =
            options.left.iter().cloned().map(Some).collect();
        let right_options: Vec<Option<GameId>> = options.right.iter().cloned().map(Some).collect();

        loop {
            if (i as usize) >= left_options.len() {
                break;
            }
            let g_l = match left_options[i as usize] {
                None => {
                    i += 1;
                    continue;
                }
                Some(id) => self.get_game(id),
            };
            for g_lr_id in g_l.options.right {
                let g_lr = self.get_game(g_lr_id);
                if self.leq_arrays(g_lr_id, &left_options, &right_options) {
                    let mut new_left_options: Vec<Option<GameId>> =
                        vec![None; left_options.len() + g_lr.options.left.len() as usize - 1];
                    for k in 0..(i as usize) {
                        new_left_options[k] = left_options[k];
                    }
                    for k in (i as usize + 1)..left_options.len() {
                        new_left_options[k - 1] = left_options[k];
                    }
                    for (k, g_lrl) in g_lr.options.left.iter().enumerate() {
                        if left_options.contains(&Some(*g_lrl)) {
                            new_left_options[left_options.len() + k - 1] = None;
                        } else {
                            new_left_options[left_options.len() + k - 1] = Some(*g_lrl);
                        }
                    }
                    left_options = new_left_options;
                    i -= 1;
                    break;
                }
            }

            i += 1;
        }
        Options {
            left: cat_options(&left_options),
            right: options.right,
        }
    }

    fn bypass_reversible_options_r(&self, options: Options) -> Options {
        let mut i: i64 = 0;

        let left_options: Vec<Option<GameId>> = options.left.iter().cloned().map(Some).collect();
        let mut right_options: Vec<Option<GameId>> =
            options.right.iter().cloned().map(Some).collect();

        loop {
            if (i as usize) >= right_options.len() {
                break;
            }
            let g_r = match right_options[i as usize] {
                None => {
                    i += 1;
                    continue;
                }
                Some(id) => self.get_game(id),
            };
            for g_rl_id in g_r.options.left {
                let g_rl = self.get_game(g_rl_id);
                if self.geq_arrays(g_rl_id, &left_options, &right_options) {
                    let mut new_right_options: Vec<Option<GameId>> =
                        vec![None; right_options.len() + g_rl.options.right.len() as usize - 1];
                    for k in 0..(i as usize) {
                        new_right_options[k] = right_options[k];
                    }
                    for k in (i as usize + 1)..right_options.len() {
                        new_right_options[k - 1] = right_options[k];
                    }
                    for (k, g_rlr) in g_rl.options.right.iter().enumerate() {
                        if right_options.contains(&Some(*g_rlr)) {
                            new_right_options[right_options.len() + k - 1] = None;
                        } else {
                            new_right_options[right_options.len() + k - 1] = Some(*g_rlr);
                        }
                    }
                    right_options = new_right_options;
                    i -= 1;
                    break;
                }
            }

            i += 1;
        }
        Options {
            left: options.left,
            right: cat_options(&right_options),
        }
    }

    /// Construct a game from list of left and right moves
    /// Unsafe if input is non canonical
    fn construct_from_canonical_options(&self, mut options: Options) -> GameId {
        options.left.sort();
        options.right.sort();

        if let Some(game_id) = self.get_game_id_by_options(&options) {
            return game_id;
        }

        if let Some(game_id) = self.construct_as_nus_entry(&options) {
            return game_id;
        }

        // Game is not a nus
        let game = Game { nus: None, options };
        self.add_new_game(game)
    }

    fn construct_as_nus_entry(&self, options: &Options) -> Option<GameId> {
        let number: DyadicRationalNumber;
        let up_multiple: i32;
        let nimber: Nimber;

        let num_lo = options.left.len();
        let num_ro = options.right.len();

        if num_lo == 0 {
            if num_ro == 0 {
                number = DyadicRationalNumber::from(0);
            } else {
                // We assume that entry is normalized, no left options, thus there must be only one
                // right entry that's a number
                debug_assert!(num_ro == 1, "Entry not normalized");
                number = self.get_game(options.right[0]).nus.unwrap().number
                    - DyadicRationalNumber::from(1);
            }
            up_multiple = 0;
            nimber = Nimber::from(0);
        } else if num_ro == 0 {
            debug_assert!(num_lo == 1, "Entry not normalized");
            number =
                self.get_game(options.left[0]).nus.unwrap().number + DyadicRationalNumber::from(1);
            up_multiple = 0;
            nimber = Nimber::from(0);
        } else if num_lo == 1
            && num_ro == 1
            && self.get_game(options.left[0]).is_number()
            && self.get_game(options.right[0]).is_number()
            && self.compare_number_parts(options.left[0], options.right[0]) < 0
        {
            // We're a number but not an integer.  Conveniently, since the
            // option lists are canonicalized, the value of this game is the
            // mean of its left & right options.
            let l_num = self.get_game(options.left[0]).nus.unwrap().number;
            let r_num = self.get_game(options.right[0]).nus.unwrap().number;
            number = DyadicRationalNumber::mean(&l_num, &r_num);
            up_multiple = 0;
            nimber = Nimber::from(0);
        } else if num_lo == 2
            && num_ro == 1
            && self.get_game(options.left[0]).is_number()
            && options.left[0] == options.right[0]
            && self.get_game(options.left[1]).is_number_up_star()
            && self.compare_number_parts(options.left[0], options.left[1]) == 0
            && self.get_game(options.left[1]).nus.unwrap().up_multiple == 0
            && self.get_game(options.left[1]).nus.unwrap().nimber == Nimber::from(1)
        {
            number = self.get_game(options.left[0]).nus.unwrap().number;
            up_multiple = 1;
            nimber = Nimber::from(1);
        } else if num_lo == 1
            && num_ro == 2
            && self.get_game(options.left[0]).is_number()
            && options.left[0] == options.right[0]
            && self.get_game(options.right[1]).is_number_up_star()
            && self.compare_number_parts(options.right[0], options.right[1]) == 0
            && self.get_game(options.right[1]).nus.unwrap().up_multiple == 0
            && self.get_game(options.right[1]).nus.unwrap().nimber == Nimber::from(1)
        {
            number = self.get_game(options.right[0]).nus.unwrap().number;
            up_multiple = -1;
            nimber = Nimber::from(1);
        } else if num_lo == 1
            && num_ro == 1
            && self.get_game(options.left[0]).is_number()
            && self.get_game(options.right[0]).is_number_up_star()
            && !self.get_game(options.right[0]).is_number()
            && self.compare_number_parts(options.left[0], options.right[0]) == 0
            && self.get_game(options.right[0]).nus.unwrap().up_multiple >= 0
        {
            // This is of the form n + {0|G} where G is a number-up-star of up multiple >= 0.
            number = self.get_game(options.left[0]).nus.unwrap().number;
            up_multiple = self.get_game(options.right[0]).nus.unwrap().up_multiple + 1;
            nimber = self.get_game(options.right[0]).nus.unwrap().nimber + Nimber::from(1);
        } else if num_lo == 1
            && num_ro == 1
            && self.get_game(options.right[0]).is_number()
            && self.get_game(options.left[0]).is_number_up_star()
            && !self.get_game(options.left[0]).is_number()
            && self.compare_number_parts(options.left[0], options.right[0]) == 0
            && self.get_game(options.left[0]).nus.unwrap().up_multiple <= 0
        {
            // This is of the form n + {0|G} where G is a number-up-star of up multiple >= 0.
            number = self.get_game(options.left[0]).nus.unwrap().number;
            up_multiple = self.get_game(options.left[0]).nus.unwrap().up_multiple - 1;
            nimber = self.get_game(options.left[0]).nus.unwrap().nimber + Nimber::from(1);
        } else if num_lo >= 1
            && num_lo == num_ro
            && self.get_game(options.left[0]).is_number()
            && options.left[0] == options.right[0]
        {
            // Last we need to check for games of the form n + *k.
            for i in 0..num_lo {
                let l_id = options.left[i];
                let l = self.get_game(l_id);
                let r_id = options.right[i];

                if l_id != r_id
                    || !l.is_number_up_star()
                    || self.compare_number_parts(l_id, r_id) != 0
                {
                    return None;
                }
                let l_nus = l.nus.unwrap();
                if l_nus.up_multiple != 0 || l_nus.nimber.get() != (i as u32) {
                    return None;
                }
            }
            // It's a nimber
            number = self.get_game(options.left[0]).nus.unwrap().number;
            up_multiple = 0;
            nimber = Nimber::from(num_lo as u32);
        } else {
            return None;
        }

        let nus = Nus {
            number,
            up_multiple,
            nimber,
        };
        let game = Game {
            nus: Some(nus),
            options: options.clone(),
        };
        Some(self.add_new_game(game))
    }

    pub fn construct_nus(&self, nus: &Nus) -> GameId {
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
                nimber: Nimber::from(nus.nimber.get() ^ parity ^ ((last_defined & 1) as u32)),
            };
            if let Some(id) = self.get_game_id_by_nus(&tmp_nus) {
                last_defined_id = Some(id);
                break;
            }
            last_defined -= sign;
        }

        let mut last_defined_id = last_defined_id
            .unwrap_or(self.construct_nimber(nus.number, nus.nimber + Nimber::from(parity)));

        let mut i = last_defined + sign;
        loop {
            if i == nus.up_multiple + sign {
                break;
            }

            let current_nimber = nus.nimber.get() ^ parity ^ (i as u32 & 1);
            let new_nus = Nus {
                number: nus.number,
                up_multiple: i,
                nimber: Nimber::from(current_nimber),
            };
            let new_options;

            if i == 1 && current_nimber == 1 {
                // special case for n^*
                let star_option = self.construct_nus(&Nus {
                    number: nus.number,
                    up_multiple: 0,
                    nimber: Nimber::from(1),
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
                    nimber: Nimber::from(1),
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
    pub fn construct_rational(&self, rational: DyadicRationalNumber) -> GameId {
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
    pub fn construct_nimber(&self, rational: DyadicRationalNumber, nimber: Nimber) -> GameId {
        // Find last defined nimber smaller than the requested one
        let mut last_defined = nimber.get();
        let mut last_defined_id = None;
        loop {
            if last_defined == 0 {
                break;
            }

            let nus = Nus {
                number: rational,
                up_multiple: 0,
                nimber: Nimber::from(last_defined),
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

        for i in (last_defined + 1)..=nimber.get() {
            let previous_nimber = self.get_game(last_defined_id);
            let nus = Nus {
                number: rational,
                up_multiple: 0,
                nimber: Nimber::from(i),
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
    pub fn construct_integer(&self, number: i64) -> GameId {
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

    pub fn thermograph(&self, id: GameId) -> Thermograph {
        let g = self.get_game(id);
        let thermograph = match &g.nus {
            None => {
                if let Some(thermograph) = self.thermograph_index.get(&id) {
                    return thermograph.clone();
                }
                self.thermograph_from_options(&g.options)
            }
            Some(nus) => {
                if let Some(int) = nus.number.to_integer() && nus.is_number() {
		    Thermograph::with_mast(Rational::new(int, 1))
		} else {
		    if nus.up_multiple == 0 || (nus.nimber == Nimber::from(1) && nus.up_multiple.abs() == 1) {
			// This looks like 0 or * (depending on whether nimberPart is 0 or 1).
			let new_id = self.construct_nus(&Nus{
			    number: nus.number,
			    up_multiple: 0,
			    nimber: Nimber::from(nus.nimber.get().cmp(&0) as u32), // signum(nus.nimber)
			});
			let new_game = self.get_game(new_id);
			self.thermograph_from_options(&new_game.options)
		    } else {
			let new_id = self.construct_nus(&Nus{
			    number: nus.number,
			    up_multiple: nus.up_multiple.cmp(&0) as i32, // signum(nus.up_multiple)
			    nimber: Nimber::from(0),
			});
			let new_game = self.get_game(new_id);
			self.thermograph_from_options(&new_game.options)
		    }
		}
            }
        };

        self.thermograph_index.insert(id, thermograph.clone());

        thermograph
    }

    pub(crate) fn thermograph_from_options(&self, options: &Options) -> Thermograph {
        let mut left_scaffold = Trajectory::new_constant(Rational::NegativeInfinity);
        let mut right_scaffold = Trajectory::new_constant(Rational::PositiveInfinity);

        for left_move in &options.left {
            left_scaffold = left_scaffold.max(&self.thermograph(*left_move).right_wall);
        }
        for right_move in &options.right {
            right_scaffold = right_scaffold.min(&self.thermograph(*right_move).left_wall);
        }

        left_scaffold = left_scaffold.tilt(Rational::from(-1));
        right_scaffold = right_scaffold.tilt(Rational::from(1));

        Thermograph::thermographic_intersection(left_scaffold, right_scaffold)
    }

    pub fn temperature(&self, id: GameId) -> Rational {
        let game = self.get_game(id);
        match &game.nus {
            Some(nus) => {
                if nus.is_number() {
                    // It's a number k/2^n, so the temperature is -1/2^n
                    // DyadicRationalNumber::new(-1, nus.number.denominator_exponent())
                    Rational::new(-1, nus.number.denominator().unwrap() as u32)
                } else {
                    // It's a number plus a nonzero infinitesimal, thus the temperature is 0
                    // DyadicRationalNumber::from(0)
                    Rational::from(0)
                }
            }
            None => self.thermograph(id).get_temperature(),
        }
    }

    pub fn dump_game<W>(&self, id: GameId, f: &mut W) -> fmt::Result
    where
        W: Write,
    {
        let game = self.get_game(id);
        if let Some(nus) = game.nus {
            write!(f, "{}", nus)?;
        } else {
            self.dump_options(&game.options, f)?;
        }
        Ok(())
    }

    pub fn dump_options<W>(&self, options: &Options, f: &mut W) -> fmt::Result
    where
        W: Write,
    {
        write!(f, "{{")?;
        for (idx, l) in options.left.iter().enumerate() {
            if idx != 0 {
                write!(f, ",")?;
            }
            self.dump_game(*l, f)?;
        }
        write!(f, "|")?;
        for (idx, r) in options.right.iter().enumerate() {
            if idx != 0 {
                write!(f, ",")?;
            }
            self.dump_game(*r, f)?;
        }
        write!(f, "}}")?;
        Ok(())
    }

    pub fn dump_options_to_str(&self, options: &Options) -> String {
        let mut buf = String::new();
        self.dump_options(options, &mut buf).unwrap();
        buf
    }

    pub fn dump_game_to_str(&self, id: GameId) -> String {
        let mut buf = String::new();
        self.dump_game(id, &mut buf).unwrap();
        buf
    }

    /// Initialize new game storage
    pub fn new() -> GameBackend {
        let mut res = GameBackend {
            known_games: RwLock::new(Vec::new()),
            nus_index: RwHashMap::new(),
            options_index: RwHashMap::new(),
            negative_index: RwHashMap::new(),
            birthday_index: RwHashMap::new(),
            leq_index: RwHashMap::new(),
            add_index: RwHashMap::new(),
            thermograph_index: RwHashMap::new(),
            zero_id: GameId(0), // Set below
            star_id: GameId(0),
            up_id: GameId(0),
            up_star_id: GameId(0),
            one_id: GameId(0),
            negative_one_id: GameId(0),
            two_id: GameId(0),
            negative_two_id: GameId(0),
        };

        // Construct 0 by hand - special case, don't use construct_* here
        let zero = Game {
            nus: Some(Nus::integer(0)),
            options: Options {
                left: Vec::new(),
                right: Vec::new(),
            },
        };
        res.zero_id = res.add_new_game(zero);

        res.star_id = res.construct_nimber(DyadicRationalNumber::from(0), Nimber::from(1));

        let up = Nus {
            number: DyadicRationalNumber::from(0),
            up_multiple: 1,
            nimber: Nimber::from(0),
        };
        res.up_id = res.construct_nus(&up);

        let up_star = Nus {
            number: DyadicRationalNumber::from(0),
            up_multiple: 1,
            nimber: Nimber::from(1),
        };
        res.up_star_id = res.construct_nus(&up_star);

        res.one_id = res.construct_integer(1);
        res.negative_one_id = res.construct_integer(-1);
        res.two_id = res.construct_integer(2);
        res.negative_two_id = res.construct_integer(-2);

        res
    }
}

#[test]
fn constructs_integers() {
    let b = GameBackend::new();

    let eight_id = b.construct_integer(8);
    let eight = b.get_game(eight_id);
    assert_eq!(format!("{}", &eight.nus.unwrap()), "8".to_string());

    let minus_forty_two_id = b.construct_integer(-42);
    let minus_forty_two = b.get_game(minus_forty_two_id);
    assert_eq!(
        format!("{}", &minus_forty_two.nus.unwrap()),
        "-42".to_string()
    );

    let duplicate = b.construct_integer(8);
    assert_eq!(eight_id, duplicate);
}

#[test]
fn constructs_rationals() {
    let b = GameBackend::new();

    let rational = DyadicRationalNumber::new(3, 4);
    let three_sixteenth_id = b.construct_rational(rational);
    let three_sixteenth = b.get_game(three_sixteenth_id);
    assert_eq!(
        format!("{}", &three_sixteenth.nus.unwrap()),
        "3/16".to_string()
    );

    let duplicate = b.construct_rational(rational);
    assert_eq!(three_sixteenth_id, duplicate);
}

#[test]
fn constructs_nimbers() {
    let b = GameBackend::new();

    let star_id = b.construct_nimber(DyadicRationalNumber::from(1), Nimber::from(4));
    let star = b.get_game(star_id);
    assert_eq!(format!("{}", &star.nus.unwrap()), "1*4".to_string());

    let duplicate = b.construct_nimber(DyadicRationalNumber::from(1), Nimber::from(4));
    assert_eq!(star_id, duplicate);
}

#[test]
fn constructs_up() {
    let b = GameBackend::new();

    let up_id = b.construct_nus(&Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: 1,
        nimber: Nimber::from(0),
    });
    let up = b.get_game(up_id);
    assert_eq!(format!("{}", &up.nus.unwrap()), "^".to_string());

    let up_star_id = b.construct_nus(&Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: 1,
        nimber: Nimber::from(1),
    });
    let up_star = b.get_game(up_star_id);
    assert_eq!(format!("{}", &up_star.nus.unwrap()), "^*".to_string());

    let down_id = b.construct_nus(&Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: -3,
        nimber: Nimber::from(0),
    });
    let down = b.get_game(down_id);
    assert_eq!(format!("{}", &down.nus.unwrap()), "v3".to_string());
}

#[test]
fn nimber_is_its_negative() {
    let b = GameBackend::new();

    let star_id = b.construct_nimber(DyadicRationalNumber::from(0), Nimber::from(4));
    let star = b.get_game(star_id);
    assert_eq!(format!("{}", &star.nus.unwrap()), "*4".to_string());

    let neg_star_id = b.construct_negative(star_id);
    assert_eq!(star_id, neg_star_id);
}

#[test]
fn simplifies_options() {
    let b = GameBackend::new();

    let options_l = Options {
        left: vec![b.one_id],
        right: vec![b.star_id],
    };
    let left_id = b.construct_from_options(options_l);
    assert_eq!(b.dump_game_to_str(left_id), "{1|*}".to_string());

    let options = Options {
        left: vec![left_id],
        right: vec![b.zero_id],
    };
    let id = b.construct_from_options(options);
    assert_eq!(b.dump_game_to_str(id), "*".to_string());

    let options = Options {
        left: vec![],
        right: vec![b.negative_one_id, b.negative_one_id, b.zero_id],
    };
    let id = b.construct_from_options(options);
    assert_eq!(b.dump_game_to_str(id), "-2".to_string());

    let options = Options {
        left: vec![b.zero_id, b.negative_one_id],
        right: vec![b.one_id],
    };
    let id = b.construct_from_options(options);
    assert_eq!(b.dump_game_to_str(id), "1/2".to_string());
}

#[test]
fn sum_works() {
    let b = GameBackend::new();
    let one_zero = b.construct_from_options(Options {
        left: vec![b.one_id],
        right: vec![b.zero_id],
    });
    let zero_one = b.construct_from_options(Options {
        left: vec![b.zero_id],
        right: vec![b.one_id],
    });
    let sum = b.construct_sum(one_zero, zero_one);
    assert_eq!(&b.dump_game_to_str(sum), "{3/2|1/2}");
}

#[test]
fn temp_of_one_minus_one_is_one() {
    let b = GameBackend::new();
    let options = Options {
        left: vec![b.one_id],
        right: vec![b.negative_one_id],
    };
    let g = b.construct_from_options(options);
    assert_eq!(b.temperature(g), Rational::from(1));
}

#[derive(Debug)]
pub enum GameBackendFileError {
    DecodeError(Box<bincode::ErrorKind>),
    FileError(std::io::Error),
}

impl From<Box<bincode::ErrorKind>> for GameBackendFileError {
    fn from(value: Box<bincode::ErrorKind>) -> Self {
        GameBackendFileError::DecodeError(value)
    }
}

impl From<std::io::Error> for GameBackendFileError {
    fn from(value: std::io::Error) -> Self {
        GameBackendFileError::FileError(value)
    }
}

impl GameBackend {
    pub fn save_to_file(&self, filepath: &str) -> Result<(), GameBackendFileError> {
        let serialized = bincode::serialize(self)?;
        let mut f = File::create(filepath)?;
        std::io::Write::write_all(&mut f, &serialized)?;
        Ok(())
    }

    pub fn load_from_file(filepath: &str) -> Result<Self, GameBackendFileError> {
        let f = File::open(filepath)?;
        let mut reader = BufReader::new(f);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        let res = bincode::deserialize::<Self>(&buffer)?;
        Ok(res)
    }
}
