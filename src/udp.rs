use std::sync::Arc;

use actix_web::rt::net::UdpSocket;
use actix::prelude::*;
use log::error;

use crate::utils::parse_samples;
use crate::processing::{ProcessingActor, AddSamples};

const PORT: u16 = 5454;
/// Size of the buffer for UDP packets.
pub const BUFFER_SIZE: usize = 65536;

pub struct UdpListenerActor {
    socket: Arc<UdpSocket>,
    subscriber: Option<Addr<ProcessingActor>>,
}

impl UdpListenerActor {
    pub async fn new() -> Self {
        let socket = UdpSocket::bind(("127.0.0.1", PORT)).await.expect("UDP socket binding should have been successful");

        Self {
            socket: Arc::new(socket),
            subscriber: None,
        }
    }
}

impl Actor for UdpListenerActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let processing_addr = self.subscriber.clone();
        let socket = self.socket.clone();

        ctx.spawn(async move {
            let mut buf = [0; BUFFER_SIZE];
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((size, _)) => {
                        let samples = parse_samples(&buf[..size]);
                        if let Some(ref addr) = processing_addr {
                            addr.do_send(AddSamples { samples });
                        }
                    }
                    Err(e) => error!("UDP read error: {}", e),
                }
            }
        }.into_actor(self));
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Subscribe(pub Addr<ProcessingActor>);

impl Handler<Subscribe> for UdpListenerActor {
    type Result = ();

    fn handle(&mut self, msg: Subscribe, _: &mut Self::Context) {
        self.subscriber = Some(msg.0)
    }
}
