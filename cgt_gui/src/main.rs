use crate::widgets::{
    amazons::AmazonsWindow, canonical_form::CanonicalFormWindow,
    digraph_placement::DigraphPlacementWindow, domineering::DomineeringWindow,
    fission::FissionWindow, graph_editor::GraphWindow, konane::KonaneWindow,
    quelhas::QuelhasWindow, resolving_set::ResolvingSetWindow, ski_jumps::SkiJumpsWindow,
    snort::SnortWindow, toads_and_frogs::ToadsAndFrogsWindow,
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
            konane::Konane,
            ski_jumps::SkiJumps,
            snort::{self, Snort},
            toads_and_frogs::ToadsAndFrogs,
        },
        partizan_game::PartizanGame,
        thermograph::Thermograph,
        transposition_table::ParallelTranspositionTable,
    },
};
use imgui::{Condition, FontId, TableColumnSetup};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{self, AtomicU64},
        mpsc,
    },
    thread,
};

mod access_tracker;
mod imgui_sdl2_boilerplate;
mod widgets;

pub(crate) use access_tracker::AccessTracker;

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
    ($title: literal, $task_kind:ident, $update_kind:ident) => {
        fn initialize(&mut self, ctx: &$crate::GuiContext) {
            ctx.schedule_task(
                $title,
                $crate::Task::$task_kind($crate::EvalTask {
                    window: self.window_id,
                    game: self.content.game.deref().clone(),
                }),
            );
        }

        fn update(&mut self, update: $crate::UpdateKind) {
            match update {
                $crate::UpdateKind::$update_kind(game, details) => {
                    if *self.content.game == game {
                        self.content.details = Some(details);
                    }
                }
                _ => { /* NOOP */ }
            }
        }
    };
}

pub(crate) use impl_game_window;

