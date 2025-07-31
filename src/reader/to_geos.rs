use std::cell::RefCell;

use crate::{common::WKBDimension, reader::*, Endianness};
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use geo_traits::*;
use geos::GResult;

/// A factory for converting WKB to GEOS geometries.
///
/// This factory uses a scratch buffer to store intermediate coordinate data.
/// The scratch buffer is reused for each conversion, which reduces memory allocations.
pub struct GEOSWkbFactory {
    scratch: RefCell<Vec<f64>>,
}

impl Default for GEOSWkbFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl GEOSWkbFactory {
    /// Create a new GEOSWkbFactory.
    pub fn new() -> Self {
        Self {
            scratch: RefCell::new(Vec::new()),
        }
    }

    /// Create a GEOS geometry from a WKB.
    pub fn create(&self, wkb: &Wkb) -> GResult<geos::Geometry> {
        let scratch = &mut self.scratch.borrow_mut();
        to_geos_geometry(scratch, wkb)
    }
}

fn to_geos_geometry(scratch: &mut Vec<f64>, wkb: &Wkb) -> GResult<geos::Geometry> {
    let geom = wkb.as_type();
    match geom {
        geo_traits::GeometryType::Point(p) => to_geos_point(scratch, p),
        geo_traits::GeometryType::LineString(ls) => to_geos_line_string(scratch, ls),
        geo_traits::GeometryType::Polygon(poly) => to_geos_polygon(scratch, poly),
        geo_traits::GeometryType::MultiPoint(mp) => to_geos_multi_point(scratch, mp),
        geo_traits::GeometryType::MultiLineString(mls) => to_geos_multi_line_string(scratch, mls),
        geo_traits::GeometryType::MultiPolygon(mpoly) => to_geos_multi_polygon(scratch, mpoly),
        geo_traits::GeometryType::GeometryCollection(gc) => {
            to_geos_geometry_collection(scratch, gc)
        }
        _ => Err(geos::Error::ConversionError(
            "Unsupported geometry type".to_string(),
        )),
    }
}

fn to_geos_point(scratch: &mut Vec<f64>, p: &Point) -> GResult<geos::Geometry> {
    if p.is_empty {
        geos::Geometry::create_empty_point()
    } else {
        let coord = &p.coord;
        let coord_seq = create_coord_sequence_from_raw_parts(
            coord.buf,
            coord.offset,
            coord.dim,
            coord.byte_order,
            1,
            scratch,
        )?;
        let point = geos::Geometry::create_point(coord_seq)?;
        Ok(point)
    }
}

fn to_geos_line_string(scratch: &mut Vec<f64>, ls: &LineString) -> GResult<geos::Geometry> {
    if ls.num_points == 0 {
        geos::Geometry::create_empty_line_string()
    } else {
        let coord_seq = create_coord_sequence_from_raw_parts(
            ls.buf,
            ls.coord_offset(0),
            ls.dim,
            ls.byte_order,
            ls.num_points,
            scratch,
        )?;
        geos::Geometry::create_line_string(coord_seq)
    }
}

fn to_geos_polygon(scratch: &mut Vec<f64>, poly: &Polygon) -> GResult<geos::Geometry> {
    // Create exterior ring
    let exterior = if let Some(ring) = poly.exterior() {
        let coord_seq = create_coord_sequence_from_raw_parts(
            ring.buf,
            ring.coord_offset(0),
            ring.dim,
            ring.byte_order,
            ring.num_points,
            scratch,
        )?;
        geos::Geometry::create_linear_ring(coord_seq)?
    } else {
        return geos::Geometry::create_empty_polygon();
    };

    // Create interior rings
    let num_interiors = poly.num_interiors();
    let mut interior_rings = Vec::with_capacity(num_interiors);
    for i in 0..num_interiors {
        let ring = poly.interior(i).unwrap();
        let coord_seq = create_coord_sequence_from_raw_parts(
            ring.buf,
            ring.coord_offset(0),
            ring.dim,
            ring.byte_order,
            ring.num_points,
            scratch,
        )?;
        let interior_ring = geos::Geometry::create_linear_ring(coord_seq)?;
        interior_rings.push(interior_ring);
    }

    geos::Geometry::create_polygon(exterior, interior_rings)
}

fn to_geos_multi_point(scratch: &mut Vec<f64>, mp: &MultiPoint) -> GResult<geos::Geometry> {
    if mp.num_points() == 0 {
        // Create an empty multi-point by creating a geometry collection with no geometries
        geos::Geometry::create_empty_collection(geos::GeometryTypes::MultiPoint)
    } else {
        let num_points = mp.num_points();
        let mut points = Vec::with_capacity(num_points);
        for i in 0..num_points {
            let point = mp.point(i).unwrap();
            let coord_seq = create_coord_sequence_from_raw_parts(
                point.coord.buf,
                point.coord.offset,
                point.coord.dim,
                point.coord.byte_order,
                1,
                scratch,
            )?;
            let geos_point = geos::Geometry::create_point(coord_seq)?;
            points.push(geos_point);
        }
        geos::Geometry::create_multipoint(points)
    }
}

