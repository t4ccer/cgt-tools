use crate::widgets::{
    amazons::AmazonsWindow, canonical_form::CanonicalFormWindow,
    digraph_placement::DigraphPlacementWindow, domineering::DomineeringWindow,
    fission::FissionWindow, ski_jumps::SkiJumpsWindow, snort::SnortWindow,
    toads_and_frogs::ToadsAndFrogsWindow,
};
use cgt::{
    graph::adjacency_matrix::{directed::DirectedGraph, undirected::UndirectedGraph},
    numeric::dyadic_rational_number::DyadicRationalNumber,
    short::partizan::{
        canonical_form::CanonicalForm,
        games::{
            amazons::Amazons,
            digraph_placement::{self, DigraphPlacement},
            domineering::Domineering,
            fission::Fission,
            ski_jumps::SkiJumps,
            snort::{self, Snort},
            toads_and_frogs::ToadsAndFrogs,
        },
        partizan_game::PartizanGame,
        thermograph::Thermograph,
        transposition_table::ParallelTranspositionTable,
    },
};
use imgui::{ComboBoxFlags, FontId};
use std::{collections::BTreeMap, marker::PhantomData, sync::mpsc, thread};

mod imgui_sdl2_boilerplate;
mod widgets;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WindowId(pub usize);

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Details {
    canonical_form: CanonicalForm,
    canonical_form_rendered: String,
    thermograph: Thermograph,
    temperature: DyadicRationalNumber,
    temperature_rendered: String,
}

impl Details {
    // HACK: Very bugly hack to let us reuse drawing macro even when details are not optional
    pub const fn as_ref(&self) -> Option<&Details> {
        Some(self)
    }
}

#[derive(Debug, Clone)]
pub struct DetailOptions {
    pub show_thermograph: bool,
    pub thermograph_fit: bool,
    pub thermograph_scale: f32,
}

impl DetailOptions {
    pub const fn new() -> DetailOptions {
        DetailOptions {
            show_thermograph: true,
            thermograph_fit: true,
            thermograph_scale: 50.0,
        }
    }
}

impl Details {
    pub fn from_canonical_form(canonical_form: CanonicalForm) -> Details {
        let canonical_form_rendered = format!("Canonical Form: {canonical_form}");
        let thermograph = canonical_form.thermograph();
        let temperature = thermograph.temperature();
        let temperature_rendered = format!("Temperature: {temperature}");
        Details {
            canonical_form,
            canonical_form_rendered,
            thermograph,
            temperature,
            temperature_rendered,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TitledWindow<G> {
    pub window_id: WindowId,
    pub title: String,
    pub is_open: bool,
    pub content: G,
    pub scratch_buffer: String,
}

impl<G> TitledWindow<G> {
    pub const fn without_title(content: G) -> TitledWindow<G> {
        TitledWindow {
            window_id: WindowId(usize::MAX),
            title: String::new(),
            is_open: true,
            content,
            scratch_buffer: String::new(),
        }
    }
}

pub trait IsCgtWindow {
    fn set_title(&mut self, id: WindowId);
    fn initialize(&mut self, ctx: &GuiContext);
    fn is_open(&self) -> bool;
    fn draw(&mut self, ui: &imgui::Ui, ctx: &mut GuiContext);
    fn update(&mut self, update: UpdateKind);
}

macro_rules! impl_titled_window {
    ($title:expr) => {
        fn set_title(&mut self, id: $crate::WindowId) {
            self.title = format!("{}##{}", $title, id.0);
            self.window_id = id;
        }

        fn is_open(&self) -> ::core::primitive::bool {
            self.is_open
        }
    };
}

pub(crate) use impl_titled_window;

macro_rules! impl_game_window {
    ($task_kind:ident, $update_kind:ident) => {
        fn initialize(&mut self, ctx: &$crate::GuiContext) {
            ctx.schedule_task($crate::Task::$task_kind($crate::EvalTask {
                window: self.window_id,
                game: self.content.game.clone(),
            }));
        }

        fn update(&mut self, update: $crate::UpdateKind) {
            match update {
                $crate::UpdateKind::$update_kind(game, details) => {
                    if self.content.game == game {
                        self.content.details = Some(details);
                    }
                }
                _ => { /* NOOP */ }
            }
        }
    };
}

pub(crate) use impl_game_window;

pub trait IsEnum: Sized
where
    Self: 'static,
{
    const LABELS: &'static [&'static str];
    const VARIANTS: &'static [Self];

    fn to_usize(self) -> usize;
    fn from_usize(raw: usize) -> Self;
    fn label(self) -> &'static str {
        Self::LABELS[self.to_usize()]
    }
}

#[derive(Debug)]
pub struct RawOf<T> {
    value: usize,
    _ty: PhantomData<T>,
}

impl<T> Clone for RawOf<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for RawOf<T> {}

impl<T> RawOf<T>
where
    T: IsEnum,
{
    pub fn new(value: T) -> RawOf<T> {
        RawOf {
            value: value.to_usize(),
            _ty: PhantomData,
        }
    }

    pub fn get(self) -> T {
        T::from_usize(self.value)
    }

    pub fn combo(&mut self, ui: &imgui::Ui, label: impl AsRef<str>, flags: ComboBoxFlags) -> bool {
        let mut changed = false;
        let preview = self.get().label();
        if let Some(_combo) = ui.begin_combo_with_flags(label, preview, flags) {
            for (mode_idx, mode) in T::LABELS.iter().enumerate() {
                let is_selected = self.value == mode_idx;
                if is_selected {
                    ui.set_item_default_focus();
                }
                let clicked = ui.selectable_config(mode).selected(is_selected).build();
                changed |= clicked;
                if clicked {
                    self.value = mode_idx;
                }
            }
        }
        changed
    }
}

macro_rules! imgui_enum {
    ($v:vis $name:ident { $($variant:ident, $pretty:expr,)*}) => {
        #[derive(Debug, Clone, Copy)]
        #[repr(usize)]
        $v enum $name {
            $($variant,)*
        }

        #[automatically_derived]
        impl $crate::IsEnum for $name {
            const LABELS: &'static [&'static str] = &[$($pretty,)*];
            const VARIANTS: &'static [$name] = &[$($name::$variant ,)*];

            fn to_usize(self) -> usize {
                self as usize
            }


            fn from_usize(raw: usize) -> $name {
                match raw {
                    $(x if x == $name::$variant as usize => $name::$variant,)*
                    _ => panic!("Invalid value for {}: {}", stringify!($name), raw),
                }
            }
        }
    };
}

