//! Deterministic verification tile. `TileId` lives here with the tile record
//! it identifies (a bare newtype with no impls of its own).

use crate::geometry::Bbox;

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
