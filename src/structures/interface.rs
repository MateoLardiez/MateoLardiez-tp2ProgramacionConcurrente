use crate::common::log::{LogLevel, Logger};
use crate::common::protocol::DTO;
use crate::common::read_file::read_file;
use crate::defines::ack::Ack;
use crate::structures::ack_manager::AckManager;
use crate::structures::order::Order;
use actix::prelude::*;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::net::TcpStream;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

#[derive(Message)]
#[rtype(result = "()")]

struct Msg {
    pub content: String,
    addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
struct GatewayMessage {
    id: usize,
    result: bool,
}

#[derive(Message)]
#[rtype(result = "()")]
struct RobotMessage {
    id: usize,
    result: bool,
    addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
struct ResilienceMessage {
    id: usize,
    result: bool,
    addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
struct AckMessage {
    msg: String,
    addr: SocketAddr,
}

pub struct Interface {
    id: usize,
    file_path: String,
    logger: Logger,
    socket: UdpSocket,
    stream: TcpStream,
    orders: HashMap<usize, Order>,
    ack_manager: AckManager,
}

impl Interface {
    pub fn new(id: usize, file: String) -> io::Result<Interface> {
        let socket = UdpSocket::bind(format!("127.0.0.1:{}", 9000 + id)).unwrap();
        let socket_clone = socket.try_clone().unwrap();

        let connection =
            TcpStream::connect("127.0.0.1:8080").expect("Could not connect to gateway");

        Ok(Interface {
            id,
            file_path: file,
            logger: Logger,
            socket,
            stream: connection,
            orders: HashMap::new(),
            ack_manager: AckManager::new(socket_clone),
        })
    }

    fn send_order_to_gateway(&mut self, message: &str) -> Result<(), std::io::Error> {
        // Send the message

        self.stream.write_all(b"Order:")?; // Especifico que es una orden lo que envio DESP DESCOMENTAR CUANDO LO IMPLEMENTE
        self.stream.write_all(message.as_bytes())?;
        self.stream.write_all(b"\n")?; // Append newline delimiter

        Ok(())
    }

    fn send_order_to_robot(&mut self, message: &str) -> Result<(), std::io::Error> {
        // Send the message
        let msg = format!("Order:{}", message);
        let addr_interface = "127.0.0.1:5000";
        if let Some(socket_interface) = addr_interface.to_socket_addrs().unwrap().next() {
            let _ = self.socket.send_to(msg.as_bytes(), socket_interface);
            self.ack_manager.add(
                Ack::new(
                    socket_interface,
                    msg.clone().to_string(),
                    "Order".to_string(),
                ),
                Duration::from_secs(5),
            );
        }
        Ok(())
    }

    fn handle_gateway(&mut self, msg: GatewayMessage) {
        if msg.result {
            self.logger.log(
                LogLevel::OrderAproved,
                format!("Order {} aproved, waiting for payment", msg.id).as_str(),
            );
            if let Some(order) = self.orders.get(&msg.id) {
                let order_cloned = order.clone();
                let order_dto: DTO = self.create_order(&order_cloned);
                let result = self.send_order_to_robot(order_dto.serialize().as_str());
                match result {
                    Ok(_) => self
                        .logger
                        .log(LogLevel::Info, "Message sent to Leader Robots"),
                    Err(e) => self.logger.log(
                        LogLevel::Error,
                        format!("Error sending message to Leader Robots: {}", e).as_str(),
                    ),
                }
            }
        } else {
            self.logger.log(
                LogLevel::OrderRejected,
                format!("Order {} Rejected", msg.id).as_str(),
            );
        }
    }

    fn handle_robot(&mut self, msg: RobotMessage) {
        let result = if msg.result {
            "Completed"
        } else {
            "Incompleted"
        };
        self.logger.log(
            LogLevel::StatusOrder,
            format!("Order {} {}", msg.id, result).as_str(),
        );
        let msg_gateway = format!("{},{}", msg.id, msg.result);
        self.stream.write_all(b"Payment:").unwrap(); // Especifico que es una orden lo que envio
        self.stream.write_all(msg_gateway.as_bytes()).unwrap();
        self.stream.write_all(b"\n").unwrap(); // Append newline delimiter

        Logger.log(
            LogLevel::Info,
            format!("Enviando ACK a {}", msg.addr).as_str(),
        );
        let _ = self
            .socket
            .send_to("Ack:Result_Interface".as_bytes(), msg.addr);
    }

    fn handle_resilience(&mut self, msg: ResilienceMessage) {
        let result = if msg.result {
            "Completed"
        } else {
            "Incompleted"
        };
        self.logger.log(
            LogLevel::StatusOrder,
            format!("Order {} {}", msg.id, result).as_str(),
        );
        let msg_gateway = format!("{},{}", msg.id, msg.result);
        self.stream.write_all(b"Payment:").unwrap(); // Especifico que es una orden lo que envio
        self.stream.write_all(msg_gateway.as_bytes()).unwrap();
        self.stream.write_all(b"\n").unwrap(); // Append newline delimiter

        Logger.log(
            LogLevel::Info,
            format!("Enviando ACK a {}", msg.addr).as_str(),
        );
        let _ = self.socket.send_to("Ack:Resilience".as_bytes(), msg.addr);
    }

