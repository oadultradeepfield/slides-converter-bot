use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

use super::{ConversionError, Slide, layout::Geometry};

const MAX_CONTENT_BYTES: usize = 8_000_000;
const MAX_PAGE_TREE_DEPTH: usize = 128;

pub(super) fn load_document(input: Vec<u8>) -> Result<Document, ConversionError> {
    if input.len() > super::MAX_INPUT_BYTES {
        return Err(ConversionError::InputTooLarge);
    }
    let document = Document::load_mem_with_options(
        &input,
        lopdf::LoadOptions::with_max_decompressed_size(MAX_CONTENT_BYTES),
    )?;
    if document.is_encrypted() || document.was_encrypted() {
        return Err(ConversionError::Encrypted);
    }
    Ok(document)
}

pub(super) fn read_geometry(
    document: &Document,
    page: ObjectId,
) -> Result<Geometry, ConversionError> {
    let media = read_rectangle(
        document,
        &inherited(document, page, b"MediaBox")?.ok_or(ConversionError::InvalidGeometry)?,
    )?;
    let crop = inherited(document, page, b"CropBox")?
        .map(|v| read_rectangle(document, &v))
        .transpose()?
        .unwrap_or(media);
    let rotation = inherited(document, page, b"Rotate")?
        .map(|v| document.dereference(&v).and_then(|(_, v)| v.as_i64()))
        .transpose()?
        .unwrap_or(0);
    Geometry::new(media, crop, rotation)
}

pub(super) fn create_slide_form(
    document: &Document,
    slide: &Slide,
) -> Result<Stream, ConversionError> {
    let mut form = lopdf::xobject::form(
        slide.geometry.crop.to_vec(),
        slide.geometry.matrix().to_vec(),
        read_content(document, slide.source)?,
    );
    form.dict.set(
        "Resources",
        inherited(document, slide.source, b"Resources")?
            .unwrap_or_else(|| Dictionary::new().into()),
    );
    if let Ok(group) = document.get_dictionary(slide.source)?.get(b"Group") {
        form.dict.set("Group", group.clone());
    }
    Ok(form)
}

pub(super) fn compress_stream(content: Vec<u8>) -> Result<Stream, ConversionError> {
    let mut stream = Stream::new(Dictionary::new(), content);
    stream.compress()?;
    Ok(stream)
}

pub(super) fn read_rectangle(
    document: &Document,
    value: &Object,
) -> Result<[f32; 4], ConversionError> {
    let values = document.dereference(value)?.1.as_array()?;
    if values.len() != 4 {
        return Err(ConversionError::InvalidGeometry);
    }
    let mut rectangle = [0.0; 4];
    for (index, value) in values.iter().enumerate() {
        rectangle[index] = document.dereference(value)?.1.as_float()?;
    }
    if rectangle.iter().any(|v| !v.is_finite())
        || rectangle[2] <= rectangle[0]
        || rectangle[3] <= rectangle[1]
    {
        return Err(ConversionError::InvalidGeometry);
    }
    Ok(rectangle)
}

fn inherited(
    document: &Document,
    mut page: ObjectId,
    key: &[u8],
) -> Result<Option<Object>, ConversionError> {
    for _ in 0..MAX_PAGE_TREE_DEPTH {
        let dict = document.get_dictionary(page)?;
        if let Ok(value) = dict.get(key) {
            return Ok(Some(value.clone()));
        }
        match dict.get(b"Parent") {
            Ok(parent) => page = parent.as_reference()?,
            Err(_) => return Ok(None),
        }
    }
    Err(ConversionError::InvalidGeometry)
}

fn read_content(document: &Document, page: ObjectId) -> Result<Vec<u8>, ConversionError> {
    let mut content = Vec::new();
    for id in document.get_page_contents(page) {
        let stream = document.get_object(id)?.as_stream()?;
        let remaining = MAX_CONTENT_BYTES.saturating_sub(content.len());
        let bytes = if stream.dict.get(b"Filter").is_ok() {
            stream.decompressed_content_with_limit(remaining)?
        } else {
            if stream.content.len() >= remaining {
                return Err(ConversionError::ContentTooLarge);
            }
            stream.content.clone()
        };
        if bytes.len() >= remaining {
            return Err(ConversionError::ContentTooLarge);
        }
        content.extend_from_slice(&bytes);
        content.push(b'\n');
    }
    Ok(content)
}
