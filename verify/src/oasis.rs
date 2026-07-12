//! Fail-closed OASIS 1.0 reader/writer for an explicit, nonmodal verification subset.
//!
//! Supported records are START, END, PAD, named CELL (14), XYABSOLUTE,
//! orthogonal named PLACEMENT (17), explicit RECTANGLE, and explicit rectilinear
//! POLYGON. Every field required to interpret geometry is present in each record.
//! Reference-number tables, omitted/modal fields, relative coordinates, repetition,
//! properties, text, paths, transformed placement, validation signatures, CBLOCK,
//! and all other records return an offset-bearing [`OasisErrorKind::Unsupported`].

use crate::gds::GdsUnits;
use crate::gds_lossless::{
    validate_hierarchy, GdsBoundary, GdsElement, GdsElementMeta, GdsEnvelope, GdsLibrary,
    GdsReference, GdsStructure, GdsTransform,
};
use crate::geometry::exact::{Point, Ring};
use std::collections::{HashMap, HashSet};
use std::fmt;

const MAGIC: &[u8; 13] = b"%SEMI-OASIS\r\n";
const PAD: u64 = 0;
const START: u64 = 1;
const END: u64 = 2;
const CELL: u64 = 14;
const XYABSOLUTE: u64 = 15;
const PLACEMENT: u64 = 17;
const RECTANGLE: u64 = 20;
const POLYGON: u64 = 21;

const EXPLICIT_RECTANGLE_INFO: u8 = 0x7b; // W,H,X,Y,L,D; no S/R/modal fields
const EXPLICIT_POLYGON_INFO: u8 = 0x3b; // point-list,X,Y,L,D; no R/modal fields
const EXPLICIT_PLACEMENT_BASE: u8 = 0xb0; // C,N,X,Y; no ref-number/repetition

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OasisCapabilities {
    pub version_1_0: bool,
    pub named_cells: bool,
    pub absolute_coordinates: bool,
    pub explicit_rectangles: bool,
    pub explicit_rectilinear_polygons: bool,
    pub named_orthogonal_placements: bool,
    pub paths: bool,
    pub text: bool,
    pub repetitions: bool,
    pub properties: bool,
    pub modal_fields: bool,
    pub relative_coordinates: bool,
    pub reference_number_tables: bool,
    pub compressed_blocks: bool,
    pub validation_signatures: bool,
    pub max_layer_or_datatype: i32,
}

