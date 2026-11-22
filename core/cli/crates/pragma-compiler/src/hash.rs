pub struct Fnv(u64);

impl Fnv {
    pub fn new() -> Self {
        Fnv(0xcbf29ce484222325)
    }

    pub fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

pub fn short_hash(h: u64) -> String {
    format!("{h:016x}")
}

pub fn hash8(bytes: &[u8]) -> String {
    let mut h = Fnv::new();
    h.write(bytes);
    short_hash(h.finish())[..8].to_string()
}

/// `data-s` token of a page's or component's CSS: the build writes it on every element that owner
/// wrote (not on child components' markup, and slot content keeps its writer's token).
pub fn css_owner(kind: &str, name: &str) -> String {
    let mut h = Fnv::new();
    h.write(kind.as_bytes());
    h.write(b":");
    h.write(name.as_bytes());
    format!("s{}", &short_hash(h.finish())[..6])
}
