use std::borrow::Cow;
use std::cmp::PartialEq;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek};
use std::str::FromStr;

use anyhow::anyhow;
use binrw::{binrw, binwrite, BinRead, BinResult, BinWrite, Endian, NullString};
use binrw::helpers::until_eof;
use bitflags::bitflags;

#[binrw]
#[derive(Debug, Clone)]
pub struct Guid([u8; 16]);

impl Display for Guid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let b = &self.0;
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            b[0], b[1], b[2], b[3],
            b[4], b[5],
            b[6], b[7],
            b[8], b[9],
            b[10], b[11], b[12], b[13], b[14], b[15]
        )
    }
}

impl FromStr for Guid {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.replace('-', "");
        if clean.len() != 32 {
            return Err(anyhow!("Invalid GUID length: {}", clean.len()));
        }
        if !clean.is_ascii() {
            return Err(anyhow!("Invalid characters in GUID"));
        }

        let mut out = [0u8; 16];
        for (i, byte) in out.iter_mut().enumerate() {
            let chunk = &clean[i * 2..i * 2 + 2];
            *byte = u8::from_str_radix(chunk, 16)
                .map_err(|e| anyhow!("Invalid hex at index {}: {}", i * 2, e))?;
        }

        Ok(Guid(out))
    }
}

#[binrw]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FString {
    #[bw(calc = string.len() as u32 + 1)]
    size: u32,
    #[br(map = |s: NullString| s.to_string(), assert(string.len() as u32 == size - 1), dbg)]
    #[bw(map = |s| NullString::from(s.as_str()))]
    string: String,
}

impl FString {
    pub const fn as_str(&self) -> &str {
        self.string.as_str()
    }

    pub const fn len(&self) -> usize {
        self.string.len()
    }

    pub const fn byte_size(&self) -> usize {
        // FIXME: this assumes that the string only contains 8-bit characters, but we don't enforce
        //  that for user input
        // +4 for length prefix, +1 for null terminator
        self.len() + 4 + 1
    }

    pub const fn as_mut(&mut self) -> &mut String {
        &mut self.string
    }
}

impl PartialEq<str> for FString {
    fn eq(&self, other: &str) -> bool {
        self.string == other
    }
}

impl PartialEq<&str> for FString {
    fn eq(&self, other: &&str) -> bool {
        self.string == *other
    }
}

impl PartialEq<String> for FString {
    fn eq(&self, other: &String) -> bool {
        self.string == *other
    }
}

impl Display for FString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.string)
    }
}

impl From<String> for FString {
    fn from(value: String) -> Self {
        Self {
            string: value,
        }
    }
}

impl From<&str> for FString {
    fn from(value: &str) -> Self {
        Self {
            string: value.into(),
        }
    }
}

