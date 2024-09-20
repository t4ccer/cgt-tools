use cgt::{
    numeric::dyadic_rational_number::DyadicRationalNumber,
    short::partizan::{
        canonical_form::CanonicalForm, thermograph::Thermograph,
        transposition_table::ParallelTranspositionTable,
    },
};
use imgui::{Condition, ImColor32};
use std::{marker::PhantomData, str::FromStr};

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

#[derive(Clone, Copy)]
pub struct WindowId(pub usize);

#[allow(dead_code)]
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

pub enum CgtWindow<'tt> {
    Domineering(DomineeringWindow<'tt>),
    CanonicalForm(CanonicalFormWindow),
    Snort(SnortWindow<'tt>),
}

pub struct CanonicalFormWindow {
    title: String,
    is_open: bool,
    details: Details,
    value_input: String,
    input_error: bool,
}

impl CanonicalFormWindow {
    pub fn draw(&mut self, ui: &imgui::Ui) {
        if !self.is_open {
            return;
        }

        ui.window(&self.title)
            .position([50.0, 50.0], Condition::Appearing)
            .size([400.0, 450.0], Condition::Appearing)
            .bring_to_front_on_focus(true)
            .opened(&mut self.is_open)
            .build(|| {
                let draw_list = ui.get_window_draw_list();
                let short_inputs = ui.push_item_width(250.0);
                if ui.input_text("Value", &mut self.value_input).build() {
                    match CanonicalForm::from_str(&self.value_input) {
                        Err(_) => self.input_error = true,
                        Ok(cf) => {
                            self.input_error = false;
                            self.details = Details::from_canonical_form(cf);
                        }
                    }
                }
                short_inputs.end();

                if self.input_error {
                    ui.text_colored(
                        ImColor32::from_rgb(0xdd, 0x00, 0x00).to_rgba_f32s(),
                        "Invalid input",
                    );
                }
                ui.text_wrapped(&self.details.canonical_form_rendered);
                ui.text(&self.details.temperature_rendered);
                widgets::thermograph(ui, &draw_list, 50.0, &self.details.thermograph);
            });
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
    let mut windows = Vec::<CgtWindow>::new();

    let domineering_tt = ParallelTranspositionTable::new();
    let snort_tt = ParallelTranspositionTable::new();

    // must be a macro because borrow checker
    macro_rules! new_domineering {
        () => {{
            let d = DomineeringWindow::new(next_id, &domineering_tt);
            next_id.0 += 1;
            windows.push(CgtWindow::Domineering(d));
        }};
    }

    macro_rules! new_canonical_form {
        () => {{
            let cf = CanonicalForm::from_str("{-1,{2|-2}|-5}").unwrap();
            let d = CanonicalFormWindow {
                value_input: cf.to_string(),
                details: Details::from_canonical_form(cf),
                is_open: true,
                title: format!("Canonical Form##{}", next_id.0),
                input_error: false,
            };
            next_id.0 += 1;
            windows.push(CgtWindow::CanonicalForm(d));
        }};
    }

    macro_rules! new_snort {
        () => {{
            let mut d = SnortWindow::new(next_id, &snort_tt);
            next_id.0 += 1;
            d.reposition_circle();
            windows.push(CgtWindow::Snort(d));
        }};
    }

    // new_domineering!();
    new_snort!();

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

        for d in windows.iter_mut() {
            match d {
                CgtWindow::Domineering(d) => d.draw(ui),
                CgtWindow::CanonicalForm(d) => d.draw(ui),
                CgtWindow::Snort(d) => d.draw(ui),
            }
        }
    });
}
