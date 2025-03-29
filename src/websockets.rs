use actix::{Actor, Addr, StreamHandler, Handler};
use actix_web_actors::ws;

use crate::detection::{DetectionActor, DetectionScore, Subscribe, Unsubscribe};

pub struct WsActor {
    pub detection_addr: Addr<DetectionActor>
}

impl Actor for WsActor {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Subscribe to DetectionActor. ctx.address is the address of the ws actor.
        self.detection_addr.do_send(Subscribe(ctx.address()));
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        // Unsubscribe from DetectionActor
        self.detection_addr.do_send(Unsubscribe(ctx.address()));
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsActor {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Close(_)) => ctx.close(None),
            _ => (),
        }
    }
}

impl Handler<DetectionScore> for WsActor {
    type Result = ();

    fn handle(&mut self, msg: DetectionScore, ctx: &mut Self::Context) {
        // Serialise to JSON and send to client.
        if let Ok(json) = serde_json::to_string(&msg) {
            ctx.text(json);
        }
    }
}
