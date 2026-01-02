use std::borrow::Cow;
use std::cmp::PartialEq;
use std::fmt::{Display, Formatter};
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::str::FromStr;

use anyhow::anyhow;
use binrw::{binrw, binwrite, BinRead, BinReaderExt, BinResult, BinWrite, Endian, NullString};
use bitflags::bitflags;

use crate::uobject::*;

const CUSTOM_STRUCT_CLASSES: [(&'static str, usize); 27] = [
    ("/Script/GameNoce.NocePlayerInventoryComponent", 8),
    // there are blueprint records inside this object that I don't know how to parse
    // "/Script/GameNoce.NoceInteractableBase",
    ("/Script/GameNoce.NocePlayerTriggerBase", 8),
    ("/Script/GameNoce.NocePlayerCharacter", 8),
    ("/Script/GameNoce.NocePlayerState", 8),
    ("/Script/GameNoce.NoceBodyPartGroupComponent", 8),
    ("/Script/GameNoce.NoceEnemyCharacter", 8),
    ("/Script/GameNoce.NoceMapIconComponent", 8),
    ("/Script/Engine.ActorComponent", 8),
    ("/Script/GameNoce.NoceEnvironmentSubsystem", 4),
    ("/Script/GameNoce.NoceWorldManagerSubsystem", 4),
    ("/Script/GameNoce.MucusSubsystem", 4),
    ("/Script/GameNoce.NoceAchievementSubsystem", 4),
    ("/Script/GameNoce.NoceActivitySubsystem", 4),
    ("/Script/GameNoce.NoceItemSubsystem", 4),
    ("/Script/GameNoce.NoceOmamoriDrawingSubsystem", 4),
    ("/Script/GameNoce.NocePickupsHelperSubsystem", 4),
    ("/Script/GameNoce.NoceTutorialSubsystem", 4),
    ("/Script/GameNoce.NoceAISystem", 4),
    ("/Script/GameNoce.NoceDialogSubsystem", 4),
    ("/Script/GameNoce.NoceGameClockSubsystem", 4),
    ("/Script/GameNoce.NoceBinkSubsystem", 4),
    ("/Script/GameNoce.NoceHitPerformDataSubsystem", 4),
    ("/Script/GameNoce.NocePlayerLookAtSubsystem", 4),
    ("/Script/GameNoce.NoceTentacleSubsystem", 4),
    ("/Script/GameNoce.NoceUIMissionSubsystem", 4),
    ("/Script/GameNoce.NoceBattlePositionSubsystem", 4),
    ("/Script/GameNoce.NocePickupsSubsystem", 4),
];
const GAMEPLAY_TAG_CONTAINER_TYPE: &str = "StructProperty</Script/GameplayTags.GameplayTagContainer>";
const CORE_UOBJECT_TYPE_PREFIX: &str = "StructProperty</Script/CoreUObject.";

/// A save object containing other objects which can be accessed by name or numeric index
pub trait Indexable {
    fn get_key(&self, key: &str) -> Option<&PropertyValue>;
    fn get_key_mut(&mut self, key: &str) -> Option<&mut PropertyValue>;
    fn get_index(&self, index: usize) -> Option<&PropertyValue>;
    fn get_index_mut(&mut self, index: usize) -> Option<&mut PropertyValue>;
}

/// A type that can be used to index into a save object
pub trait PropertyIndex {
    fn get_from<'a, I: Indexable>(&self, indexable: &'a I) -> Option<&'a PropertyValue>;
    fn get_from_mut<'a, I: Indexable>(&self, indexable: &'a mut I) -> Option<&'a mut PropertyValue>;
}

impl PropertyIndex for &str {
    fn get_from<'a, I: Indexable>(&self, indexable: &'a I) -> Option<&'a PropertyValue> {
        indexable.get_key(self)
    }

    fn get_from_mut<'a, I: Indexable>(&self, indexable: &'a mut I) -> Option<&'a mut PropertyValue> {
        indexable.get_key_mut(self)
    }
}

impl PropertyIndex for usize {
    fn get_from<'a, I: Indexable>(&self, indexable: &'a I) -> Option<&'a PropertyValue> {
        indexable.get_index(*self)
    }

    fn get_from_mut<'a, I: Indexable>(&self, indexable: &'a mut I) -> Option<&'a mut PropertyValue> {
        indexable.get_index_mut(*self)
    }
}

