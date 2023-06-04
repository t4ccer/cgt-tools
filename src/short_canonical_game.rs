use crate::{
    dyadic_rational_number::DyadicRationalNumber, nimber::Nimber, rational::Rational,
    rw_hash_map::RwHashMap, thermograph::Thermograph, trajectory::Trajectory,
};
use elsa::sync::FrozenVec;
use std::{
    cmp::Ordering,
    fmt::{self, Display, Write},
    ops::Add,
    sync::Mutex,
};

/// Opaque, unique game identifier. Must be used only with `GameBackend` that produced it.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GameId(usize);

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Game {
    pub nus: Option<Nus>,
    pub moves: Moves,
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

/// A number-up-star game position that is a sum of a number, up and, nimber.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Nus {
    pub number: DyadicRationalNumber,
    pub up_multiple: i32,
    pub nimber: Nimber,
}

impl Nus {
    /// Create new number-up-star game equal to an integer.
    pub fn new_integer(number: i64) -> Nus {
        Nus {
            number: DyadicRationalNumber::from(number),
            up_multiple: 0,
            nimber: Nimber::from(0),
        }
    }

    /// Create new number-up-star game equal to an rational.
    pub fn new_rational(number: DyadicRationalNumber) -> Self {
        Nus {
            number,
            up_multiple: 0,
            nimber: Nimber::from(0),
        }
    }

    /// Check if the game has only number part (i.e. up multiple and nimber are zero).
    pub fn is_number(&self) -> bool {
        self.up_multiple == 0 && self.nimber == Nimber::from(0)
    }

    /// Check if the game is a nimber.
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

/// Left and Right moves from a given position
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Moves {
    pub left: Vec<GameId>,
    pub right: Vec<GameId>,
}

impl Moves {
    pub fn empty() -> Self {
        Moves {
            left: Vec::new(),
            right: Vec::new(),
        }
    }

