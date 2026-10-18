use std::collections::{HashMap, HashSet};

use lopdf::{Dictionary, Document, Object, ObjectId};

use super::{ConversionError, PlannedPage, Slide, layout::Placement, objects::read_rectangle};

pub(super) struct Destinations {
    pages: HashMap<ObjectId, (ObjectId, Placement)>,
    names: HashMap<Vec<u8>, Object>,
}

const MAX_NAME_TREE_NODES: usize = 10000;

impl Destinations {
    pub fn read(document: &Document, pages: &[PlannedPage]) -> Result<Self, ConversionError> {
        let targets = pages
            .iter()
            .flat_map(|p| &p.slides)
            .map(|s| (s.source, (s.target, s.placement)))
            .collect();
        Ok(Self {
            pages: targets,
            names: read_named_destinations(document)?,
        })
    }

    fn remap(&self, document: &Document, value: &Object) -> Result<Object, ConversionError> {
        let value = document.dereference(value)?.1;
        let destination = match value {
            Object::Name(name) | Object::String(name, _) => {
                self.names.get(name).ok_or(ConversionError::Destination)?
            }
            other => other,
        };
        let destination = document.dereference(destination)?.1;
        let array = match destination {
            Object::Dictionary(dict) => document.dereference(dict.get(b"D")?)?.1.as_array()?,
            other => other.as_array()?,
        };
        let source = array
            .first()
            .ok_or(ConversionError::Destination)?
            .as_reference()?;
        let (page, area) = self
            .pages
            .get(&source)
            .ok_or(ConversionError::Destination)?;
        // Fit the linked slide, including targets sharing the same A4 page.
        Ok(vec![
            Object::Reference(*page),
            Object::Name(b"FitR".to_vec()),
            area.x.into(),
            area.y.into(),
            (area.x + area.width).into(),
            (area.y + area.height).into(),
        ]
        .into())
    }
}

pub(super) fn read_annotations(
    document: &Document,
    slide: &Slide,
    destinations: &Destinations,
) -> Result<Vec<Dictionary>, ConversionError> {
    document
        .get_page_annotations(slide.source)?
        .into_iter()
        .map(|annotation| transform_link(document, annotation.clone(), slide, destinations))
        .collect()
}

fn transform_link(
    document: &Document,
    mut link: Dictionary,
    slide: &Slide,
    destinations: &Destinations,
) -> Result<Dictionary, ConversionError> {
    if link.get(b"Subtype")?.as_name()? != b"Link" {
        return Err(ConversionError::Annotation);
    }
    if let Ok(destination) = link.get(b"Dest") {
        link.set("Dest", destinations.remap(document, destination)?);
    }
    if let Ok(action) = link.get(b"A") {
        let mut action = document.dereference(action)?.1.as_dict()?.clone();
        if action.get(b"S")?.as_name()? == b"GoTo" {
            action.set("D", destinations.remap(document, action.get(b"D")?)?);
            link.set("A", action);
        }
    }
    let rect = read_rectangle(document, link.get(b"Rect")?)?;
    let first = slide.geometry.transform(rect[0], rect[1]);
    let second = slide.geometry.transform(rect[2], rect[3]);
    let Placement { scale, x, y, .. } = slide.placement;
    link.set(
        "Rect",
        vec![
            (x + first.0.min(second.0) * scale).into(),
            (y + first.1.min(second.1) * scale).into(),
            (x + first.0.max(second.0) * scale).into(),
            (y + first.1.max(second.1) * scale).into(),
        ],
    );
    link.set("P", slide.target);
    link.remove(b"QuadPoints");
    Ok(link)
}

fn read_named_destinations(
    document: &Document,
) -> Result<HashMap<Vec<u8>, Object>, ConversionError> {
    let catalog = document.catalog()?;
    let mut destinations = document
        .get_dict_in_dict(catalog, b"Dests")
        .map(|dict| {
            dict.iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default();
    let tree = document
        .get_dict_in_dict(catalog, b"Names")
        .and_then(|names| names.get(b"Dests"));
    let Ok(tree) = tree else {
        return Ok(destinations);
    };
    collect_name_tree(document, tree, &mut destinations)?;
    Ok(destinations)
}

fn collect_name_tree(
    document: &Document,
    root: &Object,
    destinations: &mut HashMap<Vec<u8>, Object>,
) -> Result<(), ConversionError> {
    let mut pending = vec![root.clone()];
    let mut visited = HashSet::new();
    for _ in 0..MAX_NAME_TREE_NODES {
        let Some(node) = pending.pop() else {
            return Ok(());
        };
        let (id, node) = document.dereference(&node)?;
        if id.is_some_and(|id| !visited.insert(id)) {
            return Err(ConversionError::Destination);
        }
        let node = node.as_dict()?;
        if let Ok(entries) = node.get(b"Names") {
            for pair in document.dereference(entries)?.1.as_array()?.chunks_exact(2) {
                destinations.insert(pair[0].as_str()?.to_vec(), pair[1].clone());
            }
        }
        if let Ok(kids) = node.get(b"Kids") {
            pending.extend(document.dereference(kids)?.1.as_array()?.iter().cloned());
        }
    }
    Err(ConversionError::Destination)
}
