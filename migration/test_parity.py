"""Temporary migration checks: compare the Rust output with the Python oracle."""

import io
import random
import subprocess
from pathlib import Path

import pytest
import reportlab
from PIL import Image, ImageChops, ImageStat
from pypdf import PdfReader, PdfWriter
from pypdf.annotations import Text
from pypdf.generic import (
    ArrayObject,
    DecodedStreamObject,
    DictionaryObject,
    FloatObject,
    IndirectObject,
    NameObject,
    NumberObject,
    TextStringObject,
)
from reportlab.lib.utils import ImageReader
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen.canvas import Canvas

from slides_converter_bot.converter import convert_pdf

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "rust-worker/target/release/examples/convert"
SLIDE_SIZE = (960.0, 540.0)
FULL_RENDER_MAX_SLIDES = 4
MAX_MEAN_PIXEL_DIFFERENCE = 0.5


def create_deck(path: Path, pages: int, *, images: bool = False) -> None:
    canvas = Canvas(str(path), pagesize=SLIDE_SIZE, pageCompression=1)
    generator = random.Random(17)

    for index in range(pages):
        canvas.setFillColorRGB(0.9, 0.94, 1)
        canvas.rect(0, 0, *SLIDE_SIZE, fill=1, stroke=0)
        canvas.setFillColorRGB(0.1, 0.2, 0.4)
        canvas.setFont("Helvetica-Bold", 32)
        canvas.drawString(55, 460, f"Slide {index + 1}: PDF migration")
        canvas.setFont("Times-Roman", 18)
        canvas.drawString(
            55, 410, "Vectors, fonts, clipping, transparency and hyperlinks"
        )
        canvas.linkURL("https://example.com/", (55, 400, 700, 430))
        canvas.setFillAlpha(0.4)
        canvas.setFillColorRGB(0.9, 0.2, 0.1)
        canvas.circle(220, 220, 100, fill=1, stroke=0)
        canvas.setFillAlpha(1)
        canvas.setStrokeColorRGB(0.1, 0.5, 0.1)
        canvas.bezier(360, 70, 400, 400, 600, 10, 850, 300)

        if images:
            image = Image.frombytes(
                "RGB", (400, 250), generator.randbytes(400 * 250 * 3)
            )
            buffer = io.BytesIO()
            image.save(buffer, format="JPEG", quality=80)
            buffer.seek(0)
            canvas.drawImage(ImageReader(buffer), 500, 60, width=360, height=225)

        canvas.showPage()

    canvas.save()


def run_rust(source: Path, output: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(BINARY), str(source), str(output)],
        capture_output=True,
        text=True,
        check=False,
    )


def render_page(path: Path, page: int, output: Path) -> Image.Image:
    subprocess.run(
        [
            "pdftoppm",
            "-f",
            str(page),
            "-l",
            str(page),
            "-scale-to",
            "1200",
            "-singlefile",
            "-png",
            str(path),
            str(output),
        ],
        check=True,
        capture_output=True,
    )
    return Image.open(output.with_suffix(".png")).convert("RGB")


@pytest.mark.parametrize(
    ("pages", "images"),
    [(1, False), (2, False), (3, False), (4, False), (30, False), (10, True)],
)
def test_python_and_rust_render_the_same_slides(
    tmp_path: Path, pages: int, images: bool
) -> None:
    source, python, rust = (
        tmp_path / name for name in ("source.pdf", "python.pdf", "rust.pdf")
    )
    create_deck(source, pages, images=images)
    convert_pdf(source, python)
    result = run_rust(source, rust)
    assert result.returncode == 0, result.stderr
    expected, actual = PdfReader(python), PdfReader(rust)
    assert len(actual.pages) == len(expected.pages) == (pages + 2) // 3

    for index, (expected_page, actual_page) in enumerate(
        zip(expected.pages, actual.pages, strict=True)
    ):
        assert [float(v) for v in actual_page.mediabox] == pytest.approx(
            [float(v) for v in expected_page.mediabox], abs=0.001
        )
        assert (
            actual_page.extract_text().split() == expected_page.extract_text().split()
        )
        assert len(actual_page.get("/Annots", [])) == len(
            expected_page.get("/Annots", [])
        )

        if pages <= FULL_RENDER_MAX_SLIDES or index in (0, len(actual.pages) - 1):
            expected_image = render_page(python, index + 1, tmp_path / f"py-{index}")
            actual_image = render_page(rust, index + 1, tmp_path / f"rs-{index}")
            difference = ImageStat.Stat(
                ImageChops.difference(expected_image, actual_image)
            )
            assert max(difference.mean) < MAX_MEAN_PIXEL_DIFFERENCE, (
                index,
                difference.mean,
            )


