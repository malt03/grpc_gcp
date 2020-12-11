use once_cell::sync::OnceCell;

static CONFIG: OnceCell<Config> = OnceCell::new();

pub fn init(project_id: impl Into<String>) {
    CONFIG.set(Config::new(project_id)).unwrap();
}

pub fn project_id() -> &'static String {
    match CONFIG.get() {
        None => panic!("need to set project_id with grpc_gcp::init"),
        Some(config) => &config.project_id,
    }
}

#[derive(Debug)]
struct Config {
    project_id: String,
}

impl Config {
    fn new(project_id: impl Into<String>) -> Config {
        Config {
            project_id: project_id.into(),
        }
    }
}
