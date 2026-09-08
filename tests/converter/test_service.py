from pathlib import Path

import pytest
from pypdf import PdfReader
from reportlab.pdfgen.canvas import Canvas

from slides_converter_bot.converter import (
    ConversionError,
    convert_millimetres_to_points,
    convert_pdf,
    group_slide_indices,
)

SLIDE_SIZE = (960.0, 540.0)


def test_four_slides_become_two_a4_note_pages(tmp_path: Path) -> None:
    input_path = tmp_path / "slides.pdf"
    output_path = tmp_path / "notes.pdf"
    create_slide_pdf(input_path, page_count=4)

    report = convert_pdf(input_path, output_path)
    output = PdfReader(output_path)

    assert report.source_pages == 4
    assert report.output_pages == 2
    assert len(output.pages) == 2
    assert float(output.pages[0].mediabox.width) == pytest.approx(
        convert_millimetres_to_points(210.0)
    )
    assert float(output.pages[0].mediabox.height) == pytest.approx(
        convert_millimetres_to_points(297.0)
    )
    assert "Slide 1" in output.pages[0].extract_text()
    assert "Slide 4" in output.pages[1].extract_text()


def test_malformed_pdf_is_rejected(tmp_path: Path) -> None:
    input_path = tmp_path / "broken.pdf"
    input_path.write_text("not a pdf")

    with pytest.raises(ConversionError, match="Could not read input PDF"):
        convert_pdf(input_path, tmp_path / "notes.pdf")


def test_slide_indices_are_grouped_without_empty_pages() -> None:
    assert group_slide_indices(0, 3) == ()
    assert group_slide_indices(4, 3) == ((0, 1, 2), (3,))


def create_slide_pdf(path: Path, page_count: int) -> None:
    canvas = Canvas(str(path), pagesize=SLIDE_SIZE)

    for page_number in range(1, page_count + 1):
        canvas.drawString(72, 468, f"Slide {page_number}")
        canvas.showPage()

    canvas.save()