@pytest.mark.parametrize("rotation", [0, 90, 180, 270])
def test_rotated_pages_keep_all_visible_text(tmp_path: Path, rotation: int) -> None:
    source, output = tmp_path / "source.pdf", tmp_path / "rust.pdf"
    create_deck(source, 1)
    writer = PdfWriter(source)
    writer.pages[0].rotate(rotation)
    writer.write(source)
    result = run_rust(source, output)
    assert result.returncode == 0, result.stderr
    assert "Slide 1" in PdfReader(output).pages[0].extract_text()
    normalized = PdfWriter(source)
    normalized.pages[0].transfer_rotation_to_content()
    normalized_source = tmp_path / "normalized.pdf"
    normalized.write(normalized_source)
    expected_pdf = tmp_path / "expected.pdf"
    convert_pdf(normalized_source, expected_pdf)
    expected = render_page(expected_pdf, 1, tmp_path / "expected")
    actual = render_page(output, 1, tmp_path / "rotation")
    assert (
        max(ImageStat.Stat(ImageChops.difference(expected, actual)).mean)
        < MAX_MEAN_PIXEL_DIFFERENCE
    )


def test_unsupported_annotations_are_rejected_instead_of_disappearing(
    tmp_path: Path,
) -> None:
    source = tmp_path / "annotated.pdf"
    writer = PdfWriter()
    writer.add_blank_page(width=960, height=540)
    writer.add_annotation(0, Text(rect=(100, 100, 200, 200), text="A visible comment"))
    writer.write(source)
    result = run_rust(source, tmp_path / "output.pdf")
    assert result.returncode != 0
    assert "flatten annotations first" in result.stderr


def test_inherited_resources_and_nonzero_page_origin(tmp_path: Path) -> None:
    source, output = tmp_path / "source.pdf", tmp_path / "rust.pdf"
    create_deck(source, 1)
    writer = PdfWriter(source)
    page = writer.pages[0]
    parent = page["/Parent"].get_object()
    assert isinstance(parent, DictionaryObject)
    parent[NameObject("/Resources")] = page["/Resources"]
    del page["/Resources"]
    parent[NameObject("/MediaBox")] = ArrayObject(
        [FloatObject(-20), FloatObject(-10), FloatObject(960), FloatObject(540)]
    )
    del page["/MediaBox"]
    writer.write(source)
    result = run_rust(source, output)
    assert result.returncode == 0, result.stderr
    assert "Slide 1" in PdfReader(output).pages[0].extract_text()


@pytest.mark.parametrize(
    "kind",
    [
        "malformed",
        "empty",
        "encrypted",
        "empty-password",
        "oversized",
        "invalid-geometry",
        "bad-filter",
        "inflated-content",
    ],
)
def test_invalid_pdfs_are_rejected(tmp_path: Path, kind: str) -> None:
    source = tmp_path / "source.pdf"
    writer = PdfWriter()

    if kind == "malformed":
        source.write_bytes(b"not a pdf")
    elif kind == "oversized":
        source.write_bytes(b" " * 20_000_001)
    else:
        if kind != "empty":
            writer.add_blank_page(width=960, height=540)
        if kind in ("encrypted", "empty-password"):
            writer.encrypt("test-password" if kind == "encrypted" else "")
        if kind == "invalid-geometry":
            writer.pages[0][NameObject("/Rotate")] = NumberObject(45)
        if kind in ("bad-filter", "inflated-content"):
            content = DecodedStreamObject()
            content.set_data(b"q Q\n" if kind == "bad-filter" else b" " * 8_000_001)
            if kind == "bad-filter":
                content[NameObject("/Filter")] = NameObject("/UnknownFilter")
            stored_content = content if kind == "bad-filter" else content.flate_encode()
            writer.pages[0][NameObject("/Contents")] = writer._add_object(
                stored_content
            )
        writer.write(source)

    result = run_rust(source, tmp_path / "output.pdf")
    assert result.returncode != 0


