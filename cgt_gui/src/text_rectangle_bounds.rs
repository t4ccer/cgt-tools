use raylib::prelude::*;

pub fn draw_text_boxed(
    d: &mut impl RaylibDraw,
    font: &impl RaylibFont,
    text: &str,
    rec: Rectangle,
    font_size: f32,
    spacing: f32,
    word_wrap: bool,
    tint: Color,
) -> Vector2 {
    draw_text_boxed_selectable(
        d,
        font,
        text,
        rec,
        font_size,
        spacing,
        word_wrap,
        tint,
        0,
        0,
        Color::WHITE,
        Color::WHITE,
    )
}

fn draw_text_boxed_selectable(
    d: &mut impl RaylibDraw,
    font: &impl RaylibFont,
    text: &str,
    rec: Rectangle,
    font_size: f32,
    spacing: f32,
    word_wrap: bool,
    tint: Color,
    mut select_start: i32,
    select_length: i32,
    select_tint: Color,
    select_back_tint: Color,
) -> Vector2 {
    let mut text_offset_y = 0.0; // Offset between lines (on line break '\n')
    let mut text_offset_x = 0.0; // Offset X to next character to draw

    let scale_factor = font_size / font.base_size() as f32; // Character rectangle scaling factor

    // Word/character wrapping mechanism variables
    enum MeasureState {
        Measure,
        Draw,
    }

    let mut state = if word_wrap {
        MeasureState::Measure
    } else {
        MeasureState::Draw
    };

    let mut start_line = -1;
    let mut end_line = -1;
    let mut last_k: i32 = -1;

    // PORTING NOTE: This loop is very c-like and doesn't really use much of
    // rusts strings, could be improved. But it is correct.
    let mut i: i32 = -1;
    loop {
        i += 1;
        if i as usize >= text.len() {
            break;
        }

        // Get next codepoint from byte string and glyph index in font
        let codepoint = text[i as usize..].chars().next().unwrap();
        i += codepoint.len_utf8() as i32 - 1;
        let mut k = i as i32;

        let glyph_index = font.get_glyph_index(codepoint) as usize;
        let glyph = &font.chars()[glyph_index];
        let mut glyph_width = 0.0;
        if codepoint != '\n' {
            if glyph.advanceX == 0 {
                glyph_width = glyph.image.width as f32 * scale_factor;
            } else {
                glyph_width = glyph.advanceX as f32 * scale_factor;
            }

            if (i as usize + 1) < text.len() {
                glyph_width += spacing;
            }
        }

        match state {
            MeasureState::Measure => {
                // NOTE: When wordWrap is ON we first measure how much of the text we can draw
                // before going outside of the rec container.
                // We store this info in startLine and endLine, then we change states, draw the text
                // between those two variables and change states again and again recursively until
                // the end of the text (or until we get outside of the container).
                // When wordWrap is OFF we don't need the measure state so we go to the drawing state
                // immediately and begin drawing on the next line before we can get outside the container.

                if codepoint.is_whitespace() {
                    end_line = i as i32;
                }

                if text_offset_x + glyph_width > rec.width {
                    end_line = if end_line < 1 { i as i32 } else { end_line };
                    if i == end_line {
                        end_line -= codepoint.len_utf8() as i32;
                    }

                    if (start_line + codepoint.len_utf8() as i32) == end_line {
                        end_line = i as i32 - codepoint.len_utf8() as i32;
                    }

                    state = MeasureState::Draw;
                } else if i as usize + 1 == text.len() {
                    end_line = i as i32;
                    state = MeasureState::Draw;
                } else if codepoint == '\n' {
                    state = MeasureState::Draw;
                }

                if matches!(state, MeasureState::Draw) {
                    text_offset_x = 0.0;
                    i = start_line;
                    glyph_width = 0.0;

                    // Save character position when we switch states
                    k = last_k;
                    last_k = k - 1;
                }
            }
            MeasureState::Draw => {
                if codepoint == '\n' {
                    if !word_wrap {
                        text_offset_y += (font.base_size() as f32 + font.base_size() as f32 / 2.0)
                            * scale_factor;
                        text_offset_x = 0.0;
                    }
                } else {
                    if !word_wrap && (text_offset_x + glyph_width > rec.width) {
                        text_offset_y += (font.base_size() as f32 + font.base_size() as f32 / 2.0)
                            * scale_factor;
                        text_offset_x = 0.0;
                    }

                    // When text overflows rectangle height limit, just stop drawing
                    if text_offset_y + font.base_size() as f32 * scale_factor > rec.height {
                        break;
                    }

                    // Draw selection background
                    let is_glyph_selected = if select_start >= 0
                        && k as i32 >= select_start
                        && (k as i32) < (select_start + select_length)
                    {
                        d.draw_rectangle_rec(
                            Rectangle {
                                x: rec.x + text_offset_x - 1.0,
                                y: rec.y + text_offset_y,
                                width: glyph_width,
                                height: font.base_size() as f32 * scale_factor,
                            },
                            select_back_tint,
                        );
                        true
                    } else {
                        false
                    };

                    // Draw current character glyph
                    if codepoint != ' ' && codepoint != '\t' {
                        d.draw_text_codepoint(
                            font,
                            codepoint as i32,
                            Vector2 {
                                x: rec.x + text_offset_x,
                                y: rec.y + text_offset_y,
                            },
                            font_size,
                            if is_glyph_selected { select_tint } else { tint },
                        );
                    }
                }

                if word_wrap && i as i32 == end_line {
                    text_offset_y +=
                        (font.base_size() as f32 + font.base_size() as f32 / 2.0) * scale_factor;
                    text_offset_x = 0.0;
                    start_line = end_line;
                    end_line = -1;
                    select_start += last_k - k as i32;
                    state = MeasureState::Measure;
                }
            }
        }

        if text_offset_x != 0.0 || codepoint != ' ' {
            text_offset_x += glyph_width; // Avoid leading spaces
        }
    }

    text_offset_y += (font.base_size() as f32 + font.base_size() as f32 / 2.0) * scale_factor;

    Vector2 {
        x: text_offset_x + rec.x,
        y: text_offset_y + rec.y,
    }
}
