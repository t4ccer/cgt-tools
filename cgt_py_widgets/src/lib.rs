use cgt_py_messages::Sequence;
use futures_signals::signal::{Mutable, SignalExt as _};
use jupyter_rust_widget_frontend::Context;
use serde::Serialize;

pub mod canvas;
pub mod graph;
pub mod grid;
pub mod reactive;

/// Where the state that the widget is showing came from
#[derive(Clone, Copy)]
enum Provenance {
    /// Waiting for python to send the initial state
    Uninitialized,

    /// Sent down by python, which therefore already has it
    Python,

    /// Made by an edit that is still going on, like a vertex being dragged around.
    /// Python hears about it only once it is over
    InProgress,

    /// Made by an edit in the widget, which python has yet to see
    Edit,
}

impl Provenance {
    const fn should_report_to_python(self) -> bool {
        match self {
            Provenance::Uninitialized | Provenance::Python | Provenance::InProgress => false,
            Provenance::Edit => true,
        }
    }
}

#[derive(Clone)]
struct SyncState<T> {
    state: T,
    provenance: Provenance,
    sequence: Sequence,
}

impl<T> SyncState<T> {
    fn uninitialized(state: T) -> SyncState<T> {
        SyncState {
            state,
            provenance: Provenance::Uninitialized,
            sequence: Sequence::UNINITIALIZED,
        }
    }

    fn take_from_python(&mut self, sequence: Sequence, new_state: T)
    where
        T: PartialEq,
    {
        if self.is_worth_taking(sequence, &new_state) {
            self.state = new_state;
            self.provenance = Provenance::Python;
            self.sequence = sequence
        }
    }

    const fn edit(&mut self) -> &mut T {
        self.provenance = Provenance::Edit;
        self.sequence.increment();
        &mut self.state
    }

    /// Edit that is not over yet, which python is told about once it is
    const fn edit_in_progress(&mut self) -> &mut T {
        self.provenance = Provenance::InProgress;
        self.sequence.increment();
        &mut self.state
    }

    /// Whether the edit this state came from is still going on
    const fn is_in_progress(&self) -> bool {
        matches!(self.provenance, Provenance::InProgress)
    }

    fn is_worth_taking(&self, sequence: Sequence, state: &T) -> bool
    where
        T: PartialEq,
    {
        sequence > self.sequence || (sequence == self.sequence && *state != self.state)
    }
}

/// Replace the state with one an edit in the widget arrived at, which takes the number
/// after the state it replaces
fn set_edited<T>(state: &Mutable<SyncState<T>>, edited: T) {
    *state.lock_mut().edit() = edited;
}

/// Report to python every edit made in the widget, and nothing that python sent down in
/// the first place
fn report_edits_to_python<T, M>(
    state: &Mutable<SyncState<T>>,
    context: &Context<M>,
    message: impl Fn(Sequence, T) -> M + 'static,
) where
    T: Clone + 'static,
    M: Serialize + 'static,
{
    wasm_bindgen_futures::spawn_local(state.signal_cloned().for_each({
        let context = context.clone();
        move |state| {
            if state.provenance.should_report_to_python() {
                context.send_message(&message(state.sequence, state.state));
            }

            async {}
        }
    }));
}
