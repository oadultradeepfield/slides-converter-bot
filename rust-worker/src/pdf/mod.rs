mod annotations;
mod drawing;
mod layout;
mod objects;

use lopdf::{Dictionary, Document, Object, ObjectId, dictionary};
use thiserror::Error;

use annotations::Destinations;
use layout::{Geometry, PAGE_HEIGHT, PAGE_WIDTH, Placement, ROWS};

struct Slide {
    source: ObjectId,
    target: ObjectId,
    geometry: Geometry,
    placement: Placement,
}

struct PlannedPage {
    id: ObjectId,
    slides: Vec<Slide>,
}

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Input PDF exceeds 20 MB")]
    InputTooLarge,
    #[error("Encrypted PDFs are not supported")]
    Encrypted,
    #[error("PDF contains no pages")]
    Empty,
    #[error("PDF contains an invalid page rectangle or rotation")]
    InvalidGeometry,
    #[error("PDF page content exceeds the Worker processing limit")]
    ContentTooLarge,
    #[error("Processed PDF exceeds 50 MB")]
    OutputTooLarge,
    #[error("Unsupported PDF annotation; please flatten annotations first")]
    Annotation,
    #[error("PDF link destination cannot be resolved")]
    Destination,
    #[error("Could not process PDF: {0}")]
    Pdf(#[from] lopdf::Error),
    #[error("Could not write PDF: {0}")]
    Write(#[from] std::io::Error),
}

pub const MAX_INPUT_BYTES: usize = 20_000_000;
pub const MAX_OUTPUT_BYTES: usize = 50_000_000;

/// Compose three vector slides beside dotted notes on each A4 page.
pub fn convert_pdf(input: Vec<u8>) -> Result<Vec<u8>, ConversionError> {
    let mut document = objects::load_document(input)?;
    let pages = plan_pages(&mut document)?;
    let destinations = Destinations::read(&document, &pages)?;
    compose_document(&mut document, &pages, &destinations)?;
    serialize_document(document)
}

fn plan_pages(document: &mut Document) -> Result<Vec<PlannedPage>, ConversionError> {
    let sources: Vec<_> = document.get_pages().into_values().collect();
    if sources.is_empty() {
        return Err(ConversionError::Empty);
    }
    sources
        .chunks(ROWS)
        .map(|group| plan_page(document, group))
        .collect()
}

fn plan_page(
    document: &mut Document,
    sources: &[ObjectId],
) -> Result<PlannedPage, ConversionError> {
    let id = document.new_object_id();
    let slides = sources
        .iter()
        .enumerate()
        .map(|(row, &source)| {
            let geometry = objects::read_geometry(document, source)?;
            Ok(Slide {
                source,
                target: id,
                geometry,
                placement: layout::place_slide(&geometry, row),
            })
        })
        .collect::<Result<_, ConversionError>>()?;
    Ok(PlannedPage { id, slides })
}

fn compose_document(
    document: &mut Document,
    pages: &[PlannedPage],
    destinations: &Destinations,
) -> Result<(), ConversionError> {
    let parent = document.new_object_id();
    let mut grid = lopdf::xobject::form(
        vec![0.0, 0.0, layout::NOTES_WIDTH, layout::NOTES_HEIGHT],
        vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        drawing::draw_grid(),
    );
    grid.dict.set("Resources", Dictionary::new());
    let grid_id = document.add_object(grid);
    for page in pages {
        compose_page(document, page, (parent, grid_id), destinations)?;
    }
    document.objects.insert(
        parent,
        dictionary! {
            "Type" => "Pages", "Count" => pages.len() as i64,
            "Kids" => pages.iter().map(|p| Object::Reference(p.id)).collect::<Vec<_>>(),
        }
        .into(),
    );
    let catalog = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => parent });
    document.trailer = dictionary! { "Root" => catalog };
    document.prune_objects();
    Ok(())
}

fn compose_page(
    document: &mut Document,
    page: &PlannedPage,
    resources: (ObjectId, ObjectId),
    destinations: &Destinations,
) -> Result<(), ConversionError> {
    let (parent, grid) = resources;
    let mut forms = dictionary! { "Grid" => grid };
    let mut annotations = Vec::new();
    for (row, slide) in page.slides.iter().enumerate() {
        let form = objects::create_slide_form(document, slide)?;
        forms.set(format!("Slide{row}"), document.add_object(form));
        annotations.extend(annotations::read_annotations(
            document,
            slide,
            destinations,
        )?);
    }
    let annotations: Vec<_> = annotations
        .into_iter()
        .map(|a| Object::Reference(document.add_object(a)))
        .collect();
    let stream = document.add_object(objects::compress_stream(drawing::draw_page(page))?);
    let mut page_dict = dictionary! {
        "Type" => "Page", "Parent" => parent,
        "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()],
        "Resources" => dictionary! { "XObject" => forms }, "Contents" => stream,
    };
    if !annotations.is_empty() {
        page_dict.set("Annots", annotations);
    }
    document.objects.insert(page.id, page_dict.into());
    Ok(())
}

fn serialize_document(mut document: Document) -> Result<Vec<u8>, ConversionError> {
    let mut output = Vec::new();
    document.save_to(&mut output)?;
    if output.len() > MAX_OUTPUT_BYTES {
        return Err(ConversionError::OutputTooLarge);
    }
    Ok(output)
}