pub(crate) use imgui_enum;

#[derive(Debug)]
pub struct EvalTask<D> {
    pub window: WindowId,
    pub game: D,
}

#[derive(Debug)]
pub enum Task {
    EvalDomineering(EvalTask<Domineering>),
    EvalFission(EvalTask<Fission>),
    EvalAmazons(EvalTask<Amazons>),
    EvalSkiJumps(EvalTask<SkiJumps>),
    EvalToadsAndFrogs(EvalTask<ToadsAndFrogs>),
    EvalSnort(EvalTask<Snort<snort::VertexKind, UndirectedGraph<snort::VertexKind>>>),
    EvalDigraphPlacement(
        EvalTask<
            DigraphPlacement<
                digraph_placement::VertexColor,
                DirectedGraph<digraph_placement::VertexColor>,
            >,
        >,
    ),
}

pub struct GuiContext {
    pub new_windows: Vec<Box<dyn IsCgtWindow>>,
    removed_windows: Vec<WindowId>,
    large_font_id: FontId,
    tasks: mpsc::Sender<Task>,
}

impl GuiContext {
    pub fn new(tasks: mpsc::Sender<Task>) -> GuiContext {
        GuiContext {
            new_windows: Vec::new(),
            removed_windows: Vec::new(),
            large_font_id: unsafe {
                core::mem::transmute::<*const imgui::Font, imgui::FontId>(core::ptr::null::<
                    imgui::Font,
                >())
            },
            tasks,
        }
    }

    pub fn schedule_task(&self, task: Task) {
        self.tasks.send(task).unwrap();
    }
}

pub enum UpdateKind {
    DomineeringDetails(Domineering, Details),
    FissionDetails(Fission, Details),
    AmazonsDetails(Amazons, Details),
    SkiJumpsDetails(SkiJumps, Details),
    ToadsAndFrogsDetails(ToadsAndFrogs, Details),
    SnortDetails(
        Snort<snort::VertexKind, UndirectedGraph<snort::VertexKind>>,
        Details,
    ),
    DigraphPlacementDetails(
        DigraphPlacement<
            digraph_placement::VertexColor,
            DirectedGraph<digraph_placement::VertexColor>,
        >,
        Details,
    ),
}

pub struct Update {
    window: WindowId,
    kind: UpdateKind,
}

pub struct SchedulerContext {
    tasks: mpsc::Receiver<Task>,
    updates: mpsc::Sender<Update>,
    domineering_tt: ParallelTranspositionTable<Domineering>,
    fission_tt: ParallelTranspositionTable<Fission>,
    amazons_tt: ParallelTranspositionTable<Amazons>,
    ski_jumps_tt: ParallelTranspositionTable<SkiJumps>,
    toads_and_frogs_tt: ParallelTranspositionTable<ToadsAndFrogs>,
    snort_tt:
        ParallelTranspositionTable<Snort<snort::VertexKind, UndirectedGraph<snort::VertexKind>>>,
    digraph_placement_tt: ParallelTranspositionTable<
        DigraphPlacement<
            digraph_placement::VertexColor,
            DirectedGraph<digraph_placement::VertexColor>,
        >,
    >,
}

