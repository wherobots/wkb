use crate::reader::{read_wkb, to_geos::convert};
use crate::writer::{
    write_geometry_collection, write_line_string, write_multi_line_string, write_multi_point,
    write_multi_polygon, write_point, write_polygon,
};
use crate::Endianness;
use geo_types::Geometry;
use geos::Geom;

use super::data::*;

fn test_geometry_conversion(geo_geom: &Geometry, endianness: Endianness) {
    // Convert geo geometry to WKB
    let mut buf = Vec::new();
    match geo_geom {
        Geometry::Point(p) => write_point(&mut buf, p, endianness).unwrap(),
        Geometry::LineString(ls) => write_line_string(&mut buf, ls, endianness).unwrap(),
        Geometry::Polygon(p) => write_polygon(&mut buf, p, endianness).unwrap(),
        Geometry::MultiPoint(mp) => write_multi_point(&mut buf, mp, endianness).unwrap(),
        Geometry::MultiLineString(mls) => {
            write_multi_line_string(&mut buf, mls, endianness).unwrap()
        }
        Geometry::MultiPolygon(mp) => write_multi_polygon(&mut buf, mp, endianness).unwrap(),
        Geometry::GeometryCollection(gc) => {
            write_geometry_collection(&mut buf, gc, endianness).unwrap()
        }
        Geometry::Line(_) => panic!("Line geometry not supported in tests"),
        Geometry::Rect(_) => panic!("Rect geometry not supported in tests"),
        Geometry::Triangle(_) => panic!("Triangle geometry not supported in tests"),
    }

    // Read WKB back
    let wkb = read_wkb(&buf).unwrap();

    // Convert to GEOS using our ToGeos converter
    let geos_geom = convert(&wkb).unwrap();

    // Convert back to geo for comparison
    let geo_from_geos: Geometry = geos_geom.try_into().unwrap();

    // Compare the geometries
    assert_eq!(*geo_geom, geo_from_geos);
}

#[test]
fn test_point_conversion() {
    let point = point_2d();
    let geo_geom = Geometry::Point(point);

    test_geometry_conversion(&geo_geom, Endianness::LittleEndian);
    test_geometry_conversion(&geo_geom, Endianness::BigEndian);
}

#[test]
fn test_empty_point_conversion() {
    // Create an empty point by writing NaN coordinates
    let mut buf = Vec::new();
    buf.push(1); // Little endian
    buf.extend_from_slice(&1u32.to_le_bytes()); // Point type
    buf.extend_from_slice(&f64::NAN.to_le_bytes()); // x = NaN
    buf.extend_from_slice(&f64::NAN.to_le_bytes()); // y = NaN

    let wkb = read_wkb(&buf).unwrap();
    let geos_geom = convert(&wkb).unwrap();

    assert!(geos_geom.is_empty().unwrap());
}

#[test]
fn test_line_string_conversion() {
    let line_string = linestring_2d();
    let geo_geom = Geometry::LineString(line_string);

    test_geometry_conversion(&geo_geom, Endianness::LittleEndian);
    test_geometry_conversion(&geo_geom, Endianness::BigEndian);
}

#[test]
fn test_empty_line_string_conversion() {
    let mut buf = Vec::new();
    write_line_string(
        &mut buf,
        &geo_types::LineString::new(vec![]),
        Endianness::LittleEndian,
    )
    .unwrap();

    let wkb = read_wkb(&buf).unwrap();
    let geos_geom = convert(&wkb).unwrap();

    assert!(geos_geom.is_empty().unwrap());
}

#[test]
fn test_polygon_conversion() {
    let polygon = polygon_2d();
    let geo_geom = Geometry::Polygon(polygon);

    test_geometry_conversion(&geo_geom, Endianness::LittleEndian);
    test_geometry_conversion(&geo_geom, Endianness::BigEndian);
}

#[test]
fn test_polygon_with_interior_conversion() {
    let polygon = polygon_2d_with_interior();
    let geo_geom = Geometry::Polygon(polygon);

    test_geometry_conversion(&geo_geom, Endianness::LittleEndian);
    test_geometry_conversion(&geo_geom, Endianness::BigEndian);
}