macro_rules! prop {
    // first index is treated specially because we're not necessarily dealing with a PropertyValue
    // at that point
    ($obj:expr, [$idx1:expr] $( [$idx:expr] )* ) => {{
        let mut cur = $idx1.get_from($obj);
        $(
            cur = match cur {
                Some(p) => $idx.get_from(p),
                None => None,
            };
        )*
        cur
    }};
}

pub(crate) use prop;

macro_rules! prop_mut {
    // first index is treated specially because we're not necessarily dealing with a PropertyValue
    // at that point
    ($obj:expr, [$idx1:expr] $( [$idx:expr] )* ) => {{
        let mut cur = $idx1.get_from_mut($obj);
        $(
            cur = match cur {
                Some(p) => $idx.get_from_mut(p),
                None => None,
            };
        )*
        cur
    }};
}

pub(crate) use prop_mut;

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
    #[br(map = |s: NullString| s.to_string(), assert(string.len() as u32 == size - 1))]
    #[bw(map = |s| NullString::from(s.as_str()))]
    string: String,
}

impl FString {
    pub const fn new() -> Self {
        Self { string: String::new() }
    }

    pub fn from_str(s: &str) -> Self {
        Self { string: s.to_string() }
    }

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

#[binrw::parser(reader, endian)]
fn read_properties_with_footer(footer_size: u64) -> BinResult<Vec<Property>> {
    let mut props = Vec::new();

    let start = reader.stream_position()?;
    reader.seek(SeekFrom::End(0))?;
    let eof = reader.stream_position()?;
    reader.seek(SeekFrom::Start(start))?;

    let end = eof - footer_size;

    while reader.stream_position()? < end {
        match Property::read_options(reader, endian, ()) {
            Ok(prop) => props.push(prop),
            Err(e) if e.is_eof() && footer_size == 0 => break,
            Err(e) => return Err(e),
        }
    }

    Ok(props)
}

#[binrw]
#[derive(Debug)]
#[br(import(extra_bytes: usize))]
pub struct CustomStruct {
    // this field is ignored on read because it's read as part of the ArrayProperty before we detect
    // whether the array contents are a custom struct or not
    #[br(ignore)]
    #[bw(calc = self.size() as u32 - 4)]
    data_size: u32,
    pub flags: u8,
    #[br(parse_with = |r, e, _: ()| read_properties_with_footer(r, e, (extra_bytes as u64,)))]
    pub properties: Vec<Property>,
    #[br(count = extra_bytes)]
    pub extra: Vec<u8>,
}

impl CustomStruct {
    pub fn size(&self) -> usize {
        4 + 1 + self.properties.iter().map(Property::size).sum::<usize>() + self.extra.len()
    }
}

pub const SCALAR_TYPE_NAMES: [&str; 8] = [
    "BoolProperty",
    "ByteProperty",
    "IntProperty",
    "FloatProperty",
    "DoubleProperty",
    "StrProperty",
    "ObjectProperty",
    "NameProperty",
];

#[binwrite]
#[derive(Debug)]
pub enum PropertyValue {
    StrProperty(FString),
    BoolProperty(#[bw(map = |b| b.map(|b| b as u8))] Option<bool>),
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
    ObjectProperty(FString),
    StructProperty(Vec<Property>),
    CustomStructProperty(CustomStruct),
    CoreUObjectStructProperty(#[bw(write_with = write_uobject)] Box<dyn CoreUObject>),
    ArrayProperty {
        #[bw(calc = self.array_len().unwrap() as u32)]
        count: u32,
        values: Vec<PropertyValue>,
    },
    MapProperty {
        removed_count: u32,
        #[bw(calc = self.array_len().unwrap() as u32)]
        count: u32,
        values: Vec<(PropertyValue, PropertyValue)>,
    },
    UnknownProperty(Vec<u8>),
}

impl PropertyValue {
    pub fn default_for_type(type_name: &str) -> Self {
        match type_name {
            "BoolProperty" => Self::BoolProperty(None),
            "ByteProperty" => Self::ByteProperty(0),
            "IntProperty" => Self::IntProperty(0),
            "FloatProperty" => Self::FloatProperty(0.0),
            "DoubleProperty" => Self::DoubleProperty(0.0),
            "StrProperty" => Self::StrProperty(FString::new()),
            "ObjectProperty" => Self::ObjectProperty(FString::new()),
            "NameProperty" => Self::NameProperty(FString::new()),
            "EnumProperty" => Self::EnumProperty(FString::new()),
            "TextProperty" => Self::TextProperty {
                flags: TextFlags::empty(),
                data: TextData::None {
                    values: Vec::new(),
                },
            },
            "StructProperty" => Self::StructProperty(Vec::new()),
            "ArrayProperty" => Self::ArrayProperty { values: Vec::new() },
            "MapProperty" => Self::MapProperty { removed_count: 0, values: Vec::new() },
            _ => Self::UnknownProperty(Vec::new()),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Self::StrProperty(s) | Self::EnumProperty(s) | Self::NameProperty(s) | Self::ObjectProperty(s) => s.byte_size(),
            Self::BoolProperty(None) => 0,
            Self::ByteProperty(_) | Self::BoolProperty(Some(_)) => 1,
            Self::IntProperty(_) | Self::FloatProperty(_) => 4,
            Self::DoubleProperty(_) => 8,
            Self::TextProperty { data, .. } => 4 + data.size(),
            Self::StructProperty(props) => props.iter().map(Property::size).sum::<usize>(),
            Self::CustomStructProperty(s) => s.size(),
            Self::CoreUObjectStructProperty(s) => s.size(),
            Self::ArrayProperty { values } => 4 + values.iter().map(PropertyValue::size).sum::<usize>(),
            Self::MapProperty { values, .. } => 8 + values.iter().map(|(k, v)| k.size() + v.size()).sum::<usize>(),
            Self::UnknownProperty(v) => v.len(),
        }
    }

    pub fn array_len(&self) -> Option<usize> {
        Some(match self {
            // custom structs are encoded as byte arrays
            Self::CustomStructProperty(s) => s.size(),
            Self::MapProperty { values, .. } => values.len(),
            Self::ArrayProperty { values } => {
                let num_values = values.len();
                // as an optimization, we read byte arrays as a single UnknownProperty instead of a
                // series of ByteProperties
                if num_values == 1 && let Some(Self::UnknownProperty(buf)) = values.first() {
                    buf.len()
                } else {
                    num_values
                }
            }
            _ => return None,
        })
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::StrProperty(_) => "StrProperty",
            Self::BoolProperty(_) => "BoolProperty",
            Self::ByteProperty(_) => "ByteProperty",
            Self::IntProperty(_) => "IntProperty",
            Self::FloatProperty(_) => "FloatProperty",
            Self::DoubleProperty(_) => "DoubleProperty",
            Self::TextProperty { .. } => "TextProperty",
            Self::EnumProperty(_) => "EnumProperty",
            Self::NameProperty(_) => "NameProperty",
            Self::ObjectProperty(_) => "ObjectProperty",
            Self::StructProperty(_) | Self::CoreUObjectStructProperty(_) => "StructProperty",
            Self::ArrayProperty { .. } | Self::CustomStructProperty(_) => "ArrayProperty",
            Self::MapProperty { .. } => "MapProperty",
            Self::UnknownProperty(_) => "",
        }
    }
}

impl Indexable for PropertyValue {
    fn get_key(&self, key: &str) -> Option<&Self> {
        match self {
            Self::StructProperty(props) => props.iter().find_map(|p| if p.name == key {
                p.body.as_ref().map(|b| &b.value)
            } else {
                None
            }),
            Self::CustomStructProperty(s) => s.properties.iter().find_map(|p| if p.name == key {
                p.body.as_ref().map(|b| &b.value)
            } else {
                None
            }),
            Self::MapProperty { values, .. } => values.iter().find_map(|(k, v)| (k == key).then_some(v)),
            _ => None,
        }
    }

