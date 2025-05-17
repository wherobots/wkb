use std::io::Cursor;

use crate::common::WKBDimension;
use crate::error::{WKBError, WKBResult};
use crate::reader::coord::Coord;
use crate::reader::coord_iter::*;
use crate::reader::util::{has_srid, ReadBytesExt};
use crate::Endianness;
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use geo_traits::LineStringTrait;
use geo_traits_ext::forward_line_string_trait_ext_funcs;
use geo_traits_ext::LineStringTraitExt;
use geo_traits_ext::{GeoTraitExtWithTypeTag, LineStringTag};
use geo_types::{Coord as GeoCoord, Line};

const HEADER_BYTES: u64 = 5;

/// A WKB LineString
///
/// This has been preprocessed, so access to any internal coordinate is `O(1)`.
#[derive(Debug, Clone, Copy)]
pub struct LineString<'a> {
    buf: &'a [u8],
    byte_order: Endianness,

    /// The number of points in this LineString WKB
    num_points: usize,

    /// This offset will be 0 for a single LineString but it will be non zero for a
    /// LineString contained within a MultiLineString
    offset: u64,
    dim: WKBDimension,
    has_srid: bool,
}

impl<'a> LineString<'a> {
    pub fn new(buf: &'a [u8], byte_order: Endianness, offset: u64, dim: WKBDimension) -> Self {
        Self::try_new(buf, byte_order, offset, dim).unwrap()
    }

    pub fn try_new(
        buf: &'a [u8],
        byte_order: Endianness,
        mut offset: u64,
        dim: WKBDimension,
    ) -> WKBResult<Self> {
        let has_srid = has_srid(buf, byte_order, offset)?;
        if has_srid {
            offset += 4;
        }

        let mut reader = Cursor::new(buf);
        reader.set_position(HEADER_BYTES + offset);
        let num_points = reader
            .read_u32(byte_order)?
            .try_into()
            .map_err(|e| WKBError::General(format!("Invalid number of points: {}", e)))?;

        let linestring = Self {
            buf,
            byte_order,
            num_points,
            offset,
            dim,
            has_srid,
        };

        let expected_end_abs = linestring.coord_offset(num_points as u64);
        if expected_end_abs > buf.len() as u64 {
            return Self::handle_invalid_buffer_length(
                linestring.offset,
                expected_end_abs,
                buf.len(),
            );
        }

        Ok(linestring)
    }

    #[cold]
    fn handle_invalid_buffer_length(
        offset: u64,
        expected_end_abs: u64,
        buf_len: usize,
    ) -> WKBResult<Self> {
        Err(WKBError::General(format!(
            "Invalid buffer length for LineString: geometry starting at offset {} would end at byte {}, but buffer length is {}.",
            offset, expected_end_abs, buf_len
        )))
    }

    /// The number of bytes in this object, including any header
    ///
    /// Note that this is not the same as the length of the underlying buffer
    pub fn size(&self) -> u64 {
        // - 1: byteOrder
        // - 4: wkbType
        // - 4: numPoints
        // - 2 * 8 * self.num_points: two f64s for each coordinate
        let mut header = 1 + 4 + 4;
        if self.has_srid {
            header += 4;
        }
        header + (self.dim.size() as u64 * 8 * self.num_points as u64)
    }

    /// The offset into this buffer of any given coordinate
    pub fn coord_offset(&self, i: u64) -> u64 {
        self.offset + 1 + 4 + 4 + (self.dim.size() as u64 * 8 * i)
    }

    pub fn dimension(&self) -> WKBDimension {
        self.dim
    }

    // Create a coordinate iterator with compile-time endianness
    #[inline]
    fn create_coord_iter<B: ByteOrder>(&self) -> CoordIter<'a, B> {
        CoordIter::new(
            self.buf,
            self.coord_offset(0) as usize,
            self.num_points,
            self.dim.size(),
        )
    }

    // Create a line iterator with compile-time endianness
    #[inline]
    fn create_line_iter<B: ByteOrder>(&self) -> LineIter<'a, B> {
        LineIter::new(
            self.buf,
            self.coord_offset(0) as usize,
            self.num_points,
            self.dim.size(),
        )
    }

    // Helper methods that return enum-based iterators based on endianness
    #[inline]
    fn get_coord_iter(&self) -> EndianCoordIter<'a> {
        match self.byte_order {
            Endianness::LittleEndian => {
                EndianCoordIter::LE(self.create_coord_iter::<LittleEndian>())
            }
            Endianness::BigEndian => EndianCoordIter::BE(self.create_coord_iter::<BigEndian>()),
        }
    }

    #[inline]
    fn get_line_iter(&self) -> EndianLineIter<'a> {
        match self.byte_order {
            Endianness::LittleEndian => EndianLineIter::LE(self.create_line_iter::<LittleEndian>()),
            Endianness::BigEndian => EndianLineIter::BE(self.create_line_iter::<BigEndian>()),
        }
    }
}

impl<'a> LineStringTrait for LineString<'a> {
    type CoordType<'b>
        = Coord<'a>
    where
        Self: 'b;

    fn num_coords(&self) -> usize {
        self.num_points
    }

    unsafe fn coord_unchecked(&self, i: usize) -> Self::CoordType<'_> {
        Coord::new(
            self.buf,
            self.byte_order,
            self.coord_offset(i.try_into().unwrap()),
            self.dim,
        )
    }
}

impl<'a> LineStringTrait for &LineString<'a> {
    type CoordType<'b>
        = Coord<'a>
    where
        Self: 'b;

    fn num_coords(&self) -> usize {
        self.num_points
    }

    unsafe fn coord_unchecked(&self, i: usize) -> Self::CoordType<'_> {
        Coord::new(
            self.buf,
            self.byte_order,
            self.coord_offset(i.try_into().unwrap()),
            self.dim,
        )
    }
}

impl LineStringTraitExt for LineString<'_> {
    forward_line_string_trait_ext_funcs!();

    #[inline(always)]
    fn lines(&'_ self) -> impl ExactSizeIterator<Item = Line<f64>> + '_ {
        self.get_line_iter()
    }

    #[inline(always)]
    fn coord_iter(&self) -> impl Iterator<Item = GeoCoord<f64>> {
        self.get_coord_iter()
    }
}

impl GeoTraitExtWithTypeTag for LineString<'_> {
    type Tag = LineStringTag;
}

impl LineStringTraitExt for &LineString<'_> {
    forward_line_string_trait_ext_funcs!();

    #[inline(always)]
    fn lines(&'_ self) -> impl ExactSizeIterator<Item = Line<f64>> + '_ {
        (*self).get_line_iter()
    }

    #[inline(always)]
    fn coord_iter(&self) -> impl Iterator<Item = GeoCoord<f64>> {
        (*self).get_coord_iter()
    }
}

impl GeoTraitExtWithTypeTag for &LineString<'_> {
    type Tag = LineStringTag;
}