#[test]
fn test_multi_point_conversion() {
    let multi_point = multi_point_2d();
    let geo_geom = Geometry::MultiPoint(multi_point);

    test_geometry_conversion(&geo_geom, Endianness::LittleEndian);
    test_geometry_conversion(&geo_geom, Endianness::BigEndian);
}

#[test]
fn test_empty_multi_point_conversion() {
    let mut buf = Vec::new();
    write_multi_point(
        &mut buf,
        &geo_types::MultiPoint::new(vec![]),
        Endianness::LittleEndian,
    )
    .unwrap();

    let wkb = read_wkb(&buf).unwrap();
    let geos_geom = convert(&wkb).unwrap();

    assert!(geos_geom.is_empty().unwrap());
}

#[test]
fn test_multi_line_string_conversion() {
    let multi_line_string = multi_line_string_2d();
    let geo_geom = Geometry::MultiLineString(multi_line_string);

    test_geometry_conversion(&geo_geom, Endianness::LittleEndian);
    test_geometry_conversion(&geo_geom, Endianness::BigEndian);
}

#[test]
fn test_empty_multi_line_string_conversion() {
    let mut buf = Vec::new();
    write_multi_line_string(
        &mut buf,
        &geo_types::MultiLineString::new(vec![]),
        Endianness::LittleEndian,
    )
    .unwrap();

    let wkb = read_wkb(&buf).unwrap();
    let geos_geom = convert(&wkb).unwrap();

    assert!(geos_geom.is_empty().unwrap());
}

#[test]
fn test_multi_polygon_conversion() {
    let multi_polygon = multi_polygon_2d();
    let geo_geom = Geometry::MultiPolygon(multi_polygon);

    test_geometry_conversion(&geo_geom, Endianness::LittleEndian);
    test_geometry_conversion(&geo_geom, Endianness::BigEndian);
}

#[test]
fn test_empty_multi_polygon_conversion() {
    let mut buf = Vec::new();
    write_multi_polygon(
        &mut buf,
        &geo_types::MultiPolygon::new(vec![]),
        Endianness::LittleEndian,
    )
    .unwrap();

    let wkb = read_wkb(&buf).unwrap();
    let geos_geom = convert(&wkb).unwrap();

    assert!(geos_geom.is_empty().unwrap());
}

#[test]
fn test_geometry_collection_conversion() {
    let geometry_collection = geometry_collection_2d();
    let geo_geom = Geometry::GeometryCollection(geometry_collection);

    test_geometry_conversion(&geo_geom, Endianness::LittleEndian);
    test_geometry_conversion(&geo_geom, Endianness::BigEndian);
}

#[test]
fn test_empty_geometry_collection_conversion() {
    let mut buf = Vec::new();
    write_geometry_collection(
        &mut buf,
        &geo_types::GeometryCollection::new_from(vec![]),
        Endianness::LittleEndian,
    )
    .unwrap();

    let wkb = read_wkb(&buf).unwrap();
    let geos_geom = convert(&wkb).unwrap();

    assert!(geos_geom.is_empty().unwrap());
}

#[test]
fn test_nested_geometry_collection() {
    // Create a geometry collection containing other geometry collections
    let inner_gc1 = geo_types::GeometryCollection::new_from(vec![
        Geometry::Point(point_2d()),
        Geometry::LineString(linestring_2d()),
    ]);

    let inner_gc2 = geo_types::GeometryCollection::new_from(vec![
        Geometry::Polygon(polygon_2d()),
        Geometry::MultiPoint(multi_point_2d()),
    ]);

    let outer_gc = geo_types::GeometryCollection::new_from(vec![
        Geometry::GeometryCollection(inner_gc1),
        Geometry::GeometryCollection(inner_gc2),
        Geometry::MultiLineString(multi_line_string_2d()),
    ]);

    let geo_geom = Geometry::GeometryCollection(outer_gc);

    test_geometry_conversion(&geo_geom, Endianness::LittleEndian);
    test_geometry_conversion(&geo_geom, Endianness::BigEndian);
}

#[test]
fn test_coordinate_precision() {
    // Test with high precision coordinates
    let high_precision_point = geo_types::Point::new(123.456789012345, -98.765432109876);
    let geo_geom = Geometry::Point(high_precision_point);

    test_geometry_conversion(&geo_geom, Endianness::LittleEndian);
    test_geometry_conversion(&geo_geom, Endianness::BigEndian);
}

