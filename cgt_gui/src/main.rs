use cgt::{
    numeric::dyadic_rational_number::DyadicRationalNumber,
    short::partizan::{
        canonical_form::CanonicalForm, thermograph::Thermograph,
        transposition_table::ParallelTranspositionTable,
    },
};
use std::{collections::BTreeMap, marker::PhantomData};
use widgets::canonical_form::CanonicalFormWindow;

use crate::widgets::{domineering::DomineeringWindow, snort::SnortWindow};

mod imgui_sdl2_boilerplate;
mod widgets;

fn fade(mut color: [f32; 4], alpha: f32) -> [f32; 4] {
    let alpha = alpha.clamp(0.0, 1.0);
    color[3] *= alpha;
    color
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + t * (end - start)
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WindowId(pub usize);

#[allow(dead_code)]
#[derive(Clone)]
pub struct Details {
    canonical_form: CanonicalForm,
    canonical_form_rendered: String,
    thermograph: Thermograph,
    temperature: DyadicRationalNumber,
    temperature_rendered: String,
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

#[derive(Clone)]
pub struct TitledWindow<G> {
    pub title: String,
    pub is_open: bool,
    pub content: G,
    pub scratch_buffer: String,
}

impl<G> TitledWindow<G> {
    pub fn without_title(content: G) -> TitledWindow<G> {
        TitledWindow {
            title: String::new(),
            is_open: true,
            content,
            scratch_buffer: String::new(),
        }
    }
}

pub enum CgtWindow<'tt> {
    Domineering(TitledWindow<DomineeringWindow<'tt>>),
    CanonicalForm(TitledWindow<CanonicalFormWindow>),
    Snort(TitledWindow<SnortWindow<'tt>>),
}

impl<'tt> From<DomineeringWindow<'tt>> for CgtWindow<'tt> {
    fn from(value: DomineeringWindow<'tt>) -> CgtWindow<'tt> {
        CgtWindow::Domineering(TitledWindow::without_title(value))
    }
}

impl<'tt> From<CanonicalFormWindow> for CgtWindow<'tt> {
    fn from(value: CanonicalFormWindow) -> CgtWindow<'tt> {
        CgtWindow::CanonicalForm(TitledWindow::without_title(value))
    }
}

impl<'tt> From<SnortWindow<'tt>> for CgtWindow<'tt> {
    fn from(value: SnortWindow<'tt>) -> CgtWindow<'tt> {
        CgtWindow::Snort(TitledWindow::without_title(value))
    }
}

impl CgtWindow<'_> {
    pub fn set_title(&mut self, id: WindowId) {
        match self {
            CgtWindow::Domineering(d) => d.title = format!("Domineering##{}", id.0),
            CgtWindow::CanonicalForm(d) => d.title = format!("Canonical Form##{}", id.0),
            CgtWindow::Snort(d) => d.title = format!("Snort##{}", id.0),
        }
    }
}

pub trait IsEnum {
    const LABELS: &'static [&'static str];

    fn to_usize(self) -> usize;
    fn from_usize(raw: usize) -> Self;
}

#[derive(Clone, Copy)]
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
        #[derive(Clone, Copy)]
        #[repr(usize)]
        pub enum $name {
            $($variant,)*
        }

        impl crate::IsEnum for $name {
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

fn main() {
    let mut next_id = WindowId(0);
    let mut windows_to_remove = Vec::<WindowId>::new();
    let mut windows_to_add = Vec::<CgtWindow>::new();
    let mut windows: BTreeMap<WindowId, CgtWindow> = BTreeMap::new();

    let domineering_tt = ParallelTranspositionTable::new();
    let snort_tt = ParallelTranspositionTable::new();

    // must be a macro because borrow checker
    macro_rules! new_domineering {
        () => {{
            let d = DomineeringWindow::new(&domineering_tt);
            let mut d = CgtWindow::Domineering(TitledWindow::without_title(d));
            d.set_title(next_id);
            windows.insert(next_id, d);
            next_id.0 += 1;
        }};
    }

    macro_rules! new_canonical_form {
        () => {{
            let d = CanonicalFormWindow::new();
            let mut d = CgtWindow::CanonicalForm(TitledWindow::without_title(d));
            d.set_title(next_id);
            windows.insert(next_id, d);
            next_id.0 += 1;
        }};
    }

    macro_rules! new_snort {
        () => {{
            let mut d = SnortWindow::new(&snort_tt);
            d.reposition_circle();
            let mut d = CgtWindow::Snort(TitledWindow::without_title(d));
            d.set_title(next_id);
            windows.insert(next_id, d);
            next_id.0 += 1;
        }};
    }

    new_domineering!();
    // new_snort!();

    let mut show_debug = false;

    imgui_sdl2_boilerplate::run("cgt-gui", |ui| {
        ui.dockspace_over_main_viewport();

        if show_debug {
            ui.show_demo_window(&mut show_debug);
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
                show_debug = true;
            }
        }

        for (&wid, d) in windows.iter_mut() {
            macro_rules! handle_window {
                ($d:expr) => {{
                    $d.draw(ui, &mut windows_to_add);
                    if !$d.is_open {
                        windows_to_remove.push(wid);
                    }
                }};
            }

            match d {
                CgtWindow::Domineering(d) => handle_window!(d),
                CgtWindow::CanonicalForm(d) => handle_window!(d),
                CgtWindow::Snort(d) => handle_window!(d),
            }
        }

        for to_remove in windows_to_remove.drain(..) {
            windows.remove(&to_remove);
        }

        for mut to_add in windows_to_add.drain(..) {
            to_add.set_title(next_id);
            windows.insert(next_id, to_add);
            next_id.0 += 1;
        }
    });
}