fn to_geos_multi_line_string(
    scratch: &mut Vec<f64>,
    mls: &MultiLineString,
) -> GResult<geos::Geometry> {
    if mls.num_line_strings() == 0 {
        geos::Geometry::create_empty_collection(geos::GeometryTypes::MultiLineString)
    } else {
        let num_line_strings = mls.num_line_strings();
        let mut line_strings = Vec::with_capacity(num_line_strings);
        for i in 0..num_line_strings {
            let ls = mls.line_string(i).unwrap();
            let coord_seq = create_coord_sequence_from_raw_parts(
                ls.buf,
                ls.coord_offset(0),
                ls.dim,
                ls.byte_order,
                ls.num_points,
                scratch,
            )?;
            let geos_line_string = geos::Geometry::create_line_string(coord_seq)?;
            line_strings.push(geos_line_string);
        }
        geos::Geometry::create_multiline_string(line_strings)
    }
}

fn to_geos_multi_polygon(scratch: &mut Vec<f64>, mpoly: &MultiPolygon) -> GResult<geos::Geometry> {
    if mpoly.num_polygons() == 0 {
        geos::Geometry::create_empty_collection(geos::GeometryTypes::MultiPolygon)
    } else {
        let num_polygons = mpoly.num_polygons();
        let mut polygons = Vec::with_capacity(num_polygons);
        for i in 0..num_polygons {
            let poly = mpoly.polygon(i).unwrap();
            let geos_polygon = to_geos_polygon(scratch, poly)?;
            polygons.push(geos_polygon);
        }
        geos::Geometry::create_multipolygon(polygons)
    }
}

fn to_geos_geometry_collection(
    scratch: &mut Vec<f64>,
    gc: &GeometryCollection,
) -> GResult<geos::Geometry> {
    if gc.num_geometries() == 0 {
        geos::Geometry::create_empty_collection(geos::GeometryTypes::GeometryCollection)
    } else {
        let num_geometries = gc.num_geometries();
        let mut geometries = Vec::with_capacity(num_geometries);
        for i in 0..num_geometries {
            let geom = gc.geometry(i).unwrap();
            let geos_geom = to_geos_geometry(scratch, geom)?;
            geometries.push(geos_geom);
        }
        geos::Geometry::create_geometry_collection(geometries)
    }
}

const NATIVE_ENDIANNESS: Endianness = if cfg!(target_endian = "big") {
    Endianness::BigEndian
} else {
    Endianness::LittleEndian
};

fn create_coord_sequence_from_raw_parts(
    buf: &[u8],
    offset: u64,
    dim: WKBDimension,
    byte_order: Endianness,
    num_coords: usize,
    scratch: &mut Vec<f64>,
) -> GResult<geos::CoordSeq> {
    let (has_z, has_m, dim_size) = match dim {
        WKBDimension::Xy => (false, false, 2),
        WKBDimension::Xyz => (true, false, 3),
        WKBDimension::Xym => (false, true, 3),
        WKBDimension::Xyzm => (true, true, 4),
    };
    let num_ordinates = dim_size * num_coords;

    // If the byte order matches native endianness, we can potentially use zero-copy
    if byte_order == NATIVE_ENDIANNESS {
        let ptr = unsafe { buf.as_ptr().add(offset as usize) };

        // On platforms with unaligned memory access support, we can construct the coord seq
        // directly from the raw parts without copying to the scratch buffer.
        #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
        {
            let coords_f64 =
                unsafe { &*core::ptr::slice_from_raw_parts(ptr as *const f64, num_ordinates) };
            geos::CoordSeq::new_from_buffer(coords_f64, num_coords, has_z, has_m)
        }

        // On platforms without unaligned memory access support, we need to copy the data to the
        // scratch buffer to make sure the data is aligned.
        #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
        {
            unsafe {
                scratch.clear();
                scratch.reserve(num_ordinates);
                scratch.set_len(num_ordinates);
                std::ptr::copy_nonoverlapping(
                    ptr,
                    scratch.as_mut_ptr() as *mut u8,
                    num_ordinates * std::mem::size_of::<f64>(),
                );
                geos::CoordSeq::new_from_buffer(scratch.as_slice(), num_coords, has_z, has_m)
            }
        }
    } else {
        // Need to convert byte order
        match byte_order {
            Endianness::BigEndian => {
                save_f64_to_scratch::<BigEndian>(scratch, buf, offset, num_ordinates);
            }
            Endianness::LittleEndian => {
                save_f64_to_scratch::<LittleEndian>(scratch, buf, offset, num_ordinates);
            }
        }
        geos::CoordSeq::new_from_buffer(scratch.as_slice(), num_coords, has_z, has_m)
    }
}

fn save_f64_to_scratch<B: ByteOrder>(
    scratch: &mut Vec<f64>,
    buf: &[u8],
    offset: u64,
    num_ordinates: usize,
) {
    scratch.clear();
    scratch.reserve(num_ordinates);
    // Safety: we have already reserved the capacity, so we can set the length safely.
    // Justification: rewriting the loop to not use Vec::push makes it many times faster,
    // since it eliminates several memory loads and stores for vector's length and capacity,
    // and it enables the compiler to generate vectorized code.
    #[allow(clippy::uninit_vec)]
    unsafe {
        scratch.set_len(num_ordinates);
    }
    assert!(offset + num_ordinates as u64 * 8 <= buf.len() as u64);
    for (i, tgt) in scratch.iter_mut().enumerate().take(num_ordinates) {
        let offset = offset + (i as u64 * 8);
        let value = B::read_f64(&buf[offset as usize..]);
        *tgt = value;
    }
}
