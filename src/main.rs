mod api;
mod libvips;
mod service;

#[macro_use]
extern crate log;

use hyper::service::{make_service_fn, service_fn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    let addr = ([127, 0, 0, 1], 3000).into();

    let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(api::svc)) });
    let server = hyper::Server::bind(&addr).serve(service);

    info!("listening on {}", addr);
    server.await?;

    Ok(())
}