    fn process_orders(&mut self) {
        let list = read_file(&self.file_path);
        self.orders = list.clone();
        Logger.log(
            LogLevel::Info,
            format!("Interface {} has {} orders", self.id, list.len()).as_str(),
        );
        for order in list {
            let dto = self.create_order(&order.1);
            let result = self.send_order_to_gateway(dto.serialize().as_str());
            match result {
                Ok(_) => self.logger.log(LogLevel::Info, "Message sent to gateway"),
                Err(e) => self.logger.log(
                    LogLevel::Error,
                    format!("Error sending message to gateway: {}", e).as_str(),
                ),
            }
        }
    }

    fn create_order(&mut self, order: &Order) -> DTO {
        let mut result = DTO {
            id_order: order.id,
            id_interface: self.id,
            ice_creams: order.products.clone(),
            size_order: order.amount,
            cash_card: order.card_cash,
            total_amount: order.total_price,
        };

        if result.size_order == 0.25 {
            result.total_amount = 500;
        } else if result.size_order == 0.5 {
            result.total_amount = 850;
        } else if result.size_order == 1.0 {
            result.total_amount = 1500;
        }
        result
    }

    fn handle_ack(&mut self, msg: AckMessage) {
        match msg.msg.as_str() {
            "Order" => self.ack_manager.remove(msg.msg, msg.addr),
            _ => {}
        }
    }
}

impl Actor for Interface {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        self.logger.log(
            LogLevel::Info,
            format!("[Interface {}] Started", self.id).as_str(),
        );
        // self.stream = Some(connection);
        self.logger.log(LogLevel::Info, "Connected to gateway");

        let actor_addr = _ctx.address();
        let id_interface = self.id;
        let socket = match self.socket.try_clone() {
            Ok(socket) => socket,
            Err(e) => {
                Logger.log(
                    LogLevel::Error,
                    format!("Error cloning socket: {}", e).as_str(),
                );
                return;
            }
        };

        actix::spawn(async move {
            Logger.log(LogLevel::Info, "Esperando mensajes");
            let mut buffer = [0; 1024];
            loop {
                match socket.recv_from(&mut buffer) {
                    Ok((size, addr)) => {
                        let content = String::from_utf8_lossy(&buffer[..size]).to_string();
                        actor_addr.send(Msg { content, addr }).await.unwrap();
                    }
                    Err(e) => {
                        Logger.log(
                            LogLevel::Error,
                            format!(
                                "[Interface {}] Error receiving message: {}",
                                id_interface, e
                            )
                            .as_str(),
                        );
                        break;
                    }
                }
            }
        });

        self.process_orders();
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        Logger.log(
            LogLevel::Info,
            format!("[Interface {}] Stopped", self.id).as_str(),
        );
    }
}

impl Handler<Msg> for Interface {
    type Result = ();

    fn handle(&mut self, msg: Msg, _ctx: &mut Self::Context) {
        if msg.content.starts_with("Payment:") {
            let content: Vec<&str> = msg.content[8..].split(',').collect();
            if content.len() == 2 {
                let result: bool = content[0].parse::<bool>().unwrap();
                let id_order: usize = content[1].parse::<usize>().unwrap();
                _ctx.address().do_send(GatewayMessage {
                    id: id_order,
                    result,
                });
            }
        } else if msg.content.starts_with("Robot:") {
            let content: Vec<&str> = msg.content[6..].split(',').collect();
            if content.len() == 2 {
                let id_order: usize = content[0].parse::<usize>().unwrap();
                let result: bool = content[1] == "true";
                _ctx.address().do_send(RobotMessage {
                    id: id_order,
                    result,
                    addr: msg.addr,
                })
            }
        } else if msg.content.starts_with("Resilience:") {
            let content: Vec<&str> = msg.content[11..].split(',').collect();
            if content.len() == 2 {
                let id_order: usize = content[0].parse::<usize>().unwrap();
                let result: bool = content[1] == "true";
                _ctx.address().do_send(ResilienceMessage {
                    id: id_order,
                    result,
                    addr: msg.addr,
                })
            }
        } else if msg.content.starts_with("Ack:") {
            let content: String = msg.content[4..].parse().unwrap();
            _ctx.address().do_send(AckMessage {
                msg: content,
                addr: msg.addr,
            });
        }
    }
}

impl Handler<GatewayMessage> for Interface {
    type Result = ();

    fn handle(&mut self, msg: GatewayMessage, _ctx: &mut Self::Context) {
        self.handle_gateway(msg);
    }
}

impl Handler<RobotMessage> for Interface {
    type Result = ();

    fn handle(&mut self, msg: RobotMessage, _ctx: &mut Self::Context) {
        self.handle_robot(msg);
    }
}

impl Handler<ResilienceMessage> for Interface {
    type Result = ();

    fn handle(&mut self, msg: ResilienceMessage, _ctx: &mut Self::Context) {
        self.handle_resilience(msg);
    }
}

impl Handler<AckMessage> for Interface {
    type Result = ();

    fn handle(&mut self, msg: AckMessage, _ctx: &mut Self::Context) {
        self.handle_ack(msg);
    }
}