macro_rules! imgui_enum {
    ($(#[$attr:meta])* $v:vis $name:ident { $($variant:ident, $pretty:expr,)*}) => {
        $(#[$attr])*
        #[repr(usize)]
        $v enum $name {
            $($variant,)*
        }

        impl $name {
            #[inline(always)]
            fn combo(&mut self, ui: &::imgui::Ui, label: &str) -> bool {
                self.combo_with_flags(ui, label, ::imgui::ComboBoxFlags::HEIGHT_LARGE)
            }

            fn combo_with_flags(&mut self, ui: &::imgui::Ui, label: &str, flags: ::imgui::ComboBoxFlags) -> bool {
                const LABELS: &'static [&'static str] = &[$($pretty,)*];

                let mut changed = false;
                let preview = LABELS[*self as usize];
                if let Some(_combo) = ui.begin_combo_with_flags(label, preview, flags) {
                    for (mode_idx, mode) in LABELS.iter().enumerate() {
                        let is_selected = *self as usize == mode_idx;
                        if is_selected {
                            ui.set_item_default_focus();
                        }
                        let clicked = ui.selectable_config(mode).selected(is_selected).build();
                        changed |= clicked;
                        if clicked {
                            match mode_idx {
                                $(raw if raw == ($name::$variant as usize) => *self = $name::$variant,)*
                                raw => unreachable!("Invalid value for {}: {}", stringify!($name), raw),
                            }
                        }
                    }
                }
                changed

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskId(u64);

pub struct CurrentTask {
    id: TaskId,
    name: &'static str,
    canceller: Option<Box<dyn FnMut() + Sync + Send>>,
}

#[derive(Debug)]
pub struct ScheduledTask {
    task: Task,
    name: &'static str,
    id: TaskId,
}

#[derive(Debug)]
pub enum Task {
    EvalDomineering(EvalTask<Domineering>),
    EvalFission(EvalTask<Fission>),
    EvalAmazons(EvalTask<Amazons>),
    EvalKonane(EvalTask<Konane>),
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
    tasks: Arc<Mutex<VecDeque<ScheduledTask>>>,
    tasks_condvar: Arc<Condvar>,
    next_id: AtomicU64,
}

impl GuiContext {
    pub fn new(
        tasks: Arc<Mutex<VecDeque<ScheduledTask>>>,
        tasks_condvar: Arc<Condvar>,
    ) -> GuiContext {
        GuiContext {
            new_windows: Vec::new(),
            removed_windows: Vec::new(),
            large_font_id: unsafe {
                core::mem::transmute::<*const imgui::Font, imgui::FontId>(core::ptr::null::<
                    imgui::Font,
                >())
            },
            tasks,
            tasks_condvar,
            next_id: AtomicU64::new(0),
        }
    }

    pub fn schedule_task(&self, name: &'static str, task: Task) -> TaskId {
        // TODO: Cancel pending evals that would get overwritten anyway
        let id = TaskId(self.next_id.fetch_add(1, atomic::Ordering::SeqCst));
        self.tasks
            .lock()
            .unwrap()
            .push_back(ScheduledTask { task, name, id });
        self.tasks_condvar.notify_one();
        id
    }
}

pub enum UpdateKind {
    DomineeringDetails(Domineering, Details),
    FissionDetails(Fission, Details),
    AmazonsDetails(Amazons, Details),
    KonaneDetails(Konane, Details),
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
    current_task: Arc<Mutex<Option<CurrentTask>>>,
    tasks: Arc<Mutex<VecDeque<ScheduledTask>>>,
    tasks_condvar: Arc<Condvar>,
    updates: mpsc::Sender<Update>,
    domineering_tt: ParallelTranspositionTable<Domineering>,
    fission_tt: ParallelTranspositionTable<Fission>,
    amazons_tt: ParallelTranspositionTable<Amazons>,
    konane_tt: ParallelTranspositionTable<Konane>,
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

    let mut should_wait = true;
    loop {
        let task = {
            let mut tasks = ctx.tasks.lock().unwrap();

            if should_wait {
                let mut tasks = ctx.tasks_condvar.wait(tasks).unwrap();
                tasks.pop_front()
            } else {
                tasks.pop_front()
            }
        };

        should_wait = task.is_none();

        if let Some(task) = task {
            *ctx.current_task.lock().unwrap() = Some(CurrentTask {
                id: task.id,
                name: task.name,
                canceller: None,
            });

            match task.task {
                Task::EvalDomineering(task) => {
                    handle_game_update!(task, DomineeringDetails, domineering_tt);
                }
                Task::EvalFission(task) => {
                    handle_game_update!(task, FissionDetails, fission_tt);
                }
                Task::EvalAmazons(task) => {
                    handle_game_update!(task, AmazonsDetails, amazons_tt);
                }
                Task::EvalKonane(task) => {
                    handle_game_update!(task, KonaneDetails, konane_tt);
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

            *ctx.current_task.lock().unwrap() = None;
        }
    }
}

fn main() {
    let tasks = Arc::new(Mutex::new(VecDeque::new()));
    let tasks_condvar = Arc::new(Condvar::new());
    let (update_sender, update_receiver) = mpsc::channel();

    let current_task = Arc::new(Mutex::new(None));
    let scheduler_ctx = SchedulerContext {
        current_task: current_task.clone(),
        tasks: tasks.clone(),
        tasks_condvar: tasks_condvar.clone(),
        updates: update_sender,
        domineering_tt: ParallelTranspositionTable::new(),
        fission_tt: ParallelTranspositionTable::new(),
        amazons_tt: ParallelTranspositionTable::new(),
        konane_tt: ParallelTranspositionTable::new(),
        ski_jumps_tt: ParallelTranspositionTable::new(),
        toads_and_frogs_tt: ParallelTranspositionTable::new(),
        snort_tt: ParallelTranspositionTable::new(),
        digraph_placement_tt: ParallelTranspositionTable::new(),
    };

    thread::spawn(move || scheduler(scheduler_ctx));

    let mut next_id = WindowId(0);
    let mut gui_context = GuiContext::new(tasks.clone(), tasks_condvar);
    let mut windows: BTreeMap<WindowId, Box<dyn IsCgtWindow>> = BTreeMap::new();

    // must be macros because borrow checker
    macro_rules! insert_window {
        ($d:expr) => {{
            let mut d = TitledWindow::without_title($d);
            d.set_title(next_id);
            d.initialize(&gui_context);
            windows.insert(next_id, Box::new(d));
            next_id.0 += 1;
        }};
    }

    macro_rules! new_window {
        ($d:ident) => {
            insert_window!($d::new())
        };
    }

    let mut show_demo = false;
    let mut show_queue = false;

    imgui_sdl2_boilerplate::run("cgt-gui", |large_font, ui| {
        gui_context.large_font_id = large_font;

        ui.dockspace_over_main_viewport();

        if show_demo {
            ui.show_demo_window(&mut show_demo);
        }

        if show_queue {
            ui.window("Work Queue")
                .size([300.0, 600.0], Condition::Appearing)
                .bring_to_front_on_focus(true)
                .menu_bar(false)
                .opened(&mut show_queue)
                .build(|| {
                    if let Some(_table) = ui.begin_table("work table", 3) {
                        ui.table_setup_column_with(TableColumnSetup::new("Name"));
                        ui.table_setup_column_with(TableColumnSetup::new("Status"));
                        ui.table_setup_column_with(TableColumnSetup::new("##Action"));

                        ui.table_headers_row();

                        let mut to_cancel = None;
                        if let Some(current) = &mut *current_task.lock().unwrap() {
                            ui.table_next_row();
                            ui.table_next_column();

                            ui.text(format!("{}#{}", current.name, current.id.0));
                            ui.table_next_column();

                            ui.text("Running");
                            ui.table_next_column();

                            if let Some(canceller) = &mut current.canceller {
                                if ui.button("Cancel") {
                                    canceller();
                                }
                            }
                            ui.table_next_column();
                        }

                        for task in tasks.lock().unwrap().iter() {
                            ui.table_next_row();
                            ui.table_next_column();

                            ui.text(format!("{}#{}", task.name, task.id.0));
                            ui.table_next_column();

                            ui.text("Queued");
                            ui.table_next_column();

                            if ui.button("Cancel") {
                                to_cancel = Some(task.id);
                            }
                        }

                        if let Some(to_cancel) = to_cancel {
                            tasks.lock().unwrap().retain(|t| t.id != to_cancel);
                        }
                    }
                });
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
                if ui.menu_item("Konane") {
                    new_window!(KonaneWindow);
                }
                ui.separator();
                if ui.menu_item("Snort") {
                    new_window!(SnortWindow);
                }
                if ui.menu_item("Digraph Placement") {
                    new_window!(DigraphPlacementWindow);
                }
                ui.separator();
                if ui.menu_item("Resolving Set") {
                    new_window!(ResolvingSetWindow);
                }
                {
                    use crate::widgets::graph_editor::PositionedVertex;
                    ui.separator();
                    if ui.menu_item("Undirected Graph") {
                        insert_window!(GraphWindow::<UndirectedGraph<PositionedVertex>>::new());
                    }
                    if ui.menu_item("Directed Graph") {
                        insert_window!(GraphWindow::<DirectedGraph<PositionedVertex>>::new());
                    }
                }
                ui.separator();
                if ui.menu_item("Quelhas") {
                    new_window!(QuelhasWindow);
                }
            }
            if let Some(_debug_menu) = ui.begin_menu("Debug") {
                if ui.menu_item("Work Queue") {
                    show_queue = true;
                }
                if ui.menu_item("Dear ImGui") {
                    show_demo = true;
                }
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
