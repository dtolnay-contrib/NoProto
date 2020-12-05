//! All values in buffers are accessed and modified through pointers
//! 
//! NP_Ptr are the primary abstraction to read, update or delete values in a buffer.
//! Pointers should *never* be created directly, instead the various methods provided by the library to access
//! the internals of the buffer should be used.
//! 
//! Once you have a pointer you can read it's contents if it's a scalar value with `.get()` or convert it to a collection with `.deref()`.
//! When you attempt to read, update, or convert a pointer the schema is checked for that pointer location.  If the schema conflicts with the operation you're attempting it will fail.
//! As a result, you should be careful to make sure your reads and updates to the buffer line up with the schema you provided.
//! 
//! 

/// Any type
pub mod any;
pub mod string;
pub mod bytes;
pub mod numbers;
pub mod bool;
pub mod geo;
pub mod dec;
pub mod ulid;
pub mod uuid;
pub mod option;
pub mod date;

use crate::hashmap::NP_HashMap;
use core::{fmt::{Debug, Formatter}, hint::unreachable_unchecked};

use alloc::prelude::v1::Box;
use crate::{pointer::dec::NP_Dec, schema::NP_Schema_Addr};
use crate::NP_Parsed_Schema;
use crate::{json_flex::NP_JSON};
use crate::memory::{NP_Memory};
use crate::NP_Error;
use crate::{schema::{NP_TypeKeys}, collection::{map::NP_Map, table::NP_Table, list::NP_List, tuple::NP_Tuple}};

use alloc::{string::String, vec::Vec, borrow::ToOwned};
use bytes::NP_Bytes;

use self::{date::NP_Date, geo::NP_Geo, option::NP_Enum, string::NP_String, ulid::{NP_ULID, _NP_ULID}, uuid::{NP_UUID, _NP_UUID}};


#[doc(hidden)]
#[derive(Debug)]
#[repr(C)]
pub struct NP_Pointer_Scalar {
    pub addr_value: [u8; 2]
}

impl Default for NP_Pointer_Scalar {
    fn default() -> Self {
        Self { addr_value: [0; 2] }
    }
}

#[doc(hidden)]
#[derive(Debug)]
#[repr(C)]
pub struct NP_Pointer_List_Item {
    pub addr_value: [u8; 2],
    pub next_value: [u8; 2],
    pub index: u8
}

#[doc(hidden)]
#[derive(Debug)]
#[repr(C)]
pub struct NP_Pointer_Map_Item {
    pub addr_value: [u8; 2],
    pub next_value: [u8; 2],
    pub key_hash: [u8; 4]
}

pub trait NP_Pointer_Bytes {
    fn get_addr_value(&self) -> u16         { panic!() }
    fn set_addr_value(&mut self, addr: u16) { panic!() }
    fn get_next_addr(&self) -> u16          { panic!() }
    fn set_next_addr(&self, addr: u16)      { panic!() }
    fn set_index(&self, index: u8)          { panic!() }
    fn get_index(&self) -> u8               { panic!() }
    fn set_key_hash(&self, hash: u32)       { panic!() }
    fn get_key_hash(&self) -> u32           { panic!() }
    fn reset(&mut self)                     { panic!() }
    fn get_size(&self) -> usize             { panic!() }
}

