//! Interface between the rules of a game and the search and training code.

pub use cgt::short::partizan::Player;
use rand::{Rng, RngExt};
use std::fmt::Debug;

/// What the network of a game takes as input, which also decides the kind of network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// `planes` feature planes over a `height` by `width` board, for a convolutional network with
    /// `policy_planes` planes of policy logits over the same board.
    Grid {
        planes: usize,
        height: usize,
        width: usize,
        policy_planes: usize,
    },
    /// `features` features of each of the `nodes` vertices of a graph, followed by the
    /// normalized adjacency matrix of the graph, for a graph convolutional network with one
    /// policy logit per vertex.
    Graph { nodes: usize, features: usize },
}

impl Input {
    /// Length of the encoding of one position.
    pub const fn encoding_len(self) -> usize {
        match self {
            Input::Grid {
                planes,
                height,
                width,
                ..
            } => planes * height * width,
            Input::Graph { nodes, features } => nodes * (features + nodes),
        }
    }

    pub const fn num_actions(self) -> usize {
        match self {
            Input::Grid {
                height,
                width,
                policy_planes,
                ..
            } => policy_planes * height * width,
            Input::Graph { nodes, .. } => nodes,
        }
    }
}

/// Rules of a two-player game without draws, as seen by the search and the network.
///
/// Actions are indices into the policy output of the network, see [`Input`].
pub trait Ruleset: Clone + Send + Sync + 'static {
    /// Position, including whose turn it is.
    type State: Copy + Debug + Send + Sync + 'static;

    /// Name used on the command line and stored in checkpoints.
    fn name(&self) -> &'static str;

    /// A position to start a game from, drawn with `rng` for games that are dealt at random.
    fn start(&self, rng: &mut impl Rng) -> Self::State;

    /// The position every game starts from, for games that have one, which the pie rule and
    /// opening tables need.
    fn fixed_start(&self) -> Option<Self::State>;

    fn to_move(&self, state: &Self::State) -> Player;

    /// Legal actions in increasing order.
    fn legal_actions(&self, state: &Self::State) -> Vec<usize>;

    fn apply(&self, state: &Self::State, action: usize) -> Self::State;

    /// Winner of a finished game, `None` while the game goes on.
    fn winner(&self, state: &Self::State) -> Option<Player>;

    fn input(&self) -> Input;

    fn num_actions(&self) -> usize {
        self.input().num_actions()
    }

    /// Writes the network input for `state` as seen by the player to move, laid out as
    /// [`Ruleset::input`] says.
    fn encode(&self, state: &Self::State, out: &mut [f32]);

    /// Number of symmetries of the rules used for data augmentation, the identity (`0`)
    /// included.
    fn num_symmetries(&self) -> usize;

    fn transform_state(&self, state: &Self::State, symmetry: usize) -> Self::State;

    /// Maps an action of `state` to the same action of the transformed state.
    fn transform_action(&self, action: usize, symmetry: usize) -> usize;

    fn describe_action(&self, state: &Self::State, action: usize) -> String;

    /// Rough number of moves in a game, used to size progress bars before any game is played.
    fn typical_game_length(&self) -> usize;

    /// Length of the fixed-size binary encoding used to store positions on disk.
    fn state_bytes(&self) -> usize;

    fn write_state(&self, state: &Self::State, out: &mut Vec<u8>);

    /// Inverse of [`Ruleset::write_state`], `None` if `bytes` is not a valid position.
    fn read_state(&self, bytes: &[u8]) -> Option<Self::State>;
}

/// Plays up to `plies` random legal moves from a starting position, stopping early if the game
/// ends.
pub fn random_position<R: Ruleset>(rules: &R, plies: usize, rng: &mut impl Rng) -> R::State {
    let mut state = rules.start(rng);
    for _ in 0..plies {
        let actions = rules.legal_actions(&state);
        if actions.is_empty() {
            break;
        }
        state = rules.apply(&state, actions[rng.random_range(0..actions.len())]);
    }
    state
}

/// The letter of the `column`th column of a board, `a` first.
pub fn column_name(column: usize) -> char {
    char::from(b'a' + column as u8)
}

/// A square or vertex of a board, as the letter of its column and the number of its row, such as
/// `c5`.
pub fn coordinate_name(column: usize, row: usize) -> String {
    format!("{}{}", column_name(column), row + 1)
}

/// Appends whose turn it is in a position, for [`Ruleset::write_state`].
pub fn write_turn(turn: Player, out: &mut Vec<u8>) {
    out.push(u8::from(turn == Player::Right));
}

/// Whose turn the byte [`write_turn`] wrote says it is.
pub const fn read_turn(byte: u8) -> Option<Player> {
    match byte {
        0 => Some(Player::Left),
        1 => Some(Player::Right),
        _ => None,
    }
}

/// Checks on positions from all stages of games what the search, the training and the web
/// worker rely on of `rules`.
#[cfg(test)]
pub fn check<R: Ruleset>(rules: &R)
where
    R::State: PartialEq,
{
    use rand::{SeedableRng, rngs::SmallRng};

    let mut rng = SmallRng::seed_from_u64(0);
    for plies in 0..2 * rules.typical_game_length() {
        let state = random_position(rules, plies, &mut rng);
        let actions = rules.legal_actions(&state);
        assert!(actions.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(actions.iter().all(|&a| a < rules.num_actions()));
        // A game ends exactly when the player to move is stuck
        assert_eq!(rules.winner(&state).is_some(), actions.is_empty());
        assert!(
            actions
                .iter()
                .all(|&a| !rules.describe_action(&state, a).is_empty())
        );

        let mut encoded = vec![0.0; rules.input().encoding_len()];
        rules.encode(&state, &mut encoded);

        let mut bytes = Vec::new();
        rules.write_state(&state, &mut bytes);
        assert_eq!(bytes.len(), rules.state_bytes());
        assert!(rules.read_state(&bytes) == Some(state));

        assert!(rules.transform_state(&state, 0) == state);
        for symmetry in 0..rules.num_symmetries() {
            let transformed = rules.transform_state(&state, symmetry);
            let mut mapped: Vec<usize> = actions
                .iter()
                .map(|&a| rules.transform_action(a, symmetry))
                .collect();
            mapped.sort_unstable();
            assert_eq!(mapped, rules.legal_actions(&transformed));
        }
    }
}
