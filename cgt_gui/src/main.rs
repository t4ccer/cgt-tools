use cgt::{
    numeric::{dyadic_rational_number::DyadicRationalNumber, v2f::V2f},
    short::partizan::{
        canonical_form::CanonicalForm,
        games::{domineering::Domineering, snort::Snort},
        partizan_game::PartizanGame,
        thermograph::Thermograph,
        transposition_table::ParallelTranspositionTable,
    },
};
use std::{collections::BTreeMap, marker::PhantomData, sync::mpsc, thread};
use widgets::canonical_form::CanonicalFormWindow;

use crate::widgets::{domineering::DomineeringWindow, snort::SnortWindow};

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
    pub fn as_ref(&self) -> Option<&Details> {
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
    pub fn new() -> DetailOptions {
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
    pub fn without_title(content: G) -> TitledWindow<G> {
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
    fn init(&self, ctx: &Context);
    fn is_open(&self) -> bool;
    fn draw(&mut self, ui: &imgui::Ui, ctx: &mut Context);
    fn update(&mut self, update: UpdateKind);
}

macro_rules! impl_titled_window {
    ($title:expr) => {
        fn set_title(&mut self, id: $crate::WindowId) {
            self.title = format!("{}##{}", $title, id.0);
            self.window_id = id;
        }

        fn is_open(&self) -> bool {
            self.is_open
        }
    };
}

pub(crate) use impl_titled_window;

macro_rules! impl_game_window {
    ($task_kind:ident, $update_kind:ident) => {
        fn init(&self, ctx: &Context) {
            ctx.schedule_task($crate::Task::$task_kind($crate::EvalTask {
                window: self.window_id,
                game: self.content.game.clone(),
            }));
        }

        fn update(&mut self, update: $crate::UpdateKind) {
            match update {
                UpdateKind::$update_kind(game, details) => {
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

pub trait IsEnum {
    const LABELS: &'static [&'static str];

    fn to_usize(self) -> usize;
    fn from_usize(raw: usize) -> Self;
}

#[derive(Debug, Clone, Copy)]
pub struct RawOf<T> {
    pub value: usize,
    _ty: PhantomData<T>,
}

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

    pub fn as_enum(self) -> T {
        T::from_usize(self.value)
    }
}

macro_rules! imgui_enum {
    ($name:ident { $($variant:ident, $raw:expr, $pretty:expr,)*}) => {
        #[derive(Debug, Clone, Copy)]
        #[repr(usize)]
        pub enum $name {
            $($variant,)*
        }

        impl $crate::IsEnum for $name {
            const LABELS: &'static [&'static str] = &[$($pretty,)*];

            fn to_usize(self) -> usize {
                self as usize
            }

            fn from_usize(raw: usize) -> $name {
                match raw {
                    $($raw => $name::$variant,)*
                    _ => panic!("Invalid value: {raw}")
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
    EvalSnort(EvalTask<Snort>),
}

pub struct Context {
    pub new_windows: Vec<Box<dyn IsCgtWindow>>,
    pub removed_windows: Vec<WindowId>,
    tasks: mpsc::Sender<Task>,
}

impl Context {
    pub fn new(tasks: mpsc::Sender<Task>) -> Context {
        Context {
            new_windows: Vec::new(),
            removed_windows: Vec::new(),
            tasks,
        }
    }

    pub fn schedule_task(&self, task: Task) {
        self.tasks.send(task).unwrap();
    }
}

pub enum UpdateKind {
    DomineeringDetails(Domineering, Details),
    SnortDetails(Snort, Details),
}

pub struct Update {
    window: WindowId,
    kind: UpdateKind,
}

pub struct SchedulerContext {
    tasks: mpsc::Receiver<Task>,
    updates: mpsc::Sender<Update>,
    domineering_tt: ParallelTranspositionTable<Domineering>,
    snort_tt: ParallelTranspositionTable<Snort>,
}

fn scheduler(ctx: SchedulerContext) {
    while let Ok(task) = ctx.tasks.recv() {
        match task {
            Task::EvalDomineering(task) => {
                let cf = task.game.canonical_form(&ctx.domineering_tt);
                let details = Details::from_canonical_form(cf);
                ctx.updates
                    .send(Update {
                        window: task.window,
                        kind: UpdateKind::DomineeringDetails(task.game, details),
                    })
                    .unwrap();
            }
            Task::EvalSnort(task) => {
                let cf = task.game.canonical_form(&ctx.snort_tt);
                let details = Details::from_canonical_form(cf);
                ctx.updates
                    .send(Update {
                        window: task.window,
                        kind: UpdateKind::SnortDetails(task.game, details),
                    })
                    .unwrap();
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
        snort_tt: ParallelTranspositionTable::new(),
    };

    thread::spawn(move || scheduler(scheduler_ctx));

    let mut next_id = WindowId(0);
    let mut ctx = Context::new(task_sender);
    let mut windows: BTreeMap<WindowId, Box<dyn IsCgtWindow>> = BTreeMap::new();

    // must be macros because borrow checker
    macro_rules! new_window {
        ($d:expr) => {{
            $d.set_title(next_id);
            $d.init(&ctx);
            windows.insert(next_id, Box::new($d));
            next_id.0 += 1;
        }};
    }

    macro_rules! new_domineering {
        () => {{
            let mut d = TitledWindow::without_title(DomineeringWindow::new());
            new_window!(d);
        }};
    }

    macro_rules! new_canonical_form {
        () => {{
            let mut d = TitledWindow::without_title(CanonicalFormWindow::new());
            new_window!(d);
        }};
    }

    macro_rules! new_snort {
        () => {{
            let mut d = SnortWindow::new();
            d.reposition(V2f { x: 350.0, y: 400.0 });
            let mut d = TitledWindow::without_title(d);
            d.set_title(next_id);
            new_window!(d);
        }};
    }

    // new_domineering!();
    new_snort!();

    let mut show_demo = false;

    imgui_sdl2_boilerplate::run("cgt-gui", |ui| {
        ui.dockspace_over_main_viewport();

        if show_demo {
            ui.show_demo_window(&mut show_demo);
        }

        if let Some(_main_menu) = ui.begin_main_menu_bar() {
            if let Some(_new_menu) = ui.begin_menu("New") {
                if ui.menu_item("Canonical Form") {
                    new_canonical_form!();
                }
                if ui.menu_item("Domineering") {
                    new_domineering!();
                }
                if ui.menu_item("Snort") {
                    new_snort!();
                }
            }
            if ui.menu_item("Debug") {
                show_demo = true;
            }
        }

        for (&wid, d) in windows.iter_mut() {
            d.draw(ui, &mut ctx);
            if !d.is_open() {
                ctx.removed_windows.push(wid);
            }
        }

        for to_remove in ctx.removed_windows.drain(..) {
            windows.remove(&to_remove);
        }

        for mut d in ctx.new_windows.drain(..) {
            // NOTE: We don't call init() here because windows created by other windows should
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
