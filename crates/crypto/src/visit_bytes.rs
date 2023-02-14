pub trait ByteVisitor {
    fn visit_bytes(&mut self, bytes: impl AsRef<[u8]>);

    fn visit_nested(&mut self, nested: impl VisitBytes) {
        nested.visit(self)
    }
}

impl<'a, BV: ?Sized + ByteVisitor> ByteVisitor for &'a mut BV {
    fn visit_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        (self as &mut BV).visit_bytes(bytes)
    }
}

pub trait VisitBytes {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV);
}

impl<'a, VB: ?Sized + VisitBytes> VisitBytes for &'a VB {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        (self as &VB).visit(visitor)
    }
}

impl VisitBytes for u8 {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        visitor.visit_bytes([*self]);
    }
}

impl<'a> VisitBytes for &'a [u8] {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        visitor.visit_bytes(self);
    }
}

impl<'a> VisitBytes for &'a str {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        visitor.visit_bytes(self.as_bytes());
    }
}

impl VisitBytes for () {
    fn visit<BV: ?Sized + ByteVisitor>(&self, _visitor: &mut BV) {}
}

impl<T1> VisitBytes for (T1,)
where
    T1: VisitBytes,
{
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        self.0.visit(visitor);
    }
}

impl<T1, T2> VisitBytes for (T1, T2)
where
    T1: VisitBytes,
    T2: VisitBytes,
{
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        self.0.visit(visitor);
        self.1.visit(visitor);
    }
}

impl<T1, T2, T3> VisitBytes for (T1, T2, T3)
where
    T1: VisitBytes,
    T2: VisitBytes,
    T3: VisitBytes,
{
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        self.0.visit(visitor);
        self.1.visit(visitor);
        self.2.visit(visitor);
    }
}

impl<T1, T2, T3, T4> VisitBytes for (T1, T2, T3, T4)
where
    T1: VisitBytes,
    T2: VisitBytes,
    T3: VisitBytes,
    T4: VisitBytes,
{
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        self.0.visit(visitor);
        self.1.visit(visitor);
        self.2.visit(visitor);
        self.3.visit(visitor);
    }
}

impl<T1, T2, T3, T4, T5> VisitBytes for (T1, T2, T3, T4, T5)
where
    T1: VisitBytes,
    T2: VisitBytes,
    T3: VisitBytes,
    T4: VisitBytes,
    T5: VisitBytes,
{
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        self.0.visit(visitor);
        self.1.visit(visitor);
        self.2.visit(visitor);
        self.3.visit(visitor);
        self.4.visit(visitor);
    }
}

impl VisitBytes for [u8; 32] {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        visitor.visit_bytes(self.as_slice())
    }
}