@pytest.mark.parametrize("named", [False, True])
def test_internal_links_target_the_correct_output_page(
    tmp_path: Path, named: bool
) -> None:
    source, output = tmp_path / "source.pdf", tmp_path / "rust.pdf"
    canvas = Canvas(str(source), pagesize=SLIDE_SIZE)

    for page in range(4):
        canvas.bookmarkPage(f"slide-{page}")
        canvas.drawString(60, 460, f"Slide {page + 1}")
        canvas.linkRect("jump", "slide-3", (50, 400, 200, 450), relative=0)
        canvas.showPage()

    canvas.save()
    if named:
        writer = PdfWriter(source)
        writer.add_named_destination("last-slide", 3)
        annotations = writer.pages[0]["/Annots"]
        assert isinstance(annotations, ArrayObject)
        annotation = annotations[0].get_object()
        assert isinstance(annotation, DictionaryObject)
        annotation[NameObject("/Dest")] = TextStringObject("last-slide")
        writer.write(source)
    result = run_rust(source, output)
    assert result.returncode == 0, result.stderr
    actual = PdfReader(output)
    annotations = actual.pages[0]["/Annots"]
    assert isinstance(annotations, ArrayObject)
    link = annotations[0].get_object()
    assert isinstance(link, DictionaryObject)
    destination = link["/Dest"]
    assert isinstance(destination, ArrayObject)
    target = destination[0]
    assert isinstance(target, IndirectObject)
    page_reference = actual.pages[1].indirect_reference
    assert page_reference is not None
    assert target.idnum == page_reference.idnum


def test_embedded_unicode_font_cropbox_and_multiple_content_streams(
    tmp_path: Path,
) -> None:
    source = tmp_path / "source.pdf"
    font_path = Path(reportlab.__file__).parent / "fonts/Vera.ttf"
    pdfmetrics.registerFont(TTFont("MigrationVera", str(font_path)))
    canvas = Canvas(str(source), pagesize=SLIDE_SIZE)
    canvas.setFont("MigrationVera", 32)
    canvas.drawString(100, 400, "Unicode: café naïve résumé")
    canvas.showPage()
    canvas.save()
    writer = PdfWriter(source)
    page = writer.pages[0]
    contents = page.get_contents()
    assert contents is not None
    data = contents.get_data()
    first, _, second = data.partition(b"\n")
    pieces = []
    for content in (first + b"\n", second):
        stream = DecodedStreamObject()
        stream.set_data(content)
        pieces.append(writer._add_object(stream))
    page[NameObject("/Contents")] = ArrayObject(pieces)
    page.cropbox.lower_left = (40, 40)
    page.cropbox.upper_right = (920, 500)
    writer.write(source)
    python, rust = tmp_path / "python.pdf", tmp_path / "rust.pdf"
    convert_pdf(source, python)
    result = run_rust(source, rust)
    assert result.returncode == 0, result.stderr
    assert "café naïve résumé" in PdfReader(rust).pages[0].extract_text()
    expected = render_page(python, 1, tmp_path / "python")
    actual = render_page(rust, 1, tmp_path / "rust")
    assert (
        max(ImageStat.Stat(ImageChops.difference(expected, actual)).mean)
        < MAX_MEAN_PIXEL_DIFFERENCE
    )
