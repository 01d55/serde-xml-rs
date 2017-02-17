extern crate xml;
#[macro_use] extern crate serde;
#[macro_use] extern crate log;

#[macro_use] mod error;
mod map;
mod seq;
mod var;

pub use error::Error;
pub use xml::reader::{EventReader, ParserConfig};

use error::VResult;
use xml::reader::XmlEvent;
use xml::name::OwnedName;
use serde::de::{self, Visitor, Deserialize};
use std::io::Read;
use map::MapVisitor;
use seq::SeqVisitor;
use var::EnumVisitor;

pub struct Deserializer<R: Read> {
    depth: usize,
    reader: EventReader<R>,
    peeked: Option<XmlEvent>,
    is_map_value: bool
}

pub fn deserialize<R: Read, T: Deserialize>(reader: R) -> Result<T, Error> {
    T::deserialize(&mut Deserializer::new_from_reader(reader))
}

impl<R: Read> Deserializer<R> {
    pub fn new(reader: EventReader<R>) -> Self {
        Deserializer {
            depth: 0,
            reader: reader,
            peeked: None,
            is_map_value: false
        }
    }

    pub fn new_from_reader(reader: R) -> Self {
        Self::new(EventReader::new_with_config(reader, ParserConfig {
            trim_whitespace: true,
            whitespace_to_characters: true,
            cdata_to_characters: true,
            ignore_comments: true,
            coalesce_characters: true
        }))
    }

    fn peek(&mut self) -> Result<&XmlEvent, Error> {
        if self.peeked.is_none() {
            self.peeked = Some(self.inner_next()?);
        }
        debug_expect!(self.peeked.as_ref(), Some(peeked) => {
            debug!("Peeked {:?}", peeked);
            Ok(peeked)
        })
    }

    fn inner_next(&mut self) -> Result<XmlEvent, Error> {
        loop {
            match self.reader.next().map_err(Error::Syntax)? {
                XmlEvent::StartDocument { .. } |
                XmlEvent::EndDocument { .. } |
                XmlEvent::ProcessingInstruction { .. } |
                XmlEvent::Comment(_) => {/* skip */}

                other => return Ok(other)
            }
        }
    }

    fn next(&mut self) -> Result<XmlEvent, Error> {
        let next = if let Some(peeked) = self.peeked.take() {
            peeked
        } else {
            self.inner_next()?
        };
        match next {
            XmlEvent::StartElement { .. } => {
                self.depth += 1;
            },
            XmlEvent::EndElement { .. } => {
                self.depth -= 1;
            },
            _ => {}
        }
        debug!("Fetched {:?}", next);
        Ok(next)
    }

    fn set_map_value(&mut self) {
        self.is_map_value = true;
    }

    pub fn unset_map_value(&mut self) -> bool {
        ::std::mem::replace(&mut self.is_map_value, false)
    }

    fn read_inner_value<V: Visitor, F: FnOnce(&mut Self) -> VResult<V>>(&mut self, f: F) -> VResult<V> {
        if self.unset_map_value() {
            debug_expect!(self.next(), Ok(XmlEvent::StartElement { name, .. }) => {
                let result = f(self)?;
                self.expect_end_element(name)?;
                Ok(result)
            })
        } else {
            f(self)
        }
    }

    fn expect_end_element(&mut self, start_name: OwnedName) -> Result<(), Error> {
        expect!(self.next()?, XmlEvent::EndElement { name, .. } => {
            if name == start_name {
                Ok(())
            } else {
                Err(Error::Custom(format!("End tag </{}> didn't match the start tag <{}>", name.local_name, start_name.local_name)))
            }
        })
    }
}

impl<'a, R: Read> de::Deserializer for &'a mut Deserializer<R> {
    type Error = Error;

    forward_to_deserialize! {
        newtype_struct struct_field
    }

    fn deserialize_struct<V: Visitor>(self, _name: &'static str, fields: &'static [&'static str], visitor: V) -> VResult<V> {
        self.unset_map_value();
        expect!(self.next()?, XmlEvent::StartElement { name, attributes, .. } => {
            let map_value = visitor.visit_map(MapVisitor::new(self, attributes, fields.contains(&"$value")))?;
            self.expect_end_element(name)?;
            Ok(map_value)
        })
    }

    fn deserialize_u64<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_u32<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_u16<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_u8<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_i64<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_i32<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_i16<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_i8<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_f32<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_f64<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_bool<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_char<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_bytes<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_byte_buf<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.deserialize_string(visitor)
    }

    fn deserialize_unit<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.read_inner_value::<V, _>(|this| {
            expect!(this.peek()?, &XmlEvent::EndElement { .. } => {
                visitor.visit_unit()
            })
        })
    }

    fn deserialize_unit_struct<V: Visitor>(self, _name: &'static str, visitor: V) -> VResult<V> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor>(self, _name: &'static str, len: usize, visitor: V) -> VResult<V> {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_tuple<V: Visitor>(self, len: usize, visitor: V) -> VResult<V> {
        self.deserialize_seq_fixed_size(len, visitor)
    }

    fn deserialize_enum<V: Visitor>(self, _name: &'static str, _variants: &'static [&'static str], visitor: V) -> VResult<V> {
        self.read_inner_value::<V, _>(|this| {
            visitor.visit_enum(EnumVisitor::new(this))
        })
    }

    fn deserialize_string<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.read_inner_value::<V, _>(|this| {
            if let XmlEvent::EndElement { .. } = *this.peek()? {
                return visitor.visit_str("");
            }
            expect!(this.next()?, XmlEvent::Characters(s) => {
                visitor.visit_string(s)
            })
        })
    }

    fn deserialize_seq<V: Visitor>(self, visitor: V) -> VResult<V> {
        visitor.visit_seq(SeqVisitor::new(self, None))
    }

    fn deserialize_seq_fixed_size<V: Visitor>(self, len: usize, visitor: V) -> VResult<V> {
        visitor.visit_seq(SeqVisitor::new(self, Some(len)))
    }

    fn deserialize_map<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.unset_map_value();
        expect!(self.next()?, XmlEvent::StartElement { name, attributes, .. } => {
            let map_value = visitor.visit_map(MapVisitor::new(self, attributes, false))?;
            self.expect_end_element(name)?;
            Ok(map_value)
        })
    }

    fn deserialize_option<V: Visitor>(self, visitor: V) -> VResult<V> {
        if self.is_map_value {
            return self.read_inner_value::<V, _>(|this| {
                visitor.visit_some(this)
            });
        }
        match *self.peek()? {
            XmlEvent::EndElement { .. } => visitor.visit_none(),
            _ => visitor.visit_some(self)
        }
    }

    fn deserialize_ignored_any<V: Visitor>(self, visitor: V) -> VResult<V> {
        self.unset_map_value();
        let depth = self.depth;
        loop {
            self.next()?;
            if self.depth == depth {
                break;
            }
        }
        visitor.visit_unit()
    }

    fn deserialize<V: Visitor>(self, visitor: V) -> VResult<V> {
        match *self.peek()? {
            XmlEvent::StartElement { .. } => {
                self.deserialize_map(visitor)
            }
            XmlEvent::EndElement { .. } => {
                self.deserialize_unit(visitor)
            }
            _ => {
                self.deserialize_string(visitor)
            }
        }
    }
}