    fn get_key_mut(&mut self, key: &str) -> Option<&mut Self> {
        match self {
            Self::StructProperty(props) => props.iter_mut().find_map(|p| if p.name == key {
                p.body.as_mut().map(|b| &mut b.value)
            } else {
                None
            }),
            Self::CustomStructProperty(s) => s.properties.iter_mut().find_map(|p| if p.name == key {
                p.body.as_mut().map(|b| &mut b.value)
            } else {
                None
            }),
            Self::MapProperty { values, .. } => values.iter_mut().find_map(|(k, v)| (k == key).then_some(v)),
            _ => None,
        }
    }

    fn get_index(&self, index: usize) -> Option<&Self> {
        match self {
            Self::ArrayProperty { values, .. } => values.get(index),
            Self::MapProperty { values, .. } => values.iter().find_map(|(k, v)| (*k == index).then_some(v)),
            _ => None,
        }
    }

    fn get_index_mut(&mut self, index: usize) -> Option<&mut Self> {
        match self {
            Self::ArrayProperty { values, .. } => values.get_mut(index),
            Self::MapProperty { values, .. } => values.iter_mut().find_map(|(k, v)| (*k == index).then_some(v)),
            _ => None,
        }
    }
}

impl PartialEq<str> for PropertyValue {
    fn eq(&self, other: &str) -> bool {
        match self {
            Self::StrProperty(s) | Self::EnumProperty(s) | Self::NameProperty(s) | Self::ObjectProperty(s) => s == other,
            _ => false,
        }
    }
}

impl PartialEq<&str> for PropertyValue {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl PartialEq<String> for PropertyValue {
    fn eq(&self, other: &String) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<i32> for PropertyValue {
    fn eq(&self, other: &i32) -> bool {
        match self {
            Self::IntProperty(i) => *i == *other,
            Self::ByteProperty(b) => *b as i32 == *other,
            _ => false,
        }
    }
}

impl PartialEq<usize> for PropertyValue {
    fn eq(&self, other: &usize) -> bool {
        match self {
            Self::IntProperty(i) => if *i < 0 {
                false
            } else {
                *i as usize == *other
            },
            Self::ByteProperty(b) => *b as usize == *other,
            _ => false,
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
                    None
                } else {
                    Some(u8::read_options(reader, endian, ())? != 0)
                })
            }
            "ByteProperty" => {
                if args.data_size == 1 {
                    Self::ByteProperty(u8::read_options(reader, endian, ())?)
                } else if !args.property_type.tags.is_empty() && let Ok(s) = FString::read_options(reader, endian, ()) {
                    // it looks like sometimes enum values are recorded as ByteProperty? so if we have tags
                    // and data_size != 1, see if we can parse as an enum value
                    Self::EnumProperty(s)
                } else {
                    // reset stream position in case enum value parse failed
                    reader.seek(SeekFrom::Start(start))?;
                    let mut buf = vec![0u8; args.data_size as usize];
                    reader.read_exact(&mut buf)?;
                    Self::UnknownProperty(buf)
                }
            }
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
            "ObjectProperty" => Self::ObjectProperty(FString::read_options(reader, endian, ())?),
            "StructProperty" => {
                // non-zero flags (or possibly just 08) seems to indicate types that don't have explicit field descriptions
                if args.flags != 0 {
                    let description = args.property_type.describe();
                    if description == GAMEPLAY_TAG_CONTAINER_TYPE {
                        // data is effectively an ArrayProperty[NameProperty]
                        let count = u32::read_options(reader, endian, ())? as usize;
                        let mut values = Vec::with_capacity(count);
                        for _ in 0..count {
                            values.push(Self::NameProperty(FString::read_options(reader, endian, ())?));
                        }
                        Self::ArrayProperty { values }
                    } else if description.starts_with(CORE_UOBJECT_TYPE_PREFIX) {
                        // unwrap is safe because there must be a tag if the description matched the prefix
                        let type_name = args.property_type.tags.first().unwrap().value.as_str();
                        match try_read_uobject(type_name, reader, endian)? {
                            Some(object) => Self::CoreUObjectStructProperty(object),
                            None => {
                                let mut buf = vec![0u8; args.data_size as usize];
                                reader.read_exact(&mut buf)?;
                                Self::UnknownProperty(buf)
                            }
                        }
                    } else {
                        let mut buf = vec![0u8; args.data_size as usize];
                        reader.read_exact(&mut buf)?;
                        Self::UnknownProperty(buf)
                    }
                } else {
                    let mut props = Vec::new();

                    let mut custom_struct_footer_size = None;
                    while reader.stream_position()? < end {
                        let mut prop = Property::read_options(reader, endian, ())?;
                        if custom_struct_footer_size.is_none() {
                            custom_struct_footer_size = prop.custom_struct_footer_size();
                        } else if let Some(footer_size) = custom_struct_footer_size && prop.is_custom_struct_data() {
                            prop.parse_custom_struct_data(footer_size)?;
                        }
                        let is_none = prop.is_none();
                        props.push(prop);
                        if is_none {
                            // empty property signals the end of this struct
                            break;
                        }
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
                        let current = reader.stream_position()?;
                        let remaining_size = (end - current) as u32;
                        values.push(PropertyValue::read_options(reader, endian, PropertyValueArgs::new(&element_type, args.flags, remaining_size))?);
                    }
                    Self::ArrayProperty { values }
                }
            }
            "MapProperty" => {
                let key_type = args.property_type.element_type().into_owned();
                // TODO: make this an error, not a panic
                let value_type = args.property_type.inner_types.last().expect("MapProperty should have a value type").clone();
                let flags = args.flags;

                let removed_count = u32::read_options(reader, endian, ())?;
                let count = u32::read_options(reader, endian, ())? as usize;
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    let current = reader.stream_position()?;
                    let remaining_size = (end - current) as u32;
                    let args = PropertyValueArgs::new(&key_type, flags, remaining_size);
                    let key = PropertyValue::read_options(reader, endian, args)?;

                    let current = reader.stream_position()?;
                    let remaining_size = (end - current) as u32;
                    let args = PropertyValueArgs::new(&value_type, flags, remaining_size);
                    let value = PropertyValue::read_options(reader, endian, args)?;

                    values.push((key, value));
                }
                Self::MapProperty { removed_count, values }
            }
            _ => {
                let mut buf = vec![0u8; args.data_size as usize];
                reader.read_exact(&mut buf)?;
                Self::UnknownProperty(buf)
            }
        };