#[binrw]
#[derive(Debug, Clone)]
pub struct CustomFormatEntry {
    pub guid: Guid,
    pub value: i32,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct CustomFormatData {
    pub version: i32,
    #[bw(calc = entries.len() as u32)]
    num_entries: u32,
    #[br(count = num_entries)]
    pub entries: Vec<CustomFormatEntry>,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct EngineVersion {
    pub major: i16,
    pub minor: i16,
    pub patch: i16,
    pub build: i32,
    pub build_id: FString,
}

#[binrw]
#[derive(Debug, Clone)]
#[brw(magic = b"GVAS")]
pub struct SaveGameHeader {
    pub save_game_version: i32,
    pub package_version: (i32, i32),
    pub engine_version: EngineVersion,
}

#[derive(Debug, Clone)]
pub struct PropertyValueArgs<'a> {
    property_type: &'a PropertyType,
    flags: u8,
    data_size: u32,
}

impl<'a> PropertyValueArgs<'a> {
    pub const fn new(property_type: &'a PropertyType, flags: u8, data_size: u32) -> Self {
        Self { property_type, flags, data_size }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TextFlags: u32 {
        const TRANSIENT = 0x00000001;
        const CULTURE_INVARIANT = 0x00000002;
        const CONVERTED_PROPERTY = 0x00000004;
        const IMMUTABLE = 0x00000008;
        const INITIALIZED_FROM_STRING = 0x00000010;

        const _ = !0;
    }
}

#[binrw]
#[derive(Debug, Clone)]
pub enum TextData {
    #[brw(magic = -1i8)]
    None {
        #[bw(calc = values.len() as u32)]
        count: u32,
        #[br(count = count)]
        values: Vec<FString>,
    },
    #[brw(magic = 0i8)]
    Base {
        namespace: FString,
        key: FString,
        source_string: FString,
    },
    #[brw(magic = 9i8)]
    AsDateTime {
        ticks: i64,
        date_style: i8,
        time_style: i8,
        time_zone: FString,
        culture_name: FString,
    },
    #[brw(magic = 11i8)]
    StringTableEntry {
        table: FString,
        key: FString,
    },
}

impl TextData {
    pub fn size(&self) -> usize {
        // +1 for magic
        1 + match self {
            Self::None { values } => 4 + values.iter().map(FString::byte_size).sum::<usize>(),
            Self::Base { namespace, key, source_string } => namespace.byte_size() + key.byte_size() + source_string.byte_size(),
            Self::AsDateTime { time_zone, culture_name, .. } => 10 + time_zone.byte_size() + culture_name.byte_size(),
            Self::StringTableEntry { table, key } => table.byte_size() + key.byte_size(),
        }
    }
}

#[binwrite]
#[derive(Debug, Clone)]
pub enum PropertyValue {
    StrProperty(FString),
    // FIXME: bool value appears to be stored in the flag byte and have no data component
    BoolProperty(#[bw(map = |b| *b as u8)] bool),
    ByteProperty(u8),
    IntProperty(i32),
    FloatProperty(f32),
    DoubleProperty(f64),
    TextProperty {
        #[bw(map = TextFlags::bits)]
        flags: TextFlags,
        data: TextData,
    },
    EnumProperty(FString),
    NameProperty(FString),
    StructProperty(Vec<Property>),
    ArrayProperty {
        #[bw(calc = values.len() as u32)]
        count: u32,
        values: Vec<PropertyValue>,
    },
    UnknownProperty(Vec<u8>),
}

impl PropertyValue {
    pub fn size(&self) -> usize {
        match self {
            Self::StrProperty(s) | Self::EnumProperty(s) | Self::NameProperty(s) => s.byte_size(),
            Self::BoolProperty(_) => 0,
            Self::ByteProperty(_) => 1,
            Self::IntProperty(_) | Self::FloatProperty(_) => 4,
            Self::DoubleProperty(_) => 8,
            Self::TextProperty { data, .. } => 4 + data.size(),
            Self::StructProperty(props) => props.iter().map(Property::size).sum::<usize>(),
            Self::ArrayProperty { values } => 4 + values.iter().map(PropertyValue::size).sum::<usize>(),
            Self::UnknownProperty(v) => v.len(),
        }
    }
}

impl BinRead for PropertyValue {
    type Args<'a> = PropertyValueArgs<'a>;

