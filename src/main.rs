use std::{collections::HashSet, sync::{Arc, Mutex}};
use actix::{Actor, Addr};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use detection::{DetectionActor, DetectionScore, SignalWindow};
use websockets::WsActor;

mod detection;
mod websockets;

struct AppState {
    signal_window: Arc<Mutex<SignalWindow>>,
    latest_score: Arc<Mutex<Option<DetectionScore>>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Start DetectionActor (storing its Addr)
    let detection_actor = DetectionActor::new().start();

    HttpServer::new(move || {
        App::new()
            // Share DetectionActor's address via app data, accessible through web::Data
            .app_data(web::Data::new(detection_actor.clone()))
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
    data: web::Data<Addr<DetectionActor>>
) -> Result<HttpResponse, actix_web::Error> {
    let ws_actor = WsActor {
        detection_addr: data.get_ref().clone(),
    };
    ws::start(ws_actor, &req, stream)
}
