use crate::LOOPS;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::time::SystemTime;

#[derive(Serialize, Deserialize)]
struct Value<'a> {
    fruit: u32,
    initialized: bool,
    location: &'a str,
    list: Vec<Item<'a>>,
}

#[derive(Serialize, Deserialize)]
struct Item<'a> {
    name: &'a str,
    rating: f64,
    postfix: &'a str,
    sibling: Sibling<'a>,
}

#[derive(Serialize, Deserialize)]
struct Sibling<'a> {
    time: i32,
    ratio: f64,
    size: u16,
    #[serde(borrow)]
    parent: Parent<'a>,
}

#[derive(Serialize, Deserialize)]
struct Parent<'a> {
    id: u64,
    count: i16,
    prefix: &'a str,
    length: u32,
}

pub struct SerdeJsonBench;

impl SerdeJsonBench {
    pub fn size_bench() {
        let encoded = Self::encode_single();

        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write(&encoded[..]).unwrap();
        let compressed = e.finish().unwrap();

        println!(
            "serde_json:  size: {}b, zlib: {}b",
            encoded.len(),
            compressed.len()
        );
    }

    pub fn encode_bench(base: u128) {
        let start = SystemTime::now();

        for _x in 0..LOOPS {
            let buffer = Self::encode_single();
            assert_eq!(buffer.len(), 673);
        }

        let time = SystemTime::now()
            .duration_since(start)
            .expect("Time went backwards");
        println!(
            "serde_json:  {:>5.2}ms {:.2}",
            time.as_millis(),
            (base as f64 / time.as_micros() as f64)
        );
    }

    fn encode_single() -> Vec<u8> {
        let mut json_object = Value {
            fruit: 2,
            initialized: true,
            location: "http://arstechnica.com",
            list: Vec::new(),
        };

        for x in 0..3 {
            json_object.list.push(Item {
                name: "Hello, World!",
                rating: 3.1415432432445543543 + (x as f64),
                postfix: "!",
                sibling: Sibling {
                    time: 123456 + (x as i32),
                    ratio: 3.14159,
                    size: 10000 + (x as u16),
                    parent: Parent {
                        id: 0xABADCAFEABADCAFE + (x as u64),
                        count: 1000 + (x as i16),
                        prefix: "@",
                        length: 10000 + (x as u32),
                    },
                },
            });
        }

        serde_json::to_vec(&json_object).unwrap()
    }

    pub fn decode_bench(base: u128) {
        let buffer = Self::encode_single();

        let start = SystemTime::now();

        for _x in 0..LOOPS {
            let container = serde_json::from_slice::<Value>(&buffer).unwrap();

            assert_eq!(container.location, "http://arstechnica.com");
            assert_eq!(container.fruit, 2);
            assert_eq!(container.initialized, true);
            assert_eq!(container.list.len(), 3);
            for (x, foobar) in container.list.iter().enumerate() {
                assert_eq!(foobar.name, "Hello, World!");
                assert_eq!(foobar.rating, 3.1415432432445543543 + (x as f64));
                assert_eq!(foobar.postfix, "!");
                assert_eq!(foobar.sibling.time, 123456 + (x as i32));
                assert_eq!(foobar.sibling.ratio, 3.14159);
                assert_eq!(foobar.sibling.size, 10000 + (x as u16));
                assert_eq!(foobar.sibling.parent.id, 0xABADCAFEABADCAFE + (x as u64));
                assert_eq!(foobar.sibling.parent.count, 1000 + (x as i16));
                assert_eq!(foobar.sibling.parent.prefix, "@");
                assert_eq!(foobar.sibling.parent.length, 10000 + (x as u32));
            }
        }

        let time = SystemTime::now()
            .duration_since(start)
            .expect("Time went backwards");
        println!(
            "serde_json:  {:>5.2}ms {:.2}",
            time.as_millis(),
            (base as f64 / time.as_micros() as f64)
        );
    }
}
