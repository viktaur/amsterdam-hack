use actix::prelude::*;
use actix_web_actors::ws;

use crate::processing::{ProcessingActor, DetectionInfo, Subscribe, Unsubscribe};

pub struct WsActor {
    pub detection_addr: Addr<ProcessingActor>
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

#[derive(Message)]
#[rtype(result = "()")]
pub struct InfoMsg(pub DetectionInfo);

impl Handler<InfoMsg> for WsActor {
    type Result = ();

    fn handle(&mut self, msg: InfoMsg, ctx: &mut Self::Context) {
        // Serialise to JSON and send to client.
        if let Ok(json) = serde_json::to_string(&msg.0) {
            ctx.text(json);
        }
    }
}
