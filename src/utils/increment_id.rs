use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IncrementId(usize);

impl IncrementId {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}
