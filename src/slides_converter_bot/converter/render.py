from io import BytesIO

from pypdf import PageObject, PdfReader, PdfWriter, Transformation
from reportlab.pdfgen.canvas import Canvas

from .layout import (
    calculate_page_layout,
    convert_millimetres_to_points,
    fit_page_inside_rectangle,
    group_slide_indices,
)
from .models import ConversionConfig, PageLayout, PageSize, Rectangle

DOT_COLOR = (0.78, 0.78, 0.78)
DOT_RADIUS_POINTS = 0.55
SEPARATOR_COLOR = (0.65, 0.65, 0.65)
SEPARATOR_WIDTH_POINTS = 0.6
SEPARATOR_DASH_LENGTH = 4


def compose_document(reader: PdfReader, config: ConversionConfig) -> PdfWriter:
    writer = PdfWriter()
    layout = calculate_page_layout(config)

    for group in group_slide_indices(len(reader.pages), config.rows_per_page):
        compose_page(writer, reader, group, layout, config)

    optimize_document(writer)
    return writer


def compose_page(
    writer: PdfWriter,
    reader: PdfReader,
    group: tuple[int, ...],
    layout: PageLayout,
    config: ConversionConfig,
) -> None:
    output_page = writer.add_blank_page(
        width=layout.page_size.width,
        height=layout.page_size.height,
    )
    add_slides_to_page(output_page, reader, group, layout)
    output_page.merge_page(create_page_overlay(layout, len(group), config))


def add_slides_to_page(
    output_page: PageObject,
    reader: PdfReader,
    group: tuple[int, ...],
    layout: PageLayout,
) -> None:
    for row_index, source_index in enumerate(group):
        row = layout.rows[row_index]
        source_page = reader.pages[source_index]
        source_size = PageSize(
            float(source_page.mediabox.width),
            float(source_page.mediabox.height),
        )
        fitted = fit_page_inside_rectangle(source_size, row.slide_area)
        scale = fitted.width / source_size.width
        transformation = Transformation().scale(scale).translate(fitted.x, fitted.y)

        output_page.merge_transformed_page(source_page, transformation, over=True)


def create_page_overlay(
    layout: PageLayout,
    group_size: int,
    config: ConversionConfig,
) -> PageObject:
    buffer = BytesIO()
    canvas = Canvas(buffer, pagesize=(layout.page_size.width, layout.page_size.height))
    spacing = convert_millimetres_to_points(config.dot_spacing_mm)
    separator_offset = convert_millimetres_to_points(config.row_padding_mm) / 2

    for row_index in range(group_size):
        row = layout.rows[row_index]
        draw_dot_grid(canvas, row.notes_area, spacing)

        if row_index < group_size - 1:
            separator_y = row.slide_area.y - separator_offset
            separator_width = row.notes_area.x + row.notes_area.width - row.slide_area.x
            draw_separator(canvas, row.slide_area.x, separator_y, separator_width)

    canvas.save()
    buffer.seek(0)

    return PdfReader(buffer).pages[0]


def draw_dot_grid(canvas: Canvas, area: Rectangle, spacing: float) -> None:
    canvas.setFillColorRGB(*DOT_COLOR)
    x = area.x + spacing / 2
    first_y = area.y + spacing / 2

    while x < area.x + area.width:
        current_y = first_y

        while current_y < area.y + area.height:
            canvas.circle(x, current_y, DOT_RADIUS_POINTS, stroke=0, fill=1)
            current_y += spacing

        x += spacing


def draw_separator(canvas: Canvas, x: float, y: float, width: float) -> None:
    canvas.setStrokeColorRGB(*SEPARATOR_COLOR)
    canvas.setLineWidth(SEPARATOR_WIDTH_POINTS)
    canvas.setDash(SEPARATOR_DASH_LENGTH, SEPARATOR_DASH_LENGTH)
    canvas.line(x, y, x + width, y)
    canvas.setDash()


def optimize_document(writer: PdfWriter) -> None:
    for page in writer.pages:
        page.compress_content_streams()

    writer.compress_identical_objects(
        remove_duplicates=True,
        remove_unreferenced=True,
    )
