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

    /// Made by an edit in the widget, which python has yet to see
    Edit,
}

impl Provenance {
    const fn should_report_to_python(self) -> bool {
        match self {
            Provenance::Uninitialized | Provenance::Python => false,
            Provenance::Edit => true,
        }
    }
}

#[derive(Clone)]
struct SyncState<T> {
    state: T,
    provenance: Provenance,
}

impl<T> SyncState<T> {
    fn uninitialized(state: T) -> SyncState<T> {
        SyncState {
            state,
            provenance: Provenance::Uninitialized,
        }
    }

    const fn from_python(state: T) -> SyncState<T> {
        SyncState {
            state,
            provenance: Provenance::Python,
        }
    }

    const fn edited(state: T) -> SyncState<T> {
        SyncState {
            state,
            provenance: Provenance::Edit,
        }
    }

    const fn edit(&mut self) -> &mut T {
        self.provenance = Provenance::Edit;
        &mut self.state
    }
}
