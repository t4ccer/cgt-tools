use crate::{
    drawing::{self, Color},
    graph::{Graph, VertexIndex},
    grid::FiniteGrid,
    has::Has,
    numeric::v2f::V2f,
};
use imgui::{DrawListMut, FontId};

// TODO: Move cursor to the bottom after drawing?

pub struct Canvas<'ui> {
    start_position: V2f,
    ui: &'ui imgui::Ui,
    draw_list: &'ui DrawListMut<'ui>,
    large_font_id: FontId,
    clicked_position: Option<V2f>,
    pressed_position: Option<V2f>,
}

impl<'ui> Canvas<'ui> {
    pub fn new(
        ui: &'ui imgui::Ui,
        draw_list: &'ui DrawListMut<'ui>,
        large_font_id: FontId,
    ) -> Self {
        Self {
            start_position: V2f::from(ui.cursor_screen_pos()),
            ui,
            draw_list,
            large_font_id,
            clicked_position: None,
            pressed_position: None,
        }
    }

    pub fn clicked_tile<G>(&self, grid: &G) -> Option<(u8, u8)>
    where
        G: FiniteGrid,
    {
        self.clicked_position
            .and_then(|clicked_pos| grid.tile_at_position::<Canvas>(clicked_pos))
    }

    pub fn clicked_vertex<G, V>(&self, graph: &G) -> Option<VertexIndex>
    where
        V: Has<V2f>,
        G: Graph<V>,
    {
        self.clicked_position
            .and_then(|clicked_pos| self.vertex_at_position(clicked_pos, graph))
    }

    pub fn pressed_vertex<G, V>(&self, graph: &G) -> Option<VertexIndex>
    where
        V: Has<V2f>,
        G: Graph<V>,
    {
        self.pressed_position
            .and_then(|clicked_pos| self.vertex_at_position(clicked_pos, graph))
    }

    // TODO: Move to graph
    pub fn vertex_at_position<G, V>(&self, position: V2f, graph: &G) -> Option<VertexIndex>
    where
        V: Has<V2f>,
        G: Graph<V>,
    {
        for vertex_idx in graph.vertex_indices() {
            let vertex_position: V2f = *graph.get_vertex(vertex_idx).get_inner();
            if position.inside_circle(
                vertex_position,
                <Canvas as drawing::Canvas>::vertex_radius(),
            ) {
                return Some(vertex_idx);
            }
        }
        None
    }

    fn faded(&self, color: Color) -> Color {
        if self.ui.is_item_active() {
            color.faded(155)
        } else if self.ui.is_item_hovered() {
            color.faded(200)
        } else {
            color
        }
    }
}

impl drawing::Canvas for Canvas<'_> {
    fn rect(&mut self, position: V2f, size: V2f, color: drawing::Color) {
        self.draw_list
            .add_rect(
                self.start_position + position,
                self.start_position + position + size,
                color,
            )
            .filled(true)
            .build();
    }

    fn circle(&mut self, position: V2f, radius: f32, color: drawing::Color) {
        self.draw_list
            .add_circle(self.start_position + position, radius, color)
            .filled(true)
            .build();
    }

    fn line(&mut self, start: V2f, end: V2f, weight: f32, color: Color) {
        // HACK: https://github.com/ocornut/imgui/issues/3258
        let offset = V2f { x: -0.5, y: -0.5 };

        self.draw_list
            .add_line(
                self.start_position + start + offset,
                self.start_position + end + offset,
                color,
            )
            .thickness(weight)
            .build();
    }

    fn large_char(&mut self, letter: char, position: V2f, color: Color) {
        let _large_font = self.ui.push_font(self.large_font_id);
        let mut buf: [u8; 4] = [0; 4];
        let text = letter.encode_utf8(&mut buf);
        let size = V2f::from(self.ui.calc_text_size(&text));
        let text_pos = self.start_position + position + (Self::tile_size() - size) * 0.5;
        self.draw_list.add_text(text_pos, color, &text);
    }

    fn tile(&mut self, position: V2f, tile: drawing::Tile) {
        let _tile_id_x = self.ui.push_id_int(position.x as i32);
        let _tile_id_y = self.ui.push_id_int(position.y as i32);

        self.ui
            .set_cursor_screen_pos(self.start_position + position);
        if self.ui.invisible_button("", Self::tile_size()) {
            self.clicked_position = Some(position);
        }

        let tile_size = Self::tile_size();
        match tile {
            drawing::Tile::Square { color } => {
                self.rect(position, tile_size, self.faded(color));
            }
            drawing::Tile::Circle {
                tile_color,
                circle_color,
            } => {
                self.rect(position, tile_size, self.faded(tile_color));
                self.circle(
                    position + tile_size * 0.5,
                    tile_size.x * 0.4,
                    self.faded(circle_color),
                );
            }
            drawing::Tile::Char {
                tile_color,
                text_color,
                letter,
            } => {
                self.rect(position, tile_size, self.faded(tile_color));
                self.large_char(letter, position, self.faded(text_color));
            }
        }
    }

    fn highlight_tile(&mut self, position: V2f, color: Color) {
        self.draw_list
            .add_rect(
                self.start_position + position,
                self.start_position + position + Self::tile_size(),
                color,
            )
            .thickness(Self::thick_line_weight() * 2.0)
            .filled(false)
            .build();
    }

    fn tile_size() -> V2f {
        V2f { x: 64.0, y: 64.0 }
    }

    fn thick_line_weight() -> f32 {
        2.0
    }

    fn vertex(&mut self, position: V2f, color: Color, idx: VertexIndex) {
        let radius = Self::vertex_radius();

        let _tile_id = self.ui.push_id_usize(idx.index);

        self.ui.set_cursor_screen_pos(
            self.start_position + position
                - V2f {
                    x: radius,
                    y: radius,
                },
        );
        if self.ui.invisible_button(
            "",
            V2f {
                x: radius * 2.0,
                y: radius * 2.0,
            },
        ) {
            self.clicked_position = Some(position);
        }

        if self.ui.is_item_active() {
            self.pressed_position = Some(position);
        }

        self.circle(position, radius, Color::BLACK);
        self.circle(position, radius - 1.0, self.faded(color));
    }
}
