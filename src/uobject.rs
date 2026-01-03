use std::fmt::Debug;
use std::io::{Cursor, Read, Seek};
use std::str::FromStr;

use binrw::{binrw, BinRead, BinWrite, BinResult, Endian};

/// A type that can be both converted to and parsed from a string.
pub trait Stringable: ToString {
    /// Tries to set the value of this object from the given string representation.
    ///
    /// If the string cannot be parsed, the value is not updated, but there is no error.
    // FromStr is not dyn compatible, so we have to go through this wrapper
    fn try_set_from_str(&mut self, s: &str);
}

impl<T: ToString + FromStr> Stringable for T {
    fn try_set_from_str(&mut self, s: &str) {
        if let Ok(parsed) = s.parse::<T>() {
            *self = parsed;
        }
    }
}

/// A core Unreal Engine 5 type.
pub trait CoreUObject: Debug {
    /// A mutable list of fields in this object with their names.
    fn fields_mut(&mut self) -> Vec<(&'static str, &mut dyn Stringable)>;

    /// The size of this object in bytes.
    fn size(&self) -> usize;

    /// Converts this object to a byte vector.
    // BinWrite is not dyn compatible, so we have to go through this wrapper
    fn to_bytes(&self, endian: Endian) -> BinResult<Vec<u8>>;
}

/// Write a CoreUObject to a writer.
#[binrw::writer(writer, endian)]
pub fn write_uobject(object: &Box<dyn CoreUObject>) -> BinResult<()> {
    let bytes = object.to_bytes(endian)?;
    bytes.write_options(writer, endian, ())
}

fn uobject_to_bytes<'a, O: CoreUObject + BinWrite<Args<'a>=()>>(object: &O, endian: Endian) -> BinResult<Vec<u8>> {
    let mut buf = Vec::with_capacity(object.size());
    let mut writer = Cursor::new(&mut buf);
    object.write_options(&mut writer, endian, ())?;

    Ok(buf)
}

/// An Unreal Engine 5 DateTime.
///
/// Values of this type represent dates and times between Midnight 00:00:00, January 1, 0001 and
/// Midnight 23:59:59.9999999, December 31, 9999 in the Gregorian calendar. Internally, the time
/// values are stored in ticks of 0.1 microseconds (= 100 nanoseconds) since January 1, 0001.
#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct FDateTime(u64);

impl CoreUObject for FDateTime {
    fn fields_mut(&mut self) -> Vec<(&'static str, &mut dyn Stringable)> {
        vec![("Ticks", &mut self.0)]
    }

    fn size(&self) -> usize {
        8
    }

    fn to_bytes(&self, endian: Endian) -> BinResult<Vec<u8>> {
        uobject_to_bytes(self, endian)
    }
}

/// An Unreal Engine 5 Timespan.
///
/// A time span is the difference between two dates and times. For example, the time span between
/// 12:00:00 January 1, 2000 and 18:00:00 January 2, 2000 is 30.0 hours. Time spans are measured in
/// positive or negative ticks depending on whether the difference is measured forward or backward.
/// Each tick has a resolution of 0.1 microseconds (= 100 nanoseconds).
#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct FTimespan(u64);

impl CoreUObject for FTimespan {
    fn fields_mut(&mut self) -> Vec<(&'static str, &mut dyn Stringable)> {
        vec![("Ticks", &mut self.0)]
    }

    fn size(&self) -> usize {
        8
    }

    fn to_bytes(&self, endian: Endian) -> BinResult<Vec<u8>> {
        uobject_to_bytes(self, endian)
    }
}

/// An Unreal Engine 5 3D vector.
#[binrw]
#[derive(Debug, Clone, Default)]
pub struct Vector {
    x: f64,
    y: f64,
    z: f64,
}

impl CoreUObject for Vector {
    fn fields_mut(&mut self) -> Vec<(&'static str, &mut dyn Stringable)> {
        vec![("X", &mut self.x), ("Y", &mut self.y), ("Z", &mut self.z)]
    }

    fn size(&self) -> usize {
        24
    }

    fn to_bytes(&self, endian: Endian) -> BinResult<Vec<u8>> {
        uobject_to_bytes(self, endian)
    }
}

/// An Unreal Engine 5 quaternion.
#[binrw]
#[derive(Debug, Clone, Default)]
pub struct Quat {
    x: f64,
    y: f64,
    z: f64,
    w: f64,
}

impl CoreUObject for Quat {
    fn fields_mut(&mut self) -> Vec<(&'static str, &mut dyn Stringable)> {
        vec![("X", &mut self.x), ("Y", &mut self.y), ("Z", &mut self.z), ("W", &mut self.w)]
    }

    fn size(&self) -> usize {
        32
    }

    fn to_bytes(&self, endian: Endian) -> BinResult<Vec<u8>> {
        uobject_to_bytes(self, endian)
    }
}

/// An Unreal Engine 5 linear color (RGBA).
#[binrw]
#[derive(Debug, Clone, Default)]
pub struct LinearColor {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl CoreUObject for LinearColor {
    fn fields_mut(&mut self) -> Vec<(&'static str, &mut dyn Stringable)> {
        vec![("R", &mut self.r), ("G", &mut self.g), ("B", &mut self.b), ("A", &mut self.a)]
    }

    fn size(&self) -> usize {
        16
    }

    fn to_bytes(&self, endian: Endian) -> BinResult<Vec<u8>> {
        uobject_to_bytes(self, endian)
    }
}

/// Tries to read a UE5 object of the given type from a reader.
pub fn try_read_uobject<R: Read + Seek>(type_name: &str, reader: &mut R, endian: Endian) -> BinResult<Option<Box<dyn CoreUObject>>> {
    Ok(Some(match type_name {
        "DateTime" => Box::new(FDateTime::read_options(reader, endian, ())?),
        "Timespan" => Box::new(FTimespan::read_options(reader, endian, ())?),
        "Vector" => Box::new(Vector::read_options(reader, endian, ())?),
        "Quat" => Box::new(Quat::read_options(reader, endian, ())?),
        "LinearColor" => Box::new(LinearColor::read_options(reader, endian, ())?),
        _ => return Ok(None),
    }))
}

/// Creates a default value for a given Unreal Engine 5 type.
pub fn make_default_uobject(type_name: &str) -> Option<Box<dyn CoreUObject>> {
    match type_name {
        "DateTime" => Some(Box::new(FDateTime::default())),
        "Timespan" => Some(Box::new(FTimespan::default())),
        "Vector" => Some(Box::new(Vector::default())),
        "Quat" => Some(Box::new(Quat::default())),
        "LinearColor" => Some(Box::new(LinearColor::default())),
        _ => None,
    }
}