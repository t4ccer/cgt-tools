use cgt::short::partizan::canonical_form::CanonicalForm;
use imgui::{Condition, ImColor32};
use std::str::FromStr;

use crate::{widgets, Details, WindowId};

pub struct CanonicalFormWindow {
    title: String,
    is_open: bool,
    details: Details,
    value_input: String,
    input_error: bool,
}

impl CanonicalFormWindow {
    pub fn new(id: WindowId) -> CanonicalFormWindow {
        let cf = CanonicalForm::from_str("{-1,{2|-2}|-5}").unwrap();
        CanonicalFormWindow {
            value_input: cf.to_string(),
            details: Details::from_canonical_form(cf),
            is_open: true,
            title: format!("Canonical Form##{}", id.0),
            input_error: false,
        }
    }

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
