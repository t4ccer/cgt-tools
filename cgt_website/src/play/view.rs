//! The panel of a game page, the same for every game: the setup of a game, its state, the
//! analysis of the AI and the moves made

use super::game::{
    Controller, Game, Hint, MAX_SECONDS, MAX_SIMULATIONS, MIN_SECONDS, Mode, Model, Msg, Opponent,
    Participant, Phase, Setup, Starter, Unit, side_name,
};
use cgt_ai_core::{
    protocol::Budget,
    ruleset::{Player, column_name},
};
use leptos::prelude::*;

pub const fn side_class(side: Player) -> &'static str {
    match side {
        Player::Left => "left",
        Player::Right => "right",
    }
}

fn side_view<G: Game>(side: Player) -> impl IntoView {
    view! { <span class=format!("side side-{}", side_class(side))>{side_name::<G>(side)}</span> }
}

const fn participant_name<G: Game>(model: &Model<G>, side: Player) -> &'static str {
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

/// Letters over the columns and numbers beside the rows of a board, centred on `columns` and
/// `rows`
pub fn axis_labels(
    columns: impl IntoIterator<Item = (f64, f64)>,
    rows: impl IntoIterator<Item = (f64, f64)>,
) -> impl IntoView {
    let label = |(x, y): (f64, f64), text: String| {
        view! {
            <text class="board-label" x=x y=y>
                {text}
            </text>
        }
    };
    let columns = columns
        .into_iter()
        .enumerate()
        .map(|(i, at)| label(at, column_name(i).to_string()))
        .collect_view();
    let rows = rows
        .into_iter()
        .enumerate()
        .map(|(i, at)| label(at, (i + 1).to_string()))
        .collect_view();
    view! {
        {columns}
        {rows}
    }
}

