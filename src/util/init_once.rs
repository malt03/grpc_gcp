use async_trait::async_trait;
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;

pub(crate) struct InitOnce<T, AsyncCreatorType: AsyncCreator<T>> {
    initialized: Mutex<bool>,
    value: OnceCell<T>,
    creator: AsyncCreatorType,
}

#[async_trait]
pub(crate) trait AsyncCreator<T> {
    async fn create(&self) -> Result<T, Box<dyn std::error::Error>>;
}

impl<T, AsyncCreatorType: AsyncCreator<T>> InitOnce<T, AsyncCreatorType> {
    pub(crate) fn new(creator: AsyncCreatorType) -> Self {
        InitOnce {
            initialized: Mutex::new(false),
            value: OnceCell::new(),
            creator: creator,
        }
    }

    pub(crate) async fn get<'s>(&'s self) -> Result<&'s T, Box<dyn std::error::Error>> {
        if let Some(value) = self.value.get() {
            return Ok(value);
        }
        let mut initialized = self.initialized.lock().await;
        if !*initialized {
            let value = self.creator.create().await?;
            if self.value.set(value).is_err() {
                panic!("unexpected");
            }
            *initialized = true;
        }
        Ok(self.value.get().unwrap())
    }
}
