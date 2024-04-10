use log::error;
use save_cloud::{app, render, resource::Resource};

fn main() {
    if let Err(err) = match Resource::new(false) {
        Ok(resource) => render::launch(app::Main, resource),
        Err(err) => Err(err),
    } {
        error!("Failed to initialize app: {:?}", err);
    }
}