#[test]
fn test_large_coordinates() {
    // Test with very large coordinate values
    let large_point = geo_types::Point::new(1e10, -1e10);
    let geo_geom = Geometry::Point(large_point);

    test_geometry_conversion(&geo_geom, Endianness::LittleEndian);
    test_geometry_conversion(&geo_geom, Endianness::BigEndian);
}

#[test]
fn test_negative_coordinates() {
    // Test with negative coordinates
    let negative_point = geo_types::Point::new(-180.0, -90.0);
    let geo_geom = Geometry::Point(negative_point);

    test_geometry_conversion(&geo_geom, Endianness::LittleEndian);
    test_geometry_conversion(&geo_geom, Endianness::BigEndian);
}

#[test]
fn test_zero_coordinates() {
    // Test with zero coordinates
    let zero_point = geo_types::Point::new(0.0, 0.0);
    let geo_geom = Geometry::Point(zero_point);

    test_geometry_conversion(&geo_geom, Endianness::LittleEndian);
    test_geometry_conversion(&geo_geom, Endianness::BigEndian);
}

#[test]
fn test_endianness_handling() {
    // Test that both endianness variants work correctly
    let point = point_2d();
    let geo_geom = Geometry::Point(point);

    // Test little endian
    let mut buf_le = Vec::new();
    write_point(&mut buf_le, &point_2d(), Endianness::LittleEndian).unwrap();
    let wkb_le = read_wkb(&buf_le).unwrap();
    let geos_geom_le = convert(&wkb_le).unwrap();
    let geo_from_geos_le: Geometry = geos_geom_le.try_into().unwrap();

    // Test big endian
    let mut buf_be = Vec::new();
    write_point(&mut buf_be, &point_2d(), Endianness::BigEndian).unwrap();
    let wkb_be = read_wkb(&buf_be).unwrap();
    let geos_geom_be = convert(&wkb_be).unwrap();
    let geo_from_geos_be: Geometry = geos_geom_be.try_into().unwrap();

    // Both should produce the same result
    assert_eq!(geo_from_geos_le, geo_from_geos_be);
    assert_eq!(geo_geom, geo_from_geos_le);
}

#[test]
fn test_xyz_dimension_handling() {
    // Test XYZ dimension handling by manually creating WKB with XYZ coordinates
    let mut buf = Vec::new();

    // Write WKB header for LineString XYZ (type 1002)
    buf.push(1); // Little endian
    buf.extend_from_slice(&1002u32.to_le_bytes()); // LineString XYZ
    buf.extend_from_slice(&2u32.to_le_bytes()); // 2 points

    // Write XYZ coordinates: (0.0, 1.0, 10.0), (1.0, 2.0, 20.0)
    buf.extend_from_slice(&0.0f64.to_le_bytes());
    buf.extend_from_slice(&1.0f64.to_le_bytes());
    buf.extend_from_slice(&10.0f64.to_le_bytes());
    buf.extend_from_slice(&1.0f64.to_le_bytes());
    buf.extend_from_slice(&2.0f64.to_le_bytes());
    buf.extend_from_slice(&20.0f64.to_le_bytes());

    let wkb = read_wkb(&buf).unwrap();
    let geos_geom = convert(&wkb).unwrap();

    // Verify the geometry was created successfully
    assert!(!geos_geom.is_empty().unwrap());

    // Verify coordinates by checking the WKT representation
    let wkt = geos_geom.to_wkt().unwrap();
    // Expected WKT for LineString with XYZ coordinates (0.0, 1.0, 10.0), (1.0, 2.0, 20.0)
    let expected_wkt = "LINESTRING Z (0 1 10, 1 2 20)";
    assert_eq!(wkt, expected_wkt);
}

