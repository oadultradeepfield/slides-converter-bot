from dataclasses import dataclass
from pathlib import Path


class ConversionError(Exception):
    """Expected PDF conversion failure."""


@dataclass(frozen=True, slots=True)
class ConversionConfig:
    page_width_mm: float = 210.0
    page_height_mm: float = 297.0
    margin_mm: float = 10.0
    rows_per_page: int = 3
    slide_width_ratio: float = 0.64
    slide_notes_gap_mm: float = 6.0
    row_padding_mm: float = 5.0
    dot_spacing_mm: float = 5.0


@dataclass(frozen=True, slots=True)
class PageSize:
    width: float
    height: float


@dataclass(frozen=True, slots=True)
class Rectangle:
    x: float
    y: float
    width: float
    height: float


@dataclass(frozen=True, slots=True)
class RowLayout:
    slide_area: Rectangle
    notes_area: Rectangle


@dataclass(frozen=True, slots=True)
class PageLayout:
    page_size: PageSize
    rows: tuple[RowLayout, ...]


@dataclass(frozen=True, slots=True)
class ConversionReport:
    source_pages: int
    output_pages: int
    output_path: Path


DEFAULT_CONFIG = ConversionConfig()
