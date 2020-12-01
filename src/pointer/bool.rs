//! NoProto supports Rust's native [`bool`](https://doc.rust-lang.org/std/primitive.bool.html) type.
//! 
//! ```
//! use no_proto::error::NP_Error;
//! use no_proto::NP_Factory;
//! use no_proto::pointer::bytes::NP_Bytes;
//! 
//! let factory: NP_Factory = NP_Factory::new(r#"{
//!    "type": "bool"
//! }"#)?;
//!
//! let mut new_buffer = factory.empty_buffer(None, None);
//! new_buffer.set(&[], true)?;
//! 
//! assert_eq!(true, new_buffer.get::<bool>(&[])?.unwrap());
//!
//! # Ok::<(), NP_Error>(()) 
//! ```

use core::hint::unreachable_unchecked;

use crate::{json_flex::JSMAP, schema::{NP_Parsed_Schema}};
use crate::error::NP_Error;
use crate::{schema::{NP_TypeKeys}, pointer::NP_Value, json_flex::NP_JSON};

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::{borrow::ToOwned};
use super::{NP_Cursor};
use crate::NP_Memory;
use alloc::string::ToString;

impl super::NP_Scalar for bool {}

impl<'value> NP_Value<'value> for bool {

    fn type_idx() -> (&'value str, NP_TypeKeys) { ("bool", NP_TypeKeys::Boolean) }
    fn self_type_idx(&self) -> (&'value str, NP_TypeKeys) { ("bool", NP_TypeKeys::Boolean) }

    fn schema_to_json(schema: &Vec<NP_Parsed_Schema>, address: usize)-> Result<NP_JSON, NP_Error> {
        let mut schema_json = JSMAP::new();
        schema_json.insert("type".to_owned(), NP_JSON::String(Self::type_idx().0.to_string()));

        match &schema[address] {
            NP_Parsed_Schema::Boolean { i: _, sortable: _, default} => {
                if let Some(d) = default {
                    schema_json.insert("default".to_owned(), match *d {
                        true => NP_JSON::True,
                        false => NP_JSON::False
                    });
                }
            },
            _ => { unsafe { unreachable_unchecked() } }
        }

        Ok(NP_JSON::Dictionary(schema_json))
    }

    fn schema_default(schema: &'value NP_Parsed_Schema) -> Option<Self> {

        match schema {
            NP_Parsed_Schema::Boolean { default, .. } => {
                match default {
                    Some(x) => Some(*x),
                    None => None
                }
            },
            _ => { unsafe { unreachable_unchecked() } }
        }
    }

    fn set_value(mut cursor: NP_Cursor, memory: &NP_Memory, value: Self) -> Result<NP_Cursor, NP_Error> {

        let mut value_address = cursor.value.get_value_address();

        if value_address != 0 { // existing value, replace

            // overwrite existing values in buffer
            memory.write_bytes()[value_address] = if value == true {
                1
            } else {
                0
            };

            return Ok(cursor);

        } else { // new value

            let bytes = if value == true {
                [1] as [u8; 1]
            } else {
                [0] as [u8; 1]
            };

            value_address = memory.malloc_borrow(&bytes)?;
            cursor.value = cursor.value.update_value_address(value_address);
            memory.write_address(cursor.buff_addr, value_address);

            return Ok(cursor);

        }
        
    }

    fn into_value(cursor: NP_Cursor, memory: &NP_Memory) -> Result<Option<Self>, NP_Error> {

        let value_addr = cursor.value.get_value_address();

        // empty value
        if value_addr == 0 {
            return Ok(None);
        }

        Ok(match memory.get_1_byte(value_addr) {
            Some(x) => {
                Some(if x == 1 { true } else { false })
            },
            None => None
        })
    }

    fn to_json(cursor: &NP_Cursor, memory: &NP_Memory) -> NP_JSON {

        match Self::into_value(cursor.clone(), memory) {
            Ok(x) => {
                match x {
                    Some(y) => {
                        if y == true {
                            NP_JSON::True
                        } else {
                            NP_JSON::False
                        }
                    },
                    None => {
                        match memory.schema[cursor.schema_addr] {
                            NP_Parsed_Schema::Boolean { i: _, sortable: _, default} => {
                                if let Some(d) = default {
                                    if d == true {
                                        NP_JSON::True
                                    } else {
                                        NP_JSON::False
                                    }
                                } else {
                                    NP_JSON::Null
                                }
                            },
                            _ => { unsafe { unreachable_unchecked() } }
                        }
                    }
                }
            },
            Err(_e) => {
                NP_JSON::Null
            }
        }
    }

    fn get_size(cursor: NP_Cursor, _memory: &NP_Memory) -> Result<usize, NP_Error> {

        if cursor.value.get_value_address() == 0 {
            Ok(0) 
        } else {
            Ok(core::mem::size_of::<u8>())
        }
    }

    fn from_json_to_schema(mut schema: Vec<NP_Parsed_Schema>, json_schema: &Box<NP_JSON>) -> Result<(bool, Vec<u8>, Vec<NP_Parsed_Schema>), NP_Error> {

        let mut schema_data: Vec<u8> = Vec::new();
        schema_data.push(NP_TypeKeys::Boolean as u8);

        let default = match json_schema["default"] {
            NP_JSON::False => {
                schema_data.push(2);
                Some(false)
            },
            NP_JSON::True => {
                schema_data.push(1);
                Some(true)
            },
            _ => {
                schema_data.push(0);
                None
            }
        };

        schema.push(NP_Parsed_Schema::Boolean {
            i: NP_TypeKeys::Boolean,
            default: default,
            sortable: true
        });

        return Ok((true, schema_data, schema));
  
    }
    fn from_bytes_to_schema(mut schema: Vec<NP_Parsed_Schema>, address: usize, bytes: &Vec<u8>) -> (bool, Vec<NP_Parsed_Schema>) {
        schema.push(NP_Parsed_Schema::Boolean {
            i: NP_TypeKeys::Boolean,
            sortable: true,
            default: match bytes[address] {
                0 => None,
                1 => Some(true),
                2 => Some(false),
                _ => unreachable!()
            }
        });
        (true, schema)
     }
}

#[test]
fn schema_parsing_works() -> Result<(), NP_Error> {
    let schema = "{\"type\":\"bool\",\"default\":false}";
    let factory = crate::NP_Factory::new(schema)?;
    assert_eq!(schema, factory.schema.to_json()?.stringify());

    let schema = "{\"type\":\"bool\"}";
    let factory = crate::NP_Factory::new(schema)?;
    assert_eq!(schema, factory.schema.to_json()?.stringify());
    
    Ok(())
}

#[test]
fn default_value_works() -> Result<(), NP_Error> {
    let schema = "{\"type\":\"bool\",\"default\":false}";
    let factory = crate::NP_Factory::new(schema)?;
    let buffer = factory.empty_buffer(None, None);
    assert_eq!(buffer.get::<bool>(&[])?.unwrap(), false);

    Ok(())
}


#[test]
fn set_clear_value_and_compaction_works() -> Result<(), NP_Error> {
    let schema = "{\"type\":\"bool\"}";
    let factory = crate::NP_Factory::new(schema)?;
    let mut buffer = factory.empty_buffer(None, None);
    buffer.set(&[], false)?;
    assert_eq!(buffer.get::<bool>(&[])?.unwrap(), false);
    buffer.del(&[])?;
    assert_eq!(buffer.get::<bool>(&[])?, None);

    buffer.compact(None, None)?;
    assert_eq!(buffer.calc_bytes()?.current_buffer, 4usize);

    Ok(())
}