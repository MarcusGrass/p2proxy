#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub(crate) struct PeerId(u64);

#[derive(Default, Debug)]
pub(crate) struct PeerIdGenerator(u64);

impl PeerIdGenerator {
    pub fn next_id(&mut self) -> PeerId {
        self.0 += 1;
        PeerId(self.0)
    }
}
