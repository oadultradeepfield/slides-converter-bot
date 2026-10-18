use pdf_writer::{Content, Name};

use super::{
    PlannedPage,
    layout::{self, DOT_SPACING, MARGIN, NOTES_HEIGHT, NOTES_WIDTH, PAGE_WIDTH},
};

const DOT_RADIUS: f32 = 0.55;
const CIRCLE_CONTROL: f32 = DOT_RADIUS * 0.552_284_8;

pub(super) fn draw_page(page: &PlannedPage) -> Vec<u8> {
    let mut content = Content::new();
    for (row, slide) in page.slides.iter().enumerate() {
        let place = slide.placement;
        draw_form(
            &mut content,
            format!("Slide{row}").as_bytes(),
            [place.scale, 0.0, 0.0, place.scale, place.x, place.y],
        );
        let (x, y) = layout::notes_origin(row);
        draw_form(&mut content, b"Grid", [1.0, 0.0, 0.0, 1.0, x, y]);
        if row + 1 < page.slides.len() {
            draw_separator(&mut content, layout::separator_y(row));
        }
    }
    content.finish().into_vec()
}

pub(super) fn draw_grid() -> Vec<u8> {
    let mut content = Content::new();
    content.set_fill_rgb(0.78, 0.78, 0.78);
    let columns = (NOTES_WIDTH / DOT_SPACING - 0.5).ceil() as usize;
    let rows = (NOTES_HEIGHT / DOT_SPACING - 0.5).ceil() as usize;
    for column in 0..columns {
        for row in 0..rows {
            draw_dot(
                &mut content,
                (column as f32 + 0.5) * DOT_SPACING,
                (row as f32 + 0.5) * DOT_SPACING,
            );
        }
    }
    content.fill_nonzero();
    content.finish().into_vec()
}

fn draw_form(content: &mut Content, name: &[u8], matrix: [f32; 6]) {
    content
        .save_state()
        .transform(matrix)
        .x_object(Name(name))
        .restore_state();
}

fn draw_separator(content: &mut Content, y: f32) {
    content
        .save_state()
        .set_stroke_rgb(0.65, 0.65, 0.65)
        .set_line_width(0.6)
        .set_dash_pattern([4.0, 4.0], 0.0)
        .move_to(MARGIN, y)
        .line_to(PAGE_WIDTH - MARGIN, y)
        .stroke()
        .restore_state();
}

fn draw_dot(content: &mut Content, x: f32, y: f32) {
    let (r, c) = (DOT_RADIUS, CIRCLE_CONTROL);
    content
        .move_to(x + r, y)
        .cubic_to(x + r, y + c, x + c, y + r, x, y + r)
        .cubic_to(x - c, y + r, x - r, y + c, x - r, y)
        .cubic_to(x - r, y - c, x - c, y - r, x, y - r)
        .cubic_to(x + c, y - r, x + r, y - c, x + r, y);
}
