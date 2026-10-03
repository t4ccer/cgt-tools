//! Quelhas: crossing out lines of squares, played with the pie rule

use super::{
    game::{Game, Model, Msg},
    view::{game_view, side_class},
};
use cgt_ai_core::{
    quelhas::{Action, BOARD_SIZE, Quelhas, State, cell_name},
    ruleset::Player,
};
use leptos::prelude::*;

impl Game for Quelhas {
    const SIDES: [&'static str; 2] = ["Left", "Right"];
    const PIE_RULE: bool = true;
    const LAST_MOVE_LOSES: bool = true;

    fn deal(&self, _: u64) -> State {
        State::initial()
    }

    fn moves_available(&self, state: &State, side: Player) -> usize {
        State {
            empty: state.empty,
            turn: side,
        }
        .canonical()
        .count_legal_actions()
    }
}

/// The move from `a` to `b` of the side to move, if it can make it
fn segment(m: &Model<Quelhas>, a: (usize, usize), b: (usize, usize)) -> Option<Action> {
    Action::from_segment(m.turn(), a, b).filter(|&action| m.state.is_legal(action))
}

fn can_start_at(m: &Model<Quelhas>, (row, col): (usize, usize)) -> bool {
    let (dr, dc) = match m.turn() {
        Player::Left => (1, 0),
        Player::Right => (0, 1),
    };
    let (r, c) = (row as isize, col as isize);
    let empty = m.state.empty;
    empty.is_empty(row, col)
        && (empty.is_empty_at(r + dr, c + dc) || empty.is_empty_at(r - dr, c - dc))
}

/// What a click on `cell` does, when a move was started at `start`: the square the move then
/// starts at, and the move to make. A move is made by clicking the squares at both of its ends.
fn click(
    m: &Model<Quelhas>,
    start: Option<(usize, usize)>,
    cell: (usize, usize),
) -> (Option<(usize, usize)>, Option<usize>) {
    if !m.human_to_move() {
        return (None, None);
    }
    match start {
        Some(start) if start == cell => (None, None),
        Some(start) if segment(m, start, cell).is_some() => {
            (None, segment(m, start, cell).map(Action::index))
        }
        _ if can_start_at(m, cell) => (Some(cell), None),
        _ => (None, None),
    }
}

const CELL_SIZE: f64 = 44.0;
const MARGIN: f64 = 26.0;

fn cell_center((r, c): (usize, usize)) -> (f64, f64) {
    (
        (c as f64 + 0.5).mul_add(CELL_SIZE, MARGIN),
        (r as f64 + 0.5).mul_add(CELL_SIZE, MARGIN),
    )
}

fn stroke(side: Player, action: Action, class: &'static str) -> impl IntoView {
    let (start, end) = action.segment(side);
    let (x1, y1) = cell_center(start);
    let (x2, y2) = cell_center(end);
    view! {
        <line
            class=format!("stroke stroke-{} {class}", side_class(side))
            x1=x1
            y1=y1
            x2=x2
            y2=y2
        ></line>
    }
}

fn board(model: RwSignal<Model<Quelhas>>, dispatch: Callback<Msg>) -> AnyView {
    let start = RwSignal::new(None::<(usize, usize)>);
    let hover = RwSignal::new(None::<(usize, usize)>);
    // A move started before the position changed no longer fits it
    let position = Memo::new(move |_| model.with(|m| (m.moves.len(), m.human_to_move())));
    Effect::new(move |_| {
        position.track();
        start.set(None);
    });

    let cells = (0..BOARD_SIZE)
        .flat_map(|r| (0..BOARD_SIZE).map(move |c| (r, c)))
        .map(|cell| {
            let crossed = move || model.with(|m| !m.state.empty.is_empty(cell.0, cell.1));
            let selected = move || start.get() == Some(cell);
            let can_start = move |side| {
                model.with(|m| m.turn() == side && m.human_to_move() && can_start_at(m, cell))
            };
            let active = move || {
                model
                    .with(|m| m.human_to_move() && (can_start_at(m, cell) || start.get().is_some()))
            };
            view! {
                <rect
                    class="cell"
                    class:crossed=crossed
                    class:selected=selected
                    class:start-left=move || can_start(Player::Left)
                    class:start-right=move || can_start(Player::Right)
                    class:active=active
                    x=(cell.1 as f64).mul_add(CELL_SIZE, MARGIN)
                    y=(cell.0 as f64).mul_add(CELL_SIZE, MARGIN)
                    width=CELL_SIZE
                    height=CELL_SIZE
                    on:click=move |_| {
                        let (next, action) = model
                            .with_untracked(|m| click(m, start.get_untracked(), cell));
                        start.set(next);
                        if let Some(action) = action {
                            dispatch.run(Msg::Play(action));
                        }
                    }
                    on:mouseover=move |_| hover.set(Some(cell))
                ></rect>
            }
        })
        .collect_view();

    let labels = (0..BOARD_SIZE)
        .map(|i| {
            let along = (i as f64 + 0.5).mul_add(CELL_SIZE, MARGIN);
            let label = |x: f64, y: f64, text: String| {
                view! {
                    <text class="board-label" x=x y=y>
                        {text}
                    </text>
                }
            };
            let column = cell_name(0, i);
            view! {
                {label(along, MARGIN / 2.0, column[..1].to_owned())}
                {label(MARGIN / 2.0, along, (i + 1).to_string())}
            }
        })
        .collect_view();

    let strokes = move || {
        model.with(|m| {
            let last = m.moves.len().saturating_sub(1);
            m.moves
                .iter()
                .enumerate()
                .filter_map(|(i, mv)| {
                    let action = Action::from_index(mv.action)?;
                    Some(stroke(mv.side, action, if i == last { "last" } else { "" }))
                })
                .collect_view()
        })
    };
    let preview = move || {
        model.with(|m| match (start.get(), hover.get()) {
            (Some(a), Some(b)) if m.human_to_move() => {
                segment(m, a, b).map(|action| stroke(m.turn(), action, "preview"))
            }
            _ => None,
        })
    };

    let size = (BOARD_SIZE as f64).mul_add(CELL_SIZE, MARGIN) + 6.0;
    view! {
        <svg
            class="board"
            viewBox=format!("0 0 {size} {size}")
            role="img"
            aria-label="Quelhas board"
            on:mouseout=move |_| hover.set(None)
        >
            {labels}
            {cells}
            {strokes}
            {preview}
        </svg>
    }
    .into_any()
}

/// A game of Quelhas against one of the AIs in `models`, given by their names and the addresses
/// of their model files, the first being the default
#[island]
pub fn QuelhasGame(models: Vec<(String, String)>) -> impl IntoView {
    game_view::<Quelhas>(models, 0, board, |_, _| ().into_any())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::game::{Opponent, Setup};

    fn playing() -> Model<Quelhas> {
        let mut model = Model::<Quelhas>::new(Setup::new("ai".into(), 0));
        model.update(Msg::ChooseOpponent(Opponent::Human));
        model.update(Msg::Start);
        model
    }

    #[test]
    fn moves_go_from_one_end_to_the_other() {
        let model = playing();
        assert_eq!(click(&model, None, (2, 3)), (Some((2, 3)), None));
        let (start, action) = click(&model, Some((2, 3)), (5, 3));
        assert_eq!(start, None);
        assert_eq!(
            action,
            Action::from_segment(Player::Left, (2, 3), (5, 3)).map(Action::index)
        );
    }

    #[test]
    fn clicking_the_first_square_again_cancels_the_move() {
        let model = playing();
        assert_eq!(click(&model, Some((2, 3)), (2, 3)), (None, None));
    }

    #[test]
    fn clicking_another_start_moves_the_start() {
        let model = playing();
        // Left crosses out vertical lines, so a square to the side starts a new move
        assert_eq!(click(&model, Some((2, 3)), (2, 4)), (Some((2, 4)), None));
    }

    #[test]
    fn nothing_happens_while_setting_up() {
        let model = Model::<Quelhas>::new(Setup::new("ai".into(), 0));
        assert_eq!(click(&model, None, (2, 3)), (None, None));
    }
}