        let current = reader.stream_position()?;
        if current > end {
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

fn num_inner_types(name: &str, tags: &[TypeTag]) -> usize {
    match name {
        "EnumProperty" => 1,
        "MapProperty" => {
            match tags.first() {
                Some(tag) if tag.value == "EnumProperty" => 2,
                _ => 1,
            }
        }
        "ArrayProperty" => {
            match tags.first() {
                Some(tag) if tag.value == "EnumProperty" => 1,
                Some(tag) if tag.value == "MapProperty" => {
                    match tags.get(1) {
                        Some(tag) if tag.value == "EnumProperty" => 2,
                        _ => 1,
                    }
                }
                _ => 0,
            }
        }
        _ => 0,
    }
}

#[binrw]
#[derive(Debug, Clone)]
pub struct PropertyType {
    pub name: FString,
    #[br(parse_with = read_tags)]
    #[bw(write_with = write_tags)]
    pub tags: Vec<TypeTag>,
    #[br(count = num_inner_types(name.as_str(), &tags))]
    pub inner_types: Vec<Self>,
}

impl PropertyType {
    pub fn new_scalar(name: &str) -> Self {
        Self { name: FString::from_str(name), tags: Vec::new(), inner_types: Vec::new() }
    }

    fn describe_by_name(desc: &mut String, name: &str, tags: &[TypeTag], inner_types: &[Self]) {
        desc.push_str(name);

        if tags.is_empty() {
            return;
        }

        match name {
            "StructProperty" | "EnumProperty" => {
                desc.push_str("<");
                if let Some(namespace) = tags.get(1) {
                    desc.push_str(namespace.value.as_str());
                    desc.push_str(".");
                }
                desc.push_str(tags.first().unwrap().value.as_str());
                desc.push_str(">");
            }
            "ArrayProperty" => {
                desc.push_str("[");
                let inner_type = tags.first().unwrap().value.as_str();
                Self::describe_by_name(desc, inner_type, &tags[1..], inner_types);
                desc.push_str("]");
            }
            "MapProperty" if !inner_types.is_empty() => {
                desc.push_str("<");

                let key_type = tags.first().unwrap().value.as_str();
                Self::describe_by_name(desc, key_type, &tags[1..], inner_types);

                desc.push_str(", ");

                let value_type = inner_types.last().unwrap();
                Self::describe_by_name(desc, value_type.name.as_str(), &value_type.tags, &value_type.inner_types);

                desc.push_str(">");
            }
            _ => (),
        }
    }

    pub fn describe(&self) -> String {
        let mut desc = String::new();
        Self::describe_by_name(&mut desc, self.name.as_str(), &self.tags, &self.inner_types);
        desc
    }

    pub fn size(&self) -> usize {
        // +4 for the tag list terminator
        self.name.byte_size() + self.tags.iter().map(TypeTag::size).sum::<usize>() + 4 + self.inner_types.iter().map(PropertyType::size).sum::<usize>()
    }

    pub fn element_type(&self) -> Cow<'_, Self> {
        match self.name.as_str() {
            "ArrayProperty" | "MapProperty" if !self.tags.is_empty() => {
                let name = self.tags[0].value.clone();
                let tags = self.tags[1..].to_vec();
                let inner_types = if name == "EnumProperty" && let Some(inner_type) = self.inner_types.first() {
                    vec![inner_type.clone()]
                } else {
                    Vec::new()
                };
                Cow::Owned(Self { name, tags, inner_types })
            }
            "StructProperty" if self.describe() == GAMEPLAY_TAG_CONTAINER_TYPE => Cow::Owned(Self { name: FString::from_str("NameProperty"), tags: Vec::new(), inner_types: Vec::new() }),
            _ => Cow::Borrowed(self),
        }
    }

    pub fn make_default_value(&self, flags: u8) -> PropertyValue {
        match self.name.as_str() {
            "BoolProperty" => PropertyValue::BoolProperty(Some(false)),
            "ByteProperty" => PropertyValue::ByteProperty(0),
            "IntProperty" => PropertyValue::IntProperty(0),
            "FloatProperty" => PropertyValue::FloatProperty(0.0),
            "DoubleProperty" => PropertyValue::DoubleProperty(0.0),
            "StrProperty" => PropertyValue::StrProperty(FString::new()),
            "NameProperty" => PropertyValue::NameProperty(FString::new()),
            "ObjectProperty" => PropertyValue::ObjectProperty(FString::new()),
            "EnumProperty" => PropertyValue::EnumProperty(FString::new()),
            "TextProperty" => PropertyValue::TextProperty { flags: TextFlags::empty(), data: TextData::None { values: Vec::new() } },
            "StructProperty" => {
                let description = self.describe();
                if description == GAMEPLAY_TAG_CONTAINER_TYPE {
                    PropertyValue::ArrayProperty { values: Vec::new() }
                } else if description.starts_with(CORE_UOBJECT_TYPE_PREFIX) {
                    // unwrap is safe because there must be a tag if the description matched the prefix
                    let type_name = self.tags.first().unwrap().value.as_str();
                    match make_default_uobject(type_name) {
                        Some(object) => PropertyValue::CoreUObjectStructProperty(object),
                        None => PropertyValue::UnknownProperty(Vec::new()),
                    }
                } else if flags != 0 {
                    PropertyValue::UnknownProperty(Vec::new())
                } else {
                    PropertyValue::StructProperty(vec![Property::new_none()])
                }
            }
            "ArrayProperty" => PropertyValue::ArrayProperty { values: Vec::new() },
            "MapProperty" => PropertyValue::MapProperty { removed_count: 0, values: Vec::new() },
            _ => PropertyValue::UnknownProperty(Vec::new()),
        }
    }
}

#[binrw]
#[derive(Debug)]
pub struct PropertyBody {
    pub property_type: PropertyType,
    #[bw(calc = value.size() as u32)]
    data_size: u32,
    pub flags: u8,
    #[br(args_raw(PropertyValueArgs::new(&property_type, flags, data_size)))]
    pub value: PropertyValue,
}

impl PropertyBody {
    pub fn new_scalar(value: PropertyValue) -> Self {
        Self {
            property_type: PropertyType::new_scalar(value.type_name()),
            flags: 0,
            value,
        }
    }

