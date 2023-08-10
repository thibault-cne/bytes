pub trait Buf {
    fn remaining(&self) -> usize;

    fn chuncks(&self) -> &[u8];

    fn advance(&mut self, cnt: usize);

    fn has_remaining(&self) -> bool {
        self.remaining() > 0
    }

    fn get_u8(&mut self) -> u8 {
        assert!(
            self.has_remaining(),
            "cannot read from buffer, no remaining bytes"
        );
        let ret = self.chuncks()[0];
        self.advance(1);
        ret
    }

    fn peek_u8(&self) -> u8 {
        assert!(
            self.has_remaining(),
            "cannot read from buffer, no remaining bytes"
        );
        self.chuncks()[0]
    }
}

impl Buf for &[u8] {
    fn remaining(&self) -> usize {
        self.len()
    }

    fn advance(&mut self, cnt: usize) {
        *self = &self[cnt..];
    }

    fn chuncks(&self) -> &[u8] {
        self
    }
}
