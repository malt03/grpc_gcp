use async_trait::async_trait;
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;

pub(crate) struct AsyncInitOnce<T, Initializer: AsyncInitializer<T>> {
    initialized: Mutex<bool>,
    value: OnceCell<T>,
    initializer: Initializer,
}

#[async_trait]
pub(crate) trait AsyncInitializer<T> {
    async fn create(&self) -> Result<T, Box<dyn std::error::Error>>;
}

impl<T, Initializer: AsyncInitializer<T>> AsyncInitOnce<T, Initializer> {
    pub(crate) fn new(initializer: Initializer) -> Self {
        AsyncInitOnce {
            initialized: Mutex::new(false),
            value: OnceCell::new(),
            initializer: initializer,
        }
    }

    pub(crate) async fn get<'s>(&'s self) -> Result<&'s T, Box<dyn std::error::Error>> {
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
