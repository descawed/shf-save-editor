use std::fmt::Debug;
use std::io::{Cursor, Read, Seek};
use std::str::FromStr;

use binrw::{binrw, BinRead, BinWrite, BinResult, Endian};

pub trait Stringable: ToString {
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

pub trait CoreUObject: Debug {
    fn fields_mut(&mut self) -> Vec<(&'static str, &mut dyn Stringable)>;

    fn size(&self) -> usize;

    // BinWrite is not dyn compatible, so we have to go through this wrapper
    fn to_bytes(&self, endian: Endian) -> BinResult<Vec<u8>>;
}

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

#[binrw]
#[derive(Debug, Clone, Copy)]
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

#[binrw]
#[derive(Debug, Clone, Copy)]
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

#[binrw]
#[derive(Debug, Clone)]
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

#[binrw]
#[derive(Debug, Clone)]
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

#[binrw]
#[derive(Debug, Clone)]
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