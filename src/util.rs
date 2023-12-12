use std::sync::mpsc::{channel, sync_channel, Receiver, RecvError, SendError, Sender, SyncSender};

pub fn rec_duplex<T>() -> (RecDuplex<T>, RecDuplex<T>) {
    let (s1, r1) = channel();
    let (s2, r2) = channel();
    (
        RecDuplex { recv: r1, send: s2 },
        RecDuplex { recv: r2, send: s1 },
    )
}

pub struct RecDuplex<T> {
    pub recv: Receiver<T>,
    pub send: Sender<T>,
}
impl<T: 'static + Send + Sync> RecDuplex<T> {
    pub fn recv(&self) -> Result<T, RecvError> {
        self.recv.recv()
    }
    pub fn send(&self, val: T) -> Result<(), SendError<T>> {
        self.send.send(val)
    }
    pub fn with_value<U>(
        &self,
        closure: impl FnOnce(&mut T) -> eyre::Result<U>,
    ) -> eyre::Result<U> {
        let mut val = self.recv.recv()?;
        let res = closure(&mut val)?;
        self.send.send(val)?;
        Ok(res)
    }
}
pub fn sync_duplex<T>(bound: usize) -> (SyncDuplex<T>, SyncDuplex<T>) {
    let (s1, r1) = sync_channel(bound);
    let (s2, r2) = sync_channel(bound);
    (
        SyncDuplex { recv: r1, send: s2 },
        SyncDuplex { recv: r2, send: s1 },
    )
}

pub struct SyncDuplex<T> {
    pub recv: Receiver<T>,
    pub send: SyncSender<T>,
}
impl<T: 'static + Send + Sync> SyncDuplex<T> {
    pub fn recv(&self) -> Result<T, RecvError> {
        self.recv.recv()
    }
    pub fn send(&self, val: T) -> Result<(), SendError<T>> {
        self.send.send(val)
    }
    pub fn use_value<U>(&self, closure: impl FnOnce(&mut T) -> eyre::Result<U>) -> eyre::Result<U> {
        let mut val = self.recv.recv()?;
        let res = closure(&mut val)?;
        self.send.send(val)?;
        Ok(res)
    }
}