impl NP_Pointer_Bytes for NP_Pointer_Scalar {
    #[inline(always)]
    fn get_addr_value(&self) -> u16 { u16::from_be_bytes(self.addr_value) }
    #[inline(always)]
    fn set_addr_value(&mut self, addr: u16) { self.addr_value = addr.to_be_bytes() }
    #[inline(always)]
    fn reset(&mut self) { self.addr_value = [0; 2] }
    #[inline(always)]
    fn get_size(&self) -> usize { 2 }
}
impl NP_Pointer_Bytes for NP_Pointer_List_Item {
    #[inline(always)]
    fn get_addr_value(&self) -> u16 { u16::from_be_bytes(self.addr_value) }
    #[inline(always)]
    fn set_addr_value(&mut self, addr: u16) { self.addr_value = addr.to_be_bytes() }
    #[inline(always)]
    fn get_next_addr(&self) -> u16 { u16::from_be_bytes(self.next_value) }
    #[inline(always)]
    fn set_next_addr(&self, addr: u16) { self.next_value = addr.to_be_bytes() }
    #[inline(always)]
    fn set_index(&self, index: u8)  { self.index = index }
    #[inline(always)]
    fn get_index(&self) -> u8  { self.index }
    #[inline(always)]
    fn reset(&mut self) { self.addr_value = [0; 2]; self.index = 0; self.next_value = [0; 2]; }
    #[inline(always)]
    fn get_size(&self) -> usize { 5 }
}
impl NP_Pointer_Bytes for NP_Pointer_Map_Item {
    #[inline(always)]
    fn get_addr_value(&self) -> u16 { u16::from_be_bytes(self.addr_value) }
    #[inline(always)]
    fn set_addr_value(&mut self, addr: u16) { self.addr_value = addr.to_be_bytes() }
    #[inline(always)]
    fn get_next_addr(&self) -> u16 { u16::from_be_bytes(self.next_value) }
    #[inline(always)]
    fn set_next_addr(&self, addr: u16) { self.next_value = addr.to_be_bytes() }
    #[inline(always)]
    fn set_key_hash(&self, hash: u32)  { self.key_hash = hash.to_be_bytes(); }
    #[inline(always)]
    fn get_key_hash(&self) -> u32  { u32::from_be_bytes(self.key_hash) }
    #[inline(always)]
    fn reset(&mut self) { self.addr_value = [0; 2]; self.key_hash = [0; 4]; self.next_value = [0; 2]; }
    #[inline(always)]
    fn get_size(&self) -> usize { 8 }
}

impl NP_Pointer_Bytes for [u8; 8] {
    #[inline(always)]
    fn get_addr_value(&self) -> u16 { u16::from_be_bytes(unsafe { *(&self[0..2] as *const [u8] as *const [u8; 2]) }) }
    #[inline(always)]
    fn set_addr_value(&mut self, addr: u16) {
        let b = addr.to_be_bytes();
        self[0] = b[0];
        self[1] = b[1];
    }
    #[inline(always)]
    fn get_next_addr(&self) -> u16 { u16::from_be_bytes(unsafe { *(&self[2..4] as *const [u8] as *const [u8; 2]) }) }
    #[inline(always)]
    fn set_next_addr(&self, addr: u16) { 
        let b = addr.to_be_bytes();
        self[2] = b[0];
        self[3] = b[1];
    }
    #[inline(always)]
    fn set_index(&self, index: u8)  { self[4] = index }
    #[inline(always)]
    fn get_index(&self) -> u8  { self[4] }
    #[inline(always)]
    fn set_key_hash(&self, hash: u32)  { 
        let b = hash.to_be_bytes();
        self[4] = b[0];
        self[5] = b[1];
        self[6] = b[2];
        self[7] = b[3];
    }
    #[inline(always)]
    fn get_key_hash(&self) -> u32 { u32::from_be_bytes(unsafe { *(&self[4..8] as *const [u8] as *const [u8; 4]) }) }
    #[inline(always)]
    fn reset(&mut self) {
        for (i, x) in self.iter().enumerate() {
            self[i] = 0;
        }
    }
}

const DEF_TABLE: NP_Vtable = NP_Vtable { next: [0; 2], values: [NP_Pointer_Scalar::default(); 4]};

