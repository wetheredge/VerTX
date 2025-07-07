use embedded_io_async::Read;

pub(super) struct Buffer<'a> {
    inner: &'a mut [u8],
    len: usize,
}

impl<'a> Buffer<'a> {
    pub(super) const fn new(inner: &'a mut [u8]) -> Self {
        Self { inner, len: 0 }
    }
}

impl Buffer<'static> {
    pub(super) const fn empty() -> Self {
        Self {
            inner: &mut [],
            len: 0,
        }
    }
}

impl Buffer<'_> {
    pub(super) const fn len(&self) -> usize {
        self.len
    }

    pub(super) const fn capacity(&self) -> usize {
        self.inner.len()
    }

    pub(super) async fn read_from<R: Read>(&mut self, reader: &mut R) -> Result<usize, R::Error> {
        let len = reader.read(&mut self.inner[self.len..]).await?;
        self.len += len;
        Ok(len)
    }

    pub(super) fn discard_prefix(&mut self, prefix: usize) {
        let remaining = prefix..self.len;
        self.len = remaining.len();
        self.inner.copy_within(remaining, 0);
    }

    pub(super) fn split_at(&mut self, mid: usize) -> (&[u8], Buffer<'_>) {
        let (head, tail) = self.inner.split_at_mut(mid);
        let tail = Buffer {
            inner: tail,
            len: self.len - head.len(),
        };
        (head, tail)
    }
}

impl core::ops::Deref for Buffer<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.inner[..self.len]
    }
}
