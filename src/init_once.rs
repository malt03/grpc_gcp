use once_cell::sync::OnceCell;
use std::future::Future;
use tokio::sync::Mutex;

pub(crate) struct InitOnce<T> {
    initialized: Mutex<bool>,
    value: OnceCell<T>,
}

impl<T> InitOnce<T> {
    pub(crate) fn new() -> Self {
        InitOnce {
            initialized: Mutex::new(false),
            value: OnceCell::new(),
        }
    }

    pub(crate) async fn init<E: std::error::Error, Fut: Future<Output = Result<T, E>>>(
        &self,
        f: fn() -> Fut,
    ) -> Result<(), E> {
        if self.value.get().is_some() {
            return Ok(());
        }
        let initialized = self.initialized.lock().await;
        if !*initialized {
            let value = f().await?;
            if self.value.set(value).is_err() {
                panic!("unexpected");
            }
        }
        Ok(())
    }

    pub(crate) async fn get<'s>(&'s self) -> &'s T {
        if let Some(value) = self.value.get() {
            return value;
        }
        let initialized = self.initialized.lock().await;
        if !*initialized {
            panic!("call init before get.");
        }
        self.value.get().unwrap()
    }
}
