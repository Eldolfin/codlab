use std::sync::mpsc::{self, Receiver};

pub struct PeekableReceiver<T> {
    inner: Receiver<T>,
    next: Option<T>,
}

impl<T> From<Receiver<T>> for PeekableReceiver<T> {
    fn from(inner: Receiver<T>) -> Self {
        Self { inner, next: None }
    }
}

impl<T> PeekableReceiver<T> {
    pub fn try_recv_peek(&mut self) -> Result<Option<&T>, mpsc::RecvError> {
        if self.next.is_some() {
            Ok(self.next.as_ref())
        } else {
            self.next = match self.inner.try_recv() {
                Ok(next) => Some(next),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(mpsc::RecvError);
                }
            };
            Ok(self.next.as_ref())
        }
    }
    pub fn try_recv(&mut self) -> Result<T, mpsc::TryRecvError> {
        match self.next.take() {
            Some(next) => Ok(next),
            None => self.inner.try_recv(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::PeekableReceiver;

    #[test]
    fn test_peekable_receiver() -> anyhow::Result<()> {
        let (send, recv) = mpsc::channel();
        let mut recv = PeekableReceiver::from(recv);

        assert_eq!(recv.try_recv_peek()?, None);

        send.send(1).unwrap();
        assert_eq!(recv.try_recv_peek()?, Some(&1));
        assert_eq!(recv.try_recv_peek()?, Some(&1));
        assert_eq!(recv.try_recv()?, 1);
        assert_eq!(recv.try_recv_peek()?, None);
        Ok(())
    }
}
