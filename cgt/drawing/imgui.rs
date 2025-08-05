use crate::{
    drawing::{self, Color},
    numeric::v2f::V2f,
};
use imgui::DrawListMut;

// TODO: Move cursor to the bottom after drawing?

pub struct Canvas<'ui> {
    start_position: V2f,
    ui: &'ui imgui::Ui,
    draw_list: &'ui DrawListMut<'ui>,
    clicked_tile: Option<V2f>,
}

impl<'ui> Canvas<'ui> {
    pub fn new(ui: &'ui imgui::Ui, draw_list: &'ui DrawListMut<'ui>) -> Self {
        Self {
            start_position: V2f::from(ui.cursor_screen_pos()),
            ui,
            draw_list,
            clicked_tile: None,
        }
    }

    pub const fn clicked_tile_position(&self) -> Option<V2f> {
        self.clicked_tile
    }
}

impl drawing::Canvas for Canvas<'_> {
    fn tile(&mut self, position: V2f, color: drawing::Color) {
        let _tile_id_x = self.ui.push_id_int(position.x as i32);
        let _tile_id_y = self.ui.push_id_int(position.y as i32);

        self.ui
            .set_cursor_screen_pos(self.start_position + position);

        if self.ui.invisible_button("", Self::tile_size()) {
            self.clicked_tile = Some(position);
        }

        let color = if self.ui.is_item_active() {
            color.faded(155)
        } else if self.ui.is_item_hovered() {
            color.faded(200)
        } else {
            color
        };

        self.draw_list
            .add_rect(
                self.start_position + position,
                self.start_position + position + Self::tile_size(),
                color,
            )
            .filled(true)
            .build();
    }

    fn circle(&mut self, position: V2f, radius: f32, color: drawing::Color) {
        self.draw_list
            .add_circle(self.start_position + position, radius, color)
            .build();
    }

    fn line(&mut self, start: V2f, end: V2f) {
        // HACK: https://github.com/ocornut/imgui/issues/3258
        let offset = V2f { x: -0.5, y: -0.5 };

        self.draw_list
            .add_line(
                self.start_position + start + offset,
                self.start_position + end + offset,
                Color::BLACK,
            )
            .thickness(Self::default_line_weight())
            .build();
    }

    fn tile_size() -> V2f {
        V2f { x: 64.0, y: 64.0 }
    }

    fn default_line_weight() -> f32 {
        2.0
    }
}
