//! Quelhas against an AI that runs in the browser

pub mod game;
#[cfg(feature = "hydrate")]
mod worker;

use cgt_ai_core::protocol::Budget;
use cgt_ai_core::{
    quelhas::{Action, BOARD_SIZE, cell_name},
    ruleset::Player,
};
use game::{
    Controller, Hint, MAX_SECONDS, MAX_SIMULATIONS, MIN_SECONDS, Mode, Model, Move, Msg, Opponent,
    Participant, Phase, Setup, Starter, Unit,
};
use leptos::prelude::*;

const CELL_SIZE: f64 = 44.0;
const MARGIN: f64 = 26.0;

const fn side_class(side: Player) -> &'static str {
    match side {
        Player::Left => "left",
        Player::Right => "right",
    }
}

const fn side_name(side: Player) -> &'static str {
    match side {
        Player::Left => "Left",
        Player::Right => "Right",
    }
}

const fn participant_name(model: &Model, side: Player) -> &'static str {
    match (
        model.mode,
        model.participant_of(side),
        model.controller_of(side),
    ) {
        (Mode::TwoPlayers, Participant::First, _) => "Player 1",
        (Mode::TwoPlayers, Participant::Second, _) => "Player 2",
        (_, _, Controller::Human) => "You",
        (_, _, Controller::Ai) => "AI",
    }
}

fn side_view(side: Player) -> impl IntoView {
    view! { <span class=format!("side side-{}", side_class(side))>{side_name(side)}</span> }
}

/// A row of buttons of which the one for `chosen` is pressed
fn toggle<T: Copy + PartialEq + Send + Sync + 'static>(
    options: &'static [(T, &'static str)],
    chosen: Memo<Option<T>>,
    pick: impl Fn(T) + Copy + Send + Sync + 'static,
) -> AnyView {
    let buttons = options
        .iter()
        .map(|&(value, text)| {
            let pressed = move || chosen.get() == Some(value);
            view! {
                <button
                    type="button"
                    class="btn btn-sm"
                    class:btn-primary=pressed
                    class:btn-outline=move || !pressed()
                    aria-pressed=move || pressed().to_string()
                    on:click=move |_| pick(value)
                >
                    {text}
                </button>
            }
        })
        .collect_view();
    view! {
        <div class="play-buttons" role="group">
            {buttons}
        </div>
    }
    .into_any()
}

/// Puts the strength of the AI back into the field it was typed into, once typing is done,
/// because a number out of range is clamped to a value the field would not otherwise show
#[cfg(feature = "hydrate")]
fn show_strength(event: &leptos::ev::Event, model: RwSignal<Model>) {
    event_target::<web_sys::HtmlInputElement>(event)
        .set_value(&model.with_untracked(|m| m.setup.strength()));
}

#[cfg(not(feature = "hydrate"))]
const fn show_strength(_: &leptos::ev::Event, _: RwSignal<Model>) {}

/// Who plays, and with which model and how long the AI searches, as the page shows it during the
/// game
fn opponent_summary(m: &Model, models: &[(String, String)]) -> String {
    if m.mode == Mode::TwoPlayers && !m.show_analysis {
        return "Player vs Player".to_owned();
    }
    let strength = match m.budget {
        Budget::Simulations(simulations) => format!("{simulations} MCTS steps"),
        Budget::Millis(millis) => format!("{} s", f64::from(millis) / 1000.0),
    };
    let name = models
        .iter()
        .find(|(_, url)| *url == m.setup.ai)
        .filter(|_| models.len() > 1)
        .map(|(name, _)| format!(" ({name})"))
        .unwrap_or_default();
    if m.mode == Mode::TwoPlayers {
        format!("Player vs Player, analysis{name} with {strength} per move")
    } else {
        format!("Player vs AI{name}, {strength} per move")
    }
}

/// A move as the page writes it, such as `L d3-d6`
fn notation(m: Move) -> String {
    let (start, end) = m.segment();
    format!(
        "{} {}-{}",
        if m.side == Player::Left { "L" } else { "R" },
        cell_name(start.0, start.1),
        cell_name(end.0, end.1)
    )
}

/// A chance of winning in `[-1, 1]` as a percentage
fn percent(value: f64) -> String {
    format!("{}%", (50.0 * (value + 1.0)).round())
}

#[component]
fn HelpIcon() -> impl IntoView {
    view! {
        <svg
            class="icon"
            xmlns="http://www.w3.org/2000/svg"
            width="20"
            height="20"
            viewBox="0 0 24 24"
            stroke-width="2"
            stroke="currentColor"
            fill="none"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M12 12m-9 0a9 9 0 1 0 18 0a9 9 0 1 0 -18 0" />
            <path d="M12 17l0 .01" />
            <path d="M12 13.5a1.5 1.5 0 0 1 1 -1.5a2.6 2.6 0 1 0 -3 -4" />
        </svg>
    }
}

