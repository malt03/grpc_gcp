pub mod firestore;
mod proto;
#[macro_use]
mod singleton;

pub fn initialize(project_id: impl Into<String>) {
    Config::shared().lock().unwrap().project_id = Some(project_id.into());
}

struct Config {
    project_id: Option<String>,
}

impl Config {
    singleton!(Config);

    fn get_project_id(&self) -> &String {
        match self.project_id {
            None => panic!("need to set project_id with grpc_gcp::initialize"),
            Some(ref project_id) => project_id,
        }
    }

    fn new() -> Config {
        Config { project_id: None }
    }
}
