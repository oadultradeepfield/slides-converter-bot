from .models import ConversionConfig, PageLayout, PageSize, Rectangle, RowLayout

MM_TO_POINTS = 72.0 / 25.4


def calculate_page_layout(config: ConversionConfig) -> PageLayout:
    page = PageSize(
        convert_millimetres_to_points(config.page_width_mm),
        convert_millimetres_to_points(config.page_height_mm),
    )
    margin = convert_millimetres_to_points(config.margin_mm)
    gap = convert_millimetres_to_points(config.slide_notes_gap_mm)
    padding = convert_millimetres_to_points(config.row_padding_mm)
    usable_width = page.width - 2 * margin
    content_width = usable_width - gap
    slide_width = content_width * config.slide_width_ratio
    notes_width = content_width - slide_width
    row_height = (page.height - 2 * margin) / config.rows_per_page
    rows = tuple(
        build_row_layout(
            index,
            page=page,
            margin=margin,
            row_height=row_height,
            padding=padding,
            slide_width=slide_width,
            notes_width=notes_width,
            gap=gap,
        )
        for index in range(config.rows_per_page)
    )

    return PageLayout(page, rows)


def build_row_layout(
    index: int,
    *,
    page: PageSize,
    margin: float,
    row_height: float,
    padding: float,
    slide_width: float,
    notes_width: float,
    gap: float,
) -> RowLayout:
    row_top = page.height - margin - index * row_height
    content_y = row_top - row_height + padding
    content_height = row_height - 2 * padding
    slide_area = Rectangle(margin, content_y, slide_width, content_height)
    notes_area = Rectangle(
        margin + slide_width + gap,
        content_y,
        notes_width,
        content_height,
    )

    return RowLayout(slide_area, notes_area)


def group_slide_indices(page_count: int, per_page: int) -> tuple[tuple[int, ...], ...]:
    return tuple(
        tuple(range(start, min(start + per_page, page_count)))
        for start in range(0, page_count, per_page)
    )


def fit_page_inside_rectangle(source: PageSize, target: Rectangle) -> Rectangle:
    scale = min(target.width / source.width, target.height / source.height)
    width = source.width * scale
    height = source.height * scale

    return Rectangle(
        target.x + (target.width - width) / 2,
        target.y + (target.height - height) / 2,
        width,
        height,
    )


def convert_millimetres_to_points(value: float) -> float:
    return value * MM_TO_POINTS
