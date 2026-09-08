from slides_converter_bot.telegram import build_output_name


def test_output_name_removes_directories() -> None:
    assert build_output_name("../../lecture.pdf") == "lecture-notes.pdf"


def test_output_name_has_fallback() -> None:
    assert build_output_name(".pdf") == "slides-notes.pdf"
