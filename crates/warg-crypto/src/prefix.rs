use crate::ByteVisitor;

pub trait VisitPrefixEncode {
    fn visit_pe<'a, BV: ?Sized + ByteVisitor>(&self, visitor: &mut PrefixEncodeVisitor<'a, BV>);

    fn visit_bv<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        let mut prefix_visitor = PrefixEncodeVisitor::new(visitor);
        self.visit_pe(&mut prefix_visitor);
    }
}

pub struct PrefixEncodeVisitor<'a, BV>
where
    BV: ?Sized + ByteVisitor,
{
    buffer: [u8; 10],
    inner: &'a mut BV,
}

impl<'a, BV> PrefixEncodeVisitor<'a, BV>
where
    BV: ?Sized + ByteVisitor,
{
    pub fn new(inner: &'a mut BV) -> Self {
        Self {
            buffer: [0u8; 10],
            inner,
        }
    }

    pub fn visit_unsigned(&mut self, i: u64) {
        let len = leb128::write::unsigned(&mut self.buffer.as_mut_slice(), i).unwrap();
        self.inner.visit_bytes(&self.buffer[..len]);
    }

    pub fn visit_str_raw(&mut self, s: &str) {
        self.inner.visit_bytes(s.as_bytes());
    }

    pub fn visit_str(&mut self, s: &str) {
        self.visit_unsigned(s.len() as u64);
        self.visit_str_raw(s);
    }
}
