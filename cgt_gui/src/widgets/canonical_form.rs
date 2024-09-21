use cgt::short::partizan::canonical_form::CanonicalForm;
use imgui::{Condition, ImColor32};
use std::str::FromStr;

use crate::{widgets, CgtWindow, Details, TitledWindow};

#[derive(Clone)]
pub struct CanonicalFormWindow {
    details: Details,
    value_input: String,
    input_error: bool,
}

impl CanonicalFormWindow {
    pub fn new() -> CanonicalFormWindow {
        let cf = CanonicalForm::from_str("{-1,{2|-2}|-5}").unwrap();
        CanonicalFormWindow::with_details(Details::from_canonical_form(cf))
    }

    pub fn with_details(details: Details) -> CanonicalFormWindow {
        CanonicalFormWindow {
            value_input: details.canonical_form.to_string(),
            details,
            input_error: false,
        }
    }
}

impl TitledWindow<CanonicalFormWindow> {
    pub fn draw(&mut self, ui: &imgui::Ui, new_windows: &mut Vec<CgtWindow>) {
        if !self.is_open {
            return;
        }

        ui.window(&self.title)
            .position(ui.io().mouse_pos, Condition::Appearing)
            .size([400.0, 450.0], Condition::Appearing)
            .bring_to_front_on_focus(true)
            .menu_bar(true)
            .opened(&mut self.is_open)
            .build(|| {
                let draw_list = ui.get_window_draw_list();

                if let Some(_menu_bar) = ui.begin_menu_bar() {
                    if let Some(_new_menu) = ui.begin_menu("New") {
                        if ui.menu_item("Duplicate") {
                            let w = self.content.clone();
                            new_windows.push(CgtWindow::from(w));
                        };
                    }
                }

                let short_inputs = ui.push_item_width(250.0);
                if ui
                    .input_text("Value", &mut self.content.value_input)
                    .build()
                {
                    match CanonicalForm::from_str(&self.content.value_input) {
                        Err(_) => self.content.input_error = true,
                        Ok(cf) => {
                            self.content.input_error = false;
                            self.content.details = Details::from_canonical_form(cf);
                        }
                    }
                }
                short_inputs.end();

                if self.content.input_error {
                    ui.text_colored(
                        ImColor32::from_rgb(0xdd, 0x00, 0x00).to_rgba_f32s(),
                        "Invalid input",
                    );
                }
                ui.text_wrapped(&self.content.details.canonical_form_rendered);
                ui.text(&self.content.details.temperature_rendered);
                widgets::thermograph(
                    ui,
                    &draw_list,
                    50.0,
                    &mut self.scratch_buffer,
                    &self.content.details.thermograph,
                );
            });
    }
}