    fn read_options<R: Read + Seek>(reader: &mut R, endian: Endian, args: Self::Args<'_>) -> BinResult<Self> {
        let type_name = args.property_type.name.as_str();
        let start = reader.stream_position()?;
        let end = start + args.data_size as u64;

        let value = match type_name {
            "StrProperty" => Self::StrProperty(FString::read_options(reader, endian, ())?),
            "BoolProperty" => {
                Self::BoolProperty(if args.data_size == 0 {
                    args.flags & 0xf0 != 0
                } else {
                    u8::read_options(reader, endian, ())? != 0
                })
            }
            "ByteProperty" => Self::ByteProperty(u8::read_options(reader, endian, ())?),
            "IntProperty" => Self::IntProperty(i32::read_options(reader, endian, ())?),
            "FloatProperty" => Self::FloatProperty(f32::read_options(reader, endian, ())?),
            "DoubleProperty" => Self::DoubleProperty(f64::read_options(reader, endian, ())?),
            "TextProperty" => {
                // unwrap is safe because we used the unnamed field trick to make all bits legal
                let flags = TextFlags::from_bits(u32::read_options(reader, endian, ())?).unwrap();
                let data = TextData::read_options(reader, endian, ())?;
                Self::TextProperty { flags, data }
            }
            "EnumProperty" => Self::EnumProperty(FString::read_options(reader, endian, ())?),
            "NameProperty" => Self::NameProperty(FString::read_options(reader, endian, ())?),
            "StructProperty" => {
                // non-zero flags (or possibly just 08) seems to indicate types that don't have explicit field descriptions
                if args.flags != 0 {
                    let mut buf = vec![0u8; args.data_size as usize];
                    reader.read_exact(&mut buf)?;
                    Self::UnknownProperty(buf)
                } else {
                    let mut props = Vec::new();

                    while reader.stream_position()? < end {
                        props.push(Property::read_options(reader, endian, ())?);
                    }

                    Self::StructProperty(props)
                }
            }
            "ArrayProperty" => {
                let element_type = args.property_type.element_type().into_owned();
                let count = u32::read_options(reader, endian, ())? as usize;
                if element_type.name == "ByteProperty" {
                    // as an optimization, if this is an array of bytes, read them all at once instead of reading a huge
                    // number of individual ByteProperty's
                    let mut buf = vec![0u8; count];
                    reader.read_exact(&mut buf)?;
                    Self::ArrayProperty { values: vec![Self::UnknownProperty(buf)] }
                } else {
                    let mut values = Vec::with_capacity(count);
                    for _ in 0..count {
                        // FIXME: size calculation here is a problem for structs because we don't know where each
                        //  element should end
                        let current = reader.stream_position()?;
                        let remaining_size = (end - current) as u32;
                        values.push(PropertyValue::read_options(reader, endian, PropertyValueArgs::new(&element_type, args.flags, remaining_size))?);
                    }
                    Self::ArrayProperty { values }
                }
            }
            _ => {
                let mut buf = vec![0u8; args.data_size as usize];
                reader.read_exact(&mut buf)?;
                Self::UnknownProperty(buf)
            }
        };

        let current = reader.stream_position()?;
        if current != end {
            // TODO: make this an error, not a panic
            panic!("property value size mismatch: expected {} bytes, got {}", args.data_size, current);
        }

        Ok(value)
    }
}

#[binrw]
#[derive(Debug, Clone)]
pub struct TypeTag {
    pub kind: u32,
    pub value: FString,
}

impl TypeTag {
    pub const fn size(&self) -> usize {
        4 + self.value.byte_size()
    }
}

#[binrw::parser(reader, endian)]
fn read_tags() -> BinResult<Vec<TypeTag>> {
    let mut tags = Vec::new();
    loop {
        let kind = u32::read_options(reader, endian, ())?;
        if kind == 0 {
            break;
        }

        tags.push(TypeTag {
            kind,
            value: FString::read_options(reader, endian, ())?,
        });
    }

    Ok(tags)
}

#[binrw::writer(writer, endian)]
fn write_tags(tags: &Vec<TypeTag>) -> BinResult<()> {
    for tag in tags {
        tag.write_options(writer, endian, ())?;
    }
    0u32.write_options(writer, endian, ())
}

#[binrw]
#[derive(Debug, Clone)]
pub struct PropertyType {
    pub name: FString,
    #[br(parse_with = read_tags)]
    #[bw(write_with = write_tags)]
    pub tags: Vec<TypeTag>,
}

impl PropertyType {
    pub fn has_inner_type(&self) -> bool {
        self.name == "EnumProperty" || (self.name == "ArrayProperty" && matches!(self.tags.first(), Some(tag) if tag.value == "EnumProperty"))
    }

