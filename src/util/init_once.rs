use async_trait::async_trait;
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;

pub(crate) struct AsyncInitOnce<Initializer: AsyncInitializer> {
    initialized: Mutex<bool>,
    value: OnceCell<Initializer::T>,
    initializer: Initializer,
}

#[async_trait]
pub(crate) trait AsyncInitializer {
    type T;
    type Error;
    async fn create(&self) -> Result<Self::T, Self::Error>;
}

impl<Initializer: AsyncInitializer> AsyncInitOnce<Initializer> {
    pub(crate) fn new(initializer: Initializer) -> Self {
        AsyncInitOnce {
            initialized: Mutex::new(false),
            value: OnceCell::new(),
            initializer: initializer,
        }
    }

    pub(crate) async fn get<'s>(&'s self) -> Result<&'s Initializer::T, Initializer::Error> {
        if let Some(value) = self.value.get() {
            return Ok(value);
        }
        let mut initialized = self.initialized.lock().await;
        if !*initialized {
            let value = self.initializer.create().await?;
            if self.value.set(value).is_err() {
                panic!("unexpected");
            }
            *initialized = true;
        }
        Ok(self.value.get().unwrap())
    }
}
