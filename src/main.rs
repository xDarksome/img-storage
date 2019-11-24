mod api;
mod libvips;
mod service;
mod tests;

#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use hyper::service::{make_service_fn, service_fn};
use std::env::var;
use std::error::Error;
use tokio::prelude::*;
use tokio_net::signal::unix::{signal, SignalKind};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
    tokio_executor::spawn(notify_shutdown(tx));

    let port: u16 = var("PORT").unwrap_or_default().parse().unwrap_or(3000);
    let addr = ([0, 0, 0, 0], port).into();

    let svc = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(api::svc)) });
    let server = hyper::Server::bind(&addr)
        .serve(svc)
        .with_graceful_shutdown(async {
            rx.recv().await;
            info!("shutting down");
        });

    info!("listening on {}", addr);
    server.await?;
    Ok(())
}

async fn notify_shutdown(mut tx: tokio::sync::mpsc::Sender<()>) {
    let _ = signal(SignalKind::terminate())
        .expect("bind SIGTERM")
        .into_future()
        .await;
    tx.send(()).await.expect("notify");
}
