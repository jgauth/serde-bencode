use serde::de::{
    self, Deserialize, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use serde::forward_to_deserialize_any;

use crate::error::{Error, Result};
use std::ops::{AddAssign, MulAssign, Neg};

pub struct Deserializer<'de> {
    input: &'de [u8],
}

impl<'de> Deserializer<'de> {
    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer { input }
    }
}

pub fn from_bytes<'a, T>(b: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(b);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

// basic parsing functions
impl<'de> Deserializer<'de> {
    fn peek_byte(&self) -> Result<u8> {
        match self.input.iter().next() {
            Some(x) => Ok(*x),
            _ => Err(Error::Eof),
        }
    }

    fn next_byte(&mut self) -> Result<u8> {
        let b = self.peek_byte()?;
        self.input = &self.input[1..];
        Ok(b)
    }

    fn parse_unsigned<T>(&mut self) -> Result<T>
    where
        T: AddAssign<T> + MulAssign<T> + From<u8>,
    {
        let mut int = match self.next_byte()? {
            b @ b'0'..=b'9' => T::from(b - b'0'),
            _ => {
                return Err(Error::ExpectedInteger);
            }
        };
        loop {
            match self.input.iter().next() {
                Some(b @ b'0'..=b'9') => {
                    self.input = &self.input[1..];
                    int *= T::from(10);
                    int += T::from(b - b'0');
                }
                _ => {
                    return Ok(int);
                }
            }
        }
    }

    fn parse_signed<T>(&mut self) -> Result<T>
    where
        T: Neg<Output = T> + AddAssign<T> + MulAssign<T> + From<i8> + From<u8>,
    {
        let is_negative = match self.peek_byte()? {
            b'-' => {
                self.next_byte()?;
                true
            }
            _ => false,
        };

        let mut num: T = self.parse_unsigned::<T>()?;
        if is_negative {
            num = -num;
        }
        return Ok(num);
    }

    fn parse_num<T>(&mut self) -> Result<T>
    where
        T: Neg<Output = T> + AddAssign<T> + MulAssign<T> + From<i8> + From<u8>,
    {
        if self.next_byte()? != b'i' {
            return Err(Error::ExpectedI);
        }

        let n = self.parse_signed();

        match self.next_byte()? {
            b'e' => n,
            _ => Err(Error::ExpectedE),
        }
    }

    fn parse_byte_array(&mut self) -> Result<&'de [u8]> {
        let length: usize = self.parse_unsigned()?;
        if self.next_byte()? != b':' {
            return Err(Error::ExpectedColon);
        }

        let s = &self.input[..length];
        self.input = &self.input[length..];
        Ok(s)
    }
}

#[cfg(test)]
mod parser_tests {
    use super::Deserializer;
    use crate::error::Error;

    #[test]
    fn test_parse_num() {
        let mut de = Deserializer { input: b"i123e" };
        let expected = 123i64;
        assert_eq!(expected, de.parse_num().unwrap());
        assert_eq!(Err(Error::Eof), de.next_byte());
    }

    #[test]
    fn test_parse_num_negative() {
        let mut de = Deserializer { input: b"i-123e" };
        let expected = -123i64;
        assert_eq!(expected, de.parse_num().unwrap());
        assert_eq!(Err(Error::Eof), de.next_byte());
    }

    #[test]
    fn test_parse_num_no_i() {
        let mut de = Deserializer { input: b"123e" };
        assert_eq!(Err(Error::ExpectedI), de.parse_num::<i32>());
    }

    #[test]
    fn test_parse_num_no_e() {
        let mut de = Deserializer { input: b"i123F" };
        assert_eq!(Err(Error::ExpectedE), de.parse_num::<i32>());
    }

    #[test]
    fn test_parse_byte_array() {
        let mut de = Deserializer { input: b"5:Hello" };
        let expected = b"Hello";
        assert_eq!(expected, de.parse_byte_array().unwrap());
        assert_eq!(Err(Error::Eof), de.next_byte());
    }

    #[test]
    fn test_parse_signed() {
        let mut de = Deserializer { input: b"-321" };
        let expected = -321i32;
        assert_eq!(expected, de.parse_signed().unwrap())
    }

    #[test]
    fn test_parse_unsigned() {
        let mut de = Deserializer { input: b"321" };
        let expected = 321u32;
        assert_eq!(expected, de.parse_unsigned().unwrap())
    }