    pub fn size(&self) -> usize {
        self.property_type.size() + 4 + 1 + self.value.size()
    }

    pub fn parse_custom_struct(&mut self, footer_size: usize) -> BinResult<()> {
        let custom_struct: CustomStruct = {
            let PropertyValue::ArrayProperty { values } = &self.value else {
                return Ok(());
            };
            let Some(PropertyValue::UnknownProperty(data)) = values.first() else {
                return Ok(());
            };

            let mut reader = Cursor::new(data);
            reader.read_le_args((footer_size,))?
        };

        self.value = PropertyValue::CustomStructProperty(custom_struct);
        Ok(())
    }
}

impl Indexable for PropertyBody {
    fn get_key(&self, name: &str) -> Option<&PropertyValue> {
        self.value.get_key(name)
    }

    fn get_key_mut(&mut self, name: &str) -> Option<&mut PropertyValue> {
        self.value.get_key_mut(name)
    }

    fn get_index(&self, index: usize) -> Option<&PropertyValue> {
        self.value.get_index(index)
    }

    fn get_index_mut(&mut self, index: usize) -> Option<&mut PropertyValue> {
        self.value.get_index_mut(index)
    }
}

#[binrw]
#[derive(Debug)]
pub struct Property {
    pub name: FString,
    #[br(if(name != "None" && name != ""))]
    pub body: Option<PropertyBody>,
}

impl Property {
    pub fn new_scalar(name: &str, value: PropertyValue) -> Self {
        Self { name: FString::from_str(name), body: Some(PropertyBody::new_scalar(value)) }
    }

