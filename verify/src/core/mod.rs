pub mod geometry;
pub mod device_plane;

pub use geometry::{Rect, RectSet, decompose_rectilinear, decompose_all,
                   rect_overlap_area_pos, rect_touch};
pub use device_plane::DevicePlane;