fn cell_center((r, c): (usize, usize)) -> (f64, f64) {
    (
        (c as f64 + 0.5).mul_add(CELL_SIZE, MARGIN),
        (r as f64 + 0.5).mul_add(CELL_SIZE, MARGIN),
    )
}

fn stroke(m: Move, class: &'static str) -> impl IntoView {
    let (start, end) = m.segment();
    let (x1, y1) = cell_center(start);
    let (x2, y2) = cell_center(end);
    view! {
        <line
            class=format!("stroke stroke-{} {class}", side_class(m.side))
            x1=x1
            y1=y1
            x2=x2
            y2=y2
        ></line>
    }
}

/// A game of Quelhas against one of the AIs in `models`, given by their names and the addresses
/// of their model files, the first being the default
#[island]
pub fn QuelhasGame(models: Vec<(String, String)>) -> impl IntoView {
    let first = models
        .first()
        .map(|(_, url)| url.clone())
        .unwrap_or_default();
    let model = RwSignal::new(Model::new(Setup::new(first)));
    #[cfg(feature = "hydrate")]
    let dispatch = worker::connect(model);
    #[cfg(not(feature = "hydrate"))]
    let dispatch = Callback::new(move |msg: Msg| {
        model.update(|m| {
            m.update(msg);
        });
    });

    let status = move || {
        model.with(|m| match &m.phase {
            Phase::Over(winner) => view! {
                {participant_name(m, *winner)}
                " ("
                {side_view(*winner)}
                ") won, "
                {participant_name(m, winner.opposite())}
                " made the last move"
            }
            .into_any(),
            Phase::Failed(err) => {
                view! { <span class="play-error">{format!("Error: {err}")}</span> }.into_any()
            }
            Phase::Setup => ().into_any(),
            Phase::Playing => view! {
                {side_view(m.turn)}
                " to move: "
                {participant_name(m, m.turn)}
                {m.ai_thinking().then_some(" (thinking…)")}
            }
            .into_any(),
        })
    };
    let moves_available = move || {
        model.with(|m| {
            format!(
                "Moves available: Left {}, Right {}",
                m.count_legal_moves(Player::Left),
                m.count_legal_moves(Player::Right)
            )
        })
    };
    let estimate = move || {
        model.with(|m| {
            let updating = m.analyzing.is_some();
            let estimate = match m.estimate() {
                Some(e) => view! {
                    {side_view(e.side)}
                    " wins with "
                    {percent(e.value)}
                    {updating.then_some(" (updating…)")}
                }
                .into_any(),
                None if updating => "analysing…".into_any(),
                None => return None,
            };
            Some(view! { <p class="play-detail">"AI estimate: " {estimate}</p> })
        })
    };
    let analysis_error = move || {
        model.with(|m| {
            m.analysis_error.clone().map(|err| {
                view! { <p class="play-detail play-error">{format!("Analysis failed: {err}")}</p> }
            })
        })
    };
    let pie = move || {
        model.with(|m| {
            (m.pie_offered() && m.controller_of(m.turn) == Controller::Human && !m.ai_thinking())
                .then(|| {
                    view! {
                        <div class="play-pie">
                            <button
                                class="btn btn-primary btn-sm"
                                on:click=move |_| dispatch.run(Msg::Swap)
                            >
                                "Swap sides"
                            </button>
                            <span>"or move to keep playing Right"</span>
                        </div>
                    }
                })
        })
    };

    let cells = (0..BOARD_SIZE)
        .flat_map(|r| (0..BOARD_SIZE).map(move |c| (r, c)))
        .map(|cell| {
            let crossed = move || model.with(|m| !m.empty.is_empty(cell.0, cell.1));
            let selected = move || model.with(|m| m.anchor == Some(cell));
            let start = move |side| {
                model.with(|m| m.turn == side && m.human_to_move() && m.can_start_at(cell))
            };
            let active = move || {
                model.with(|m| m.human_to_move() && (m.can_start_at(cell) || m.anchor.is_some()))
            };
            view! {
                <rect
                    class="cell"
                    class:crossed=crossed
                    class:selected=selected
                    class:start-left=move || start(Player::Left)
                    class:start-right=move || start(Player::Right)
                    class:active=active
                    x=(cell.1 as f64).mul_add(CELL_SIZE, MARGIN)
                    y=(cell.0 as f64).mul_add(CELL_SIZE, MARGIN)
                    width=CELL_SIZE
                    height=CELL_SIZE
                    on:click=move |_| dispatch.run(Msg::ClickCell(cell))
                    on:mouseover=move |_| dispatch.run(Msg::HoverCell(Some(cell)))
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
                .map(|(i, &mv)| stroke(mv, if i == last { "last" } else { "" }))
                .collect_view()
        })
    };
    let preview = move || {
        model.with(|m| match (m.anchor, m.hover) {
            (Some(anchor), Some(hover)) if m.human_to_move() && m.valid_segment(anchor, hover) => {
                Action::from_segment(m.turn, anchor, hover).map(|action| {
                    stroke(
                        Move {
                            side: m.turn,
                            action,
                        },
                        "preview",
                    )
                })
            }
            _ => None,
        })
    };

    let size = (BOARD_SIZE as f64).mul_add(CELL_SIZE, MARGIN) + 6.0;
    let board = view! {
        <svg
            class="board"
            viewBox=format!("0 0 {size} {size}")
            role="img"
            aria-label="Quelhas board"
            on:mouseout=move |_| dispatch.run(Msg::HoverCell(None))
        >
            {labels}
            {cells}
            {strokes}
            {preview}
        </svg>
    }
    .into_any();

    let move_list = move || {
        model.with(|m| {
            m.moves
                .iter()
                .enumerate()
                .map(|(index, &mv)| {
                    let chance =
                        m.show_analysis
                            .then(|| m.move_chance(index))
                            .flatten()
                            .map(|chance| {
                                let title = format!(
                                    "Chance of {} to win after this move",
                                    side_name(mv.side)
                                );
                                view! {
                                    " "
                                    <span class="move-chance" title=title>
                                        {percent(chance)}
                                    </span>
                                }
                            });
                    view! {
                        <li class=format!("side-{}", side_class(mv.side))>
                            {notation(mv)}
                            {chance}
                        </li>
                    }
                })
                .collect_view()
        })
    };
    let hint = move || {
        model.with(|m| {
            let suggestion = match m.hint() {
                Some(Hint::Move(mv)) => {
                    view! { <span class=format!("side-{}", side_class(mv.side))>{notation(mv)}</span> }
                        .into_any()
                }
                Some(Hint::Swap) => "Swap sides".into_any(),
                None if m.human_to_move() && m.analyzing.is_some() => "analysing…".into_any(),
                None => return None,
            };
            Some(view! {
                <p class="play-detail">
                    <span class="best-move" tabindex="0">
                        <HelpIcon />
                        "Best move"
                        <span class="best-move-tip" role="tooltip">
                            {suggestion}
                        </span>
                    </span>
                </p>
            })
        })
    };
    let swapped_note = move || {
        model.with(|m| m.swapped).then(|| {
            view! { <p class="play-detail">"Sides were swapped after the first move."</p> }
        })
    };

    let models = StoredValue::new(models);
    let setting_up = Memo::new(move |_| model.with(|m| m.phase == Phase::Setup));
    let opponent = Memo::new(move |_| model.with(|m| m.setup.opponent));
    let starter = Memo::new(move |_| model.with(|m| Some(m.setup.starter)));
    let unit = Memo::new(move |_| model.with(|m| Some(m.setup.unit)));
    let ai = Memo::new(move |_| model.with(|m| m.setup.ai.clone()));
    let ready = Memo::new(move |_| model.with(|m| m.setup.mode().is_some()));
    let analysis = Memo::new(move |_| model.with(|m| Some(m.setup.analysis)));
    let show_analysis = Memo::new(move |_| model.with(|m| m.show_analysis));

    let ai_choice = move || {
        models.with_value(|models| {
            (models.len() > 1).then(|| {
                let options = models
                    .iter()
                    .map(|(name, url)| {
                        let selected = {
                            let url = url.clone();
                            move || ai.with(|ai| *ai == url)
                        };
                        view! {
                            <option value=url.clone() selected=selected>
                                {name.clone()}
                            </option>
                        }
                    })
                    .collect_view();
                view! {
                    <label class="play-field">
                        <span>"Model"</span>
                        <select on:change=move |ev| {
                            dispatch.run(Msg::ChooseAi(event_target_value(&ev)));
                        }>{options}</select>
                    </label>
                }
            })
        })
    };
    let strength = move || {
        unit.get().map(|unit| {
            let (label, min, step, max) = match unit {
                Unit::Simulations => ("MCTS steps per move", 1.0, 1.0, f64::from(MAX_SIMULATIONS)),
                Unit::Seconds => ("Seconds per move", MIN_SECONDS, 0.1, MAX_SECONDS),
            };
            view! {
                <label class="play-field">
                    <span>{label}</span>
                    <input
                        type="number"
                        min=min
                        step=step
                        max=max
                        value=model.with_untracked(|m| m.setup.strength())
                        on:input=move |ev| {
                            dispatch.run(Msg::StrengthChanged(event_target_value(&ev)));
                        }
                        on:change=move |ev| show_strength(&ev, model)
                    />
                </label>
            }
        })
    };
    let unit_choice = move || {
        toggle(
            &[
                (Unit::Simulations, "MCTS steps"),
                (Unit::Seconds, "Seconds"),
            ],
            unit,
            move |u| dispatch.run(Msg::ChooseUnit(u)),
        )
    };
    let setup = move || {
        view! {
            <div class="play-setup">
                <h2>"New game"</h2>
                <fieldset>
                    <legend>"Opponent"</legend>
                    {toggle(
                        &[(Opponent::Ai, "Player vs AI"), (Opponent::Human, "Player vs Player (local)")],
                        opponent,
                        move |o| dispatch.run(Msg::ChooseOpponent(o)),
                    )}
                </fieldset>
                {move || {
                    (opponent.get() == Some(Opponent::Ai))
                        .then(|| {
                            view! {
                                <fieldset>
                                    <legend>"First move"</legend>
                                    {toggle(
                                        &[(Starter::Human, "You"), (Starter::Ai, "AI")],
                                        starter,
                                        move |s| dispatch.run(Msg::ChooseStarter(s)),
                                    )}
                                </fieldset>
                            }
                            .into_any()
                        })
                }}
                {move || {
                    (opponent.get() == Some(Opponent::Ai))
                        .then(|| {
                            view! {
                                <fieldset>
                                    <legend>"Difficulty"</legend>
                                    {ai_choice}
                                    {unit_choice}
                                    {strength}
                                </fieldset>
                            }
                            .into_any()
                        })
                }}
                {move || {
                    opponent
                        .get()
                        .is_some()
                        .then(|| {
                            view! {
                                <fieldset>
                                    <legend>"Analysis"</legend>
                                    {toggle(
                                        &[(true, "Show"), (false, "Hide")],
                                        analysis,
                                        move |a| dispatch.run(Msg::ChooseAnalysis(a)),
                                    )}
                                    // Without an AI to play against, a model of its own analyses
                                    // the game
                                    {move || {
                                        (opponent.get() == Some(Opponent::Human)
                                            && analysis.get() == Some(true))
                                            .then(|| {
                                                view! {
                                                    {ai_choice}
                                                    {unit_choice}
                                                    {strength}
                                                }
                                                    .into_any()
                                            })
                                    }}
                                </fieldset>
                            }
                            .into_any()
                        })
                }}
                {move || {
                    ready
                        .get()
                        .then(|| {
                            view! {
                                <button
                                    type="button"
                                    class="btn btn-primary play-start"
                                    on:click=move |_| dispatch.run(Msg::Start)
                                >
                                    "Start Game"
                                </button>
                            }
                            .into_any()
                        })
                }}
            </div>
        }
        .into_any()
    };
    let game = move || {
        view! {
            <div class="play-game">
                <p class="play-status">{status}</p>
                {move || {
                    show_analysis
                        .get()
                        .then(|| {
                            view! {
                                <p class="play-detail">{moves_available}</p>
                                {estimate}
                                {hint}
                                {analysis_error}
                            }
                                .into_any()
                        })
                }}
                {pie}
                <p class="play-detail">
                    {move || models.with_value(|models| model.with(|m| opponent_summary(m, models)))}
                </p>
                <div class="play-buttons">
                    <button
                        type="button"
                        class="btn btn-outline btn-sm"
                        on:click=move |_| dispatch.run(Msg::NewGame)
                    >
                        "New game"
                    </button>
                    <button
                        type="button"
                        class="btn btn-outline btn-sm"
                        aria-pressed=move || show_analysis.get().to_string()
                        on:click=move |_| dispatch.run(Msg::ToggleAnalysis)
                    >
                        {move || if show_analysis.get() { "Hide analysis" } else { "Show analysis" }}
                    </button>
                </div>
                <h2>"Moves"</h2>
                <ol class="move-list">{move_list}</ol>
                {swapped_note}
            </div>
        }
        .into_any()
    };

    view! {
        <div class="play-layout">
            <div class="play-board">{board}</div>
            <div class="play-panel">{move || if setting_up.get() { setup() } else { game() }}</div>
        </div>
    }
}
