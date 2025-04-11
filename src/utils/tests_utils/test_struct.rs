pub(crate) struct TestStruct;

impl TestStruct {
    pub(crate) fn consume(self) {}
    pub(crate) fn consume_ref(&self) {}
    pub(crate) fn consume_mut(&mut self) {}
}
