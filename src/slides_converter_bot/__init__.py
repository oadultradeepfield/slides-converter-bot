"""Convert slide PDFs into annotation-friendly notes."""

from .converter import ConversionError, ConversionReport, convert_pdf

__all__ = ["ConversionError", "ConversionReport", "convert_pdf"]