    #[test]
    fn test_peek_byte() {
        let de = Deserializer { input: b"Hello" };
        let expected = b'H';

        assert_eq!(expected, de.peek_byte().unwrap())
    }

    #[test]
    fn test_peek_byte_empty() {
        let de = Deserializer { input: &[] };
        let expected = Err(Error::Eof);

        assert_eq!(expected, de.peek_byte())
    }

    #[test]
    fn test_next_byte() {
        let mut de = Deserializer { input: b"Hello" };

        assert_eq!(b'H', de.next_byte().unwrap());
        assert_eq!(b"ello", de.input);
        assert_eq!(b'e', de.next_byte().unwrap());
        assert_eq!(b'l', de.next_byte().unwrap());
        assert_eq!(b'l', de.next_byte().unwrap());
        assert_eq!(b'o', de.next_byte().unwrap());
        assert_eq!(Err(Error::Eof), de.next_byte());
    }
}

impl<'de> de::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.peek_byte()? {
            b'i' => self.deserialize_i64(visitor),
            b'0'..=b'9' => self.deserialize_bytes(visitor),
            b'l' => self.deserialize_seq(visitor),
            b'd' => self.deserialize_map(visitor),
            _ => Err(Error::Syntax),
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_num()?)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self.parse_byte_array()?)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.next_byte()? {
            b'l' => {
                let value = visitor.visit_seq(SeqReader::new(self))?;
                match self.next_byte()? {
                    b'e' => Ok(value),
                    _ => Err(Error::ExpectedListEnd),
                }
            }
            _ => Err(Error::ExpectedList),
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.next_byte()? {
            b'd' => {
                let value = visitor.visit_map(MapReader::new(self))?;
                match self.next_byte()? {
                    b'e' => Ok(value),
                    _ => Err(Error::ExpectedDictEnd),
                }
            }
            _ => Err(Error::ExpectedDict),
        }
    }

    // fn deserialize_enum<V>(
    //     self,
    //     _name: &'static str,
    //     _variants: &'static [&'static str],
    //     visitor: V,
    // ) -> Result<V::Value>
    // where
    //     V: Visitor<'de>,
    // {
    //     visitor.visit_enum(EnumReader::new(self))
    // }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        byte_buf option unit unit_struct newtype_struct tuple
        tuple_struct struct identifier ignored_any enum
    }
}

struct SeqReader<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> SeqReader<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        SeqReader { de }
    }
}

impl<'de, 'a> SeqAccess<'de> for SeqReader<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.de.peek_byte()? == b'e' {
            return Ok(None);
        }

        Ok(Some(seed.deserialize(&mut *self.de)?))
    }
}

struct MapReader<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> MapReader<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        MapReader { de }
    }
}

impl<'a, 'de> MapAccess<'de> for MapReader<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.de.peek_byte()? == b'e' {
            return Ok(None);
        }

        Ok(Some(seed.deserialize(&mut *self.de)?))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

// struct EnumReader<'a, 'de: 'a> {
//     de: &'a mut Deserializer<'de>,
// }

// impl<'a, 'de> EnumReader<'a, 'de> {
//     fn new(de: &'a mut Deserializer<'de>) -> Self {
//         EnumReader { de }
//     }
// }

// impl<'a, 'de> EnumAccess<'de> for EnumReader<'a, 'de> {
//     type Error = Error;
//     type Variant = Self;

//     fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
//     where
//         V: DeserializeSeed<'de>,
//     {
//         Ok((seed.deserialize(&mut *self.de)?, self))
//     }
// }

// impl<'a, 'de> VariantAccess<'de> for EnumReader<'a, 'de> {
//     type Error = Error;

//     // I have no idea how this applies here
//     fn unit_variant(self) -> Result<()> {
//         Ok(())
//     }

//     fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
//     where
//         T: DeserializeSeed<'de>,
//     {
//         let value = seed.deserialize(&mut *self.de)?;
//         match self.de.next_byte()? {
//             b'e' => Ok(value),
//             _ => Err(Error::ExpectedDictEnd),
//         }
//     }

//     fn struct_variant<V>(
//         self,
//         _fields: &'static [&'static str],
//         visitor: V,
//     ) -> std::result::Result<V::Value, Self::Error>
//     where
//         V: Visitor<'de>,
//     {
//         let value = serde::de::Deserializer::deserialize_map(&mut *self.de, visitor)?;
//         match self.de.next_byte()? {
//             b'e' => Ok(value),
//             _ => Err(Error::ExpectedDictEnd),
//         }
//     }

