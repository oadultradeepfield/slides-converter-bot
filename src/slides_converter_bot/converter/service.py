from pathlib import Path

from pypdf import PdfReader, PdfWriter
from pypdf.errors import PdfReadError

from .models import (
    DEFAULT_CONFIG,
    ConversionConfig,
    ConversionError,
    ConversionReport,
)
from .render import compose_document

PDF_SUFFIX = ".pdf"
PDF_WRITE_MODE = "wb"


def convert_pdf(
    input_path: Path,
    output_path: Path,
    config: ConversionConfig = DEFAULT_CONFIG,
) -> ConversionReport:
    """Convert a slide PDF into A4 pages with note-taking areas."""
    validate_paths(input_path, output_path)
    reader = read_pdf(input_path)
    writer = compose_document(reader, config)
    write_pdf(writer, output_path)

    return ConversionReport(len(reader.pages), len(writer.pages), output_path)


def validate_paths(input_path: Path, output_path: Path) -> None:
    if not input_path.is_file():
        raise ConversionError(f"Input PDF does not exist: {input_path}")

    if input_path.suffix.lower() != PDF_SUFFIX:
        raise ConversionError("Input file must have a .pdf extension")

    if output_path.resolve() == input_path.resolve():
        raise ConversionError("Output PDF must be different from input PDF")


def read_pdf(input_path: Path) -> PdfReader:
    try:
        reader = PdfReader(input_path)

        if reader.is_encrypted:
            raise ConversionError("Encrypted PDFs are not supported")

        if not reader.pages:
            raise ConversionError("Input PDF contains no pages")
    except ConversionError:
        raise
    except (OSError, PdfReadError) as error:
        raise ConversionError("Could not read input PDF") from error

    return reader


def write_pdf(writer: PdfWriter, output_path: Path) -> None:
    try:
        output_path.parent.mkdir(parents=True, exist_ok=True)

        with output_path.open(PDF_WRITE_MODE) as handle:
            writer.write(handle)
    except OSError as error:
        raise ConversionError("Could not write output PDF") from error
