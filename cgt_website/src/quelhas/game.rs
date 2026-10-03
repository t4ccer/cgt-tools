//! State of a game of Quelhas on the page, and how clicks and replies of the AI change it

use cgt_ai_core::{
    openings::decide_swap,
    protocol::{Budget, Request},
    quelhas::{Action, Board, State},
    ruleset::Player,
};

pub const DEFAULT_SIMULATIONS: u32 = 400;
pub const MAX_SIMULATIONS: u32 = 5000;
pub const DEFAULT_SECONDS: f64 = 1.0;
pub const MIN_SECONDS: f64 = 0.1;
pub const MAX_SECONDS: f64 = 60.0;

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
#[derive(Clone, PartialEq, Debug)]
pub struct Setup {
    pub opponent: Option<Opponent>,
    pub starter: Starter,
    /// Address of the model file that plays for the AI
    pub ai: String,
    pub unit: Unit,
    pub simulations: u32,
    pub seconds: f64,
    /// Whether the page shows the moves available and the chances of winning
    pub analysis: bool,
}

impl Setup {
    pub const fn new(ai: String) -> Setup {
        Setup {
            opponent: None,
            starter: Starter::Human,
            ai,
            unit: Unit::Seconds,
            simulations: DEFAULT_SIMULATIONS,
            seconds: DEFAULT_SECONDS,
            analysis: true,
        }
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Move {
    pub side: Player,
    pub action: Action,
}

impl Move {
    pub const fn segment(self) -> ((usize, usize), (usize, usize)) {
        self.action.segment(self.side)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Phase {
    /// The game is being set up and has not started
    Setup,
    Playing,
    Over(Player),
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
    pub best: Option<Action>,
}

/// What the AI suggests to the person to move
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hint {
    Move(Move),
    /// Taking over the side of the first move, under the pie rule
    Swap,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Msg {
    ChooseOpponent(Opponent),
    ChooseStarter(Starter),
    ChooseUnit(Unit),
    /// The model file at this address plays for the AI from now on
    ChooseAi(String),
    /// The strength of the AI as typed, in the chosen unit
    StrengthChanged(String),
    ChooseAnalysis(bool),
    Start,
    /// Back to the setup of a new game
    NewGame,
    ClickCell((usize, usize)),
    HoverCell(Option<(usize, usize)>),
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
    AiError {
        id: u32,
        message: String,
    },
}

#[derive(Clone, PartialEq, Debug)]
pub struct Model {
    pub setup: Setup,
    pub empty: Board,
    pub moves: Vec<Move>,
    pub turn: Player,
    pub mode: Mode,
    /// How long the AI searches, fixed when the game starts
    pub budget: Budget,
    pub swapped: bool,
    pub pie_decided: bool,
    pub phase: Phase,
    /// The square a move of a person starts at, once they have clicked it
    pub anchor: Option<(usize, usize)>,
    pub hover: Option<(usize, usize)>,
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
    next_request: u32,
}

impl Model {
    /// The empty board of a game still to be set up
    pub fn new(setup: Setup) -> Model {
        Model {
            budget: setup.budget(),
            show_analysis: setup.analysis,
            setup,
            empty: Board::FULL,
            moves: Vec::new(),
            turn: Player::Left,
            mode: Mode::TwoPlayers,
            swapped: false,
            pie_decided: false,
            phase: Phase::Setup,
            anchor: None,
            hover: None,
            analyses: vec![None],
            analysis_error: None,
            pending: None,
            analyzing: None,
            next_request: 0,
        }
    }

    pub const fn ai_thinking(&self) -> bool {
        self.pending.is_some()
    }

    const fn state(&self, side: Player) -> State {
        State {
            empty: self.empty,
            turn: side,
        }
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
        self.moves.len() == 1 && !self.pie_decided && self.phase == Phase::Playing
    }

    pub fn human_to_move(&self) -> bool {
        self.phase == Phase::Playing
            && !self.ai_thinking()
            && self.controller_of(self.turn) == Controller::Human
    }

    pub fn valid_segment(&self, a: (usize, usize), b: (usize, usize)) -> bool {
        Action::from_segment(self.turn, a, b)
            .is_some_and(|action| self.state(self.turn).is_legal(action))
    }

    pub fn can_start_at(&self, (row, col): (usize, usize)) -> bool {
        let (dr, dc) = match self.turn {
            Player::Left => (1, 0),
            Player::Right => (0, 1),
        };
        let (r, c) = (row as isize, col as isize);
        self.empty.is_empty(row, col)
            && (self.empty.is_empty_at(r + dr, c + dc) || self.empty.is_empty_at(r - dr, c - dc))
    }

    pub fn count_legal_moves(&self, side: Player) -> usize {
        self.state(side).canonical().count_legal_actions()
    }

    fn apply_move(&mut self, action: Action) {
        let next = self.state(self.turn).apply(action);
        self.pie_decided = self.pie_decided || !self.moves.is_empty();
        self.moves.push(Move {
            side: self.turn,
            action,
        });
        self.empty = next.empty;
        self.turn = next.turn;
        self.anchor = None;
        self.hover = None;
        self.phase = next.winner().map_or(Phase::Playing, Phase::Over);
        self.analyzing = None;
        // The side that cannot move did not make the last move, so it has won
        self.analyses.push(next.winner().map(|winner| Analysis {
            side: winner,
            value: 1.0,
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
        current.best.map(|action| {
            Hint::Move(Move {
                side: self.turn,
                action,
            })
        })
    }

    fn record(&mut self, value: f64, best: Option<usize>) {
        let best = best
            .and_then(Action::from_index)
            .filter(|&action| self.state(self.turn).is_legal(action));
        if let Some(current) = self.analyses.last_mut() {
            *current = Some(Analysis {
                side: self.turn,
                value,
                best,
            });
        }
    }

    fn history(&self) -> Vec<usize> {
        self.moves.iter().map(|m| m.action.index()).collect()
    }

    const fn next_id(&mut self) -> u32 {
        self.next_request += 1;
        self.next_request
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
        let request = if self.moves.is_empty() {
            Request::Opening
        } else {
            Request::Move {
                history: self.history(),
                budget: self.budget,
            }
        };
        Some((id, request))
    }

    /// The request the AI has to answer after a move, if any
    fn trigger_ai_if_needed(&mut self) -> Option<(u32, Request)> {
        if self.phase != Phase::Playing || self.controller_of(self.turn) != Controller::Ai {
            self.pending = None;
            return self.analysis_request();
        }
        let request = if self.moves.is_empty() {
            Request::Opening
        } else if self.pie_offered() {
            Request::Pie {
                first: self.moves[0].action.index(),
                budget: self.budget,
            }
        } else {
            Request::Move {
                history: self.history(),
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
                self.setup.opponent = Some(opponent);
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
                self.setup.analysis = analysis;
                None
            }
            Msg::ToggleAnalysis => {
                self.show_analysis = !self.show_analysis;
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
            Msg::ClickCell(cell) => {
                if !self.human_to_move() {
                    return None;
                }
                match self.anchor {
                    Some(anchor) if anchor == cell => self.anchor = None,
                    Some(anchor) if self.valid_segment(anchor, cell) => {
                        let action = Action::from_segment(self.turn, anchor, cell)?;
                        self.apply_move(action);
                        return self.trigger_ai_if_needed();
                    }
                    _ if self.can_start_at(cell) => self.anchor = Some(cell),
                    Some(_) => self.anchor = None,
                    None => {}
                }
                None
            }
            Msg::HoverCell(cell) => {
                self.hover = cell;
                None
            }
            Msg::Swap => {
                if !(self.pie_offered() && self.controller_of(self.turn) == Controller::Human) {
                    return None;
                }
                self.swapped = true;
                self.pie_decided = true;
                self.anchor = None;
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
                let action = Action::from_index(action)
                    .filter(|&action| self.state(self.turn).is_legal(action));
                let Some(action) = action.filter(|_| self.phase == Phase::Playing) else {
                    self.phase = Phase::Failed("the AI suggested an illegal move".into());
                    self.pending = None;
                    return None;
                };
                self.record(value, Some(action.index()));
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(side: Player, start: (usize, usize), end: (usize, usize)) -> usize {
        Action::from_segment(side, start, end).unwrap().index()
    }

    fn started(opponent: Opponent, starter: Starter) -> (Model, Option<(u32, Request)>) {
        let mut model = Model::new(Setup::new("ai".into()));
        model.update(Msg::ChooseOpponent(opponent));
        model.update(Msg::ChooseStarter(starter));
        model.update(Msg::ChooseUnit(Unit::Simulations));
        let request = model.update(Msg::Start);
        assert_eq!(model.phase, Phase::Playing);
        (model, request)
    }

    #[test]
    fn games_start_once_set_up() {
        let mut model = Model::new(Setup::new("ai".into()));
        assert_eq!(model.update(Msg::Start), None);
        assert_eq!(model.phase, Phase::Setup);
        model.update(Msg::ClickCell((2, 3)));
        assert_eq!(model.anchor, None);

        // The person moves first, and the AI thinks for a second, at first analysing the
        // person's options
        model.update(Msg::ChooseOpponent(Opponent::Ai));
        let (_, request) = model.update(Msg::Start).unwrap();
        assert_eq!(request, Request::Opening);
        assert!(model.human_to_move());
        assert_eq!(model.phase, Phase::Playing);
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
    fn strengths_stay_in_range() {
        let mut setup = Model::new(Setup::new("ai".into()));
        setup.update(Msg::ChooseUnit(Unit::Simulations));
        for (raw, simulations) in [
            ("0", 1),
            ("123.4", 123),
            ("1e9", MAX_SIMULATIONS),
            ("x", MAX_SIMULATIONS),
            ("NaN", MAX_SIMULATIONS),
        ] {
            setup.update(Msg::StrengthChanged(raw.into()));
            assert_eq!(setup.setup.simulations, simulations, "{raw}");
        }
        setup.update(Msg::ChooseUnit(Unit::Seconds));
        for (raw, seconds) in [("0", "0.1"), ("0.5", "0.5"), ("1000", "60"), ("NaN", "60")] {
            setup.update(Msg::StrengthChanged(raw.into()));
            assert_eq!(setup.setup.strength(), seconds, "{raw}");
        }
        assert_eq!(setup.setup.simulations, MAX_SIMULATIONS);
    }

    #[test]
    fn clicking_the_first_square_again_cancels_the_move() {
        let (mut model, _) = started(Opponent::Human, Starter::Human);
        model.update(Msg::ClickCell((2, 3)));
        assert_eq!(model.anchor, Some((2, 3)));
        model.update(Msg::ClickCell((2, 3)));
        assert_eq!(model.anchor, None);
        assert!(model.moves.is_empty());
    }

    #[test]
    fn human_moves_then_ai_is_asked_for_pie() {
        let (mut model, request) = started(Opponent::Ai, Starter::Human);
        assert!(matches!(request, Some((_, Request::Opening))));
        assert_eq!(model.update(Msg::ClickCell((2, 3))), None);
        assert_eq!(model.anchor, Some((2, 3)));
        let (id, request) = model.update(Msg::ClickCell((5, 3))).unwrap();
        let budget = Budget::Simulations(DEFAULT_SIMULATIONS);
        assert_eq!(
            request,
            Request::Pie {
                first: action(Player::Left, (2, 3), (5, 3)),
                budget,
            }
        );
        assert_eq!(model.turn, Player::Right);
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
                history: vec![action(Player::Left, (2, 3), (5, 3))],
                budget,
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
                history: vec![first],
                budget: Budget::Simulations(DEFAULT_SIMULATIONS),
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
    fn two_people_get_analysis_after_each_move() {
        let (mut model, request) = started(Opponent::Human, Starter::Human);
        assert!(matches!(request, Some((_, Request::Opening))));
        assert_eq!(model.update(Msg::ClickCell((2, 3))), None);
        let (id, request) = model.update(Msg::ClickCell((5, 3))).unwrap();
        let first = action(Player::Left, (2, 3), (5, 3));
        assert_eq!(
            request,
            Request::Move {
                history: vec![first],
                budget: Budget::Simulations(DEFAULT_SIMULATIONS),
            }
        );
        // The analysis does not hold up the next move, and its answer only updates the estimate
        assert!(model.human_to_move());
        let reply = Action::from_segment(Player::Right, (0, 0), (0, 1)).unwrap();
        assert_eq!(
            model.update(Msg::AiMove {
                id,
                action: reply.index(),
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

        // A move made before the analysis is done makes it stale
        model.update(Msg::ClickCell((0, 0)));
        let (stale, _) = model.update(Msg::ClickCell((0, 1))).unwrap();
        model.update(Msg::ClickCell((8, 0)));
        let (fresh, _) = model.update(Msg::ClickCell((9, 0))).unwrap();
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
        model.update(Msg::ClickCell((0, 5)));
        assert_eq!(model.update(Msg::ClickCell((0, 6))), None);
        assert_eq!(model.moves.len(), 4);
        let (_, request) = model.update(Msg::ToggleAnalysis).unwrap();
        assert!(matches!(request, Request::Move { history, .. } if history.len() == 4));
    }

    #[test]
    fn analysis_can_start_hidden() {
        let mut model = Model::new(Setup::new("ai".into()));
        model.update(Msg::ChooseOpponent(Opponent::Human));
        model.update(Msg::ChooseAnalysis(false));
        model.update(Msg::Start);
        assert!(!model.show_analysis);
        model.update(Msg::ClickCell((2, 3)));
        assert_eq!(model.update(Msg::ClickCell((5, 3))), None);
    }

    #[test]
    fn last_move_loses() {
        let (mut model, _) = started(Opponent::Human, Starter::Human);
        model.empty = "..########
                       ##########
                       ##########
                       ##########
                       ##########
                       ##########
                       ##########
                       ##########
                       ##########
                       ##########"
            .parse()
            .unwrap();
        model.turn = Player::Right;
        model.update(Msg::ClickCell((0, 0)));
        model.update(Msg::ClickCell((0, 1)));
        assert_eq!(model.phase, Phase::Over(Player::Left));
        assert_eq!(model.move_chance(0), Some(-1.0));
        assert_eq!(model.hint(), None);
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

        let reply = Action::from_segment(Player::Right, (5, 2), (5, 4)).unwrap();
        model.update(Msg::AiMove {
            id: analysis,
            action: reply.index(),
            value: 0.3,
        });
        assert_eq!(model.move_chance(0), Some(-0.3));
        assert_eq!(
            model.hint(),
            Some(Hint::Move(Move {
                side: Player::Right,
                action: reply
            }))
        );
        assert_eq!(model.estimate().map(|e| e.side), Some(Player::Right));

        // Under the pie rule a position bad for the side to move is best swapped
        model.analyses[1] = Some(Analysis {
            side: Player::Right,
            value: -0.3,
            best: Some(reply),
        });
        assert_eq!(model.hint(), Some(Hint::Swap));
        model.update(Msg::ClickCell((5, 2)));
        model.update(Msg::ClickCell((5, 4)));
        assert_eq!(model.move_chance(1), Some(-0.3));
        assert_eq!(model.move_chance(2), None);
    }
}