    fn eliminate_duplicates(&mut self) {
        self.left.sort();
        self.left.dedup();

        self.right.sort();
        self.right.dedup();
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GameBackend {
    /// Lock that **MUST** be taken when adding new game
    add_game_lock: Mutex<()>,
    /// Games that were already constructed
    known_games: FrozenVec<Box<Game>>,
    /// Lookup table for number-up-star games
    nus_index: RwHashMap<Nus, GameId>,
    /// Lookup table for list of moves
    moves_index: RwHashMap<Moves, GameId>,
    /// Lookup table for game inverses
    negative_index: RwHashMap<GameId, GameId>,
    /// Lookup table for comparison
    leq_index: RwHashMap<(GameId, GameId), bool>,
    /// Lookup table for game addition
    add_index: RwHashMap<(GameId, GameId), GameId>,
    /// Lookup table for already constructed thermographs of non-trivial games
    thermograph_index: RwHashMap<GameId, Thermograph>,
    /// Precomputated `0` game
    pub zero_id: GameId,
    /// Precomputated `*` game
    pub star_id: GameId,
    /// Precomputated `^` game
    pub up_id: GameId,
    /// Precomputated `^ + *` game
    pub up_star_id: GameId,
    /// Precomputated `1` game
    pub one_id: GameId,
    /// Precomputated `-1` game
    pub negative_one_id: GameId,
    /// Precomputated `2` game
    pub two_id: GameId,
    /// Precomputated `-2` game
    pub negative_two_id: GameId,
}

fn cat_options<T>(options: &[Option<T>]) -> Vec<T>
where
    T: Copy,
{
    options.iter().flatten().copied().collect()
}

impl GameBackend {
    /// Initialize new game storage
    pub fn new() -> GameBackend {
        let mut res = GameBackend {
            add_game_lock: Mutex::new(()),
            known_games: FrozenVec::new(),
            nus_index: RwHashMap::new(),
            moves_index: RwHashMap::new(),
            negative_index: RwHashMap::new(),
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
            nus: Some(Nus::new_integer(0)),
            moves: Moves {
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

    /// Add new game to the index
    fn add_new_game(&self, game: Game) -> GameId {
        // Locking here guarantees that no two threads will try to insert the same game
        let lock = self.add_game_lock.lock().unwrap();

        // Check if already present
        if let Some(id) = game.nus.as_ref().and_then(|nus| self.nus_index.get(nus)) {
            return id;
        }
        if let Some(id) = self.moves_index.get(&game.moves) {
            return id;
        }

        let id = GameId(self.known_games.push_get_index(Box::new(game.clone())));

        // Populate indices
        game.nus.map(|nus| self.nus_index.insert(nus, id));
        self.moves_index.insert(game.moves, id);

        drop(lock);
        id
    }

    fn get_game_id_by_nus(&self, nus: &Nus) -> Option<GameId> {
        self.nus_index.get(nus)
    }

    fn get_game_id_by_moves(&self, moves: &Moves) -> Option<GameId> {
        self.moves_index.get(moves)
    }

    fn get_game(&self, game_id: GameId) -> &Game {
        self.known_games.get(game_id.0).unwrap()
    }

    /// Construct a negative of given game
    pub fn construct_negative(&self, game_id: GameId) -> GameId {
        // If game is a nus, just take the negative of components
        let game = self.get_game(game_id);
        if let Some(nus) = &game.nus {
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

        let new_left_moves = game
            .moves
            .left
            .iter()
            .map(|left_id| self.construct_negative(*left_id))
            .collect::<Vec<_>>();

        let new_right_moves = game
            .moves
            .right
            .iter()
            .map(|left_id| self.construct_negative(*left_id))
            .collect::<Vec<_>>();

        let neg_moves = Moves {
            left: new_left_moves,
            right: new_right_moves,
        };
        let neg_id = self.construct_from_canonical_moves(neg_moves);

        self.negative_index.insert(game_id, neg_id);

        neg_id
    }

    /// Construct a sum of two games
    pub fn construct_sum(&self, g_id: GameId, h_id: GameId) -> GameId {
        let g = self.get_game(g_id);
        let h = self.get_game(h_id);

        if let (Some(g_nus), Some(h_nus)) = (&g.nus, &h.nus) {
            return self.construct_nus(&(g_nus + h_nus));
        }

        if let Some(result) = self.add_index.get(&(g_id, h_id)) {
            return result;
        }

        // We want to return { GL+H, G+HL | GR+H, G+HR }

        // By the number translation theorem

        let mut moves = Moves::empty();

        if !g.is_number() {
            for g_l in &g.moves.left {
                moves.left.push(self.construct_sum(*g_l, h_id));
            }
            for g_r in &g.moves.right {
                moves.right.push(self.construct_sum(*g_r, h_id));
            }
        }
        if !h.is_number() {
            for h_l in &h.moves.left {
                moves.left.push(self.construct_sum(g_id, *h_l));
            }
            for h_r in &h.moves.right {
                moves.right.push(self.construct_sum(g_id, *h_r));
            }
        }

        let result = self.construct_from_moves(moves);
        self.add_index.insert((g_id, h_id), result);
        self.add_index.insert((h_id, g_id), result);
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

            if game.nus.as_ref().unwrap().nimber == Nimber::from(mex) {
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

    fn eliminate_dominated_moves(
        &self,
        moves: &[GameId],
        eliminate_smaller_moves: bool,
    ) -> Vec<GameId> {
        let mut moves: Vec<Option<GameId>> = moves.iter().cloned().map(Some).collect();

        for i in 0..moves.len() {
            let move_i = match moves[i] {
                None => continue,
                Some(id) => id,
            };
            for j in 0..i {
                let move_j = match moves[j] {
                    None => continue,
                    Some(id) => id,
                };

                if (eliminate_smaller_moves && self.leq(move_i, move_j))
                    || (!eliminate_smaller_moves && self.leq(move_j, move_i))
                {
                    moves[i] = None;
                }
                if (eliminate_smaller_moves && self.leq(move_j, move_i))
                    || (!eliminate_smaller_moves && self.leq(move_i, move_j))
                {
                    moves[j] = None;
                }
            }
        }

        cat_options(&moves)
    }

    fn canonicalize_moves(&self, moves: Moves) -> Moves {
        let moves = self.bypass_reversible_moves_l(moves);
        let moves = self.bypass_reversible_moves_r(moves);

        let left = self.eliminate_dominated_moves(&moves.left, true);
        let right = self.eliminate_dominated_moves(&moves.right, false);

        Moves { left, right }
    }

    /// Safe function to construct a game from possible moves
    pub fn construct_from_moves(&self, mut moves: Moves) -> GameId {
        moves.eliminate_duplicates();

        let left_mex = self.mex(&moves.left);
        let right_mex = self.mex(&moves.right);
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

        moves = self.canonicalize_moves(moves);

        self.construct_from_canonical_moves(moves)
    }

    fn leq(&self, lhs: GameId, rhs: GameId) -> bool {
        if lhs == rhs {
            return true;
        }

        let lhs_game = self.get_game(lhs);
        let rhs_game = self.get_game(rhs);

        if let (Some(lhs_nus), Some(rhs_nus)) = (&lhs_game.nus, &rhs_game.nus) {
            match lhs_nus.number.cmp(&rhs_nus.number) {
                Ordering::Less => return true,
                Ordering::Greater => return false,
                Ordering::Equal => {
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
            for lhs_l in &lhs_game.moves.left {
                if self.leq(rhs, *lhs_l) {
                    leq = false;
                    break;
                }
            }
        }

        if leq && !rhs_game.is_number() {
            for rhs_r in &rhs_game.moves.right {
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
        left_moves: &[Option<GameId>],
        right_moves: &[Option<GameId>],
    ) -> bool {
        for r_move in right_moves {
            if let Some(r_opt) = r_move {
                if self.leq(*r_opt, game_id) {
                    return false;
                }
            }
        }

        let game = self.get_game(game_id);
        for l_move in &game.moves.left {
            if self.geq_arrays(*l_move, left_moves, right_moves) {
                return false;
            }
        }

        true
    }

    fn geq_arrays(
        &self,
        game_id: GameId,
        left_moves: &[Option<GameId>],
        right_moves: &[Option<GameId>],
    ) -> bool {
        for l_move in left_moves {
            if let Some(l_opt) = l_move {
                if self.leq(game_id, *l_opt) {
                    return false;
                }
            }
        }

        let game = self.get_game(game_id);
        for r_move in &game.moves.right {
            if self.leq_arrays(*r_move, left_moves, right_moves) {
                return false;
            }
        }

        true
    }

    // TODO: Write it in "Rust way"
    fn bypass_reversible_moves_l(&self, moves: Moves) -> Moves {
        let mut i: i64 = 0;

        let mut left_moves: Vec<Option<GameId>> = moves.left.iter().cloned().map(Some).collect();
        let right_moves: Vec<Option<GameId>> = moves.right.iter().cloned().map(Some).collect();

        loop {
            if (i as usize) >= left_moves.len() {
                break;
            }
            let g_l = match left_moves[i as usize] {
                None => {
                    i += 1;
                    continue;
                }
                Some(id) => self.get_game(id),
            };
            for g_lr_id in &g_l.moves.right {
                let g_lr = self.get_game(*g_lr_id);
                if self.leq_arrays(*g_lr_id, &left_moves, &right_moves) {
                    let mut new_left_moves: Vec<Option<GameId>> =
                        vec![None; left_moves.len() + g_lr.moves.left.len() as usize - 1];
                    for k in 0..(i as usize) {
                        new_left_moves[k] = left_moves[k];
                    }
                    for k in (i as usize + 1)..left_moves.len() {
                        new_left_moves[k - 1] = left_moves[k];
                    }
                    for (k, g_lrl) in g_lr.moves.left.iter().enumerate() {
                        if left_moves.contains(&Some(*g_lrl)) {
                            new_left_moves[left_moves.len() + k - 1] = None;
                        } else {
                            new_left_moves[left_moves.len() + k - 1] = Some(*g_lrl);
                        }
                    }
                    left_moves = new_left_moves;
                    i -= 1;
                    break;
                }
            }

            i += 1;
        }
        Moves {
            left: cat_options(&left_moves),
            right: moves.right,
        }
    }

    fn bypass_reversible_moves_r(&self, moves: Moves) -> Moves {
        let mut i: i64 = 0;

        let left_moves: Vec<Option<GameId>> = moves.left.iter().cloned().map(Some).collect();
        let mut right_moves: Vec<Option<GameId>> = moves.right.iter().cloned().map(Some).collect();

        loop {
            if (i as usize) >= right_moves.len() {
                break;
            }
            let g_r = match right_moves[i as usize] {
                None => {
                    i += 1;
                    continue;
                }
                Some(id) => self.get_game(id),
            };
            for g_rl_id in &g_r.moves.left {
                let g_rl = self.get_game(*g_rl_id);
                if self.geq_arrays(*g_rl_id, &left_moves, &right_moves) {
                    let mut new_right_moves: Vec<Option<GameId>> =
                        vec![None; right_moves.len() + g_rl.moves.right.len() as usize - 1];
                    for k in 0..(i as usize) {
                        new_right_moves[k] = right_moves[k];
                    }
                    for k in (i as usize + 1)..right_moves.len() {
                        new_right_moves[k - 1] = right_moves[k];
                    }
                    for (k, g_rlr) in g_rl.moves.right.iter().enumerate() {
                        if right_moves.contains(&Some(*g_rlr)) {
                            new_right_moves[right_moves.len() + k - 1] = None;
                        } else {
                            new_right_moves[right_moves.len() + k - 1] = Some(*g_rlr);
                        }
                    }
                    right_moves = new_right_moves;
                    i -= 1;
                    break;
                }
            }

            i += 1;
        }
        Moves {
            left: moves.left,
            right: cat_options(&right_moves),
        }
    }

    /// Construct a game from list of left and right moves
    /// Unsafe if input is non canonical
    fn construct_from_canonical_moves(&self, mut moves: Moves) -> GameId {
        moves.left.sort();
        moves.right.sort();

        if let Some(game_id) = self.get_game_id_by_moves(&moves) {
            return game_id;
        }

        if let Some(game_id) = self.construct_as_nus_entry(&moves) {
            return game_id;
        }

        // Game is not a nus
        let game = Game { nus: None, moves };
        self.add_new_game(game)
    }

    fn construct_as_nus_entry(&self, moves: &Moves) -> Option<GameId> {
        let number: DyadicRationalNumber;
        let up_multiple: i32;
        let nimber: Nimber;

        let num_lo = moves.left.len();
        let num_ro = moves.right.len();

        let left_moves: Vec<_> = moves.left.iter().map(|l_id| self.get_game(*l_id)).collect();
        let right_moves: Vec<_> = moves
            .right
            .iter()
            .map(|l_id| self.get_game(*l_id))
            .collect();

        if num_lo == 0 {
            if num_ro == 0 {
                // Case: {|}
                // No left or right moves so the game is 0
                number = DyadicRationalNumber::from(0);
            } else {
                // Case: {|n}
                // We assume that entry is normalized, no left moves, thus there must be only one
                // right entry that's a number
                debug_assert!(num_ro == 1, "Entry not normalized");
                number =
                    right_moves[0].nus.as_ref().unwrap().number - DyadicRationalNumber::from(1);
            }
            up_multiple = 0;
            nimber = Nimber::from(0);
        } else if num_ro == 0 {
            // Case: {n|}
            // No right options so there must be a left move that is a number
            debug_assert!(num_lo == 1, "Entry not normalized");
            number = left_moves[0].nus.as_ref().unwrap().number + DyadicRationalNumber::from(1);
            up_multiple = 0;
            nimber = Nimber::from(0);
        } else if num_lo == 1
            && num_ro == 1
            && left_moves[0].is_number()
            && right_moves[0].is_number()
            && left_moves[0]
                .nus
                .as_ref()
                .unwrap()
                .number
                .cmp(&right_moves[0].nus.as_ref().unwrap().number)
                .is_lt()
        {
            // Case: {n|m}, n < m
            // We're a number but not an integer.  Conveniently, since the option lists are
            // canonicalized, the value of this game is the mean of its left & right moves.
            let l_num = left_moves[0].nus.as_ref().unwrap().number;
            let r_num = right_moves[0].nus.as_ref().unwrap().number;
            number = DyadicRationalNumber::mean(&l_num, &r_num);
            up_multiple = 0;
            nimber = Nimber::from(0);
        } else if num_lo == 2
            && num_ro == 1
            && left_moves[0].is_number()
            && moves.left[0] == moves.right[0]
            && left_moves[1].is_number_up_star()
            && left_moves[0]
                .nus
                .as_ref()
                .unwrap()
                .number
                .cmp(&left_moves[1].nus.as_ref().unwrap().number)
                .is_eq()
            && left_moves[1].nus.as_ref().unwrap().up_multiple == 0
            && left_moves[1].nus.as_ref().unwrap().nimber == Nimber::from(1)
        {
            // Case: {G,H|G}
            number = left_moves[0].nus.as_ref().unwrap().number;
            up_multiple = 1;
            nimber = Nimber::from(1);
        } else if num_lo == 1
            && num_ro == 2
            && left_moves[0].is_number()
            && moves.left[0] == moves.right[0]
            && right_moves[1].is_number_up_star()
            && right_moves[0]
                .nus
                .as_ref()
                .unwrap()
                .number
                .cmp(&right_moves[1].nus.as_ref().unwrap().number)
                .is_eq()
            && right_moves[1].nus.as_ref().unwrap().up_multiple == 0
            && right_moves[1].nus.as_ref().unwrap().nimber == Nimber::from(1)
        {
            // Inverse of the previous one
            number = right_moves[0].nus.as_ref().unwrap().number;
            up_multiple = -1;
            nimber = Nimber::from(1);
        } else if num_lo == 1
            && num_ro == 1
            && left_moves[0].is_number()
            && right_moves[0].is_number_up_star()
            && !right_moves[0].is_number()
            && left_moves[0]
                .nus
                .as_ref()
                .unwrap()
                .number
                .cmp(&right_moves[0].nus.as_ref().unwrap().number)
                .is_eq()
            && right_moves[0].nus.as_ref().unwrap().up_multiple >= 0
        {
            // Case: n + {0|G}, G is a number-up-star of up multiple >= 0
            number = left_moves[0].nus.as_ref().unwrap().number;
            up_multiple = right_moves[0].nus.as_ref().unwrap().up_multiple + 1;
            nimber = right_moves[0].nus.as_ref().unwrap().nimber + Nimber::from(1);
        } else if num_lo == 1
            && num_ro == 1
            && right_moves[0].is_number()
            && left_moves[0].is_number_up_star()
            && !left_moves[0].is_number()
            && left_moves[0]
                .nus
                .as_ref()
                .unwrap()
                .number
                .cmp(&right_moves[0].nus.as_ref().unwrap().number)
                .is_eq()
            && left_moves[0].nus.as_ref().unwrap().up_multiple <= 0
        {
            // Inverse of the previous one
            number = left_moves[0].nus.as_ref().unwrap().number;
            up_multiple = left_moves[0].nus.as_ref().unwrap().up_multiple - 1;
            nimber = left_moves[0].nus.as_ref().unwrap().nimber + Nimber::from(1);
        } else if num_lo >= 1
            && num_lo == num_ro
            && left_moves[0].is_number()
            && moves.left[0] == moves.right[0]
        {
            // Case: n + *k
            // If doesn't hold then it's not a NUS
            for i in 0..num_lo {
                let l_id = moves.left[i];
                let l = left_moves[i];

                let r_id = moves.right[i];
                let r = right_moves[i];

                if l_id != r_id
                    || !l.is_number_up_star()
                    || l.nus
                        .as_ref()
                        .unwrap()
                        .number
                        .cmp(&r.nus.as_ref().unwrap().number)
                        .is_ne()
                {
                    return None;
                }

                if l.nus.as_ref().unwrap().up_multiple != 0
                    || l.nus.as_ref().unwrap().nimber.get() != (i as u32)
                {
                    return None;
                }
            }
            // It's a nimber
            number = left_moves[0].nus.as_ref().unwrap().number;
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
            moves: moves.clone(),
        };
        Some(self.add_new_game(game))
    }

    /// Construct a number-up-star game.
    /// If game is a number or nimber use `construct_number` or `construct_nimber` for better performance.
    pub fn construct_nus(&self, nus: &Nus) -> GameId {
        let parity: u32 = (nus.up_multiple & 1) as u32;
        let sign = if nus.up_multiple >= 0 { 1 } else { -1 };
        let number_move = self.construct_rational(nus.number);
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
            let new_moves;

            if i == 1 && current_nimber == 1 {
                // special case for n^*
                let star_move = self.construct_nus(&Nus {
                    number: nus.number,
                    up_multiple: 0,
                    nimber: Nimber::from(1),
                });
                new_moves = Moves {
                    left: vec![number_move, star_move],
                    right: vec![number_move],
                };
            } else if i == -1 && current_nimber == 1 {
                // special case for nv*
                let star_move = self.construct_nus(&Nus {
                    number: nus.number,
                    up_multiple: 0,
                    nimber: Nimber::from(1),
                });
                new_moves = Moves {
                    left: vec![number_move],
                    right: vec![number_move, star_move],
                };
            } else if i > 0 {
                new_moves = Moves {
                    left: vec![number_move],
                    right: vec![last_defined_id],
                };
            } else {
                new_moves = Moves {
                    left: vec![last_defined_id],
                    right: vec![number_move],
                };
            }

            let new_game = Game {
                nus: Some(new_nus),
                moves: new_moves,
            };
            last_defined_id = self.add_new_game(new_game);
            i += sign;
        }

        last_defined_id
    }

    /// Create a game equal to a rational number
    pub fn construct_rational(&self, rational: DyadicRationalNumber) -> GameId {
        // Return id if already constructed
        let nus = Nus::new_rational(rational);
        if let Some(id) = self.get_game_id_by_nus(&nus) {
            return id;
        }

        // Recursion base case - rational is integer
        if let Some(int) = rational.to_integer() {
            return self.construct_integer(int);
        }

        // Recursively construct the game
        let left_move = self.construct_rational(rational.step(-1));
        let right_move = self.construct_rational(rational.step(1));
        let game = Game {
            nus: Some(nus),
            moves: Moves {
                left: vec![left_move],
                right: vec![right_move],
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

            let mut moves = Moves {
                left: previous_nimber.moves.left.clone(),
                right: previous_nimber.moves.right.clone(),
            };
            moves.left.push(last_defined_id);
            moves.right.push(last_defined_id);
            let game = Game {
                nus: Some(nus),
                moves,
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
            if let Some(g_id) = self.get_game_id_by_nus(&Nus::new_integer(last_defined)) {
                last_defined_id = g_id;
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

            let prev = self
                .get_game_id_by_nus(&Nus::new_integer(i - sign))
                .unwrap();
            let new_game = Game {
                nus: Some(Nus::new_integer(i)),
                moves: Moves {
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
                self.thermograph_from_moves(&g.moves)
            }
            Some(nus) => {
                if nus.number.to_integer().is_some() && nus.is_number() {
                    Thermograph::with_mast(Rational::new(nus.number.to_integer().unwrap(), 1))
                } else {
                    if nus.up_multiple == 0
                        || (nus.nimber == Nimber::from(1) && nus.up_multiple.abs() == 1)
                    {
                        // This looks like 0 or * (depending on whether nimberPart is 0 or 1).
                        let new_id = self.construct_nus(&Nus {
                            number: nus.number,
                            up_multiple: 0,
                            nimber: Nimber::from(nus.nimber.get().cmp(&0) as u32), // signum(nus.nimber)
                        });
                        let new_game = self.get_game(new_id);
                        self.thermograph_from_moves(&new_game.moves)
                    } else {
                        let new_id = self.construct_nus(&Nus {
                            number: nus.number,
                            up_multiple: nus.up_multiple.cmp(&0) as i32, // signum(nus.up_multiple)
                            nimber: Nimber::from(0),
                        });
                        let new_game = self.get_game(new_id);
                        self.thermograph_from_moves(&new_game.moves)
                    }
                }
            }
        };

        self.thermograph_index.insert(id, thermograph.clone());

        thermograph
    }

    pub fn known_games_len(&self) -> usize {
        self.known_games.len()
    }

    fn thermograph_from_moves(&self, moves: &Moves) -> Thermograph {
        let mut left_scaffold = Trajectory::new_constant(Rational::NegativeInfinity);
        let mut right_scaffold = Trajectory::new_constant(Rational::PositiveInfinity);

        for left_move in &moves.left {
            left_scaffold = left_scaffold.max(&self.thermograph(*left_move).right_wall);
        }
        for right_move in &moves.right {
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

    pub fn print_game(&self, id: GameId, f: &mut impl Write) -> fmt::Result {
        let game = self.get_game(id);
        if let Some(nus) = &game.nus {
            write!(f, "{}", nus)?;
        } else {
            self.print_moves(&game.moves, f)?;
        }
        Ok(())
    }

    pub fn print_game_to_str(&self, id: GameId) -> String {
        let mut buf = String::new();
        self.print_game(id, &mut buf).unwrap();
        buf
    }

    pub fn print_moves(&self, moves: &Moves, f: &mut impl Write) -> fmt::Result {
        write!(f, "{{")?;
        for (idx, l) in moves.left.iter().enumerate() {
            if idx != 0 {
                write!(f, ",")?;
            }
            self.print_game(*l, f)?;
        }
        write!(f, "|")?;
        for (idx, r) in moves.right.iter().enumerate() {
            if idx != 0 {
                write!(f, ",")?;
            }
            self.print_game(*r, f)?;
        }
        write!(f, "}}")?;
        Ok(())
    }

    pub fn print_moves_to_str(&self, moves: &Moves) -> String {
        let mut buf = String::new();
        self.print_moves(moves, &mut buf).unwrap();
        buf
    }
}

#[test]
fn constructs_integers() {
    let b = GameBackend::new();

    let eight = b.construct_integer(8);
    assert_eq!(&b.print_game_to_str(eight), "8");

    let minus_forty_two = b.construct_integer(-42);
    assert_eq!(&b.print_game_to_str(minus_forty_two), "-42");

    let duplicate = b.construct_integer(8);
    assert_eq!(eight, duplicate);
}

#[test]
fn constructs_rationals() {
    let b = GameBackend::new();

    let rational = DyadicRationalNumber::new(3, 4);
    let three_sixteenth = b.construct_rational(rational);
    assert_eq!(&b.print_game_to_str(three_sixteenth), "3/16");

    let duplicate = b.construct_rational(rational);
    assert_eq!(three_sixteenth, duplicate);
}

#[test]
fn constructs_nimbers() {
    let b = GameBackend::new();

    let star = b.construct_nimber(DyadicRationalNumber::from(1), Nimber::from(4));
    assert_eq!(&b.print_game_to_str(star), "1*4");

    let duplicate = b.construct_nimber(DyadicRationalNumber::from(1), Nimber::from(4));
    assert_eq!(star, duplicate);
}

#[test]
fn constructs_up() {
    let b = GameBackend::new();

    let up = b.construct_nus(&Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: 1,
        nimber: Nimber::from(0),
    });
    assert_eq!(&b.print_game_to_str(up), "^");

    let up_star = b.construct_nus(&Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: 1,
        nimber: Nimber::from(1),
    });
    assert_eq!(&b.print_game_to_str(up_star), "^*");

    let down = b.construct_nus(&Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: -3,
        nimber: Nimber::from(0),
    });
    assert_eq!(&b.print_game_to_str(down), "v3");
}

#[test]
fn nimber_is_its_negative() {
    let b = GameBackend::new();

    let star = b.construct_nimber(DyadicRationalNumber::from(0), Nimber::from(4));
    assert_eq!(&b.print_game_to_str(star), "*4");

    let neg_star = b.construct_negative(star);
    assert_eq!(star, neg_star);
}

#[test]
fn simplifies_moves() {
    let b = GameBackend::new();

    let moves_l = Moves {
        left: vec![b.one_id],
        right: vec![b.star_id],
    };
    let left_id = b.construct_from_moves(moves_l);
    assert_eq!(&b.print_game_to_str(left_id), "{1|*}");

    let moves = Moves {
        left: vec![left_id],
        right: vec![b.zero_id],
    };
    let id = b.construct_from_moves(moves);
    assert_eq!(&b.print_game_to_str(id), "*");

    let moves = Moves {
        left: vec![],
        right: vec![b.negative_one_id, b.negative_one_id, b.zero_id],
    };
    let id = b.construct_from_moves(moves);
    assert_eq!(&b.print_game_to_str(id), "-2");

    let moves = Moves {
        left: vec![b.zero_id, b.negative_one_id],
        right: vec![b.one_id],
    };
    let id = b.construct_from_moves(moves);
    assert_eq!(&b.print_game_to_str(id), "1/2");
}

#[test]
fn sum_works() {
    let b = GameBackend::new();
    let one_zero = b.construct_from_moves(Moves {
        left: vec![b.one_id],
        right: vec![b.zero_id],
    });
    let zero_one = b.construct_from_moves(Moves {
        left: vec![b.zero_id],
        right: vec![b.one_id],
    });
    let sum = b.construct_sum(one_zero, zero_one);
    assert_eq!(&b.print_game_to_str(sum), "{3/2|1/2}");
}

#[test]
fn temp_of_one_minus_one_is_one() {
    let b = GameBackend::new();
    let moves = Moves {
        left: vec![b.one_id],
        right: vec![b.negative_one_id],
    };
    let g = b.construct_from_moves(moves);
    assert_eq!(b.temperature(g), Rational::from(1));
}

pub trait PartizanShortGame: Sized {
    fn left_moves(&self) -> Vec<Self>;
    fn right_moves(&self) -> Vec<Self>;
}
