use crate::common::log::{LogLevel, Logger};
use crate::common::protocol::DTO;
use crate::structures::handle_connection::HandleConnection;
use actix::prelude::*;
use rand::Rng;
use std::net::{ToSocketAddrs, UdpSocket};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Message)]
#[rtype(result = "()")]

struct Msg {
    pub content: String,
}

pub struct GatewayPayment {
    logger: Logger,
}

impl Default for GatewayPayment {
    fn default() -> Self {
        Self::new()
    }
}

impl GatewayPayment {
    pub fn new() -> GatewayPayment {
        GatewayPayment { logger: Logger }
    }

    pub fn finish_order(&mut self, content: Vec<&str>) {
        let id_order: usize = content[0].parse::<usize>().unwrap();
        let result: bool = content[1].trim_matches('\n') == "true";
        if result {
            self.logger.log(
                LogLevel::GatewayPayment,
                format!("Order {} completed, payment done.", id_order).as_str(),
            );
        } else {
            self.logger.log(
                LogLevel::GatewayPayment,
                format!("Order {} rejected, payment not done.", id_order).as_str(),
            );
        }
    }

    pub fn send_message_to_interface(&mut self, message: &str, id: usize) {
        let socket = UdpSocket::bind(format!("127.0.0.1:{}", 8081)).unwrap(); // Puerto para enviar a la interfaz
        let addr_interface = format!("127.0.0.1:{}", 9000 + id);
        if let Some(socket_interface) = addr_interface.to_socket_addrs().unwrap().next() {
            let _ = socket.send_to(message.as_bytes(), socket_interface);
        }
    }

    pub fn process_order(&mut self, order: String) {
        match serde_json::from_str::<DTO>(order.as_str()) {
            Ok(dto) => {
                Logger.log(LogLevel::Info, format!("New order: {:?}", dto).as_str());
                let mut rng = rand::thread_rng();
                let num = rng.gen_range(1, 10); // 10% de probabilidad de que el pago sea rechazado

                if dto.cash_card < dto.total_amount || num == 1 {
                    Logger.log(
                        LogLevel::Error,
                        format!("Order {} is rejected", dto.id_order).as_str(),
                    );
                    self.send_message_to_interface(
                        format!("Payment:{},{}", false, dto.id_order).as_str(),
                        dto.id_interface,
                    );
                } else {
                    Logger.log(
                        LogLevel::Info,
                        format!("Order {} is aproved, payment pending", dto.id_order).as_str(),
                    );
                    self.send_message_to_interface(
                        format!("Payment:{},{}", true, dto.id_order).as_str(),
                        dto.id_interface,
                    );
                }
            }
            Err(e) => {
                Logger.log(
                    LogLevel::Error,
                    format!("[{:?}] Error al parsear DTO: {:?}", order, e).as_str(),
                );
            }
        }
    }
}

impl Actor for GatewayPayment {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        self.logger.log(LogLevel::Info, "Gateway Payment started");
        self.logger
            .log(LogLevel::Connection, "Gateway listening in 127.0.0.1:8080");
    }
}

impl Handler<HandleConnection> for GatewayPayment {
    type Result = ();

    fn handle(&mut self, msg: HandleConnection, _ctx: &mut Self::Context) -> Self::Result {
        let HandleConnection { mut stream, addr } = msg;
        self.logger.log(
            LogLevel::Connection,
            format!("Client connected: {:?}", addr).as_str(),
        );

        let actor_addr = _ctx.address();

        actix::spawn(async move {
            let mut reader = BufReader::new(&mut stream);
            let mut buffer = Vec::new();

            loop {
                match reader.read_until(b'\n', &mut buffer).await {
                    Ok(0) => {
                        break;
                    }
                    Ok(_) => {
                        // Convertir los bytes a una cadena y luego a un DTO
                        let data = String::from_utf8_lossy(&buffer);
                        let data_str = data.to_string();
                        actor_addr.do_send(Msg { content: data_str });
                        // Clear the buffer for the next read
                        buffer.clear();
                    }
                    Err(e) => {
                        Logger.log(
                            LogLevel::Error,
                            format!("[{:?}] Error al leer bytes: {:?}", addr, e).as_str(),
                        );
                        break;
                    }
                }
            }
        });
    }
}

impl Handler<Msg> for GatewayPayment {
    type Result = ();

    fn handle(&mut self, msg: Msg, _ctx: &mut Self::Context) {
        if msg.content.starts_with("Order:") {
            let content: String = msg.content[6..].parse().unwrap();
            self.process_order(content);
        } else if msg.content.starts_with("Payment:") {
            let content: Vec<&str> = msg.content[8..].split(',').collect();
            self.finish_order(content); // La interfaz envia un booleano. i es true, se efectua el pago, sino no
        }
    }
}
