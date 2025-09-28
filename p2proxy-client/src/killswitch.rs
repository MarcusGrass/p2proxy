use tokio::sync::broadcast::error::TryRecvError;

#[derive(Debug)]
pub struct ProxyKillSwitch {
    inner: tokio::sync::broadcast::Sender<()>,
}

impl Drop for ProxyKillSwitch {
    #[inline]
    fn drop(&mut self) {
        let _ = self.inner.send(());
    }
}

pub struct ProxyKillSwitchListener {
    inner: tokio::sync::broadcast::Receiver<()>,
}

pub enum KillSwitchResult<T> {
    Killed,
    Finished(T),
}

impl ProxyKillSwitchListener {
    pub async fn killed(&mut self) -> () {
        Self::consume_broadcast_result(&self.inner.recv().await);
    }

    pub async fn if_not_killed<T, F: Future<Output = T>>(
        &mut self,
        action: F,
    ) -> KillSwitchResult<T> {
        tokio::select! {
            res = self.inner.recv() => {
                Self::consume_broadcast_result(&res);
                KillSwitchResult::Killed
            }
            res = action => {
                KillSwitchResult::Finished(res)
            }
        }
    }

    fn consume_broadcast_result(result: &Result<(), tokio::sync::broadcast::error::RecvError>) {
        match result {
            Ok(()) => {
                tracing::debug!("killswitch manually triggered");
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                tracing::debug!("killswitch triggered on drop");
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(l)) => {
                tracing::debug!("killswitch triggered on lag: {l}");
            }
        }
    }

    pub fn duplicate(&mut self) -> Option<Self> {
        let cloned = self.inner.resubscribe();
        if self.has_closed() {
            None
        } else {
            Some(Self { inner: cloned })
        }
    }

    fn has_closed(&mut self) -> bool {
        match self.inner.try_recv() {
            Ok(()) => {
                // Triggered
                true
            }
            Err(e) => match e {
                TryRecvError::Empty => false,
                TryRecvError::Closed | TryRecvError::Lagged(_) => true,
            },
        }
    }
}

impl ProxyKillSwitch {
    #[must_use]
    pub fn new_pair() -> (Self, ProxyKillSwitchListener) {
        let (kill_send, kill_recv) = tokio::sync::broadcast::channel(1);
        let listener = ProxyKillSwitchListener { inner: kill_recv };
        (Self { inner: kill_send }, listener)
    }
    #[inline]
    pub fn signal(&self) {
        let _ = self.inner.send(());
    }
}
