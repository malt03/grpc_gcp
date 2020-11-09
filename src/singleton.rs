#[macro_export]
macro_rules! singleton {
    ($t:ty) => {
        fn shared() -> std::sync::Arc<std::sync::Mutex<$t>> {
            static mut SINGLETON: Option<std::sync::Arc<std::sync::Mutex<$t>>> = None;
            static ONCE: std::sync::Once = std::sync::Once::new();
            unsafe {
                ONCE.call_once(|| {
                    let s = std::sync::Arc::new(std::sync::Mutex::new(<$t>::new()));
                    SINGLETON = Some(s);
                });
                SINGLETON.clone().unwrap()
            }
        }
    };
}
