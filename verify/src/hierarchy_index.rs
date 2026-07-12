//! Hierarchy-preserving spatial queries and deterministic tile ownership.
//!
//! The index retains one local shape table per GDS structure. Queries walk SREF/AREF
//! nodes and use cached cell bboxes only as conservative rejects; every returned
//! candidate includes its exact transformed ring and stable hierarchy identity.

use crate::gds_lossless::{
    checked_array_coordinate, ensure_meta_supported, exact_pitch, stroke_path, validate_hierarchy,
    Affine, GdsElement, GdsLibrary, GdsStructure, LayoutError, LayoutErrorKind,
};
use crate::geometry::exact::{Point, Ring};
use crate::geometry::Bbox;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GdsLayerIdentity {
    pub layer: i16,
    pub datatype: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexedShapeKind {
    Boundary,
    Box,
    Path,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstancePathEntry {
    pub parent_structure: String,
    pub element_index: u32,
    pub referenced_structure: String,
    pub column: u16,
    pub row: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HierarchyCandidate {
    pub structure: String,
    pub element_index: u32,
    /// PATH elements can produce multiple exact polygon parts.
    pub part_index: u32,
    pub kind: IndexedShapeKind,
    pub layer: GdsLayerIdentity,
    pub instance_path: Vec<InstancePathEntry>,
    pub ring: Vec<Point>,
    pub bbox: Bbox,
}

impl HierarchyCandidate {
    /// Stable non-cryptographic identity for marker/result deduplication.
    pub fn stable_hash(&self) -> u64 {
        let mut hash = Fnv64::new();
        hash.string(&self.structure);
        hash.u32(self.element_index);
        hash.u32(self.part_index);
        hash.u8(self.kind as u8);
        hash.i16(self.layer.layer);
        hash.i16(self.layer.datatype);
        for entry in &self.instance_path {
            hash.string(&entry.parent_structure);
            hash.u32(entry.element_index);
            hash.string(&entry.referenced_structure);
            hash.u16(entry.column);
            hash.u16(entry.row);
        }
        for point in &self.ring {
            hash.i32(point.x);
            hash.i32(point.y);
        }
        hash.finish()
    }
}

#[derive(Clone, Debug)]
pub struct HierarchyIndexOptions {
    pub max_depth: usize,
    pub max_expanded_visits_per_query: usize,
    pub max_array_instances: usize,
}

impl Default for HierarchyIndexOptions {
    fn default() -> Self {
        Self {
            max_depth: 1_024,
            max_expanded_visits_per_query: 10_000_000,
            max_array_instances: 10_000_000,
        }
    }
}

#[derive(Clone, Debug)]
struct LocalShape {
    element_index: u32,
    part_index: u32,
    kind: IndexedShapeKind,
    layer: GdsLayerIdentity,
    ring: Vec<Point>,
    bbox: Bbox,
}

#[derive(Clone, Debug)]
struct IndexedCell {
    parts_by_element: Vec<Vec<LocalShape>>,
    hierarchy_bbox: Option<Bbox>,
}

/// Spatial index borrowing the lossless library. Cell geometry is stored once;
/// instance expansions are produced only by queries.
pub struct HierarchySpatialIndex<'a> {
    library: &'a GdsLibrary,
    by_name: HashMap<&'a str, &'a GdsStructure>,
    cells: HashMap<&'a str, IndexedCell>,
    top_cells: Vec<String>,
    options: HierarchyIndexOptions,
}

impl<'a> HierarchySpatialIndex<'a> {
    pub fn build(
        library: &'a GdsLibrary,
        options: HierarchyIndexOptions,
    ) -> Result<Self, LayoutError> {
        if !library.unhandled_records.is_empty() {
            return Err(LayoutError::layout(
                LayoutErrorKind::Unsupported,
                "library contains unhandled records",
            ));
        }
        let by_name: HashMap<&str, &GdsStructure> = library
            .structures
            .iter()
            .map(|structure| (structure.name.as_str(), structure))
            .collect();
        validate_hierarchy(library, &by_name)?;

        let referenced: HashSet<&str> = library
            .structures
            .iter()
            .flat_map(|structure| structure.elements.iter())
            .filter_map(|element| match element {
                GdsElement::Sref(reference) => Some(reference.structure.as_str()),
                GdsElement::Aref(reference) => Some(reference.structure.as_str()),
                _ => None,
            })
            .collect();
        let top_cells = library
            .structures
            .iter()
            .filter(|structure| !referenced.contains(structure.name.as_str()))
            .map(|structure| structure.name.clone())
            .collect();

        let mut cells = HashMap::new();
        for structure in &library.structures {
            if !structure.unhandled_records.is_empty() {
                return Err(LayoutError::layout(
                    LayoutErrorKind::Unsupported,
                    format!("structure `{}` contains unhandled records", structure.name),
                ));
            }
            let mut parts_by_element = vec![Vec::new(); structure.elements.len()];
            for (element_index, element) in structure.elements.iter().enumerate() {
                let element_index = u32::try_from(element_index).map_err(|_| {
                    LayoutError::layout(
                        LayoutErrorKind::CapacityExceeded,
                        format!(
                            "structure `{}` has more than u32::MAX elements",
                            structure.name
                        ),
                    )
                })?;
                let parts = index_element(element, element_index)?;
                parts_by_element[element_index as usize] = parts;
            }
            cells.insert(
                structure.name.as_str(),
                IndexedCell {
                    parts_by_element,
                    hierarchy_bbox: None,
                },
            );
        }

        let mut index = Self {
            library,
            by_name,
            cells,
            top_cells,
            options,
        };
        let names: Vec<&str> = index
            .library
            .structures
            .iter()
            .map(|structure| structure.name.as_str())
            .collect();
        for name in names {
            let mut stack = Vec::new();
            index.compute_hierarchy_bbox(name, 0, &mut stack)?;
        }
        Ok(index)
    }

    pub fn top_cells(&self) -> &[String] {
        &self.top_cells
    }

    pub fn cell_bbox(&self, name: &str) -> Option<Bbox> {
        self.cells.get(name).and_then(|cell| cell.hierarchy_bbox)
    }

    /// Return exact candidates whose conservative transformed bboxes intersect
    /// `region`. Optional layer selection happens before polygon transformation.
    pub fn query(
        &self,
        top: &str,
        region: Bbox,
        layer: Option<GdsLayerIdentity>,
    ) -> Result<Vec<HierarchyCandidate>, LayoutError> {
        validate_bbox(region, "query region")?;
        let root = self.by_name.get(top).copied().ok_or_else(|| {
            LayoutError::layout(
                LayoutErrorKind::UndefinedReference,
                format!("query top `{top}` is undefined"),
            )
        })?;
        let mut output = Vec::new();
        let mut path = Vec::new();
        let mut visits = 0usize;
        self.query_structure(
            root,
            Affine::IDENTITY,
            region,
            layer,
            0,
            &mut visits,
            &mut path,
            &mut output,
        )?;
        Ok(output)
    }

    /// Hash exact candidate identity and geometry in deterministic traversal order.
    pub fn equivalent_layout_hash(&self, top: &str) -> Result<u64, LayoutError> {
        let bbox = self.cell_bbox(top).ok_or_else(|| {
            LayoutError::layout(
                LayoutErrorKind::Malformed,
                format!("top `{top}` has no area geometry"),
            )
        })?;
        let candidates = self.query(top, bbox, None)?;
        let mut hash = Fnv64::new();
        for candidate in candidates {
            hash.u64(candidate.stable_hash());
        }
        Ok(hash.finish())
    }

    fn compute_hierarchy_bbox(
        &mut self,
        name: &'a str,
        depth: usize,
        stack: &mut Vec<&'a str>,
    ) -> Result<Option<Bbox>, LayoutError> {
        if let Some(bbox) = self.cells[name].hierarchy_bbox {
            return Ok(Some(bbox));
        }
        if depth > self.options.max_depth {
            return Err(LayoutError::layout(
                LayoutErrorKind::CapacityExceeded,
                format!(
                    "hierarchy depth exceeded {} at `{name}`",
                    self.options.max_depth
                ),
            ));
        }
        if stack.contains(&name) {
            return Err(LayoutError::layout(
                LayoutErrorKind::HierarchyCycle,
                format!("cycle through `{name}`"),
            ));
        }
        stack.push(name);
        let structure = self.by_name[name];
        let mut bbox = None;
        for (element_index, element) in structure.elements.iter().enumerate() {
            for part in &self.cells[name].parts_by_element[element_index] {
                include_bbox(&mut bbox, part.bbox);
            }
            match element {
                GdsElement::Sref(reference) => {
                    let child = self.compute_hierarchy_bbox(
                        self.structure_name(&reference.structure)?,
                        depth + 1,
                        stack,
                    )?;
                    if let Some(child) = child {
                        let transform = Affine::instance(reference.transform, reference.origin)?;
                        include_bbox(&mut bbox, transform_bbox(transform, child)?);
                    }
                }
                GdsElement::Aref(reference) => {
                    let count = usize::from(reference.columns)
                        .checked_mul(usize::from(reference.rows))
                        .ok_or_else(|| {
                            LayoutError::layout(
                                LayoutErrorKind::ArithmeticOverflow,
                                "AREF instance count overflow",
                            )
                        })?;
                    if count > self.options.max_array_instances {
                        return Err(LayoutError::layout(
                            LayoutErrorKind::CapacityExceeded,
                            format!(
                                "AREF has {count} instances; limit is {}",
                                self.options.max_array_instances
                            ),
                        ));
                    }
                    let child = self.compute_hierarchy_bbox(
                        self.structure_name(&reference.structure)?,
                        depth + 1,
                        stack,
                    )?;
                    if let Some(child) = child {
                        let pitches = array_pitches(reference)?;
                        // A linear array's extrema are attained at its four corner origins.
                        for column in [0, reference.columns - 1] {
                            for row in [0, reference.rows - 1] {
                                let origin = array_origin(reference.origin, pitches, column, row)?;
                                let transform = Affine::instance(reference.transform, origin)?;
                                include_bbox(&mut bbox, transform_bbox(transform, child)?);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        stack.pop();
        self.cells.get_mut(name).unwrap().hierarchy_bbox = bbox;
        Ok(bbox)
    }

    fn structure_name(&self, name: &str) -> Result<&'a str, LayoutError> {
        self.by_name
            .get_key_value(name)
            .map(|(stored, _)| *stored)
            .ok_or_else(|| {
                LayoutError::layout(
                    LayoutErrorKind::UndefinedReference,
                    format!("reference to undefined `{name}`"),
                )
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn query_structure(
        &self,
        structure: &'a GdsStructure,
        transform: Affine,
        region: Bbox,
        layer: Option<GdsLayerIdentity>,
        depth: usize,
        visits: &mut usize,
        path: &mut Vec<InstancePathEntry>,
        output: &mut Vec<HierarchyCandidate>,
    ) -> Result<(), LayoutError> {
        if depth > self.options.max_depth {
            return Err(LayoutError::layout(
                LayoutErrorKind::CapacityExceeded,
                format!("hierarchy query depth exceeded {}", self.options.max_depth),
            ));
        }
        if let Some(local_bbox) = self.cells[structure.name.as_str()].hierarchy_bbox {
            let world_bbox = transform_bbox(transform, local_bbox)?;
            if !bbox_intersects(world_bbox, region) {
                return Ok(());
            }
        }
        for (element_index, element) in structure.elements.iter().enumerate() {
            *visits = visits.checked_add(1).ok_or_else(|| {
                LayoutError::layout(
                    LayoutErrorKind::ArithmeticOverflow,
                    "query visit counter overflow",
                )
            })?;
            if *visits > self.options.max_expanded_visits_per_query {
                return Err(LayoutError::layout(
                    LayoutErrorKind::CapacityExceeded,
                    format!(
                        "query exceeded {} expanded visits",
                        self.options.max_expanded_visits_per_query
                    ),
                ));
            }
            for part in &self.cells[structure.name.as_str()].parts_by_element[element_index] {
                if layer.is_some_and(|wanted| wanted != part.layer) {
                    continue;
                }
                let conservative = transform_bbox(transform, part.bbox)?;
                if !bbox_intersects(conservative, region) {
                    continue;
                }
                let mut ring = part
                    .ring
                    .iter()
                    .copied()
                    .map(|point| transform.apply(point))
                    .collect::<Result<Vec<_>, _>>()?;
                if signed_area2(&ring) < 0 {
                    ring.reverse();
                }
                Ring::new(ring.clone()).map_err(|error| {
                    LayoutError::layout(
                        LayoutErrorKind::Malformed,
                        format!("indexed transformed ring is invalid: {error}"),
                    )
                })?;
                let bbox = points_bbox(&ring)?;
                if bbox_intersects(bbox, region) {
                    output.push(HierarchyCandidate {
                        structure: structure.name.clone(),
                        element_index: part.element_index,
                        part_index: part.part_index,
                        kind: part.kind,
                        layer: part.layer,
                        instance_path: path.clone(),
                        ring,
                        bbox,
                    });
                }
            }
            match element {
                GdsElement::Sref(reference) => {
                    let child = self.by_name[reference.structure.as_str()];
                    let child_transform = transform
                        .compose(Affine::instance(reference.transform, reference.origin)?)?;
                    path.push(InstancePathEntry {
                        parent_structure: structure.name.clone(),
                        element_index: element_index as u32,
                        referenced_structure: reference.structure.clone(),
                        column: 0,
                        row: 0,
                    });
                    self.query_structure(
                        child,
                        child_transform,
                        region,
                        layer,
                        depth + 1,
                        visits,
                        path,
                        output,
                    )?;
                    path.pop();
                }
                GdsElement::Aref(reference) => {
                    let count = usize::from(reference.columns)
                        .checked_mul(usize::from(reference.rows))
                        .ok_or_else(|| {
                            LayoutError::layout(
                                LayoutErrorKind::ArithmeticOverflow,
                                "AREF instance count overflow",
                            )
                        })?;
                    if count > self.options.max_array_instances {
                        return Err(LayoutError::layout(
                            LayoutErrorKind::CapacityExceeded,
                            format!(
                                "AREF has {count} instances; limit is {}",
                                self.options.max_array_instances
                            ),
                        ));
                    }
                    let pitches = array_pitches(reference)?;
                    let child = self.by_name[reference.structure.as_str()];
                    for column in 0..reference.columns {
                        for row in 0..reference.rows {
                            let origin = array_origin(reference.origin, pitches, column, row)?;
                            let child_transform = transform
                                .compose(Affine::instance(reference.transform, origin)?)?;
                            path.push(InstancePathEntry {
                                parent_structure: structure.name.clone(),
                                element_index: element_index as u32,
                                referenced_structure: reference.structure.clone(),
                                column,
                                row,
                            });
                            self.query_structure(
                                child,
                                child_transform,
                                region,
                                layer,
                                depth + 1,
                                visits,
                                path,
                                output,
                            )?;
                            path.pop();
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn index_element(element: &GdsElement, element_index: u32) -> Result<Vec<LocalShape>, LayoutError> {
    let (kind, layer, rings) = match element {
        GdsElement::Boundary(boundary) => {
            ensure_meta_supported(&boundary.meta, "BOUNDARY")?;
            Ring::new(boundary.ring.clone()).map_err(|error| {
                LayoutError::layout(
                    LayoutErrorKind::Malformed,
                    format!("invalid BOUNDARY: {error}"),
                )
            })?;
            (
                IndexedShapeKind::Boundary,
                GdsLayerIdentity {
                    layer: boundary.layer,
                    datatype: boundary.datatype,
                },
                vec![boundary.ring.clone()],
            )
        }
        GdsElement::Box(box_element) => {
            ensure_meta_supported(&box_element.meta, "BOX")?;
            Ring::new(box_element.ring.clone()).map_err(|error| {
                LayoutError::layout(LayoutErrorKind::Malformed, format!("invalid BOX: {error}"))
            })?;
            (
                IndexedShapeKind::Box,
                GdsLayerIdentity {
                    layer: box_element.layer,
                    datatype: box_element.box_type,
                },
                vec![box_element.ring.clone()],
            )
        }
        GdsElement::Path(path) => (
            IndexedShapeKind::Path,
            GdsLayerIdentity {
                layer: path.layer,
                datatype: path.datatype,
            },
            stroke_path(path)?,
        ),
        GdsElement::Text(text) => {
            ensure_meta_supported(&text.meta, "TEXT")?;
            return Ok(Vec::new());
        }
        GdsElement::Sref(reference) => {
            ensure_meta_supported(&reference.meta, "SREF")?;
            Affine::instance(reference.transform, reference.origin)?;
            return Ok(Vec::new());
        }
        GdsElement::Aref(reference) => {
            ensure_meta_supported(&reference.meta, "AREF")?;
            Affine::instance(reference.transform, reference.origin)?;
            array_pitches(reference)?;
            return Ok(Vec::new());
        }
        GdsElement::Node(_) => {
            return Err(LayoutError::layout(
                LayoutErrorKind::Unsupported,
                "NODE has no polygon spatial-index representation",
            ));
        }
        GdsElement::Unsupported(element) => {
            return Err(LayoutError::layout(
                LayoutErrorKind::Unsupported,
                format!(
                    "element record 0x{:02x} is unsupported",
                    element.start_record.record_type
                ),
            ));
        }
    };
    rings
        .into_iter()
        .enumerate()
        .map(|(part_index, ring)| {
            let bbox = points_bbox(&ring)?;
            Ok(LocalShape {
                element_index,
                part_index: part_index as u32,
                kind,
                layer,
                ring,
                bbox,
            })
        })
        .collect()
}

fn array_pitches(
    reference: &crate::gds_lossless::GdsArrayReference,
) -> Result<(i32, i32, i32, i32), LayoutError> {
    Ok((
        exact_pitch(
            reference.column_endpoint.x,
            reference.origin.x,
            reference.columns,
            "column x",
        )?,
        exact_pitch(
            reference.column_endpoint.y,
            reference.origin.y,
            reference.columns,
            "column y",
        )?,
        exact_pitch(
            reference.row_endpoint.x,
            reference.origin.x,
            reference.rows,
            "row x",
        )?,
        exact_pitch(
            reference.row_endpoint.y,
            reference.origin.y,
            reference.rows,
            "row y",
        )?,
    ))
}

fn array_origin(
    origin: Point,
    pitches: (i32, i32, i32, i32),
    column: u16,
    row: u16,
) -> Result<Point, LayoutError> {
    Ok(Point {
        x: checked_array_coordinate(origin.x, pitches.0, column, pitches.2, row)?,
        y: checked_array_coordinate(origin.y, pitches.1, column, pitches.3, row)?,
    })
}

fn points_bbox(points: &[Point]) -> Result<Bbox, LayoutError> {
    let first = points
        .first()
        .ok_or_else(|| LayoutError::layout(LayoutErrorKind::Malformed, "empty polygon ring"))?;
    let mut bbox = Bbox {
        xmin: first.x,
        ymin: first.y,
        xmax: first.x,
        ymax: first.y,
    };
    for point in &points[1..] {
        bbox.xmin = bbox.xmin.min(point.x);
        bbox.ymin = bbox.ymin.min(point.y);
        bbox.xmax = bbox.xmax.max(point.x);
        bbox.ymax = bbox.ymax.max(point.y);
    }
    Ok(bbox)
}

fn transform_bbox(transform: Affine, bbox: Bbox) -> Result<Bbox, LayoutError> {
    points_bbox(&[
        transform.apply(Point::new(bbox.xmin, bbox.ymin))?,
        transform.apply(Point::new(bbox.xmax, bbox.ymin))?,
        transform.apply(Point::new(bbox.xmax, bbox.ymax))?,
        transform.apply(Point::new(bbox.xmin, bbox.ymax))?,
    ])
}

fn include_bbox(target: &mut Option<Bbox>, value: Bbox) {
    if let Some(target) = target {
        target.xmin = target.xmin.min(value.xmin);
        target.ymin = target.ymin.min(value.ymin);
        target.xmax = target.xmax.max(value.xmax);
        target.ymax = target.ymax.max(value.ymax);
    } else {
        *target = Some(value);
    }
}

fn validate_bbox(bbox: Bbox, name: &str) -> Result<(), LayoutError> {
    if bbox.xmin > bbox.xmax || bbox.ymin > bbox.ymax {
        Err(LayoutError::layout(
            LayoutErrorKind::Malformed,
            format!("{name} is inverted"),
        ))
    } else {
        Ok(())
    }
}

fn bbox_intersects(a: Bbox, b: Bbox) -> bool {
    i64::from(a.xmin) <= i64::from(b.xmax)
        && i64::from(b.xmin) <= i64::from(a.xmax)
        && i64::from(a.ymin) <= i64::from(b.ymax)
        && i64::from(b.ymin) <= i64::from(a.ymax)
}

fn signed_area2(ring: &[Point]) -> i128 {
    ring.iter()
        .enumerate()
        .map(|(index, point)| {
            let next = ring[(index + 1) % ring.len()];
            point.x as i128 * next.y as i128 - next.x as i128 * point.y as i128
        })
        .sum()
}

// ---------------------------------------------------------------------------
// Deterministic tiling and marker ownership
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerificationTile {
    pub id: TileId,
    pub column: u32,
    pub row: u32,
    pub core: Bbox,
    pub query_region: Bbox,
}

#[derive(Clone, Debug)]
pub struct TileGrid {
    bounds: Bbox,
    tile_width: i32,
    tile_height: i32,
    halo: i32,
    columns: u32,
    rows: u32,
}

impl TileGrid {
    pub const MAX_TILES: u64 = 10_000_000;

    pub fn new(
        bounds: Bbox,
        tile_width: i32,
        tile_height: i32,
        halo: i32,
    ) -> Result<Self, LayoutError> {
        validate_bbox(bounds, "tile bounds")?;
        if tile_width <= 0 || tile_height <= 0 || halo < 0 {
            return Err(LayoutError::layout(
                LayoutErrorKind::Malformed,
                "tile dimensions must be positive and halo non-negative",
            ));
        }
        let width = i64::from(bounds.xmax) - i64::from(bounds.xmin);
        let height = i64::from(bounds.ymax) - i64::from(bounds.ymin);
        if width <= 0 || height <= 0 {
            return Err(LayoutError::layout(
                LayoutErrorKind::Malformed,
                "tile bounds must have positive area",
            ));
        }
        let columns = ceil_div(width, i64::from(tile_width));
        let rows = ceil_div(height, i64::from(tile_height));
        let columns = u32::try_from(columns).map_err(|_| {
            LayoutError::layout(
                LayoutErrorKind::CapacityExceeded,
                "tile column count exceeds u32",
            )
        })?;
        let rows = u32::try_from(rows).map_err(|_| {
            LayoutError::layout(
                LayoutErrorKind::CapacityExceeded,
                "tile row count exceeds u32",
            )
        })?;
        let tile_count = u64::from(columns)
            .checked_mul(u64::from(rows))
            .ok_or_else(|| {
                LayoutError::layout(LayoutErrorKind::CapacityExceeded, "tile count overflow")
            })?;
        if tile_count > Self::MAX_TILES {
            return Err(LayoutError::layout(
                LayoutErrorKind::CapacityExceeded,
                format!(
                    "tile count {tile_count} exceeds capacity {}",
                    Self::MAX_TILES
                ),
            ));
        }
        Ok(Self {
            bounds,
            tile_width,
            tile_height,
            halo,
            columns,
            rows,
        })
    }

    pub fn columns(&self) -> u32 {
        self.columns
    }
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Row-major tiles, clipped at the layout bounds.
    pub fn tiles(&self) -> Result<Vec<VerificationTile>, LayoutError> {
        let capacity =
            usize::try_from(u64::from(self.columns) * u64::from(self.rows)).map_err(|_| {
                LayoutError::layout(
                    LayoutErrorKind::CapacityExceeded,
                    "tile vector exceeds usize",
                )
            })?;
        let mut tiles = Vec::with_capacity(capacity);
        for row in 0..self.rows {
            for column in 0..self.columns {
                tiles.push(self.tile(column, row)?);
            }
        }
        Ok(tiles)
    }

    pub fn owner_of_marker(&self, marker: Bbox) -> Result<TileId, LayoutError> {
        validate_bbox(marker, "marker bbox")?;
        if marker.xmin < self.bounds.xmin
            || marker.xmin > self.bounds.xmax
            || marker.ymin < self.bounds.ymin
            || marker.ymin > self.bounds.ymax
        {
            return Err(LayoutError::layout(
                LayoutErrorKind::Malformed,
                "marker lower-left anchor is outside tile bounds",
            ));
        }
        let dx = i64::from(marker.xmin) - i64::from(self.bounds.xmin);
        let dy = i64::from(marker.ymin) - i64::from(self.bounds.ymin);
        let column = (dx / i64::from(self.tile_width)).min(i64::from(self.columns - 1));
        let row = (dy / i64::from(self.tile_height)).min(i64::from(self.rows - 1));
        Ok(TileId(row as u64 * u64::from(self.columns) + column as u64))
    }

    pub fn tile_owns_marker(
        &self,
        tile: VerificationTile,
        marker: Bbox,
    ) -> Result<bool, LayoutError> {
        Ok(self.owner_of_marker(marker)? == tile.id)
    }

    pub fn query_parallel<'a>(
        &self,
        index: &HierarchySpatialIndex<'a>,
        top: &str,
        layer: Option<GdsLayerIdentity>,
    ) -> Result<Vec<TileCandidates>, LayoutError> {
        let tiles = self.tiles()?;
        let mut results: Vec<Result<TileCandidates, LayoutError>> = tiles
            .par_iter()
            .map(|tile| {
                index
                    .query(top, tile.query_region, layer)
                    .map(|candidates| TileCandidates {
                        tile: *tile,
                        candidates,
                    })
            })
            .collect();
        let mut output = Vec::with_capacity(results.len());
        for result in results.drain(..) {
            output.push(result?);
        }
        output.sort_by_key(|entry| entry.tile.id);
        Ok(output)
    }

    fn tile(&self, column: u32, row: u32) -> Result<VerificationTile, LayoutError> {
        let xmin = i64::from(self.bounds.xmin) + i64::from(column) * i64::from(self.tile_width);
        let ymin = i64::from(self.bounds.ymin) + i64::from(row) * i64::from(self.tile_height);
        let xmax = (xmin + i64::from(self.tile_width)).min(i64::from(self.bounds.xmax));
        let ymax = (ymin + i64::from(self.tile_height)).min(i64::from(self.bounds.ymax));
        let core = Bbox {
            xmin: to_i32(xmin, "tile xmin")?,
            ymin: to_i32(ymin, "tile ymin")?,
            xmax: to_i32(xmax, "tile xmax")?,
            ymax: to_i32(ymax, "tile ymax")?,
        };
        let query_region = Bbox {
            xmin: to_i32(
                (xmin - i64::from(self.halo)).max(i64::from(self.bounds.xmin)),
                "halo xmin",
            )?,
            ymin: to_i32(
                (ymin - i64::from(self.halo)).max(i64::from(self.bounds.ymin)),
                "halo ymin",
            )?,
            xmax: to_i32(
                (xmax + i64::from(self.halo)).min(i64::from(self.bounds.xmax)),
                "halo xmax",
            )?,
            ymax: to_i32(
                (ymax + i64::from(self.halo)).min(i64::from(self.bounds.ymax)),
                "halo ymax",
            )?,
        };
        let id = TileId(u64::from(row) * u64::from(self.columns) + u64::from(column));
        Ok(VerificationTile {
            id,
            column,
            row,
            core,
            query_region,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TileCandidates {
    pub tile: VerificationTile,
    pub candidates: Vec<HierarchyCandidate>,
}

fn ceil_div(value: i64, divisor: i64) -> i64 {
    (value + divisor - 1) / divisor
}

fn to_i32(value: i64, context: &str) -> Result<i32, LayoutError> {
    i32::try_from(value).map_err(|_| {
        LayoutError::layout(
            LayoutErrorKind::ArithmeticOverflow,
            format!("{context} outside i32"),
        )
    })
}

struct Fnv64(u64);

impl Fnv64 {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }
    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
    fn u8(&mut self, value: u8) {
        self.bytes(&[value]);
    }
    fn i16(&mut self, value: i16) {
        self.bytes(&value.to_le_bytes());
    }
    fn u16(&mut self, value: u16) {
        self.bytes(&value.to_le_bytes());
    }
    fn i32(&mut self, value: i32) {
        self.bytes(&value.to_le_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }
    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }
    fn string(&mut self, value: &str) {
        self.u64(value.len() as u64);
        self.bytes(value.as_bytes());
    }
    fn finish(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gds::GdsUnits;
    use crate::gds_lossless::{
        flatten_gds_library, GdsArrayReference, GdsBoundary, GdsElementMeta, GdsEnvelope,
        GdsFlattenOptions, GdsReference, GdsTransform,
    };
    use crate::params::{LayerDef, LayerTable};

    fn rectangle(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<Point> {
        vec![
            Point::new(x0, y0),
            Point::new(x1, y0),
            Point::new(x1, y1),
            Point::new(x0, y1),
        ]
    }

    fn boundary(x0: i32, y0: i32, x1: i32, y1: i32) -> GdsElement {
        GdsElement::Boundary(GdsBoundary {
            layer: 7,
            datatype: 0,
            ring: rectangle(x0, y0, x1, y1),
            meta: GdsElementMeta::default(),
        })
    }

    fn structure(name: &str, elements: Vec<GdsElement>) -> GdsStructure {
        GdsStructure {
            timestamps: [0; 12],
            name: name.to_string(),
            elements,
            unhandled_records: Vec::new(),
        }
    }

    fn library(structures: Vec<GdsStructure>) -> GdsLibrary {
        GdsLibrary {
            version: 600,
            timestamps: [0; 12],
            name: "index".to_string(),
            units: GdsUnits {
                user_units_per_database_unit: 1.0e-3,
                meters_per_database_unit: 1.0e-9,
            },
            structures,
            unhandled_records: Vec::new(),
            envelope: GdsEnvelope::complete(),
        }
    }

    fn layers() -> LayerTable {
        LayerTable::from_defs(
            &[(
                "met1".to_string(),
                LayerDef {
                    layer: 7,
                    datatype: 0,
                },
            )]
            .into_iter()
            .collect(),
        )
    }

    fn sref(name: &str, origin: Point, transform: GdsTransform) -> GdsElement {
        GdsElement::Sref(GdsReference {
            structure: name.to_string(),
            origin,
            transform,
            meta: GdsElementMeta::default(),
        })
    }

    #[test]
    fn flat_and_hierarchical_candidates_are_exactly_equivalent() {
        let leaf = structure("leaf", vec![boundary(0, 0, 10, 20)]);
        let middle = structure(
            "middle",
            vec![sref(
                "leaf",
                Point::new(30, 40),
                GdsTransform {
                    angle_degrees: Some(90.0),
                    ..Default::default()
                },
            )],
        );
        let top = structure(
            "top",
            vec![
                sref(
                    "leaf",
                    Point::new(100, 0),
                    GdsTransform {
                        reflected: true,
                        ..Default::default()
                    },
                ),
                sref(
                    "middle",
                    Point::new(0, 100),
                    GdsTransform {
                        angle_degrees: Some(270.0),
                        magnification: Some(2.0),
                        ..Default::default()
                    },
                ),
                GdsElement::Aref(GdsArrayReference {
                    structure: "leaf".to_string(),
                    columns: 3,
                    rows: 2,
                    origin: Point::new(0, 200),
                    column_endpoint: Point::new(90, 200),
                    row_endpoint: Point::new(0, 280),
                    transform: GdsTransform {
                        angle_degrees: Some(180.0),
                        ..Default::default()
                    },
                    meta: GdsElementMeta::default(),
                }),
            ],
        );
        let library = library(vec![leaf, middle, top]);
        let index = HierarchySpatialIndex::build(&library, Default::default()).unwrap();
        let bbox = index.cell_bbox("top").unwrap();
        let candidates = index.query("top", bbox, None).unwrap();
        let flat = flatten_gds_library(
            &library,
            &layers(),
            &GdsFlattenOptions {
                selected_top: Some("top".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        let store = &flat.cells["top"];
        assert_eq!(candidates.len(), store.poly_count());
        for (index, candidate) in candidates.iter().enumerate() {
            assert_eq!(candidate.bbox, store.poly_bbox[index]);
            let (start, end) = store.poly_range(crate::geometry::PolyId(index as u32));
            let flat_ring: Vec<Point> = (start..end)
                .map(|vertex| Point::new(store.verts_x[vertex], store.verts_y[vertex]))
                .collect();
            assert_eq!(candidate.ring, flat_ring);
        }
        assert_eq!(candidates[0].instance_path.len(), 1);
        assert_eq!(candidates[1].instance_path.len(), 2);
        assert_eq!(candidates[2].instance_path[0].column, 0);
        assert_eq!(candidates[7].instance_path[0].column, 2);
        assert_eq!(candidates[7].instance_path[0].row, 1);
    }

    #[test]
    fn query_layer_filter_and_bbox_pruning_are_deterministic() {
        let mut second = match boundary(1_000, 1_000, 1_010, 1_010) {
            GdsElement::Boundary(value) => value,
            _ => unreachable!(),
        };
        second.layer = 8;
        let library = library(vec![structure(
            "top",
            vec![boundary(0, 0, 10, 10), GdsElement::Boundary(second)],
        )]);
        let index = HierarchySpatialIndex::build(&library, Default::default()).unwrap();
        let local = index
            .query(
                "top",
                Bbox {
                    xmin: -1,
                    ymin: -1,
                    xmax: 20,
                    ymax: 20,
                },
                None,
            )
            .unwrap();
        assert_eq!(local.len(), 1);
        assert_eq!(local[0].layer.layer, 7);
        let filtered = index
            .query(
                "top",
                index.cell_bbox("top").unwrap(),
                Some(GdsLayerIdentity {
                    layer: 8,
                    datatype: 0,
                }),
            )
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].element_index, 1);
        assert_eq!(
            index.equivalent_layout_hash("top").unwrap(),
            index.equivalent_layout_hash("top").unwrap()
        );
    }

    #[test]
    fn parallel_tiles_are_row_major_and_seam_markers_have_one_owner() {
        let library = library(vec![structure("top", vec![boundary(95, 20, 105, 40)])]);
        let index = HierarchySpatialIndex::build(&library, Default::default()).unwrap();
        let grid = TileGrid::new(
            Bbox {
                xmin: 0,
                ymin: 0,
                xmax: 200,
                ymax: 200,
            },
            100,
            100,
            10,
        )
        .unwrap();
        let results = grid.query_parallel(&index, "top", None).unwrap();
        assert_eq!(
            results
                .iter()
                .map(|result| result.tile.id)
                .collect::<Vec<_>>(),
            [TileId(0), TileId(1), TileId(2), TileId(3)]
        );
        assert_eq!(results[0].candidates.len(), 1);
        assert_eq!(results[1].candidates.len(), 1);
        let seam_marker = Bbox {
            xmin: 100,
            ymin: 25,
            xmax: 100,
            ymax: 35,
        };
        assert_eq!(grid.owner_of_marker(seam_marker).unwrap(), TileId(1));
        assert_eq!(
            results
                .iter()
                .filter(|result| grid.tile_owns_marker(result.tile, seam_marker).unwrap())
                .count(),
            1
        );
    }

    #[test]
    fn deep_hierarchy_and_expansion_limits_fail_before_overflow() {
        let mut structures = vec![structure("leaf", vec![boundary(0, 0, 10, 10)])];
        for depth in 1..=32 {
            structures.push(structure(
                &format!("d{depth}"),
                vec![sref(
                    if depth == 1 {
                        "leaf".to_string()
                    } else {
                        format!("d{}", depth - 1)
                    }
                    .as_str(),
                    Point::new(1, 1),
                    GdsTransform::default(),
                )],
            ));
        }
        let deep_library = library(structures);
        let index = HierarchySpatialIndex::build(
            &deep_library,
            HierarchyIndexOptions {
                max_depth: 64,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            index.cell_bbox("d32"),
            Some(Bbox {
                xmin: 32,
                ymin: 32,
                xmax: 42,
                ymax: 42,
            })
        );
        assert_eq!(
            index
                .query("d32", index.cell_bbox("d32").unwrap(), None)
                .unwrap()
                .len(),
            1
        );

        let shallow_index = HierarchySpatialIndex::build(
            &deep_library,
            HierarchyIndexOptions {
                max_depth: 8,
                ..Default::default()
            },
        )
        .unwrap();
        let depth_error = shallow_index
            .query("d32", shallow_index.cell_bbox("d32").unwrap(), None)
            .err()
            .unwrap();
        assert_eq!(depth_error.kind, LayoutErrorKind::CapacityExceeded);

        let array = library(vec![
            structure("leaf", vec![boundary(0, 0, 10, 10)]),
            structure(
                "top",
                vec![GdsElement::Aref(GdsArrayReference {
                    structure: "leaf".to_string(),
                    columns: 100,
                    rows: 100,
                    origin: Point::new(0, 0),
                    column_endpoint: Point::new(1_000, 0),
                    row_endpoint: Point::new(0, 1_000),
                    transform: GdsTransform::default(),
                    meta: GdsElementMeta::default(),
                })],
            ),
        ]);
        let array_error = HierarchySpatialIndex::build(
            &array,
            HierarchyIndexOptions {
                max_array_instances: 9_999,
                ..Default::default()
            },
        )
        .err()
        .unwrap();
        assert_eq!(array_error.kind, LayoutErrorKind::CapacityExceeded);
    }

    #[test]
    fn transform_and_tile_arithmetic_overflow_is_typed() {
        let fractional = library(vec![
            structure("leaf", vec![boundary(0, 0, 11, 10)]),
            structure(
                "top",
                vec![sref(
                    "leaf",
                    Point::new(0, 0),
                    GdsTransform {
                        magnification: Some(0.5),
                        ..Default::default()
                    },
                )],
            ),
        ]);
        let error = HierarchySpatialIndex::build(&fractional, Default::default())
            .err()
            .unwrap();
        assert_eq!(error.kind, LayoutErrorKind::NonIntegralTransform);

        let library = library(vec![
            structure("leaf", vec![boundary(i32::MAX - 10, 0, i32::MAX, 10)]),
            structure(
                "top",
                vec![sref("leaf", Point::new(100, 0), GdsTransform::default())],
            ),
        ]);
        let error = HierarchySpatialIndex::build(&library, Default::default())
            .err()
            .unwrap();
        assert_eq!(error.kind, LayoutErrorKind::ArithmeticOverflow);

        let error = TileGrid::new(
            Bbox {
                xmin: i32::MIN,
                ymin: 0,
                xmax: i32::MAX,
                ymax: 10,
            },
            1,
            1,
            0,
        )
        .unwrap_err();
        assert_eq!(error.kind, LayoutErrorKind::CapacityExceeded);
    }
}