//     fn tuple_variant<V>(self, _len: usize, visitor: V) -> std::result::Result<V::Value, Self::Error>
//     where
//         V: Visitor<'de>,
//     {
//         let value = serde::de::Deserializer::deserialize_seq(&mut *self.de, visitor)?;
//         match self.de.next_byte()? {
//             b'e' => Ok(value),
//             _ => Err(Error::ExpectedListEnd),
//         }
//     }
// }

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use super::from_bytes;
    use serde::Deserialize;

    #[test]
    fn test_num() {
        assert_eq!(123i64, from_bytes::<i64>(b"i123e").unwrap());
        assert_eq!(-123i64, from_bytes::<i64>(b"i-123e").unwrap());
        assert_eq!(-023i64, from_bytes::<i64>(b"i-23e").unwrap());
    }

    #[test]
    fn test_byte_string() {
        assert_eq!(b"hello", from_bytes::<&[u8]>(b"5:hello").unwrap());
        assert_eq!(b"", from_bytes::<&[u8]>(b"0:").unwrap());
        assert_eq!(b"@@!", from_bytes::<&[u8]>(b"3:@@!").unwrap());
    }

    #[test]
    fn test_homo_list() {
        let expected = vec![b"hello", b"world"];
        let v: Vec<&[u8]> = from_bytes(b"l5:hello5:worlde").unwrap();
        assert_eq!(expected, v);
    }

    #[test]
    fn test_hetero_list_to_tuple() {
        #[derive(Deserialize, PartialEq, Debug)]
        enum Value<'a> {
            Number(i64),
            ByteString(&'a [u8]),
        }
        let expected: (i64, &[u8], i64) = (
            10,
            b"Hello",
            69
        );
        let b = b"li10e5:Helloi69ee";
        let v = from_bytes(b).unwrap();
        assert_eq!(expected, v);
    }

    #[test]
    fn test_dict_to_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Test<'a> {
            a: i64,
            b: &'a [u8]
        }

        let expected = Test { a: 69, b: b"Hello" };
        let b = b"d1:ai69e1:b5:Helloe";
        let v = from_bytes(b).unwrap();
        assert_eq!(expected, v);
    }

    #[test]
    fn test_map_str_to_int() {
        let expected = HashMap::from([
            ("a", 69),
            ("b", 20)
        ]);
        let b = b"d1:ai69e1:bi20ee";
        assert_eq!(expected, from_bytes(b).unwrap());
    }

    #[test]
    fn test_map_str_to_bytes() {
        let mut expected: HashMap<&[u8], &[u8]> = HashMap::new();
        expected.insert(b"first key", b"hello");
        expected.insert(b"2nd key", b"world");
        let b = b"d9:first key5:hello7:2nd key5:worlde";
        assert_eq!(expected, from_bytes(b).unwrap());
    }

    #[test]
    fn test_nested_structs() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Info<'a> {
            length: i64,
            name: &'a [u8],
        }
        #[derive(Deserialize, PartialEq, Debug)]
        struct Parent<'a> {
            announce: &'a [u8],
            info: Info<'a>,
        }

        let expected = Parent{
            announce: b"hello",
            info: Info {
                length: 5,
                name: b"john",
            }
        };
        let b = b"d1:ai1e8:announce5:hello4:infod6:lengthi5e4:name4:johnee";
        assert_eq!(expected, from_bytes(b).unwrap());

    }


    #[test]
    fn test_torrent() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Torrent<'a>{
            announce: String,
            comment: String,
            #[serde(rename = "created by")]
            created_by: String,
            #[serde(rename = "creation date")]
            creation_date: i64,
            #[serde(borrow)]
            info: Info<'a>,
            #[serde(rename = "url-list")]
            url_list: Vec<String>
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct Info<'a> {
            length: i64,
            name: String,
            #[serde(rename = "piece length")]
            piece_length: i64,
            pieces: &'a [u8],
        }

        let contents = std::fs::read("debian.torrent").unwrap();
        let b = contents.as_slice();
        let v: Torrent = from_bytes(b).unwrap();
        println!("{:?}", v);
        // assert_eq!(expected, from_bytes(b).unwrap());

    }
}