#[allow(clippy::needless_pass_by_value)]
fn scheduler(ctx: SchedulerContext) {
    macro_rules! handle_game_update {
        ($task:expr, $details:ident, $tt:ident) => {
            let cf = $task.game.canonical_form(&ctx.$tt);
            let details = Details::from_canonical_form(cf);
            ctx.updates
                .send(Update {
                    window: $task.window,
                    kind: UpdateKind::$details($task.game, details),
                })
                .unwrap();
        };
    }

    while let Ok(task) = ctx.tasks.recv() {
        match task {
            Task::EvalDomineering(task) => {
                handle_game_update!(task, DomineeringDetails, domineering_tt);
            }
            Task::EvalFission(task) => {
                handle_game_update!(task, FissionDetails, fission_tt);
            }
            Task::EvalAmazons(task) => {
                handle_game_update!(task, AmazonsDetails, amazons_tt);
            }
            Task::EvalSkiJumps(task) => {
                handle_game_update!(task, SkiJumpsDetails, ski_jumps_tt);
            }
            Task::EvalToadsAndFrogs(task) => {
                handle_game_update!(task, ToadsAndFrogsDetails, toads_and_frogs_tt);
            }
            Task::EvalSnort(task) => {
                handle_game_update!(task, SnortDetails, snort_tt);
            }
            Task::EvalDigraphPlacement(task) => {
                handle_game_update!(task, DigraphPlacementDetails, digraph_placement_tt);
            }
        }
    }
}

fn main() {
    // let snort_tt = ParallelTranspositionTable::new();

    let (task_sender, task_receiver) = mpsc::channel();
    let (update_sender, update_receiver) = mpsc::channel();

    let scheduler_ctx = SchedulerContext {
        tasks: task_receiver,
        updates: update_sender,
        domineering_tt: ParallelTranspositionTable::new(),
        fission_tt: ParallelTranspositionTable::new(),
        amazons_tt: ParallelTranspositionTable::new(),
        ski_jumps_tt: ParallelTranspositionTable::new(),
        toads_and_frogs_tt: ParallelTranspositionTable::new(),
        snort_tt: ParallelTranspositionTable::new(),
        digraph_placement_tt: ParallelTranspositionTable::new(),
    };

    thread::spawn(move || scheduler(scheduler_ctx));

    let mut next_id = WindowId(0);
    let mut gui_context = GuiContext::new(task_sender);
    let mut windows: BTreeMap<WindowId, Box<dyn IsCgtWindow>> = BTreeMap::new();

    // must be macros because borrow checker
    macro_rules! new_window {
        ($d:ident) => {{
            let mut d = TitledWindow::without_title($d::new());
            d.set_title(next_id);
            d.initialize(&gui_context);
            windows.insert(next_id, Box::new(d));
            next_id.0 += 1;
        }};
    }

    let mut show_demo = false;

    imgui_sdl2_boilerplate::run("cgt-gui", |large_font, ui| {
        gui_context.large_font_id = large_font;

        ui.dockspace_over_main_viewport();

        if show_demo {
            ui.show_demo_window(&mut show_demo);
        }

        if let Some(_main_menu) = ui.begin_main_menu_bar() {
            if let Some(_new_menu) = ui.begin_menu("New") {
                if ui.menu_item("Canonical Form") {
                    new_window!(CanonicalFormWindow);
                }
                ui.separator();
                if ui.menu_item("Domineering") {
                    new_window!(DomineeringWindow);
                }
                if ui.menu_item("Fission") {
                    new_window!(FissionWindow);
                }
                if ui.menu_item("Amazons") {
                    new_window!(AmazonsWindow);
                }
                if ui.menu_item("Ski Jumps") {
                    new_window!(SkiJumpsWindow);
                }
                if ui.menu_item("Toads and Frogs") {
                    new_window!(ToadsAndFrogsWindow);
                }
                ui.separator();
                if ui.menu_item("Snort") {
                    new_window!(SnortWindow);
                }
                if ui.menu_item("Digraph Placement") {
                    new_window!(DigraphPlacementWindow);
                }
            }
            if ui.menu_item("Debug") {
                show_demo = true;
            }
        }

        for (&wid, d) in &mut windows {
            d.draw(ui, &mut gui_context);
            if !d.is_open() {
                gui_context.removed_windows.push(wid);
            }
        }

        for to_remove in gui_context.removed_windows.drain(..) {
            windows.remove(&to_remove);
        }

        for mut d in gui_context.new_windows.drain(..) {
            // NOTE: We don't call initialize() here because windows created by other windows should
            // be already initialized. Creating windows from top menu bar creates them directly
            // also, borrow checker complains a lot
            d.set_title(next_id);
            windows.insert(next_id, d);
            next_id.0 += 1;
        }

        while let Ok(update) = update_receiver.try_recv() {
            if let Some(window) = windows.get_mut(&update.window) {
                window.update(update.kind);
            }
        }
    });
}
