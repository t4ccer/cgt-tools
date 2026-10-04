//! State of a game on the page, for any game, and how choices, moves and replies of the AI change
//! it

use cgt_ai_core::{
    openings::decide_swap,
    protocol::{Budget, Request},
    ruleset::{Player, Ruleset},
};
use std::collections::VecDeque;

pub const DEFAULT_SIMULATIONS: u32 = 400;
pub const MAX_SIMULATIONS: u32 = 5000;
pub const DEFAULT_SECONDS: f64 = 1.0;
pub const MIN_SECONDS: f64 = 0.1;
pub const MAX_SECONDS: f64 = 60.0;

/// How far from even, in `[-1, 1]`, an even board may be: 5% of the chances of winning
pub const EVEN: f64 = 0.1;
/// Boards the network looks at together while searching for an even one
const BOARDS_AT_ONCE: u64 = 8;
/// Boards the search for an even one looks at before it gives up
pub const MAX_BOARDS: u64 = 400;

/// What the page needs to know about a game beyond its rules
pub trait Game: Ruleset<State: PartialEq> + Copy + Default {
    /// Names of the sides, Left first
    const SIDES: [&'static str; 2];
    /// Whether the second player may swap sides after the first move
    const PIE_RULE: bool;
    /// Whether the player who makes the last move loses
    const LAST_MOVE_LOSES: bool;
    /// Whether [`Game::settled`] can end games early
    const SETTLES: bool = false;
    /// The position a game starts from: the one every game starts from, or for a game dealt at
    /// random, the one `seed` deals
    fn deal(&self, _: u64) -> Self::State {
        self.fixed_start()
            .expect("a game not dealt at random starts from a fixed position")
    }

    /// How many moves `side` has in `state`, whether or not it is their turn
    fn moves_available(&self, state: &Self::State, side: Player) -> usize;

    /// The winner and their margin, once the result is certain before the game ends
    fn settled(&self, _: &Self::State) -> Option<(Player, usize)> {
        None
    }
}

pub const fn side_name<G: Game>(side: Player) -> &'static str {
    match side {
        Player::Left => G::SIDES[0],
        Player::Right => G::SIDES[1],
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Opponent {
    Ai,
    /// Another person at the same device
    Human,
}

/// Who makes the first move of a game against the AI
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Starter {
    Human,
    Ai,
}

/// What the strength of the AI is given in
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unit {
    Simulations,
    Seconds,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    TwoPlayers,
    HumanFirst,
    AiFirst,
}

/// The choices made before a game starts
#[derive(Clone, Debug)]
pub struct Setup<G: Game> {
    pub opponent: Option<Opponent>,
    pub starter: Starter,
    /// Address of the model file that plays for the AI
    pub ai: String,
    pub unit: Unit,
    pub simulations: u32,
    pub seconds: f64,
    /// Whether the page shows the moves available and the chances of winning
    pub analysis: bool,
    /// Whether a game ends as soon as [`Game::settled`] knows its result
    pub end_settled: bool,
    pub seed: u64,
    pub start: G::State,
}

impl<G: Game> Setup<G> {
    pub fn new(ai: String, seed: u64) -> Setup<G> {
        Setup {
            opponent: None,
            starter: Starter::Human,
            ai,
            unit: Unit::Seconds,
            simulations: DEFAULT_SIMULATIONS,
            seconds: DEFAULT_SECONDS,
            analysis: false,
            end_settled: true,
            seed,
            start: G::default().deal(seed),
        }
    }

    /// Whether the site has a model of the game for the AI to play and analyse with
    pub const fn has_ai(&self) -> bool {
        !self.ai.is_empty()
    }

    /// The game these choices make, once the opponent is chosen
    pub const fn mode(&self) -> Option<Mode> {
        match (self.opponent, self.starter) {
            (None, _) => None,
            (Some(Opponent::Human), _) => Some(Mode::TwoPlayers),
            (Some(Opponent::Ai), Starter::Human) => Some(Mode::HumanFirst),
            (Some(Opponent::Ai), Starter::Ai) => Some(Mode::AiFirst),
        }
    }

    pub fn budget(&self) -> Budget {
        match self.unit {
            Unit::Simulations => Budget::Simulations(self.simulations),
            Unit::Seconds => Budget::Millis((self.seconds * 1000.0).round() as u32),
        }
    }

    /// The strength of the AI in the chosen unit, as the page shows it
    pub fn strength(&self) -> String {
        match self.unit {
            Unit::Simulations => self.simulations.to_string(),
            Unit::Seconds => self.seconds.to_string(),
        }
    }
}

/// Who made the first move, which the pie rule keeps apart from the side they end up playing
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Participant {
    First,
    Second,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Controller {
    Human,
    Ai,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Move {
    pub side: Player,
    pub action: usize,
    /// The move as the page writes it, such as `L d3-d6`
    pub notation: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Phase {
    /// The game is being set up and has not started
    Setup,
    Playing,
    Over {
        winner: Player,
        /// For a game ended once settled, how many more moves the winner had left
        margin: Option<usize>,
    },
    Failed(String),
}

/// What the AI made of a position
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Analysis {
    /// The side to move
    pub side: Player,
    /// Chance of the side to move to win, in `[-1, 1]`
    pub value: f64,
    /// The move the AI would make
    pub best: Option<usize>,
}

/// What the AI suggests to the person to move
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Hint {
    Move(String),
    /// Taking over the side of the first move, under the pie rule
    Swap,
}

#[derive(Clone, Debug)]
pub enum Msg {
    ChooseOpponent(Opponent),
    ChooseStarter(Starter),
    ChooseUnit(Unit),
    /// The model file at this address plays for the AI from now on
    ChooseAi(String),
    /// The strength of the AI as typed, in the chosen unit
    StrengthChanged(String),
    ChooseAnalysis(bool),
    ChooseEndSettled(bool),
    /// Games start from the position this seed deals from now on
    ChooseSeed(u64),
    /// Looks for an even board, dealing boards from this seed on
    FindEven(u64),
    StopFinding,
    Start,
    /// Back to the setup of a new game
    NewGame,
    /// A person makes the move `action`
    Play(usize),
    Swap,
    /// Shows the analysis if it is hidden, and hides it otherwise
    ToggleAnalysis,
    AiMove {
        id: u32,
        action: usize,
        value: f64,
    },
    AiPie {
        id: u32,
        swap: bool,
        value: f64,
    },
    AiValues {
        id: u32,
        values: Vec<f64>,
    },
    AiError {
        id: u32,
        message: String,
    },
    /// The AI stopped, and with it every request it was given
    AiLost(String),
}

/// A search for a board on which neither side is ahead, which the network screens a few boards
/// at a time and a search then confirms
#[derive(Clone, Debug)]
pub struct Balancing {
    /// The first seed not dealt yet
    next: u64,
    /// Boards looked at so far
    pub tried: u64,
    /// Seeds of boards the network finds even, which a search has yet to confirm
    candidates: VecDeque<u64>,
    /// The request the AI is working on
    request: u32,
    /// The seeds of the boards the network is screening, or the seed of the board a search is
    /// confirming
    asked: Vec<u64>,
    confirming: bool,
}

#[derive(Clone, Debug)]
pub struct Model<G: Game> {
    pub rules: G,
    pub setup: Setup<G>,
    pub state: G::State,
    pub moves: Vec<Move>,
    pub mode: Mode,
    /// How long the AI searches, fixed when the game starts
    pub budget: Budget,
    pub swapped: bool,
    pub pie_decided: bool,
    pub phase: Phase,
    pub show_analysis: bool,
    /// What the AI made of each position of the game so far, the last being the current one,
    /// from its searches for its own moves and from analysing the positions people move in
    pub analyses: Vec<Option<Analysis>>,
    /// Why the analysis of the last position failed
    pub analysis_error: Option<String>,
    /// The request for a move the AI is working on
    pub pending: Option<u32>,
    /// The request for an analysis the AI is working on, which unlike a move nobody waits for
    pub analyzing: Option<u32>,
    pub balancing: Option<Balancing>,
    /// Why the last search for an even board failed
    pub balancing_error: Option<String>,
    next_request: u32,
}

impl<G: Game> Model<G> {
    /// The starting position of a game still to be set up
    pub fn new(setup: Setup<G>) -> Model<G> {
        Model {
            rules: G::default(),
            budget: setup.budget(),
            show_analysis: setup.analysis,
            state: setup.start,
            setup,
            moves: Vec::new(),
            mode: Mode::TwoPlayers,
            swapped: false,
            pie_decided: false,
            phase: Phase::Setup,
            analyses: vec![None],
            analysis_error: None,
            pending: None,
            analyzing: None,
            balancing: None,
            balancing_error: None,
            next_request: 0,
        }
    }

    pub fn turn(&self) -> Player {
        self.rules.to_move(&self.state)
    }

    pub const fn ai_thinking(&self) -> bool {
        self.pending.is_some()
    }

    pub const fn participant_of(&self, side: Player) -> Participant {
        if matches!(side, Player::Left) == self.swapped {
            Participant::Second
        } else {
            Participant::First
        }
    }

    pub const fn controller_of(&self, side: Player) -> Controller {
        match (self.mode, self.participant_of(side)) {
            (Mode::TwoPlayers, _)
            | (Mode::HumanFirst, Participant::First)
            | (Mode::AiFirst, Participant::Second) => Controller::Human,
            (Mode::HumanFirst, Participant::Second) | (Mode::AiFirst, Participant::First) => {
                Controller::Ai
            }
        }
    }

    pub fn pie_offered(&self) -> bool {
        G::PIE_RULE && self.moves.len() == 1 && !self.pie_decided && self.phase == Phase::Playing
    }

    pub fn human_to_move(&self) -> bool {
        self.phase == Phase::Playing
            && !self.ai_thinking()
            && self.controller_of(self.turn()) == Controller::Human
    }

    pub fn is_legal(&self, action: usize) -> bool {
        self.rules.legal_actions(&self.state).contains(&action)
    }

    /// A move of the side to move, as the page writes it
    pub fn notation(&self, action: usize) -> String {
        let side = side_name::<G>(self.turn());
        format!(
            "{} {}",
            &side[..1],
            self.rules.describe_action(&self.state, action)
        )
    }

    fn apply_move(&mut self, action: usize) {
        let side = self.turn();
        let notation = self.notation(action);
        self.state = self.rules.apply(&self.state, action);
        self.pie_decided = self.pie_decided || !self.moves.is_empty();
        self.moves.push(Move {
            side,
            action,
            notation,
        });
        self.analyzing = None;
        let settled = self
            .setup
            .end_settled
            .then(|| self.rules.settled(&self.state))
            .flatten();
        let (phase, analysis) = match (self.rules.winner(&self.state), settled) {
            (Some(winner), _) => (
                Phase::Over {
                    winner,
                    margin: None,
                },
                Some(winner),
            ),
            (None, Some((winner, margin))) => (
                Phase::Over {
                    winner,
                    margin: Some(margin),
                },
                Some(winner),
            ),
            (None, None) => (Phase::Playing, None),
        };
        self.phase = phase;
        // The result of a finished game is certain
        let turn = self.turn();
        self.analyses.push(analysis.map(|winner| Analysis {
            side: turn,
            value: if winner == turn { 1.0 } else { -1.0 },
            best: None,
        }));
    }

    /// What the AI made of the current position
    fn current(&self) -> Option<Analysis> {
        self.analyses.last().copied().flatten()
    }

    /// The latest chances of winning the AI has found
    pub fn estimate(&self) -> Option<Analysis> {
        self.analyses.iter().rev().find_map(|analysis| *analysis)
    }

    /// Chance in `[-1, 1]` of the side that made move `index` to win, as the AI sees the
    /// position after it, or else as it saw the position before it, if it would have made the
    /// same move
    pub fn move_chance(&self, index: usize) -> Option<f64> {
        let played = self.moves.get(index)?.action;
        let analysis = |index: usize| self.analyses.get(index).copied().flatten();
        match (analysis(index + 1), analysis(index)) {
            (Some(after), _) => Some(-after.value),
            (None, Some(before)) if before.best == Some(played) => Some(before.value),
            _ => None,
        }
    }

    /// What the AI suggests to the person to move
    pub fn hint(&self) -> Option<Hint> {
        if !self.human_to_move() {
            return None;
        }
        let current = self.current()?;
        if self.pie_offered() && decide_swap(current.value) {
            return Some(Hint::Swap);
        }
        current.best.map(|action| Hint::Move(self.notation(action)))
    }

    fn record(&mut self, value: f64, best: Option<usize>) {
        let best = best.filter(|&action| self.is_legal(action));
        let side = self.turn();
        if let Some(current) = self.analyses.last_mut() {
            *current = Some(Analysis { side, value, best });
        }
    }

    const fn next_id(&mut self) -> u32 {
        self.next_request += 1;
        self.next_request
    }

    fn bytes(&self, state: &G::State) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.rules.write_state(state, &mut bytes);
        bytes
    }

    fn position(&self) -> Vec<u8> {
        self.bytes(&self.state)
    }

    /// The next request of the search for an even board: a search to confirm the next candidate,
    /// or else the next few boards for the network to screen
    fn balancing_request(&mut self) -> Option<(u32, Request)> {
        let balancing = self.balancing.as_mut()?;
        let candidate = balancing.candidates.pop_front();
        if candidate.is_none() && balancing.tried >= MAX_BOARDS {
            self.balancing_error = Some(format!(
                "the AI finds none of {} boards even",
                balancing.tried
            ));
            self.balancing = None;
            return None;
        }
        let seeds: Vec<u64> = if let Some(seed) = candidate {
            vec![seed]
        } else {
            let seeds = (balancing.next..balancing.next + BOARDS_AT_ONCE).collect();
            balancing.next += BOARDS_AT_ONCE;
            balancing.tried += BOARDS_AT_ONCE;
            seeds
        };
        balancing.confirming = candidate.is_some();
        balancing.asked.clone_from(&seeds);
        let id = self.next_id();
        let mut positions: Vec<Vec<u8>> = seeds
            .iter()
            .map(|&seed| self.bytes(&self.rules.deal(seed)))
            .collect();
        let budget = self.setup.budget();
        let balancing = self.balancing.as_mut()?;
        balancing.request = id;
        let request = if balancing.confirming {
            Request::Move {
                position: positions.pop()?,
                budget,
            }
        } else {
            Request::Evaluate { positions }
        };
        Some((id, request))
    }

    fn choose_seed(&mut self, seed: u64) {
        self.setup.seed = seed;
        self.setup.start = self.rules.deal(seed);
        if self.phase == Phase::Setup {
            self.state = self.setup.start;
        }
    }

    const fn balancing_asked(&self, id: u32) -> bool {
        matches!(&self.balancing, Some(balancing) if balancing.request == id)
    }

    /// A request to analyse the position a person is to move in, unless that is done or hidden
    fn analysis_request(&mut self) -> Option<(u32, Request)> {
        if !self.show_analysis
            || !self.human_to_move()
            || self.current().is_some()
            || self.analyzing.is_some()
        {
            return None;
        }
        let id = self.next_id();
        self.analyzing = Some(id);
        // Under the pie rule the best first move is the most balanced one, which the opening table
        // knows
        let request = if G::PIE_RULE && self.moves.is_empty() {
            Request::Opening
        } else {
            Request::Move {
                position: self.position(),
                budget: self.budget,
            }
        };
        Some((id, request))
    }

    /// The request the AI has to answer after a move, if any
    fn trigger_ai_if_needed(&mut self) -> Option<(u32, Request)> {
        if self.phase != Phase::Playing || self.controller_of(self.turn()) != Controller::Ai {
            self.pending = None;
            return self.analysis_request();
        }
        let request = if G::PIE_RULE && self.moves.is_empty() {
            Request::Opening
        } else if self.pie_offered() {
            Request::Pie {
                first: self.moves[0].action,
                budget: self.budget,
            }
        } else {
            Request::Move {
                position: self.position(),
                budget: self.budget,
            }
        };
        let id = self.next_id();
        self.pending = Some(id);
        Some((id, request))
    }

    /// Applies `msg`, and returns the request the AI has to answer next, if any
    pub fn update(&mut self, msg: Msg) -> Option<(u32, Request)> {
        match msg {
            Msg::ChooseOpponent(opponent) => {
                if opponent == Opponent::Human || self.setup.has_ai() {
                    self.setup.opponent = Some(opponent);
                }
                None
            }
            Msg::ChooseStarter(starter) => {
                self.setup.starter = starter;
                None
            }
            Msg::ChooseUnit(unit) => {
                self.setup.unit = unit;
                None
            }
            Msg::ChooseAi(ai) => {
                self.setup.ai = ai;
                None
            }
            Msg::StrengthChanged(raw) => {
                let value = raw.trim().parse::<f64>().ok().filter(|v| v.is_finite())?;
                match self.setup.unit {
                    Unit::Simulations => {
                        self.setup.simulations =
                            value.round().clamp(1.0, f64::from(MAX_SIMULATIONS)) as u32;
                    }
                    Unit::Seconds => self.setup.seconds = value.clamp(MIN_SECONDS, MAX_SECONDS),
                }
                None
            }
            Msg::ChooseAnalysis(analysis) => {
                self.setup.analysis = analysis && self.setup.has_ai();
                None
            }
            Msg::ChooseEndSettled(end_settled) => {
                self.setup.end_settled = end_settled;
                None
            }
            Msg::ChooseSeed(seed) => {
                self.choose_seed(seed);
                None
            }
            Msg::FindEven(seed) => {
                if self.rules.fixed_start().is_some()
                    || self.phase != Phase::Setup
                    || !self.setup.has_ai()
                {
                    return None;
                }
                self.balancing = Some(Balancing {
                    next: seed,
                    tried: 0,
                    candidates: VecDeque::new(),
                    request: 0,
                    asked: Vec::new(),
                    confirming: false,
                });
                self.balancing_error = None;
                self.balancing_request()
            }
            Msg::StopFinding => {
                self.balancing = None;
                None
            }
            Msg::AiValues { id, values } => {
                if !self.balancing_asked(id) {
                    return None;
                }
                let balancing = self.balancing.as_mut()?;
                balancing.candidates.extend(
                    balancing
                        .asked
                        .iter()
                        .zip(values)
                        .filter(|&(_, value)| value.abs() <= EVEN)
                        .map(|(&seed, _)| seed),
                );
                self.balancing_request()
            }
            Msg::AiMove { id, value, .. } if self.balancing_asked(id) => {
                let balancing = self.balancing.as_ref()?;
                match balancing.asked[..] {
                    [seed] if balancing.confirming && value.abs() <= EVEN => {
                        self.choose_seed(seed);
                        self.balancing = None;
                        None
                    }
                    _ => self.balancing_request(),
                }
            }
            Msg::AiError { id, message } if self.balancing_asked(id) => {
                // A board where the side to move is stuck has no move to search for
                if self.balancing.as_ref().is_some_and(|b| b.confirming) {
                    self.balancing_request()
                } else {
                    self.balancing = None;
                    self.balancing_error = Some(message);
                    None
                }
            }
            Msg::ToggleAnalysis => {
                self.show_analysis = !self.show_analysis && self.setup.has_ai();
                self.analysis_request()
            }
            Msg::Start => {
                let mode = self.setup.mode().filter(|_| self.phase == Phase::Setup)?;
                let next_request = self.next_request;
                *self = Model {
                    mode,
                    phase: Phase::Playing,
                    next_request,
                    ..Model::new(self.setup.clone())
                };
                // A dealt board may leave the side to move without a move
                if let Some(winner) = self.rules.winner(&self.state) {
                    self.phase = Phase::Over {
                        winner,
                        margin: None,
                    };
                }
                self.trigger_ai_if_needed()
            }
            Msg::NewGame => {
                let next_request = self.next_request;
                *self = Model {
                    next_request,
                    ..Model::new(self.setup.clone())
                };
                None
            }
            Msg::Play(action) => {
                if !self.human_to_move() || !self.is_legal(action) {
                    return None;
                }
                self.apply_move(action);
                self.trigger_ai_if_needed()
            }
            Msg::Swap => {
                if !(self.pie_offered() && self.controller_of(self.turn()) == Controller::Human) {
                    return None;
                }
                self.swapped = true;
                self.pie_decided = true;
                self.trigger_ai_if_needed()
            }
            Msg::AiMove { id, action, value } if self.analyzing == Some(id) => {
                self.analyzing = None;
                self.analysis_error = None;
                self.record(value, Some(action));
                None
            }
            Msg::AiMove { id, action, value } => {
                if self.pending != Some(id) {
                    return None;
                }
                if self.phase != Phase::Playing || !self.is_legal(action) {
                    self.phase = Phase::Failed("the AI suggested an illegal move".into());
                    self.pending = None;
                    return None;
                }
                self.record(value, Some(action));
                self.apply_move(action);
                self.trigger_ai_if_needed()
            }
            Msg::AiPie { id, swap, value } => {
                if self.pending != Some(id) || !self.pie_offered() {
                    return None;
                }
                self.record(value, None);
                self.swapped = swap;
                self.pie_decided = true;
                self.pending = None;
                self.trigger_ai_if_needed()
            }
            Msg::AiError { id, message } if self.analyzing == Some(id) => {
                self.analyzing = None;
                self.analysis_error = Some(message);
                None
            }
            Msg::AiError { id, message } => {
                if self.pending == Some(id) {
                    self.phase = Phase::Failed(message);
                    self.pending = None;
                }
                None
            }
            Msg::AiLost(message) => {
                if self.pending.take().is_some() {
                    self.phase = Phase::Failed(message.clone());
                }
                if self.analyzing.take().is_some() {
                    self.analysis_error = Some(message.clone());
                }
                if self.balancing.take().is_some() {
                    self.balancing_error = Some(message);
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgt_ai_core::quelhas::{Action, Board, Quelhas, State};

    fn action(side: Player, start: (usize, usize), end: (usize, usize)) -> usize {
        Action::from_segment(side, start, end).unwrap().index()
    }

    fn position(state: &State) -> Vec<u8> {
        let mut bytes = Vec::new();
        Quelhas.write_state(state, &mut bytes);
        bytes
    }

    fn new() -> Model<Quelhas> {
        Model::new(Setup::new("ai".into(), 0))
    }

    fn started(opponent: Opponent, starter: Starter) -> (Model<Quelhas>, Option<(u32, Request)>) {
        let mut model = new();
        model.update(Msg::ChooseOpponent(opponent));
        model.update(Msg::ChooseStarter(starter));
        model.update(Msg::ChooseUnit(Unit::Simulations));
        model.update(Msg::ChooseAnalysis(true));
        let request = model.update(Msg::Start);
        assert_eq!(model.phase, Phase::Playing);
        (model, request)
    }

    const BUDGET: Budget = Budget::Simulations(DEFAULT_SIMULATIONS);

    #[test]
    fn games_start_once_set_up() {
        let mut model = new();
        assert_eq!(model.update(Msg::Start), None);
        assert_eq!(model.phase, Phase::Setup);
        model.update(Msg::Play(action(Player::Left, (2, 3), (5, 3))));
        assert!(model.moves.is_empty());

        // The person moves first and the AI thinks for a second, and without analysis it has
        // nothing to do until the person has moved
        model.update(Msg::ChooseOpponent(Opponent::Ai));
        assert_eq!(model.update(Msg::Start), None);
        assert!(model.human_to_move());
        assert!(!model.show_analysis);
        assert_eq!(model.mode, Mode::HumanFirst);
        assert_eq!(model.budget, Budget::Millis(1000));

        model.update(Msg::NewGame);
        assert_eq!(model.phase, Phase::Setup);
        model.update(Msg::ChooseStarter(Starter::Ai));
        model.update(Msg::StrengthChanged("2.5".into()));
        let (_, request) = model.update(Msg::Start).unwrap();
        assert_eq!(request, Request::Opening);
        assert_eq!(model.budget, Budget::Millis(2500));
        assert_eq!(model.mode, Mode::AiFirst);

        model.update(Msg::NewGame);
        assert_eq!(model.phase, Phase::Setup);
        assert!(!model.ai_thinking());
        assert_eq!(model.setup.starter, Starter::Ai);
    }

    #[test]
    fn games_without_ai_are_between_people() {
        let mut model = Model::<Quelhas>::new(Setup::new(String::new(), 0));
        model.update(Msg::ChooseOpponent(Opponent::Ai));
        assert_eq!(model.setup.opponent, None);
        model.update(Msg::ChooseOpponent(Opponent::Human));
        model.update(Msg::ChooseAnalysis(true));
        assert_eq!(model.update(Msg::Start), None);
        assert!(!model.show_analysis);
        assert_eq!(model.update(Msg::ToggleAnalysis), None);
        assert!(!model.show_analysis);
        assert_eq!(
            model.update(Msg::Play(action(Player::Left, (2, 3), (5, 3)))),
            None
        );
        assert_eq!(model.moves.len(), 1);
    }

    #[test]
    fn strengths_stay_in_range() {
        let mut model = new();
        model.update(Msg::ChooseUnit(Unit::Simulations));
        for (raw, simulations) in [
            ("0", 1),
            ("123.4", 123),
            ("1e9", MAX_SIMULATIONS),
            ("x", MAX_SIMULATIONS),
            ("NaN", MAX_SIMULATIONS),
        ] {
            model.update(Msg::StrengthChanged(raw.into()));
            assert_eq!(model.setup.simulations, simulations, "{raw}");
        }
        model.update(Msg::ChooseUnit(Unit::Seconds));
        for (raw, seconds) in [("0", "0.1"), ("0.5", "0.5"), ("1000", "60"), ("NaN", "60")] {
            model.update(Msg::StrengthChanged(raw.into()));
            assert_eq!(model.setup.strength(), seconds, "{raw}");
        }
    }

    #[test]
    fn people_move_only_legally_and_in_turn() {
        let (mut model, _) = started(Opponent::Ai, Starter::Human);
        // A horizontal line is not a move of Left
        model.update(Msg::Play(action(Player::Right, (0, 0), (0, 3)) + 1000));
        assert!(model.moves.is_empty());
        let first = action(Player::Left, (2, 3), (5, 3));
        let (_, request) = model.update(Msg::Play(first)).unwrap();
        assert_eq!(
            request,
            Request::Pie {
                first,
                budget: BUDGET
            }
        );
        assert_eq!(model.moves[0].notation, "L d3-d6");
        // The AI is to move
        model.update(Msg::Play(action(Player::Right, (0, 0), (0, 1))));
        assert_eq!(model.moves.len(), 1);
    }

    #[test]
    fn human_moves_then_ai_is_asked_for_pie() {
        let (mut model, request) = started(Opponent::Ai, Starter::Human);
        assert!(matches!(request, Some((_, Request::Opening))));
        let first = action(Player::Left, (2, 3), (5, 3));
        let (id, _) = model.update(Msg::Play(first)).unwrap();
        assert!(!model.human_to_move());

        let (_, request) = model
            .update(Msg::AiPie {
                id,
                swap: false,
                value: 0.1,
            })
            .unwrap();
        assert_eq!(
            request,
            Request::Move {
                position: position(&model.state),
                budget: BUDGET,
            }
        );
    }

    #[test]
    fn swapping_hands_ai_the_reply() {
        let (mut model, request) = started(Opponent::Ai, Starter::Ai);
        let (id, request) = request.unwrap();
        assert_eq!(request, Request::Opening);
        let first = action(Player::Left, (0, 0), (1, 0));
        let (analysis, _) = model
            .update(Msg::AiMove {
                id,
                action: first,
                value: 0.0,
            })
            .unwrap();
        assert_eq!(model.analyzing, Some(analysis));
        assert!(model.pie_offered() && model.human_to_move());
        let (_, request) = model.update(Msg::Swap).unwrap();
        assert_eq!(
            request,
            Request::Move {
                position: position(&model.state),
                budget: BUDGET,
            }
        );
        assert_eq!(model.controller_of(Player::Left), Controller::Human);
        assert_eq!(model.controller_of(Player::Right), Controller::Ai);
    }

    #[test]
    fn stale_ai_replies_are_ignored() {
        let (mut model, request) = started(Opponent::Ai, Starter::Ai);
        let (id, _) = request.unwrap();
        model.update(Msg::NewGame);
        model.update(Msg::ChooseOpponent(Opponent::Human));
        model.update(Msg::Start);
        assert_eq!(
            model.update(Msg::AiMove {
                id,
                action: action(Player::Left, (0, 0), (1, 0)),
                value: 0.0
            }),
            None
        );
        assert_eq!(model.phase, Phase::Playing);
        assert!(model.moves.is_empty());
    }

    #[test]
    fn illegal_ai_moves_end_the_game() {
        let (mut model, request) = started(Opponent::Ai, Starter::Ai);
        let (id, _) = request.unwrap();
        model.update(Msg::AiMove {
            id,
            action: usize::MAX,
            value: 0.0,
        });
        assert!(matches!(model.phase, Phase::Failed(_)));
        assert!(!model.ai_thinking());
    }

    #[test]
    fn a_lost_ai_fails_every_request() {
        let (mut model, request) = started(Opponent::Ai, Starter::Ai);
        assert!(request.is_some());
        assert_eq!(model.update(Msg::AiLost("gone".into())), None);
        assert_eq!(model.phase, Phase::Failed("gone".into()));
        assert!(!model.ai_thinking());

        let (mut model, _) = started(Opponent::Human, Starter::Human);
        assert!(model.analyzing.is_some());
        assert_eq!(model.update(Msg::AiLost("gone".into())), None);
        assert_eq!(model.analyzing, None);
        assert_eq!(model.analysis_error.as_deref(), Some("gone"));
        assert_eq!(model.phase, Phase::Playing);
    }

    #[test]
    fn two_people_get_analysis_after_each_move() {
        let (mut model, request) = started(Opponent::Human, Starter::Human);
        assert!(matches!(request, Some((_, Request::Opening))));
        let first = action(Player::Left, (2, 3), (5, 3));
        let (id, request) = model.update(Msg::Play(first)).unwrap();
        assert_eq!(
            request,
            Request::Move {
                position: position(&model.state),
                budget: BUDGET,
            }
        );
        // The analysis does not hold up the next move, and its answer only updates the estimate
        assert!(model.human_to_move());
        let reply = action(Player::Right, (0, 0), (0, 1));
        assert_eq!(
            model.update(Msg::AiMove {
                id,
                action: reply,
                value: 0.4
            }),
            None
        );
        assert_eq!(model.moves.len(), 1);
        let estimate = Some(Analysis {
            side: Player::Right,
            value: 0.4,
            best: Some(reply),
        });
        assert_eq!(model.estimate(), estimate);
        assert_eq!(model.hint(), Some(Hint::Move("R a1-b1".into())));

        // A move made before the analysis is done makes it stale
        let (stale, _) = model.update(Msg::Play(reply)).unwrap();
        let (fresh, _) = model
            .update(Msg::Play(action(Player::Left, (8, 0), (9, 0))))
            .unwrap();
        model.update(Msg::AiMove {
            id: stale,
            action: 0,
            value: 0.9,
        });
        assert_eq!(model.estimate(), estimate);
        model.update(Msg::AiError {
            id: fresh,
            message: "no model".into(),
        });
        assert_eq!(model.phase, Phase::Playing);
        assert_eq!(model.analysis_error.as_deref(), Some("no model"));

        // Hidden analysis is not asked for, until it is shown again
        model.update(Msg::ToggleAnalysis);
        assert_eq!(
            model.update(Msg::Play(action(Player::Right, (0, 5), (0, 6)))),
            None
        );
        assert_eq!(model.moves.len(), 4);
        let (_, request) = model.update(Msg::ToggleAnalysis).unwrap();
        assert_eq!(
            request,
            Request::Move {
                position: position(&model.state),
                budget: BUDGET,
            }
        );
    }

    #[test]
    fn analysis_can_start_hidden() {
        let mut model = new();
        model.update(Msg::ChooseOpponent(Opponent::Human));
        model.update(Msg::ChooseAnalysis(false));
        assert_eq!(model.update(Msg::Start), None);
        assert!(!model.show_analysis);
        assert_eq!(
            model.update(Msg::Play(action(Player::Left, (2, 3), (5, 3)))),
            None
        );
    }

    #[test]
    fn moves_get_chances_and_people_get_hints() {
        let (mut model, request) = started(Opponent::Ai, Starter::Ai);
        let (id, _) = request.unwrap();
        let first = action(Player::Left, (0, 0), (1, 0));
        let (analysis, _) = model
            .update(Msg::AiMove {
                id,
                action: first,
                value: -0.1,
            })
            .unwrap();
        // The AI made the move it found best, so its search tells its chances
        assert_eq!(model.move_chance(0), Some(-0.1));
        assert_eq!(model.hint(), None);

        let reply = action(Player::Right, (5, 2), (5, 4));
        model.update(Msg::AiMove {
            id: analysis,
            action: reply,
            value: 0.3,
        });
        assert_eq!(model.move_chance(0), Some(-0.3));
        assert_eq!(model.hint(), Some(Hint::Move("R c6-e6".into())));
        assert_eq!(model.estimate().map(|e| e.side), Some(Player::Right));

        // Under the pie rule a position bad for the side to move is best swapped
        model.analyses[1] = Some(Analysis {
            side: Player::Right,
            value: -0.3,
            best: Some(reply),
        });
        assert_eq!(model.hint(), Some(Hint::Swap));
        model.update(Msg::Play(reply));
        assert_eq!(model.move_chance(1), Some(-0.3));
        assert_eq!(model.move_chance(2), None);
    }

    #[test]
    fn last_move_loses() {
        let (mut model, _) = started(Opponent::Human, Starter::Human);
        model.state = State {
            empty: "..########
                    ##########
                    ##########
                    ##########
                    ##########
                    ##########
                    ##########
                    ##########
                    ##########
                    ##########"
                .parse::<Board>()
                .unwrap(),
            turn: Player::Right,
        };
        model.update(Msg::Play(action(Player::Right, (0, 0), (0, 1))));
        assert_eq!(
            model.phase,
            Phase::Over {
                winner: Player::Left,
                margin: None
            }
        );
        assert_eq!(model.move_chance(0), Some(-1.0));
        assert_eq!(model.hint(), None);
    }
}