#[derive(Debug)]
pub enum NP_Cursor_Data<'data> {
    Empty,
    Scalar,
    List { list_addrs: [u16; 255], bytes: &'data mut NP_List_Bytes },
    Map { value_map: NP_HashMap },
    Tuple { bytes: [(usize, &'data mut NP_Vtable); 64] }, // (buffer_addr, VTable )
    Table { bytes: [(usize, &'data mut NP_Vtable); 64] }  // (buffer_addr, VTable )
}

#[repr(C)]
#[derive(Debug)]
pub struct NP_List_Bytes {
    head: [u8; 2],
    tail: [u8; 2]
}

impl NP_List_Bytes {
    #[inline(always)]
    pub fn set_head(&mut self, head: u16) {
        self.head = head.to_be_bytes();
    }
    #[inline(always)]
    pub fn get_head(&self) -> u16 {
        u16::from_be_bytes(self.head)
    }
    #[inline(always)]
    pub fn set_tail(&mut self, tail: u16) {
        self.tail = tail.to_be_bytes();
    }
    #[inline(always)]
    pub fn get_tail(&self) -> u16 {
        u16::from_be_bytes(self.tail)
    }
}

// holds 4 u16 addresses and a next value (10 bytes)
#[repr(C)]
#[derive(Debug)]
pub struct NP_Vtable {
    pub values: [NP_Pointer_Scalar; 4],
    next: [u8; 2]
}

impl NP_Vtable {

    #[inline(always)]
    pub fn get_next(&self) -> u16 {
        u16::from_be_bytes(unsafe { *(&self.next as *const [u8] as *const [u8; 2]) }) 
    }

    #[inline(always)]
    pub fn set_next(&mut self, value: u16) {
        let bytes = value.to_be_bytes();
        self.next[0] = bytes[0];
        self.next[1] = bytes[1];
    }
}


impl<'data> Default for NP_Cursor_Data<'data> {
    fn default() -> Self {
        NP_Cursor_Data::Empty
    }
}

/// Cursor for pointer value in buffer
/// 
pub struct NP_Cursor<'cursor> {
    /// The location of this cursor in the buffer
    pub buff_addr: usize,
    /// Stores information about the data at this pointer
    pub data: NP_Cursor_Data<'cursor> ,
    /// The address of the schema for this cursor
    pub schema_addr: NP_Schema_Addr,
    /// Virtual cursor bytes
    pub temp_bytes: Option<[u8; 8]>,
    /// the values of the buffer pointer
    pub value: &'cursor mut dyn NP_Pointer_Bytes,
    /// Information about the parent cursor
    pub parent_addr: usize,
    /// The previous cursor
    pub prev_cursor: Option<usize>
}

/// Represents a cursor address in the memory
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NP_Cursor_Addr {
    Virtual,
    Real(usize)
}

impl<'cursor> NP_Cursor<'cursor> {

    pub fn new_virtual() -> Self {
        let bytes = [0u8; 8];
        Self {
            buff_addr: 0,
            data: NP_Cursor_Data::Empty,
            schema_addr: 0,
            temp_bytes: Some(bytes),
            value: unsafe { &mut *(&mut bytes as *mut dyn NP_Pointer_Bytes) },
            parent_addr: 0,
            prev_cursor: None
        }
    }

    pub fn reset(&mut self) {
        self.buff_addr = 0;
        self.data = NP_Cursor_Data::Empty;
        self.schema_addr = 0;
        self.value.reset();
        self.parent_addr = 0;
        self.prev_cursor = None;
    }


    pub fn parse(buff_addr: usize, schema_addr: NP_Schema_Addr, parent_addr: usize, parent_schema_addr: usize, memory: &NP_Memory<'cursor>) -> Result<(), NP_Error> {

        if buff_addr > memory.read_bytes().len() {
            panic!()
        }

        match memory.schema[schema_addr] {
            _ => { // scalar items
                
                let new_cursor = NP_Cursor { 
                    buff_addr: buff_addr, 
                    schema_addr: schema_addr, 
                    data: NP_Cursor_Data::Scalar,
                    temp_bytes: None,
                    value: NP_Cursor::parse_cursor_value(buff_addr, parent_schema_addr, parent_addr, &memory), 
                    parent_addr: parent_addr,
                    prev_cursor: None,
                };

                memory.insert_parsed(buff_addr, new_cursor);
            },
            NP_Parsed_Schema::Table { columns, .. } => {
                NP_Table::parse(buff_addr, schema_addr, parent_addr, parent_schema_addr, &memory, &columns);
            },
            NP_Parsed_Schema::List  { of, .. } => {
                NP_List::parse(buff_addr, schema_addr, parent_addr, parent_schema_addr, &memory, of);
            },
            NP_Parsed_Schema::Tuple { values, .. } => {
                NP_Tuple::parse(buff_addr, schema_addr, parent_addr, parent_schema_addr, &memory, &values);
            },
            NP_Parsed_Schema::Map   { value, .. } => {
                NP_List::parse(buff_addr, schema_addr, parent_addr, parent_schema_addr, &memory, value);
            }
        }

        Ok(())
    }

    
    #[inline(always)]
    pub fn parse_cursor_value(buff_addr: usize, parent_addr: usize, parent_schema_addr: usize, memory: &NP_Memory<'cursor>) -> &'cursor mut dyn NP_Pointer_Bytes {
        if parent_addr == 0 { // parent is root, no possible colleciton above
            unsafe { &mut *(memory.write_bytes().as_ptr().add(buff_addr) as *mut NP_Pointer_Scalar) }
        } else {

            match memory.schema[parent_schema_addr] {
                NP_Parsed_Schema::List { .. } => {
                    unsafe { &mut *(memory.write_bytes().as_ptr().add(buff_addr) as *mut NP_Pointer_List_Item) }
                },
                NP_Parsed_Schema::Map { .. } => {
                    unsafe { &mut *(memory.write_bytes().as_ptr().add(buff_addr) as *mut NP_Pointer_Map_Item) }
                },
                _ => { // parent is scalar, table or tuple
                    unsafe { &mut *(memory.write_bytes().as_ptr().add(buff_addr) as *mut NP_Pointer_Scalar) }
                }
            }
        }
    }

    /// Exports this pointer and all it's descendants into a JSON object.
    /// This will create a copy of the underlying data and return default values where there isn't data.
    /// 
    pub fn json_encode(cursor: NP_Cursor_Addr, memory: &NP_Memory<'cursor>) -> NP_JSON {

        match memory.schema[memory.get_parsed(&cursor).schema_addr].get_type_key() {
            NP_TypeKeys::None           => { NP_JSON::Null },
            NP_TypeKeys::Any            => { NP_JSON::Null },
            NP_TypeKeys::UTF8String     => { NP_String::to_json(cursor, memory) },
            NP_TypeKeys::Bytes          => {  NP_Bytes::to_json(cursor, memory) },
            NP_TypeKeys::Int8           => {        i8::to_json(cursor, memory) },
            NP_TypeKeys::Int16          => {       i16::to_json(cursor, memory) },
            NP_TypeKeys::Int32          => {       i32::to_json(cursor, memory) },
            NP_TypeKeys::Int64          => {       i64::to_json(cursor, memory) },
            NP_TypeKeys::Uint8          => {        u8::to_json(cursor, memory) },
            NP_TypeKeys::Uint16         => {       u16::to_json(cursor, memory) },
            NP_TypeKeys::Uint32         => {       u32::to_json(cursor, memory) },
            NP_TypeKeys::Uint64         => {       u64::to_json(cursor, memory) },
            NP_TypeKeys::Float          => {       f32::to_json(cursor, memory) },
            NP_TypeKeys::Double         => {       f64::to_json(cursor, memory) },
            NP_TypeKeys::Decimal        => {    NP_Dec::to_json(cursor, memory) },
            NP_TypeKeys::Boolean        => {      bool::to_json(cursor, memory) },
            NP_TypeKeys::Geo            => {    NP_Geo::to_json(cursor, memory) },
            NP_TypeKeys::Uuid           => {  _NP_UUID::to_json(cursor, memory) },
            NP_TypeKeys::Ulid           => {  _NP_ULID::to_json(cursor, memory) },
            NP_TypeKeys::Date           => {   NP_Date::to_json(cursor, memory) },
            NP_TypeKeys::Enum           => {   NP_Enum::to_json(cursor, memory) },
            NP_TypeKeys::Table          => {  NP_Table::to_json(cursor, memory) },
            NP_TypeKeys::Map            => {    NP_Map::to_json(cursor, memory) },
            NP_TypeKeys::List           => {   NP_List::to_json(cursor, memory) },
            NP_TypeKeys::Tuple          => {  NP_Tuple::to_json(cursor, memory) }
        }

    }

    /// Compact from old cursor and memory into new cursor and memory
    /// 
    pub fn compact(from_cursor: NP_Cursor_Addr, from_memory: &NP_Memory<'cursor>, to_cursor: NP_Cursor_Addr, to_memory: &NP_Memory<'cursor>) -> Result<NP_Cursor_Addr, NP_Error> {

        match from_memory.schema[from_memory.get_parsed(&from_cursor).schema_addr].get_type_key() {
            NP_TypeKeys::Any           => { Ok(to_cursor) }
            NP_TypeKeys::UTF8String    => { NP_String::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Bytes         => {  NP_Bytes::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Int8          => {        i8::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Int16         => {       i16::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Int32         => {       i32::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Int64         => {       i64::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Uint8         => {        u8::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Uint16        => {       u16::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Uint32        => {       u32::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Uint64        => {       u64::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Float         => {       f32::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Double        => {       f64::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Decimal       => {    NP_Dec::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Boolean       => {      bool::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Geo           => {    NP_Geo::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Uuid          => {  _NP_UUID::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Ulid          => {  _NP_ULID::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Date          => {   NP_Date::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Enum          => {   NP_Enum::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Table         => {  NP_Table::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Map           => {    NP_Map::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::List          => {   NP_List::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            NP_TypeKeys::Tuple         => {  NP_Tuple::do_compact(from_cursor, from_memory, to_cursor, to_memory) }
            _ => { panic!() }
        }
    }

    /// Set default for this value.  Not related to the schema default, this is the default value for this data type
    /// 
    pub fn set_default(cursor: NP_Cursor_Addr, memory: &NP_Memory<'cursor>) -> Result<(), NP_Error> {

        match memory.schema[memory.get_parsed(&cursor).schema_addr].get_type_key() {
            NP_TypeKeys::None        => { panic!() },
            NP_TypeKeys::Any         => { panic!() },
            NP_TypeKeys::Table       => { panic!() },
            NP_TypeKeys::Map         => { panic!() },
            NP_TypeKeys::List        => { panic!() },
            NP_TypeKeys::Tuple       => { panic!() },
            NP_TypeKeys::UTF8String  => {  NP_String::set_value(cursor, memory, &String::default())?; },
            NP_TypeKeys::Bytes       => {   NP_Bytes::set_value(cursor, memory, &NP_Bytes::default())?; },
            NP_TypeKeys::Int8        => {         i8::set_value(cursor, memory, i8::default())?; },
            NP_TypeKeys::Int16       => {        i16::set_value(cursor, memory, i16::default())?; },
            NP_TypeKeys::Int32       => {        i32::set_value(cursor, memory, i32::default())?; },
            NP_TypeKeys::Int64       => {        i64::set_value(cursor, memory, i64::default())?; },
            NP_TypeKeys::Uint8       => {         u8::set_value(cursor, memory, u8::default())?; },
            NP_TypeKeys::Uint16      => {        u16::set_value(cursor, memory, u16::default())?; },
            NP_TypeKeys::Uint32      => {        u32::set_value(cursor, memory, u32::default())?; },
            NP_TypeKeys::Uint64      => {        u64::set_value(cursor, memory, u64::default())?; },
            NP_TypeKeys::Float       => {        f32::set_value(cursor, memory, f32::default())?; },
            NP_TypeKeys::Double      => {        f64::set_value(cursor, memory, f64::default())?; },
            NP_TypeKeys::Decimal     => {     NP_Dec::set_value(cursor, memory, NP_Dec::default())?; },
            NP_TypeKeys::Boolean     => {       bool::set_value(cursor, memory, bool::default())?; },
            NP_TypeKeys::Geo         => {     NP_Geo::set_value(cursor, memory, NP_Geo::default())?; },
            NP_TypeKeys::Uuid        => {   _NP_UUID::set_value(cursor, memory, &NP_UUID::default())?; },
            NP_TypeKeys::Ulid        => {   _NP_ULID::set_value(cursor, memory, &NP_ULID::default())?; },
            NP_TypeKeys::Date        => {    NP_Date::set_value(cursor, memory, NP_Date::default())?; },
            NP_TypeKeys::Enum        => {    NP_Enum::set_value(cursor, memory, NP_Enum::default())?; }
        }

        Ok(())
    }

    /// Calculate the number of bytes used by this pointer and it's descendants.
    /// 
    pub fn calc_size(cursor_addr: NP_Cursor_Addr, memory: &NP_Memory<'cursor>) -> Result<usize, NP_Error> {

        if let NP_Cursor_Addr::Real(buff_addr) = cursor_addr {

            let cursor = memory.get_parsed(&cursor_addr);

            // size of pointer
            let base_size = unsafe { (*cursor.value).get_size() };

            // pointer is in buffer but has no value set
            if unsafe { (*cursor.value).get_addr_value() } == 0 { // no value, just base size
                return Ok(base_size);
            }

            // get the size of the value based on schema
            let type_size = match memory.schema[cursor.schema_addr].get_type_key() {
                NP_TypeKeys::None         => { Ok(0) },
                NP_TypeKeys::Any          => { Ok(0) },
                NP_TypeKeys::UTF8String   => { NP_String::get_size(cursor_addr, memory) },
                NP_TypeKeys::Bytes        => {  NP_Bytes::get_size(cursor_addr, memory) },
                NP_TypeKeys::Int8         => {        i8::get_size(cursor_addr, memory) },
                NP_TypeKeys::Int16        => {       i16::get_size(cursor_addr, memory) },
                NP_TypeKeys::Int32        => {       i32::get_size(cursor_addr, memory) },
                NP_TypeKeys::Int64        => {       i64::get_size(cursor_addr, memory) },
                NP_TypeKeys::Uint8        => {        u8::get_size(cursor_addr, memory) },
                NP_TypeKeys::Uint16       => {       u16::get_size(cursor_addr, memory) },
                NP_TypeKeys::Uint32       => {       u32::get_size(cursor_addr, memory) },
                NP_TypeKeys::Uint64       => {       u64::get_size(cursor_addr, memory) },
                NP_TypeKeys::Float        => {       f32::get_size(cursor_addr, memory) },
                NP_TypeKeys::Double       => {       f64::get_size(cursor_addr, memory) },
                NP_TypeKeys::Decimal      => {    NP_Dec::get_size(cursor_addr, memory) },
                NP_TypeKeys::Boolean      => {      bool::get_size(cursor_addr, memory) },
                NP_TypeKeys::Geo          => {    NP_Geo::get_size(cursor_addr, memory) },
                NP_TypeKeys::Uuid         => {  _NP_UUID::get_size(cursor_addr, memory) },
                NP_TypeKeys::Ulid         => {  _NP_ULID::get_size(cursor_addr, memory) },
                NP_TypeKeys::Date         => {   NP_Date::get_size(cursor_addr, memory) },
                NP_TypeKeys::Enum         => {   NP_Enum::get_size(cursor_addr, memory) },
                NP_TypeKeys::Table        => {  NP_Table::get_size(cursor_addr, memory) },
                NP_TypeKeys::Map          => {    NP_Map::get_size(cursor_addr, memory) },
                NP_TypeKeys::List         => {   NP_List::get_size(cursor_addr, memory) },
                NP_TypeKeys::Tuple        => {  NP_Tuple::get_size(cursor_addr, memory) }
            }?;

            Ok(type_size + base_size)
        } else {
            Ok(0)
        }


    }
}


/// This trait is used to restrict which types can be set/get in the buffer
pub trait NP_Scalar {}

/// This trait is used to implement types as NoProto buffer types.
/// This includes all the type data, encoding and decoding methods.
#[doc(hidden)]
pub trait NP_Value<'value> {

    /// Get the type information for this type (static)
    /// 
    fn type_idx() -> (&'value str, NP_TypeKeys);

    /// Get the type information for this type (instance)
    /// 
    fn self_type_idx(&self) -> (&'value str, NP_TypeKeys);

    /// Convert the schema byte array for this type into JSON
    /// 
    fn schema_to_json(schema: &Vec<NP_Parsed_Schema>, address: usize)-> Result<NP_JSON, NP_Error>;

    /// Get the default schema value for this type
    /// 
    fn schema_default(_schema: &'value NP_Parsed_Schema) -> Option<Self> where Self: Sized;

    /// Parse JSON schema into schema
    ///
    fn from_json_to_schema(schema: Vec<NP_Parsed_Schema>, json_schema: &Box<NP_JSON>) -> Result<(bool, Vec<u8>, Vec<NP_Parsed_Schema>), NP_Error>;

    /// Parse bytes into schema
    /// 
    fn from_bytes_to_schema(schema: Vec<NP_Parsed_Schema>, address: usize, bytes: &Vec<u8>) -> (bool, Vec<NP_Parsed_Schema>);

    /// Set the value of this scalar into the buffer
    /// 
    fn set_value(_cursor: NP_Cursor_Addr, _memory: &NP_Memory<'value>, _value: Self) -> Result<NP_Cursor_Addr, NP_Error> where Self: Sized {
        let message = "This type doesn't support set_value!".to_owned();
        Err(NP_Error::new(message.as_str()))
    }

    /// Pull the data from the buffer and convert into type
    /// 
    fn into_value(_cursor: NP_Cursor_Addr, _memory: &NP_Memory<'value>) -> Result<Option<Self>, NP_Error> where Self: Sized {
        let message = "This type doesn't support into!".to_owned();
        Err(NP_Error::new(message.as_str()))
    }

    /// Convert this type into a JSON value (recursive for collections)
    /// 
    fn to_json(_cursor: NP_Cursor_Addr, _memory: &NP_Memory<'value>) -> NP_JSON;

    /// Calculate the size of this pointer and it's children (recursive for collections)
    /// 
    fn get_size(_cursor: NP_Cursor_Addr, memory: &NP_Memory<'value>) -> Result<usize, NP_Error>;
    
    /// Handle copying from old pointer/buffer to new pointer/buffer (recursive for collections)
    /// 
    fn do_compact(from_cursor: NP_Cursor_Addr, from_memory: &NP_Memory<'value>, to_cursor: NP_Cursor_Addr, to_memory: &NP_Memory<'value>) -> Result<NP_Cursor_Addr, NP_Error> where Self: 'value + Sized {

        match Self::into_value(from_cursor.clone(), from_memory)? {
            Some(x) => {
                return Self::set_value(to_cursor, to_memory, x);
            },
            None => { }
        }

        Ok(to_cursor)
    }
}



/*
// unsigned integer size:        0 to (2^i) -1
//   signed integer size: -2^(i-1) to  2^(i-1) 
*/