    pub fn new_none() -> Self {
        Self { name: FString::from_str("None"), body: None }
    }

    pub const fn is_none(&self) -> bool {
        self.body.is_none()
    }

    pub fn custom_struct_footer_size(&self) -> Option<usize> {
        match (self.name.as_str(), self.body.as_ref().map(|b| &b.value)) {
            ("Class", Some(PropertyValue::ObjectProperty(s))) => {
                CUSTOM_STRUCT_CLASSES.iter().find_map(|(class, footer_size)| (s == class).then_some(*footer_size))
            }
            _ => None,
        }
    }

    pub fn is_custom_struct_data(&self) -> bool {
        match (self.name.as_str(), self.body.as_ref().map(|b| &b.value)) {
            ("Data", Some(PropertyValue::ArrayProperty { values })) => {
                values.len() == 1 && matches!(values.first(), Some(PropertyValue::UnknownProperty(_)))
            }
            _ => false,
        }
    }

    pub fn parse_custom_struct_data(&mut self, footer_size: usize) -> BinResult<()> {
        if self.is_custom_struct_data() {
            self.body.as_mut().unwrap().parse_custom_struct(footer_size)
        } else {
            Ok(())
        }
    }

    pub fn size(&self) -> usize {
        self.name.byte_size() + self.body.as_ref().map(PropertyBody::size).unwrap_or(0)
    }
}

impl Indexable for Property {
    fn get_key(&self, name: &str) -> Option<&PropertyValue> {
        self.body.as_ref().and_then(|b| b.get_key(name))
    }

