use std::{collections::HashSet, sync::{Arc, Mutex}};
use actix::{Actor, Addr};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use log::info;
use processing::{ProcessingActor, DetectionScore, SignalWindow};
use udp::UdpListenerActor;
use websockets::WsActor;

mod udp;
mod processing;
mod websockets;
mod utils;

struct AppState {
    processing_actor: Addr<ProcessingActor>,
    udp_listener_actor: Addr<UdpListenerActor>,
    latest_score: Arc<Mutex<DetectionScore>>,
}

impl AppState {
    fn new(
        udp_listener_actor: Addr<UdpListenerActor>,
        processing_actor: Addr<ProcessingActor>,
    ) -> Self {
        Self {
            processing_actor,
            udp_listener_actor,
            latest_score: Arc::new(Mutex::new(DetectionScore::new())),
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("Starting server");

    // Start the UdpListenerActor and store its Addr
    let udp_listener_actor = UdpListenerActor::new().await.start();
    info!("UDP listener actor started");

    // Start ProcessingActor and store its Addr
    let processing_actor = ProcessingActor::new().start();
    info!("Processing actor started");

    HttpServer::new(move || {
        App::new()
            // Share DetectionActor's address via app data, accessible through web::Data
            .app_data(web::Data::new(AppState::new(
                udp_listener_actor.clone(), processing_actor.clone()
            )))
            .route("/", web::get().to(|| async { HttpResponse::Ok().finish() }))
            .route("/ws", web::get().to(ws_route))
    })
    .bind(("127.0.0.1", 4001))?
    .run()
    .await
}

async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<AppState>
) -> Result<HttpResponse, actix_web::Error> {
    let ws_actor = WsActor {
        detection_addr: data.processing_actor.clone(),
    };
    ws::start(ws_actor, &req, stream)
}