    fn describe_by_name(desc: &mut String, name: &str, tags: &[TypeTag]) {
        desc.push_str(name);

        if (name == "StructProperty" || name == "EnumProperty") && !tags.is_empty() {
            desc.push_str("<");
            if let Some(namespace) = tags.get(1) {
                desc.push_str(namespace.value.as_str());
                desc.push_str(".");
            }
            desc.push_str(tags.first().unwrap().value.as_str());
            desc.push_str(">");
        }

        if name == "ArrayProperty" && !tags.is_empty() {
            desc.push_str("[");
            let inner_type = tags.first().unwrap().value.as_str();
            Self::describe_by_name(desc, inner_type, &tags[1..]);
            desc.push_str("]");
        }
    }

    pub fn describe(&self) -> String {
        let mut desc = String::new();
        Self::describe_by_name(&mut desc, self.name.as_str(), &self.tags);
        desc
    }

    pub fn size(&self) -> usize {
        self.name.byte_size() + self.tags.iter().map(TypeTag::size).sum::<usize>()
    }

    pub fn element_type(&self) -> Cow<'_, Self> {
        match self.name.as_str() {
            "ArrayProperty" if !self.tags.is_empty() => {
                let name = self.tags[0].value.clone();
                let tags = self.tags[1..].to_vec();
                Cow::Owned(Self { name, tags })
            }
            _ => Cow::Borrowed(self),
        }
    }
}

#[binrw]
#[derive(Debug, Clone)]
pub struct PropertyBody {
    pub property_type: PropertyType,
    #[br(if(property_type.has_inner_type()))]
    pub inner_type: Option<PropertyType>,
    #[bw(calc = value.size() as u32)]
    data_size: u32,
    pub flags: u8,
    #[br(args_raw(PropertyValueArgs::new(&property_type, flags, data_size)))]
    pub value: PropertyValue,
}

impl PropertyBody {
    pub fn size(&self) -> usize {
        self.property_type.size() + self.inner_type.as_ref().map(PropertyType::size).unwrap_or(0) + 4 + 1 + self.value.size()
    }
}

#[binrw]
#[derive(Debug, Clone)]
pub struct Property {
    pub name: FString,
    #[br(if(name != "None" && name != ""))]
    pub body: Option<PropertyBody>,
}

impl Property {
    pub fn size(&self) -> usize {
        self.name.byte_size() + self.body.as_ref().map(PropertyBody::size).unwrap_or(0)
    }
}

#[binrw::parser(reader, endian)]
fn read_properties_until_eof() -> BinResult<Vec<Property>> {
    let mut props = Vec::new();

    loop {
        match Property::read_options(reader, endian, ()) {
            Ok(prop) => props.push(prop),
            Err(e) if e.is_eof() => {
                println!("{:06X}:\n{}", reader.stream_position()?, e);
                break
            }
            Err(e) => return Err(e),
        }
    }

    Ok(props)
}

#[binrw]
#[derive(Debug, Clone)]
pub struct SaveGameData {
    pub type_name: FString,
    pub flags: u8,
    #[br(parse_with = read_properties_until_eof)]
    pub properties: Vec<Property>,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct SaveGame {
    pub header: SaveGameHeader,
    pub custom_format_data: CustomFormatData,
    pub save_data: SaveGameData,
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;
    use binrw::{BinReaderExt, BinWriterExt};

    #[test]
    fn test_guid_string() {
        let guid = Guid([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10]);
        let s = "01020304-0506-0708-090a-0b0c0d0e0f10";
        assert_eq!(guid.to_string(), s);

        let parsed = Guid::from_str(s).unwrap();
        assert_eq!(parsed.0, guid.0);
    }

    #[test]
    fn test_fstring_read() {
        let data = b"\x0D\x00\x00\x00Hello World!\x00";
        let mut reader = Cursor::new(data);
        let fstr: FString = reader.read_le().unwrap();
        assert_eq!(fstr.to_string(), "Hello World!");
    }

    #[test]
    fn test_fstring_write() {
        let fstr: FString = "Hello World!".into();
        let mut data = Vec::<u8>::new();
        let mut writer = Cursor::new(&mut data);
        writer.write_le(&fstr).unwrap();
        assert_eq!(data, b"\x0D\x00\x00\x00Hello World!\x00");
    }
}