pub const OASIS_CAPABILITIES: OasisCapabilities = OasisCapabilities {
    version_1_0: true,
    named_cells: true,
    absolute_coordinates: true,
    explicit_rectangles: true,
    explicit_rectilinear_polygons: true,
    named_orthogonal_placements: true,
    paths: false,
    text: false,
    repetitions: false,
    properties: false,
    modal_fields: false,
    relative_coordinates: false,
    reference_number_tables: false,
    compressed_blocks: false,
    validation_signatures: false,
    max_layer_or_datatype: i16::MAX as i32,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OasisErrorKind {
    Malformed,
    InvalidOrder,
    Duplicate,
    Unsupported,
    UndefinedReference,
    HierarchyCycle,
    ArithmeticOverflow,
    CapacityExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OasisError {
    pub kind: OasisErrorKind,
    pub offset: usize,
    pub message: String,
}

impl OasisError {
    fn new(kind: OasisErrorKind, offset: usize, message: impl Into<String>) -> Self {
        Self {
            kind,
            offset,
            message: message.into(),
        }
    }

    fn unsupported(offset: usize, message: impl Into<String>) -> Self {
        Self::new(OasisErrorKind::Unsupported, offset, message)
    }
}

impl fmt::Display for OasisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OASIS at byte offset {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for OasisError {}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn byte(&mut self, context: &str) -> Result<u8, OasisError> {
        let byte = self.bytes.get(self.at).copied().ok_or_else(|| {
            OasisError::new(
                OasisErrorKind::Malformed,
                self.at,
                format!("truncated {context}"),
            )
        })?;
        self.at += 1;
        Ok(byte)
    }

    fn unsigned(&mut self, context: &str) -> Result<u64, OasisError> {
        let start = self.at;
        let mut result = 0_u64;
        let mut shift = 0_u32;
        for index in 0..10 {
            let byte = self.byte(context)?;
            let value = u64::from(byte & 0x7f);
            if shift == 63 && value > 1 {
                return Err(OasisError::new(
                    OasisErrorKind::ArithmeticOverflow,
                    start,
                    format!("{context} exceeds u64"),
                ));
            }
            result |= value << shift;
            if byte & 0x80 == 0 {
                if index > 0 && value == 0 {
                    return Err(OasisError::new(
                        OasisErrorKind::Malformed,
                        start,
                        format!("{context} uses a non-canonical varint"),
                    ));
                }
                return Ok(result);
            }
            shift += 7;
        }
        Err(OasisError::new(
            OasisErrorKind::ArithmeticOverflow,
            start,
            format!("{context} varint exceeds ten bytes"),
        ))
    }

    fn signed(&mut self, context: &str) -> Result<i64, OasisError> {
        let start = self.at;
        let encoded = self.internal_integer(1, context)?;
        let magnitude = encoded.0;
        if magnitude > i64::MAX as u64 {
            return Err(OasisError::new(
                OasisErrorKind::ArithmeticOverflow,
                start,
                format!("{context} magnitude exceeds i64"),
            ));
        }
        let magnitude = magnitude as i64;
        Ok(if encoded.1 & 1 != 0 {
            -magnitude
        } else {
            magnitude
        })
    }

    /// Returns `(magnitude, low discriminator bits)`.
    fn internal_integer(&mut self, skip_bits: u8, context: &str) -> Result<(u64, u8), OasisError> {
        let start = self.at;
        let first = self.byte(context)?;
        let bits = first & ((1 << skip_bits) - 1);
        let mut result = u64::from(first & 0x7f) >> skip_bits;
        let mut shift = u32::from(7 - skip_bits);
        let mut byte = first;
        let mut count = 1;
        while byte & 0x80 != 0 {
            if count == 10 || shift >= 64 {
                return Err(OasisError::new(
                    OasisErrorKind::ArithmeticOverflow,
                    start,
                    format!("{context} exceeds u64"),
                ));
            }
            byte = self.byte(context)?;
            let value = u64::from(byte & 0x7f);
            if shift == 63 && value > 1 {
                return Err(OasisError::new(
                    OasisErrorKind::ArithmeticOverflow,
                    start,
                    format!("{context} exceeds u64"),
                ));
            }
            result |= value << shift;
            shift += 7;
            count += 1;
        }
        Ok((result, bits))
    }

    fn nstring(&mut self, context: &str) -> Result<String, OasisError> {
        let start = self.at;
        let length = self.unsigned(&format!("{context} length"))?;
        let length = usize::try_from(length).map_err(|_| {
            OasisError::new(
                OasisErrorKind::CapacityExceeded,
                start,
                format!("{context} length exceeds usize"),
            )
        })?;
        if length == 0 {
            return Err(OasisError::new(
                OasisErrorKind::Malformed,
                start,
                format!("{context} N-string must not be empty"),
            ));
        }
        let end = self.at.checked_add(length).ok_or_else(|| {
            OasisError::new(
                OasisErrorKind::ArithmeticOverflow,
                start,
                "string end overflow",
            )
        })?;
        let bytes = self.bytes.get(self.at..end).ok_or_else(|| {
            OasisError::new(
                OasisErrorKind::Malformed,
                self.at,
                format!("truncated {context}"),
            )
        })?;
        if !bytes.iter().all(|byte| (0x21..=0x7e).contains(byte)) {
            return Err(OasisError::new(
                OasisErrorKind::Malformed,
                self.at,
                format!("{context} is not an OASIS N-string"),
            ));
        }
        self.at = end;
        Ok(String::from_utf8(bytes.to_vec()).expect("N-string is ASCII"))
    }

    fn bstring(&mut self, context: &str) -> Result<&'a [u8], OasisError> {
        let start = self.at;
        let length =
            usize::try_from(self.unsigned(&format!("{context} length"))?).map_err(|_| {
                OasisError::new(
                    OasisErrorKind::CapacityExceeded,
                    start,
                    format!("{context} length exceeds usize"),
                )
            })?;
        let end = self.at.checked_add(length).ok_or_else(|| {
            OasisError::new(
                OasisErrorKind::ArithmeticOverflow,
                start,
                "string end overflow",
            )
        })?;
        let value = self.bytes.get(self.at..end).ok_or_else(|| {
            OasisError::new(
                OasisErrorKind::Malformed,
                self.at,
                format!("truncated {context}"),
            )
        })?;
        self.at = end;
        Ok(value)
    }
}

/// Read the declared nonmodal OASIS subset into the shared lossless hierarchy DB.
pub fn read_oasis(bytes: &[u8]) -> Result<GdsLibrary, OasisError> {
    if bytes.len() < MAGIC.len() || &bytes[..MAGIC.len().min(bytes.len())] != MAGIC {
        return Err(OasisError::new(
            OasisErrorKind::Malformed,
            0,
            "invalid or truncated `%SEMI-OASIS\\r\\n` magic",
        ));
    }
    let mut reader = Reader {
        bytes,
        at: MAGIC.len(),
    };
    let start_offset = reader.at;
    if reader.unsigned("START record ID")? != START {
        return Err(OasisError::new(
            OasisErrorKind::InvalidOrder,
            start_offset,
            "START must immediately follow the magic",
        ));
    }
    let version = reader.nstring("START version")?;
    if version != "1.0" {
        return Err(OasisError::unsupported(
            start_offset,
            format!("OASIS version `{version}`; only 1.0 is supported"),
        ));
    }
    let real_offset = reader.at;
    let real_type = reader.byte("START unit real type")?;
    if real_type != 0 {
        return Err(OasisError::unsupported(
            real_offset,
            format!(
                "START unit real encoding {real_type}; only positive-integer real is supported"
            ),
        ));
    }
    let database_units_per_micron = reader.unsigned("START unit")?;
    if database_units_per_micron == 0 {
        return Err(OasisError::new(
            OasisErrorKind::Malformed,
            real_offset,
            "START unit must be positive",
        ));
    }
    let offset_flag_offset = reader.at;
    let offset_table_flag = reader.unsigned("START offset-table flag")?;
    if offset_table_flag != 0 {
        return Err(OasisError::unsupported(
            offset_flag_offset,
            "offset table in END; subset requires an explicit zero table in START",
        ));
    }
    for _ in 0..6 {
        let strict = reader.unsigned("START table strict flag")?;
        let offset = reader.unsigned("START table offset")?;
        if strict != 0 || offset != 0 {
            return Err(OasisError::unsupported(
                offset_flag_offset,
                "name/property/layer reference-number tables are not supported",
            ));
        }
    }

    let mut structures = Vec::new();
    let mut names = HashSet::new();
    let mut current: Option<GdsStructure> = None;
    let mut current_has_absolute = false;
    let mut saw_end = false;
    while reader.at < bytes.len() {
        let record_offset = reader.at;
        let record = reader.unsigned("record ID")?;
        match record {
            PAD => {}
            END => {
                if let Some(structure) = current.take() {
                    structures.push(structure);
                }
                read_end(&mut reader, record_offset)?;
                saw_end = true;
                break;
            }
            CELL => {
                if let Some(structure) = current.take() {
                    structures.push(structure);
                }
                let name = reader.nstring("CELL name")?;
                if !names.insert(name.clone()) {
                    return Err(OasisError::new(
                        OasisErrorKind::Duplicate,
                        record_offset,
                        format!("duplicate CELL `{name}`"),
                    ));
                }
                current = Some(GdsStructure {
                    timestamps: [0; 12],
                    name,
                    elements: Vec::new(),
                    unhandled_records: Vec::new(),
                });
                current_has_absolute = false;
            }
            XYABSOLUTE => {
                require_cell(&current, record_offset, "XYABSOLUTE")?;
                if current_has_absolute {
                    return Err(OasisError::new(
                        OasisErrorKind::Duplicate,
                        record_offset,
                        "duplicate XYABSOLUTE in CELL",
                    ));
                }
                current_has_absolute = true;
            }
            RECTANGLE => {
                require_explicit_cell(&current, current_has_absolute, record_offset, "RECTANGLE")?;
                let boundary = read_rectangle(&mut reader, record_offset)?;
                current
                    .as_mut()
                    .unwrap()
                    .elements
                    .push(GdsElement::Boundary(boundary));
            }
            POLYGON => {
                require_explicit_cell(&current, current_has_absolute, record_offset, "POLYGON")?;
                let boundary = read_polygon(&mut reader, record_offset)?;
                current
                    .as_mut()
                    .unwrap()
                    .elements
                    .push(GdsElement::Boundary(boundary));
            }
            PLACEMENT => {
                require_explicit_cell(&current, current_has_absolute, record_offset, "PLACEMENT")?;
                let reference = read_placement(&mut reader, record_offset)?;
                current
                    .as_mut()
                    .unwrap()
                    .elements
                    .push(GdsElement::Sref(reference));
            }
            START => {
                return Err(OasisError::new(
                    OasisErrorKind::InvalidOrder,
                    record_offset,
                    "duplicate/out-of-order START",
                ));
            }
            unsupported => {
                return Err(OasisError::unsupported(
                    record_offset,
                    unsupported_record_message(unsupported),
                ));
            }
        }
    }
    if !saw_end {
        return Err(OasisError::new(
            OasisErrorKind::InvalidOrder,
            reader.at,
            "missing END record",
        ));
    }
    if reader.at != bytes.len() {
        return Err(OasisError::new(
            OasisErrorKind::InvalidOrder,
            reader.at,
            "data follows END record",
        ));
    }
    if structures.is_empty() {
        return Err(OasisError::new(
            OasisErrorKind::Malformed,
            start_offset,
            "OASIS library has no cells",
        ));
    }
    let units = GdsUnits {
        user_units_per_database_unit: 1.0 / database_units_per_micron as f64,
        meters_per_database_unit: 1.0e-6 / database_units_per_micron as f64,
    };
    let library = GdsLibrary {
        version: 600,
        timestamps: [0; 12],
        name: "OASIS".to_string(),
        units,
        structures,
        unhandled_records: Vec::new(),
        envelope: GdsEnvelope::complete(),
    };
    validate_oasis_hierarchy(&library, start_offset)?;
    Ok(library)
}

fn require_cell(
    current: &Option<GdsStructure>,
    offset: usize,
    record: &str,
) -> Result<(), OasisError> {
    if current.is_none() {
        Err(OasisError::new(
            OasisErrorKind::InvalidOrder,
            offset,
            format!("{record} outside CELL"),
        ))
    } else {
        Ok(())
    }
}

fn require_explicit_cell(
    current: &Option<GdsStructure>,
    absolute: bool,
    offset: usize,
    record: &str,
) -> Result<(), OasisError> {
    require_cell(current, offset, record)?;
    if !absolute {
        Err(OasisError::unsupported(
            offset,
            format!("{record} without an explicit preceding XYABSOLUTE"),
        ))
    } else {
        Ok(())
    }
}

fn read_layer(reader: &mut Reader<'_>, context: &str) -> Result<i16, OasisError> {
    let offset = reader.at;
    let value = reader.unsigned(context)?;
    i16::try_from(value).map_err(|_| {
        OasisError::new(
            OasisErrorKind::CapacityExceeded,
            offset,
            format!(
                "{context} {value} exceeds declared subset maximum {}",
                i16::MAX
            ),
        )
    })
}

fn coordinate(reader: &mut Reader<'_>, context: &str) -> Result<i32, OasisError> {
    let offset = reader.at;
    let value = reader.signed(context)?;
    i32::try_from(value).map_err(|_| {
        OasisError::new(
            OasisErrorKind::ArithmeticOverflow,
            offset,
            format!("{context} {value} exceeds i32 database coordinates"),
        )
    })
}

fn dimension(reader: &mut Reader<'_>, context: &str) -> Result<i32, OasisError> {
    let offset = reader.at;
    let value = reader.unsigned(context)?;
    if value == 0 || value > i32::MAX as u64 {
        return Err(OasisError::new(
            if value == 0 {
                OasisErrorKind::Malformed
            } else {
                OasisErrorKind::ArithmeticOverflow
            },
            offset,
            format!("{context} must be in 1..=i32::MAX, got {value}"),
        ));
    }
    Ok(value as i32)
}

fn read_rectangle(
    reader: &mut Reader<'_>,
    record_offset: usize,
) -> Result<GdsBoundary, OasisError> {
    let info = reader.byte("RECTANGLE info")?;
    if info != EXPLICIT_RECTANGLE_INFO {
        return Err(OasisError::unsupported(
            record_offset,
            format!("RECTANGLE info 0x{info:02x} uses modal/square/repetition/omitted fields"),
        ));
    }
    let layer = read_layer(reader, "RECTANGLE layer")?;
    let datatype = read_layer(reader, "RECTANGLE datatype")?;
    let width = dimension(reader, "RECTANGLE width")?;
    let height = dimension(reader, "RECTANGLE height")?;
    let x = coordinate(reader, "RECTANGLE x")?;
    let y = coordinate(reader, "RECTANGLE y")?;
    let xmax = x.checked_add(width).ok_or_else(|| {
        OasisError::new(
            OasisErrorKind::ArithmeticOverflow,
            record_offset,
            "RECTANGLE x + width overflow",
        )
    })?;
    let ymax = y.checked_add(height).ok_or_else(|| {
        OasisError::new(
            OasisErrorKind::ArithmeticOverflow,
            record_offset,
            "RECTANGLE y + height overflow",
        )
    })?;
    Ok(GdsBoundary {
        layer,
        datatype,
        ring: vec![
            Point::new(x, y),
            Point::new(xmax, y),
            Point::new(xmax, ymax),
            Point::new(x, ymax),
        ],
        meta: GdsElementMeta::default(),
    })
}

fn read_polygon(reader: &mut Reader<'_>, record_offset: usize) -> Result<GdsBoundary, OasisError> {
    let info = reader.byte("POLYGON info")?;
    if info != EXPLICIT_POLYGON_INFO {
        return Err(OasisError::unsupported(
            record_offset,
            format!("POLYGON info 0x{info:02x} uses modal/repetition/omitted fields"),
        ));
    }
    let layer = read_layer(reader, "POLYGON layer")?;
    let datatype = read_layer(reader, "POLYGON datatype")?;
    let point_list_offset = reader.at;
    let point_list_type = reader.byte("POLYGON point-list type")?;
    if point_list_type != 2 {
        return Err(OasisError::unsupported(
            point_list_offset,
            format!("POLYGON point-list type {point_list_type}; subset supports Manhattan type 2"),
        ));
    }
    let delta_count = reader.unsigned("POLYGON point count")?;
    if delta_count < 2 {
        return Err(OasisError::new(
            OasisErrorKind::Malformed,
            point_list_offset,
            "POLYGON needs at least three points",
        ));
    }
    let count = usize::try_from(delta_count).map_err(|_| {
        OasisError::new(
            OasisErrorKind::CapacityExceeded,
            point_list_offset,
            "POLYGON point count exceeds usize",
        )
    })?;
    let mut relative = Vec::with_capacity(count + 1);
    relative.push(Point::new(0, 0));
    for _ in 0..count {
        let delta_offset = reader.at;
        let (magnitude, direction) = reader.internal_integer(2, "POLYGON 2-delta")?;
        let magnitude = i32::try_from(magnitude).map_err(|_| {
            OasisError::new(
                OasisErrorKind::ArithmeticOverflow,
                delta_offset,
                "POLYGON delta exceeds i32",
            )
        })?;
        if magnitude == 0 {
            return Err(OasisError::new(
                OasisErrorKind::Malformed,
                delta_offset,
                "zero POLYGON delta",
            ));
        }
        let previous = *relative.last().unwrap();
        let (dx, dy) = match direction {
            0 => (magnitude, 0),
            1 => (0, magnitude),
            2 => (-magnitude, 0),
            3 => (0, -magnitude),
            _ => unreachable!(),
        };
        relative.push(Point::new(
            previous.x.checked_add(dx).ok_or_else(|| {
                OasisError::new(
                    OasisErrorKind::ArithmeticOverflow,
                    delta_offset,
                    "POLYGON x delta overflow",
                )
            })?,
            previous.y.checked_add(dy).ok_or_else(|| {
                OasisError::new(
                    OasisErrorKind::ArithmeticOverflow,
                    delta_offset,
                    "POLYGON y delta overflow",
                )
            })?,
        ));
    }
    let origin_x = coordinate(reader, "POLYGON x")?;
    let origin_y = coordinate(reader, "POLYGON y")?;
    let ring: Vec<Point> = relative
        .into_iter()
        .map(|point| {
            Ok(Point::new(
                origin_x.checked_add(point.x).ok_or_else(|| {
                    OasisError::new(
                        OasisErrorKind::ArithmeticOverflow,
                        record_offset,
                        "POLYGON x translation overflow",
                    )
                })?,
                origin_y.checked_add(point.y).ok_or_else(|| {
                    OasisError::new(
                        OasisErrorKind::ArithmeticOverflow,
                        record_offset,
                        "POLYGON y translation overflow",
                    )
                })?,
            ))
        })
        .collect::<Result<_, OasisError>>()?;
    Ring::new(ring.clone()).map_err(|error| {
        OasisError::new(
            OasisErrorKind::Malformed,
            record_offset,
            format!("invalid POLYGON: {error}"),
        )
    })?;
    Ok(GdsBoundary {
        layer,
        datatype,
        ring,
        meta: GdsElementMeta::default(),
    })
}

fn read_placement(
    reader: &mut Reader<'_>,
    record_offset: usize,
) -> Result<GdsReference, OasisError> {
    let info = reader.byte("PLACEMENT info")?;
    if info & 0xf8 != EXPLICIT_PLACEMENT_BASE {
        return Err(OasisError::unsupported(
            record_offset,
            format!(
                "PLACEMENT info 0x{info:02x} uses modal/ref-number/repetition/omitted coordinates"
            ),
        ));
    }
    let structure = reader.nstring("PLACEMENT cell name")?;
    let x = coordinate(reader, "PLACEMENT x")?;
    let y = coordinate(reader, "PLACEMENT y")?;
    Ok(GdsReference {
        structure,
        origin: Point::new(x, y),
        transform: GdsTransform {
            reflected: info & 0x01 != 0,
            angle_degrees: Some(f64::from((info >> 1) & 0x03) * 90.0),
            ..Default::default()
        },
        meta: GdsElementMeta::default(),
    })
}

fn read_end(reader: &mut Reader<'_>, record_offset: usize) -> Result<(), OasisError> {
    let padding = reader.bstring("END padding")?;
    if padding.iter().any(|byte| *byte != 0) {
        return Err(OasisError::new(
            OasisErrorKind::Malformed,
            record_offset,
            "END padding contains nonzero bytes",
        ));
    }
    let validation_offset = reader.at;
    let validation = reader.byte("END validation scheme")?;
    if validation != 0 {
        return Err(OasisError::unsupported(
            validation_offset,
            format!("END validation scheme {validation}; signatures are not implemented"),
        ));
    }
    if reader.at - record_offset != 256 {
        return Err(OasisError::new(
            OasisErrorKind::Malformed,
            record_offset,
            format!(
                "END record occupies {} bytes; required size is 256",
                reader.at - record_offset
            ),
        ));
    }
    Ok(())
}

fn validate_oasis_hierarchy(library: &GdsLibrary, offset: usize) -> Result<(), OasisError> {
    let by_name: HashMap<&str, &GdsStructure> = library
        .structures
        .iter()
        .map(|structure| (structure.name.as_str(), structure))
        .collect();
    validate_hierarchy(library, &by_name).map_err(|error| {
        let kind = match error.kind {
            crate::gds_lossless::LayoutErrorKind::UndefinedReference => {
                OasisErrorKind::UndefinedReference
            }
            crate::gds_lossless::LayoutErrorKind::HierarchyCycle => OasisErrorKind::HierarchyCycle,
            _ => OasisErrorKind::Malformed,
        };
        OasisError::new(kind, offset, error.message)
    })
}

fn unsupported_record_message(record: u64) -> String {
    let name = match record {
        3 | 4 => "CELLNAME/reference-number table",
        5 | 6 => "TEXTSTRING",
        7..=10 => "property name/string table",
        11 | 12 => "LAYERNAME table",
        13 => "reference-number CELL",
        16 => "XYRELATIVE",
        18 => "PLACEMENT_TRANSFORM",
        19 => "TEXT",
        22 => "PATH",
        23..=27 => "trapezoid/circle geometry",
        28 | 29 => "PROPERTY",
        30..=33 => "extension record",
        34 => "CBLOCK compression",
        _ => "unknown record",
    };
    format!("record {record} ({name}) is outside the declared OASIS subset")
}

// ---------------------------------------------------------------------------
// Deterministic subset writer
// ---------------------------------------------------------------------------

pub fn write_oasis(library: &GdsLibrary) -> Result<Vec<u8>, OasisError> {
    let unit = 1.0e-6 / library.units.meters_per_database_unit;
    let rounded_unit = unit.round();
    if !unit.is_finite()
        || unit <= 0.0
        || (unit - rounded_unit).abs() > 1.0e-9 * unit.abs().max(1.0)
        || rounded_unit > u64::MAX as f64
    {
        return Err(OasisError::unsupported(
            0,
            "OASIS subset writer requires an integer database-units-per-micron value",
        ));
    }
    let mut out = MAGIC.to_vec();
    put_unsigned(&mut out, START);
    put_nstring(&mut out, "1.0")?;
    out.push(0); // positive-integer real
    put_unsigned(&mut out, rounded_unit as u64);
    put_unsigned(&mut out, 0); // offset table is in START
    for _ in 0..12 {
        put_unsigned(&mut out, 0);
    }

    let mut names = HashSet::new();
    for structure in &library.structures {
        if !names.insert(structure.name.as_str()) {
            return Err(OasisError::new(
                OasisErrorKind::Duplicate,
                out.len(),
                format!("duplicate CELL `{}`", structure.name),
            ));
        }
        if !structure.unhandled_records.is_empty() {
            return Err(OasisError::unsupported(
                out.len(),
                format!("CELL `{}` has unhandled GDS records", structure.name),
            ));
        }
        put_unsigned(&mut out, CELL);
        put_nstring(&mut out, &structure.name)?;
        put_unsigned(&mut out, XYABSOLUTE);
        for element in &structure.elements {
            write_oasis_element(&mut out, element)?;
        }
    }
    validate_oasis_hierarchy(library, 0)?;
    write_end(&mut out);
    Ok(out)
}

fn write_oasis_element(out: &mut Vec<u8>, element: &GdsElement) -> Result<(), OasisError> {
    match element {
        GdsElement::Boundary(boundary) => {
            ensure_empty_meta(&boundary.meta, out.len(), "BOUNDARY")?;
            Ring::new(boundary.ring.clone()).map_err(|error| {
                OasisError::new(
                    OasisErrorKind::Malformed,
                    out.len(),
                    format!("invalid BOUNDARY: {error}"),
                )
            })?;
            if let Some((x, y, width, height)) = rectangle_dimensions(&boundary.ring) {
                put_unsigned(out, RECTANGLE);
                out.push(EXPLICIT_RECTANGLE_INFO);
                put_layer(out, boundary.layer, "RECTANGLE layer")?;
                put_layer(out, boundary.datatype, "RECTANGLE datatype")?;
                put_unsigned(out, width as u64);
                put_unsigned(out, height as u64);
                put_signed(out, i64::from(x));
                put_signed(out, i64::from(y));
            } else {
                if !boundary.ring.iter().enumerate().all(|(index, point)| {
                    let next = boundary.ring[(index + 1) % boundary.ring.len()];
                    point.x == next.x || point.y == next.y
                }) {
                    return Err(OasisError::unsupported(
                        out.len(),
                        "non-rectilinear POLYGON",
                    ));
                }
                put_unsigned(out, POLYGON);
                out.push(EXPLICIT_POLYGON_INFO);
                put_layer(out, boundary.layer, "POLYGON layer")?;
                put_layer(out, boundary.datatype, "POLYGON datatype")?;
                out.push(2); // Manhattan 2-delta point list
                put_unsigned(out, (boundary.ring.len() - 1) as u64);
                for edge in boundary.ring.windows(2) {
                    put_2delta(out, edge[1].x - edge[0].x, edge[1].y - edge[0].y)?;
                }
                put_signed(out, i64::from(boundary.ring[0].x));
                put_signed(out, i64::from(boundary.ring[0].y));
            }
        }
        GdsElement::Sref(reference) => {
            ensure_empty_meta(&reference.meta, out.len(), "SREF")?;
            if reference.transform.magnification != None
                || reference.transform.absolute_angle
                || reference.transform.absolute_magnification
                || reference.transform.reserved_bits != 0
            {
                return Err(OasisError::unsupported(
                    out.len(),
                    "SREF transform outside orthogonal PLACEMENT",
                ));
            }
            let angle = reference.transform.angle_degrees().rem_euclid(360.0);
            let quarter = (angle / 90.0).round();
            if (angle - quarter * 90.0).abs() > 1.0e-12 {
                return Err(OasisError::unsupported(
                    out.len(),
                    "non-orthogonal SREF angle",
                ));
            }
            put_unsigned(out, PLACEMENT);
            out.push(
                EXPLICIT_PLACEMENT_BASE
                    | (((quarter as i64).rem_euclid(4) as u8) << 1)
                    | u8::from(reference.transform.reflected),
            );
            put_nstring(out, &reference.structure)?;
            put_signed(out, i64::from(reference.origin.x));
            put_signed(out, i64::from(reference.origin.y));
        }
        GdsElement::Path(_) => {
            return Err(OasisError::unsupported(
                out.len(),
                "PATH is not implemented",
            ))
        }
        GdsElement::Text(_) => {
            return Err(OasisError::unsupported(
                out.len(),
                "TEXT is not implemented",
            ))
        }
        GdsElement::Aref(_) => {
            return Err(OasisError::unsupported(
                out.len(),
                "repetition/AREF is not implemented",
            ))
        }
        GdsElement::Box(_) => {
            return Err(OasisError::unsupported(
                out.len(),
                "BOX has no direct subset mapping; use BOUNDARY",
            ))
        }
        GdsElement::Node(_) => {
            return Err(OasisError::unsupported(
                out.len(),
                "NODE is not implemented",
            ))
        }
        GdsElement::Unsupported(_) => {
            return Err(OasisError::unsupported(
                out.len(),
                "unsupported retained GDS element",
            ))
        }
    }
    Ok(())
}

fn ensure_empty_meta(meta: &GdsElementMeta, offset: usize, kind: &str) -> Result<(), OasisError> {
    if meta.properties.is_empty()
        && meta.element_flags.is_none()
        && meta.plex.is_none()
        && meta.unhandled_records.is_empty()
    {
        Ok(())
    } else {
        Err(OasisError::unsupported(
            offset,
            format!("{kind} properties/metadata are not implemented"),
        ))
    }
}

fn rectangle_dimensions(ring: &[Point]) -> Option<(i32, i32, i32, i32)> {
    if ring.len() != 4 {
        return None;
    }
    let xmin = ring.iter().map(|point| point.x).min()?;
    let xmax = ring.iter().map(|point| point.x).max()?;
    let ymin = ring.iter().map(|point| point.y).min()?;
    let ymax = ring.iter().map(|point| point.y).max()?;
    let corners: HashSet<(i32, i32)> = ring.iter().map(|point| (point.x, point.y)).collect();
    if corners
        == [(xmin, ymin), (xmax, ymin), (xmax, ymax), (xmin, ymax)]
            .into_iter()
            .collect()
    {
        Some((xmin, ymin, xmax.checked_sub(xmin)?, ymax.checked_sub(ymin)?))
    } else {
        None
    }
}

fn put_layer(out: &mut Vec<u8>, value: i16, context: &str) -> Result<(), OasisError> {
    if value < 0 {
        return Err(OasisError::new(
            OasisErrorKind::Malformed,
            out.len(),
            format!("negative {context}"),
        ));
    }
    put_unsigned(out, value as u64);
    Ok(())
}

fn put_nstring(out: &mut Vec<u8>, value: &str) -> Result<(), OasisError> {
    if value.is_empty() || !value.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
        return Err(OasisError::new(
            OasisErrorKind::Malformed,
            out.len(),
            format!("`{value}` is not an OASIS N-string"),
        ));
    }
    put_unsigned(out, value.len() as u64);
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn put_unsigned(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn put_signed(out: &mut Vec<u8>, value: i64) {
    let (magnitude, sign) = if value < 0 {
        (value.unsigned_abs(), 1)
    } else {
        (value as u64, 0)
    };
    put_internal(out, magnitude, 1, sign);
}

fn put_2delta(out: &mut Vec<u8>, dx: i32, dy: i32) -> Result<(), OasisError> {
    let (magnitude, direction) = match (dx, dy) {
        (x, 0) if x > 0 => (x as u64, 0),
        (0, y) if y > 0 => (y as u64, 1),
        (x, 0) if x < 0 => (u64::from(x.unsigned_abs()), 2),
        (0, y) if y < 0 => (u64::from(y.unsigned_abs()), 3),
        _ => {
            return Err(OasisError::new(
                OasisErrorKind::Malformed,
                out.len(),
                "POLYGON edge is zero-length or non-Manhattan",
            ))
        }
    };
    put_internal(out, magnitude, 2, direction);
    Ok(())
}

fn put_internal(out: &mut Vec<u8>, mut magnitude: u64, skip_bits: u8, bits: u8) {
    let first_value_bits = 7 - skip_bits;
    let mut byte = bits | (((magnitude & ((1 << first_value_bits) - 1)) as u8) << skip_bits);
    magnitude >>= first_value_bits;
    if magnitude != 0 {
        byte |= 0x80;
    }
    out.push(byte);
    while magnitude != 0 {
        let mut byte = (magnitude & 0x7f) as u8;
        magnitude >>= 7;
        if magnitude != 0 {
            byte |= 0x80;
        }
        out.push(byte);
    }
}

fn write_end(out: &mut Vec<u8>) {
    let start = out.len();
    put_unsigned(out, END);
    put_unsigned(out, 252);
    out.resize(out.len() + 252, 0);
    out.push(0); // no validation signature
    debug_assert_eq!(out.len() - start, 256);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gds_lossless::{read_gds_library, write_gds_library, GdsReadMode};
    use crate::hierarchy_index::{HierarchyIndexOptions, HierarchySpatialIndex};

    fn rectangle(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<Point> {
        vec![
            Point::new(x0, y0),
            Point::new(x1, y0),
            Point::new(x1, y1),
            Point::new(x0, y1),
        ]
    }

    fn boundary(ring: Vec<Point>) -> GdsElement {
        GdsElement::Boundary(GdsBoundary {
            layer: 7,
            datatype: 0,
            ring,
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

    fn library() -> GdsLibrary {
        let l_shape = vec![
            Point::new(0, 0),
            Point::new(30, 0),
            Point::new(30, 10),
            Point::new(10, 10),
            Point::new(10, 30),
            Point::new(0, 30),
        ];
        GdsLibrary {
            version: 600,
            timestamps: [0; 12],
            name: "source".to_string(),
            units: GdsUnits {
                user_units_per_database_unit: 1.0e-3,
                meters_per_database_unit: 1.0e-9,
            },
            structures: vec![
                structure(
                    "leaf",
                    vec![boundary(rectangle(-10, -20, 10, 20)), boundary(l_shape)],
                ),
                structure(
                    "top",
                    vec![GdsElement::Sref(GdsReference {
                        structure: "leaf".to_string(),
                        origin: Point::new(100, 200),
                        transform: GdsTransform {
                            reflected: true,
                            angle_degrees: Some(270.0),
                            ..Default::default()
                        },
                        meta: GdsElementMeta::default(),
                    })],
                ),
            ],
            unhandled_records: Vec::new(),
            envelope: GdsEnvelope::complete(),
        }
    }

    #[test]
    fn declared_capability_matrix_is_explicit() {
        assert!(OASIS_CAPABILITIES.version_1_0);
        assert!(OASIS_CAPABILITIES.explicit_rectangles);
        assert!(OASIS_CAPABILITIES.explicit_rectilinear_polygons);
        assert!(OASIS_CAPABILITIES.named_orthogonal_placements);
        assert!(!OASIS_CAPABILITIES.paths);
        assert!(!OASIS_CAPABILITIES.text);
        assert!(!OASIS_CAPABILITIES.repetitions);
        assert!(!OASIS_CAPABILITIES.properties);
        assert!(!OASIS_CAPABILITIES.modal_fields);
        assert!(!OASIS_CAPABILITIES.compressed_blocks);
    }

    #[test]
    fn oasis_round_trip_and_gds_layout_hash_are_equivalent() {
        let source = library();
        let oasis1 = write_oasis(&source).expect("supported OASIS write");
        assert_eq!(&oasis1[..MAGIC.len()], MAGIC);
        let parsed1 = read_oasis(&oasis1).expect("supported OASIS read");
        let oasis2 = write_oasis(&parsed1).expect("deterministic rewrite");
        let parsed2 = read_oasis(&oasis2).expect("rewritten OASIS read");
        assert_eq!(oasis1, oasis2);
        assert_eq!(parsed1.structures, parsed2.structures);
        assert!((parsed1.units.database_unit_nm() - 1.0).abs() < 1.0e-12);

        let source_index =
            HierarchySpatialIndex::build(&source, HierarchyIndexOptions::default()).unwrap();
        let oasis_index =
            HierarchySpatialIndex::build(&parsed1, HierarchyIndexOptions::default()).unwrap();
        assert_eq!(
            source_index.equivalent_layout_hash("top").unwrap(),
            oasis_index.equivalent_layout_hash("top").unwrap()
        );

        let gds = write_gds_library(&parsed1).expect("OASIS DB is GDS-writable");
        let reparsed_gds = read_gds_library(&gds, GdsReadMode::Strict).unwrap();
        let gds_index =
            HierarchySpatialIndex::build(&reparsed_gds, HierarchyIndexOptions::default()).unwrap();
        assert_eq!(
            oasis_index.equivalent_layout_hash("top").unwrap(),
            gds_index.equivalent_layout_hash("top").unwrap()
        );
    }

    fn start_prefix() -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        put_unsigned(&mut bytes, START);
        put_nstring(&mut bytes, "1.0").unwrap();
        bytes.push(0);
        put_unsigned(&mut bytes, 1_000);
        put_unsigned(&mut bytes, 0);
        for _ in 0..12 {
            put_unsigned(&mut bytes, 0);
        }
        bytes
    }

    #[test]
    fn every_omitted_record_is_offset_bearing_unsupported() {
        let unsupported = [
            3_u64, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16, 18, 19, 22, 23, 24, 25, 26, 27, 28, 29,
            30, 31, 32, 33, 34, 35, 127,
        ];
        for record in unsupported {
            let mut bytes = start_prefix();
            let offset = bytes.len();
            put_unsigned(&mut bytes, record);
            let error = read_oasis(&bytes).unwrap_err();
            assert_eq!(error.kind, OasisErrorKind::Unsupported, "record {record}");
            assert_eq!(error.offset, offset, "record {record}");
        }
    }

    #[test]
    fn modal_fields_relative_coordinates_and_validation_fail_closed() {
        let mut modal = start_prefix();
        put_unsigned(&mut modal, CELL);
        put_nstring(&mut modal, "top").unwrap();
        put_unsigned(&mut modal, XYABSOLUTE);
        let record_offset = modal.len();
        put_unsigned(&mut modal, RECTANGLE);
        modal.push(EXPLICIT_RECTANGLE_INFO & !0x01); // omit layer => modal layer
        let error = read_oasis(&modal).unwrap_err();
        assert_eq!(error.kind, OasisErrorKind::Unsupported);
        assert_eq!(error.offset, record_offset);

        let mut relative = start_prefix();
        put_unsigned(&mut relative, CELL);
        put_nstring(&mut relative, "top").unwrap();
        let relative_offset = relative.len();
        put_unsigned(&mut relative, 16);
        let error = read_oasis(&relative).unwrap_err();
        assert_eq!(error.kind, OasisErrorKind::Unsupported);
        assert_eq!(error.offset, relative_offset);

        let mut validation = write_oasis(&library()).unwrap();
        let validation_offset = validation.len() - 1;
        validation[validation_offset] = 1;
        let error = read_oasis(&validation).unwrap_err();
        assert_eq!(error.kind, OasisErrorKind::Unsupported);
        assert_eq!(error.offset, validation_offset);
    }

    #[test]
    fn malformed_truncation_and_varint_corpus_never_panics() {
        let valid = write_oasis(&library()).unwrap();
        let mut corpus = vec![
            Vec::new(),
            b"%SEMI".to_vec(),
            [MAGIC.as_slice(), &[0x81]].concat(),
            [MAGIC.as_slice(), &[1, 3, b'1']].concat(),
            [MAGIC.as_slice(), &[0x80; 11]].concat(),
        ];
        for end in [13, 14, 15, 20, valid.len() - 1] {
            corpus.push(valid[..end].to_vec());
        }
        for bytes in corpus {
            let result = std::panic::catch_unwind(|| read_oasis(&bytes));
            assert!(result.is_ok(), "panic for {bytes:?}");
            assert!(result.unwrap().is_err());
        }
    }

    #[test]
    fn writer_rejects_unimplemented_path_text_repetition_and_properties() {
        let mut source = library();
        let GdsElement::Boundary(boundary) = &mut source.structures[0].elements[0] else {
            unreachable!()
        };
        boundary
            .meta
            .properties
            .push(crate::gds_lossless::GdsProperty {
                attribute: 1,
                value: "not-supported".to_string(),
            });
        assert_eq!(
            write_oasis(&source).unwrap_err().kind,
            OasisErrorKind::Unsupported
        );

        let mut source = library();
        source.structures[1].elements.push(GdsElement::Aref(
            crate::gds_lossless::GdsArrayReference {
                structure: "leaf".to_string(),
                columns: 2,
                rows: 2,
                origin: Point::new(0, 0),
                column_endpoint: Point::new(20, 0),
                row_endpoint: Point::new(0, 20),
                transform: GdsTransform::default(),
                meta: GdsElementMeta::default(),
            },
        ));
        assert_eq!(
            write_oasis(&source).unwrap_err().kind,
            OasisErrorKind::Unsupported
        );
    }
}