/// A chance of winning in `[-1, 1]` as a percentage
fn percent(value: f64) -> String {
    format!("{}%", (50.0 * (value + 1.0)).round())
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
fn show_strength<G: Game>(event: &leptos::ev::Event, model: RwSignal<Model<G>>) {
    event_target::<web_sys::HtmlInputElement>(event)
        .set_value(&model.with_untracked(|m| m.setup.strength()));
}

#[cfg(not(feature = "hydrate"))]
const fn show_strength<G: Game>(_: &leptos::ev::Event, _: RwSignal<Model<G>>) {}

/// Who plays, and with which model and how long the AI searches, as the page shows it during the
/// game
fn opponent_summary<G: Game>(m: &Model<G>, models: &[(String, String)]) -> String {
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

/// A game page: the board, which `board` draws and plays moves on, next to the panel. The setup
/// of a game offers the AIs in `models`, given by their names and the addresses of their model
/// files, the first being the default, starts from the board dealt by `seed`, and shows the
/// steps of `setup_steps`, which choose what the game is played on, after the choice of
/// opponent.
pub fn game_view<G: Game>(
    models: Vec<(String, String)>,
    seed: u64,
    board: impl FnOnce(RwSignal<Model<G>>, Callback<Msg>) -> AnyView,
    setup_steps: impl Fn(RwSignal<Model<G>>, Callback<Msg>) -> AnyView + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let first = models
        .first()
        .map(|(_, url)| url.clone())
        .unwrap_or_default();
    let model = RwSignal::new(Model::new(Setup::<G>::new(first, seed)));
    #[cfg(feature = "hydrate")]
    let dispatch = super::worker::connect(model);
    #[cfg(not(feature = "hydrate"))]
    let dispatch = Callback::new(move |msg: Msg| {
        model.update(|m| {
            m.update(msg);
        });
    });

    let status = move || {
        model.with(|m| match &m.phase {
            Phase::Over { winner, margin } => {
                let loser = participant_name(m, winner.opposite());
                let reason = match margin {
                    Some(margin) if *margin > 0 => format!(" by {margin}, once settled"),
                    Some(_) => ", once settled".to_owned(),
                    None if G::LAST_MOVE_LOSES => format!(", {loser} made the last move"),
                    None => format!(", {loser} cannot move"),
                };
                view! {
                    {participant_name(m, *winner)}
                    " ("
                    {side_view::<G>(*winner)}
                    ") won"
                    {reason}
                }
                .into_any()
            }
            Phase::Failed(err) => {
                view! { <span class="play-error">{format!("Error: {err}")}</span> }.into_any()
            }
            Phase::Setup => ().into_any(),
            Phase::Playing => view! {
                {side_view::<G>(m.turn())}
                " to move: "
                {participant_name(m, m.turn())}
                {m.ai_thinking().then_some(" (thinking…)")}
            }
            .into_any(),
        })
    };
    let moves_available = move || {
        model.with(|m| {
            format!(
                "Moves available: {} {}, {} {}",
                G::SIDES[0],
                m.rules.moves_available(&m.state, Player::Left),
                G::SIDES[1],
                m.rules.moves_available(&m.state, Player::Right)
            )
        })
    };
    let estimate = move || {
        model.with(|m| {
            let updating = m.analyzing.is_some();
            let estimate = match m.estimate() {
                Some(e) => view! {
                    {side_view::<G>(e.side)}
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
    let hint = move || {
        model.with(|m| {
            let suggestion = match m.hint() {
                Some(Hint::Move(notation)) => view! {
                    <span class=format!("side-{}", side_class(m.turn()))>{notation}</span>
                }
                .into_any(),
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
    let analysis_error = move || {
        model.with(|m| {
            m.analysis_error.clone().map(|err| {
                view! { <p class="play-detail play-error">{format!("Analysis failed: {err}")}</p> }
            })
        })
    };
    let pie = move || {
        model.with(|m| {
            (m.pie_offered() && m.controller_of(m.turn()) == Controller::Human && !m.ai_thinking())
                .then(|| {
                    view! {
                        <div class="play-pie">
                            <button
                                class="btn btn-primary btn-sm"
                                on:click=move |_| dispatch.run(Msg::Swap)
                            >
                                "Swap sides"
                            </button>
                            <span>
                                {format!("or move to keep playing {}", side_name::<G>(m.turn()))}
                            </span>
                        </div>
                    }
                })
        })
    };
    let move_list = move || {
        model.with(|m| {
            m.moves
                .iter()
                .enumerate()
                .map(|(index, mv)| {
                    let chance =
                        m.show_analysis
                            .then(|| m.move_chance(index))
                            .flatten()
                            .map(|chance| {
                                let title = format!(
                                    "Chance of {} to win after this move",
                                    side_name::<G>(mv.side)
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
                            {mv.notation.clone()} {chance}
                        </li>
                    }
                })
                .collect_view()
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
    let end_settled = Memo::new(move |_| model.with(|m| Some(m.setup.end_settled)));
    let show_analysis = Memo::new(move |_| model.with(|m| m.show_analysis));
    let has_ai = model.with_untracked(|m| m.setup.has_ai());

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
                        <select name="model" on:change=move |ev| {
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
                        name="strength"
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
                        if has_ai {
                            &[(Opponent::Ai, "Player vs AI"), (Opponent::Human, "Player vs Player (local)")]
                        } else {
                            &[(Opponent::Human, "Player vs Player (local)")]
                        },
                        opponent,
                        move |o| dispatch.run(Msg::ChooseOpponent(o)),
                    )}
                </fieldset>
                {setup_steps(model, dispatch)}
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
                                {has_ai
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
                                    })}
                                {G::SETTLES
                                    .then(|| {
                                        view! {
                                            <fieldset>
                                                <legend>"End of game"</legend>
                                                {toggle(
                                                    &[(true, "Once settled"), (false, "Play it out")],
                                                    end_settled,
                                                    move |e| dispatch.run(Msg::ChooseEndSettled(e)),
                                                )}
                                            </fieldset>
                                        }
                                    })}
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
                                    "Start"
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
                    {has_ai
                        .then(|| {
                            view! {
                                <button
                                    type="button"
                                    class="btn btn-outline btn-sm"
                                    aria-pressed=move || show_analysis.get().to_string()
                                    on:click=move |_| dispatch.run(Msg::ToggleAnalysis)
                                >
                                    {move || {
                                        if show_analysis.get() { "Hide analysis" } else { "Show analysis" }
                                    }}
                                </button>
                            }
                        })}
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
            <div class="play-board">{board(model, dispatch)}</div>
            <div class="play-panel">{move || if setting_up.get() { setup() } else { game() }}</div>
        </div>
    }
}
