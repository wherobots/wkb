use std::io::Cursor;

use crate::common::WKBDimension;
use crate::error::{WKBError, WKBResult};
use crate::reader::coord::Coord;
use crate::reader::coord_iter::*;
use crate::reader::util::ReadBytesExt;
use crate::Endianness;
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use geo_traits::LineStringTrait;
use geo_traits_ext::forward_line_string_trait_ext_funcs;
use geo_traits_ext::LineStringTraitExt;
use geo_traits_ext::{GeoTraitExtWithTypeTag, LineStringTag};
use geo_types::{Coord as GeoCoord, Line};

/// A linear ring in a WKB buffer.
///
/// This has been preprocessed, so access to any internal coordinate is `O(1)`.
///
/// See page 65 of <https://portal.ogc.org/files/?artifact_id=25355>.
#[derive(Debug, Clone, Copy)]
pub struct WKBLinearRing<'a> {
    /// The underlying WKB buffer
    pub buf: &'a [u8],

    /// The byte order of this WKB buffer
    pub byte_order: Endianness,

    /// The offset into the buffer where this linear ring is located
    ///
    /// Note that this does not have to be immediately after the WKB header! For a `Point`, the
    /// `Point` is immediately after the header, but the `Point` also appears in other geometry
    /// types. I.e. the `LineString` has a header, then the number of points, then a sequence of
    /// `Point` objects.
    pub offset: u64,

    /// The number of points in this linear ring
    pub num_points: usize,

    pub dim: WKBDimension,
}

impl<'a> WKBLinearRing<'a> {
    pub fn new(buf: &'a [u8], byte_order: Endianness, offset: u64, dim: WKBDimension) -> Self {
        Self::try_new(buf, byte_order, offset, dim).unwrap()
    }

    pub fn try_new(
        buf: &'a [u8],
        byte_order: Endianness,
        offset: u64,
        dim: WKBDimension,
    ) -> WKBResult<Self> {
        let mut reader = Cursor::new(buf);
        reader.set_position(offset);
        let num_points = reader
            .read_u32(byte_order)?
            .try_into()
            .map_err(|e| WKBError::General(format!("Invalid number of points: {e}")))?;

        let ring = Self {
            buf,
            byte_order,
            offset, // This offset is where num_points for this ring starts
            num_points,
            dim,
        };

        // `offset` is the start of the num_points field. `ring.size()` is `4 (for num_points) + coord_data`.
        let expected_end_abs = offset + ring.size();
        if expected_end_abs > buf.len() as u64 {
            return Self::handle_invalid_buffer_length(offset, expected_end_abs, buf.len());
        }

        Ok(ring)
    }

    #[cold]
    fn handle_invalid_buffer_length(
        offset: u64,
        expected_end_abs: u64,
        buf_len: usize,
    ) -> WKBResult<Self> {
        Err(WKBError::General(format!(
            "Invalid buffer length for LinearRing: data starting at offset {offset} would end at byte {expected_end_abs}, but buffer length is {buf_len}."
        )))
    }

    /// The number of bytes in this object, including any header
    ///
    /// Note that this is not the same as the length of the underlying buffer
    pub fn size(&self) -> u64 {
        // - 4: numPoints
        // - 2 * 8 * self.num_points: two f64s for each coordinate
        4 + (self.dim.size() as u64 * 8 * self.num_points as u64)
    }

    /// The offset into this buffer of any given coordinate
    #[inline]
    pub fn coord_offset(&self, i: u64) -> u64 {
        self.offset + 4 + (self.dim.size() as u64 * 8 * i)
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

impl<'a> LineStringTrait for WKBLinearRing<'a> {
    type CoordType<'b>
        = Coord<'a>
    where
        Self: 'b;

    #[inline]
    fn num_coords(&self) -> usize {
        self.num_points
    }

    #[inline]
    unsafe fn coord_unchecked(&self, i: usize) -> Self::CoordType<'_> {
        Coord::new(
            self.buf,
            self.byte_order,
            self.coord_offset(i.try_into().unwrap()),
            self.dim,
        )
    }
}
impl<'a> LineStringTrait for &WKBLinearRing<'a> {
    type CoordType<'c>
        = Coord<'a>
    where
        Self: 'c;

    #[inline]
    fn num_coords(&self) -> usize {
        self.num_points
    }

    #[inline]
    unsafe fn coord_unchecked(&self, i: usize) -> Self::CoordType<'_> {
        Coord::new(
            self.buf,
            self.byte_order,
            self.coord_offset(i.try_into().unwrap()),
            self.dim,
        )
    }
}

impl LineStringTraitExt for WKBLinearRing<'_> {
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

impl GeoTraitExtWithTypeTag for WKBLinearRing<'_> {
    type Tag = LineStringTag;
}

impl LineStringTraitExt for &WKBLinearRing<'_> {
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

impl GeoTraitExtWithTypeTag for &WKBLinearRing<'_> {
    type Tag = LineStringTag;
}
