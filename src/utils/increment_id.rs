use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IncrementId(usize);

impl IncrementId {
    pub fn increment(&mut self) -> Self {
        self.0 += 1; // usize is big enough to never overflow
        *self
    }
}
