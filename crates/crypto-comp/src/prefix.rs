use crate::ByteVisitor;

pub trait VisitPrefixEncode {
    fn visit_pe<BV: ?Sized + ByteVisitor + std::fmt::Debug>(&self, visitor: &mut PrefixEncodeVisitor<BV>);

    fn visit_bv<BV: ?Sized + ByteVisitor + std::fmt::Debug>(&self, visitor: &mut BV) {
        let mut prefix_visitor = PrefixEncodeVisitor::new(visitor);
        self.visit_pe(&mut prefix_visitor);
    }
}

pub struct PrefixEncodeVisitor<'a, BV>
where
    BV: ?Sized + ByteVisitor + std::fmt::Debug,
{
    buffer: [u8; 10],
    pub inner: &'a mut BV,
}

impl<'a, BV> PrefixEncodeVisitor<'a, BV>
where
    BV: ?Sized + ByteVisitor + std::fmt::Debug,
{
    pub fn new(inner: &'a mut BV) -> Self {
        Self {
            buffer: [0u8; 10],
            inner,
        }
    }

    pub fn visit_unsigned(&mut self, i: u64) {
        let len = leb128::write::unsigned(&mut self.buffer.as_mut_slice(), i).unwrap();
        println!("len {}", len);
        println!("VISITED BYTES {:?}", &self.buffer[..len]);
        self.inner.visit_bytes(&self.buffer[..len]);
    }

    pub fn visit_str_raw(&mut self, s: &str) {
        self.inner.visit_bytes(s.as_bytes());
    }

    pub fn visit_str(&mut self, s: &str) {
        println!("THE LENGTH OF STRING {}", s.len());
        println!("THE RAW STRING {}", s);
        self.visit_unsigned(s.len() as u64);
        self.visit_str_raw(s);
    }
}
