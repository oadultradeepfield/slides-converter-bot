use super::ConversionError;

#[derive(Clone, Copy)]
pub(super) struct Geometry {
    pub media: [f32; 4],
    pub crop: [f32; 4],
    pub rotation: i64,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy)]
pub(super) struct Placement {
    pub scale: f32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

const POINTS_PER_MM: f32 = 72.0 / 25.4;
pub(super) const PAGE_WIDTH: f32 = 210.0 * POINTS_PER_MM;
pub(super) const PAGE_HEIGHT: f32 = 297.0 * POINTS_PER_MM;
pub(super) const ROWS: usize = 3;
pub(super) const MARGIN: f32 = 10.0 * POINTS_PER_MM;
const ROW_PADDING: f32 = 5.0 * POINTS_PER_MM;
const GAP: f32 = 6.0 * POINTS_PER_MM;
const SLIDE_RATIO: f32 = 0.64;
const ROW_HEIGHT: f32 = (PAGE_HEIGHT - 2.0 * MARGIN) / ROWS as f32;
const CONTENT_WIDTH: f32 = PAGE_WIDTH - 2.0 * MARGIN - GAP;
const SLIDE_WIDTH: f32 = CONTENT_WIDTH * SLIDE_RATIO;
const SLIDE_HEIGHT: f32 = ROW_HEIGHT - 2.0 * ROW_PADDING;
pub(super) const NOTES_WIDTH: f32 = CONTENT_WIDTH * (1.0 - SLIDE_RATIO);
pub(super) const NOTES_HEIGHT: f32 = SLIDE_HEIGHT;
pub(super) const DOT_SPACING: f32 = 5.0 * POINTS_PER_MM;

impl Geometry {
    pub fn new(media: [f32; 4], crop: [f32; 4], rotation: i64) -> Result<Self, ConversionError> {
        let rotation = rotation.rem_euclid(360);
        if rotation % 90 != 0 {
            return Err(ConversionError::InvalidGeometry);
        }
        let (width, height) = (media[2] - media[0], media[3] - media[1]);
        let (width, height) = match rotation {
            90 | 270 => (height, width),
            _ => (width, height),
        };
        Ok(Self {
            media,
            crop,
            rotation,
            width,
            height,
        })
    }

    pub fn transform(&self, x: f32, y: f32) -> (f32, f32) {
        let [left, bottom, right, top] = self.media;
        match self.rotation {
            90 => (y - bottom, right - x),
            180 => (right - x, top - y),
            270 => (top - y, x - left),
            _ => (x - left, y - bottom),
        }
    }

    pub fn matrix(&self) -> [f32; 6] {
        let (x, y) = self.transform(0.0, 0.0);
        match self.rotation {
            90 => [0.0, -1.0, 1.0, 0.0, x, y],
            180 => [-1.0, 0.0, 0.0, -1.0, x, y],
            270 => [0.0, 1.0, -1.0, 0.0, x, y],
            _ => [1.0, 0.0, 0.0, 1.0, x, y],
        }
    }
}

pub(super) fn place_slide(geometry: &Geometry, row: usize) -> Placement {
    let scale = (SLIDE_WIDTH / geometry.width).min(SLIDE_HEIGHT / geometry.height);
    let (_, bottom) = notes_origin(row);
    Placement {
        scale,
        x: MARGIN + (SLIDE_WIDTH - geometry.width * scale) / 2.0,
        y: bottom + (SLIDE_HEIGHT - geometry.height * scale) / 2.0,
        width: geometry.width * scale,
        height: geometry.height * scale,
    }
}

pub(super) fn notes_origin(row: usize) -> (f32, f32) {
    (
        MARGIN + SLIDE_WIDTH + GAP,
        PAGE_HEIGHT - MARGIN - (row + 1) as f32 * ROW_HEIGHT + ROW_PADDING,
    )
}

pub(super) fn separator_y(row: usize) -> f32 {
    notes_origin(row).1 - ROW_PADDING / 2.0
}