    fn get_key_mut(&mut self, name: &str) -> Option<&mut PropertyValue> {
        self.body.as_mut().and_then(|b| b.get_key_mut(name))
    }

    fn get_index(&self, index: usize) -> Option<&PropertyValue> {
        self.body.as_ref().and_then(|b| b.get_index(index))
    }

    fn get_index_mut(&mut self, index: usize) -> Option<&mut PropertyValue> {
        self.body.as_mut().and_then(|b| b.get_index_mut(index))
    }
}

#[binrw]
#[derive(Debug)]
pub struct SaveGameData {
    pub type_name: FString,
    pub flags: u8,
    #[br(parse_with = |r, e, _: ()| read_properties_with_footer(r, e, (4,)))]
    pub properties: Vec<Property>,
    pub extra: u32,
}

impl Indexable for SaveGameData {
    fn get_key(&self, name: &str) -> Option<&PropertyValue> {
        self.properties.iter().find_map(|p| if p.name == name {
            p.body.as_ref().map(|b| &b.value)
        } else {
            None
        })
    }

    fn get_key_mut(&mut self, name: &str) -> Option<&mut PropertyValue> {
        self.properties.iter_mut().find_map(|p| if p.name == name {
            p.body.as_mut().map(|b| &mut b.value)
        } else {
            None
        })
    }

    fn get_index(&self, _index: usize) -> Option<&PropertyValue> {
        None
    }

    fn get_index_mut(&mut self, _index: usize) -> Option<&mut PropertyValue> {
        None
    }
}

#[binrw]
#[derive(Debug)]
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