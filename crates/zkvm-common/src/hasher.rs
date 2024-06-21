use std::hash::{BuildHasher, Hasher};

pub struct BytesHasher {
    hash: u64,
}

impl Hasher for BytesHasher {
    fn write(&mut self, bytes: &[u8]) {
        // Assuming the bytes are exactly 32 bytes, interpret the first 8 bytes as a u64
        println!("bytes: {:?}", bytes);
        self.hash = u64::from_ne_bytes(bytes[0..8].try_into().unwrap());
        println!("hash: {:?}", self.hash);
    }

    fn finish(&self) -> u64 {
        println!("finish: {:?}", self.hash);
        self.hash
    }
}

pub struct BytesHasherBuilder;

impl BuildHasher for BytesHasherBuilder {
    type Hasher = BytesHasher;

    fn build_hasher(&self) -> BytesHasher {
        BytesHasher { hash: 0 }
    }
}