#[test]
fn test_xym_dimension_handling() {
    // Test XYM dimension handling by manually creating WKB with XYM coordinates
    let mut buf = Vec::new();

    // Write WKB header for LineString XYM (type 2002)
    buf.push(1); // Little endian
    buf.extend_from_slice(&2002u32.to_le_bytes()); // LineString XYM
    buf.extend_from_slice(&2u32.to_le_bytes()); // 2 points

    // Write XYM coordinates: (0.0, 1.0, 100.0), (1.0, 2.0, 200.0)
    buf.extend_from_slice(&0.0f64.to_le_bytes());
    buf.extend_from_slice(&1.0f64.to_le_bytes());
    buf.extend_from_slice(&100.0f64.to_le_bytes());
    buf.extend_from_slice(&1.0f64.to_le_bytes());
    buf.extend_from_slice(&2.0f64.to_le_bytes());
    buf.extend_from_slice(&200.0f64.to_le_bytes());

    let wkb = read_wkb(&buf).unwrap();
    let geos_geom = convert(&wkb).unwrap();

    // Verify the geometry was created successfully
    assert!(!geos_geom.is_empty().unwrap());

    // Verify coordinates by checking the WKT representation
    let wkt = geos_geom.to_wkt().unwrap();
    // Expected WKT for LineString with XYM coordinates (0.0, 1.0, 100.0), (1.0, 2.0, 200.0)
    let expected_wkt = "LINESTRING M (0 1 100, 1 2 200)";
    assert_eq!(wkt, expected_wkt);
}

#[test]
fn test_xyzm_dimension_handling() {
    // Test XYZM dimension handling by manually creating WKB with XYZM coordinates
    let mut buf = Vec::new();

    // Write WKB header for LineString XYZM (type 3002)
    buf.push(1); // Little endian
    buf.extend_from_slice(&3002u32.to_le_bytes()); // LineString XYZM
    buf.extend_from_slice(&2u32.to_le_bytes()); // 2 points

    // Write XYZM coordinates: (0.0, 1.0, 10.0, 100.0), (1.0, 2.0, 20.0, 200.0)
    buf.extend_from_slice(&0.0f64.to_le_bytes());
    buf.extend_from_slice(&1.0f64.to_le_bytes());
    buf.extend_from_slice(&10.0f64.to_le_bytes());
    buf.extend_from_slice(&100.0f64.to_le_bytes());
    buf.extend_from_slice(&1.0f64.to_le_bytes());
    buf.extend_from_slice(&2.0f64.to_le_bytes());
    buf.extend_from_slice(&20.0f64.to_le_bytes());
    buf.extend_from_slice(&200.0f64.to_le_bytes());

    let wkb = read_wkb(&buf).unwrap();
    let geos_geom = convert(&wkb).unwrap();

    // Verify the geometry was created successfully
    assert!(!geos_geom.is_empty().unwrap());

    // Verify coordinates by checking the WKT representation
    let wkt = geos_geom.to_wkt().unwrap();
    // Expected WKT for LineString with XYZM coordinates (0.0, 1.0, 10.0, 100.0), (1.0, 2.0, 20.0, 200.0)
    let expected_wkt = "LINESTRING ZM (0 1 10 100, 1 2 20 200)";
    assert_eq!(wkt, expected_wkt);
}

#[test]
fn test_big_endian_xyz_dimension_handling() {
    // Test XYZ dimension handling with big endian byte order
    let mut buf = Vec::new();

    // Write WKB header for LineString XYZ (type 1002) in big endian
    buf.push(0); // Big endian
    buf.extend_from_slice(&1002u32.to_be_bytes()); // LineString XYZ
    buf.extend_from_slice(&2u32.to_be_bytes()); // 2 points

    // Write XYZ coordinates in big endian: (0.0, 1.0, 10.0), (1.0, 2.0, 20.0)
    buf.extend_from_slice(&0.0f64.to_be_bytes());
    buf.extend_from_slice(&1.0f64.to_be_bytes());
    buf.extend_from_slice(&10.0f64.to_be_bytes());
    buf.extend_from_slice(&1.0f64.to_be_bytes());
    buf.extend_from_slice(&2.0f64.to_be_bytes());
    buf.extend_from_slice(&20.0f64.to_be_bytes());

    let wkb = read_wkb(&buf).unwrap();
    let geos_geom = convert(&wkb).unwrap();

    // Verify the geometry was created successfully
    assert!(!geos_geom.is_empty().unwrap());

    // Verify coordinates by checking the WKT representation
    let wkt = geos_geom.to_wkt().unwrap();
    // Expected WKT for LineString with XYZ coordinates (0.0, 1.0, 10.0), (1.0, 2.0, 20.0)
    let expected_wkt = "LINESTRING Z (0 1 10, 1 2 20)";
    assert_eq!(wkt, expected_wkt);
}
