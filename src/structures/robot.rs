use crate::common::log::{LogLevel, Logger};
use crate::common::protocol::DTO;
use crate::defines::ack::Ack;
use crate::structures::ack_manager::AckManager;
use crate::structures::leader_order_processing::LeaderOrderProcessing;
use actix::prelude::*;
use rand::Rng;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;
use std::{io, thread};

#[derive(Message)]
#[rtype(result = "()")]
pub struct Msg {
    pub content: String,
    pub sender: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Election {
    id: usize,
    sender: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Leader {
    leader_id: usize,
    sender: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Announce {
    id: usize,
    addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Hello {
    id: usize,
    addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
struct WorkMessage {
    dto: DTO,
    addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
struct AvailabilityMessage {
    addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
struct UseStock {
    ice_cream: Vec<String>,
    mount: f64,
    addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
struct StockResult {
    result: bool,
    addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
struct AckRobot {
    addr: SocketAddr,
    type_ack: String,
}

pub struct Robot {
    id: usize,
    socket: UdpSocket,
    leader_id: Option<SocketAddr>,
    peers: Vec<SocketAddr>,
    im_leader: bool,
    leader_order_processing: LeaderOrderProcessing,
    current_order: Option<DTO>,
    current_order_result: Option<bool>,
    ack_manager: AckManager,
}

impl Clone for Robot {
    fn clone(&self) -> Robot {
        Robot {
            id: self.id,
            socket: self.socket.try_clone().unwrap(),
            leader_id: self.leader_id,
            peers: self.peers.clone(),
            im_leader: self.im_leader,
            leader_order_processing: self.leader_order_processing.clone(),
            current_order: self.current_order.clone(),
            current_order_result: self.current_order_result.clone(),
            ack_manager: self.ack_manager.clone(),
        }
    }
}

impl Robot {
    pub fn new(id: usize) -> io::Result<Robot> {
        let socket = UdpSocket::bind(format!("127.0.0.1:{}", 6000 + id))?;
        let socket_clone = socket.try_clone().unwrap();

        let robot = Robot {
            id,
            socket,
            leader_id: None,
            peers: Vec::new(),
            im_leader: false,
            leader_order_processing: LeaderOrderProcessing::new(), // Pass the 'id' argument to the 'new' function
            current_order: None,
            current_order_result: None,
            ack_manager: AckManager::new(socket_clone),
        };

        Ok(robot)
    }

    fn send_message(&self, message: String, addr: SocketAddr) -> io::Result<usize> {
        self.socket.send_to(message.as_bytes(), addr)
    }

    fn handle_election(&mut self, msg: Election) {
        Logger.log(
            LogLevel::Info,
            format!(
                "[Robot {}] received election message from Robot {}",
                self.id, msg.id
            )
            .as_str(),
        );
        if self.id > msg.id {
            if let Some(next_peer) = self.peers.iter().find(|&&peer| peer != msg.sender) {
                let _ = self.send_message(format!("Election:{}", self.id), *next_peer);
            } else {
                self.leader_id = Some(self.socket.local_addr().unwrap());
                for peer in self.peers.iter() {
                    let _ = self.send_message(format!("Leader:{}", self.id), *peer);
                }
            }
        }
    }

    fn handle_leader(&mut self, msg: Leader) {
        Logger.log(
            LogLevel::Info,
            format!(
                "[Robot {}] received leader message from Robot {}",
                self.id, msg.leader_id
            )
            .as_str(),
        );
        self.leader_id = Some(msg.sender);
        if !self.peers.contains(&msg.sender) {
            self.peers.push(msg.sender);
        }
        let _ = self.send_message("Ack:Leader".to_string(), msg.sender);
    }

    fn announce(&mut self) {
        Logger.log(
            LogLevel::Info,
            format!(
                "[Robot {}] Searching connection with working ring...",
                self.id
            )
            .as_str(),
        );
        for i in 1..10 {
            if i == self.id {
                continue;
            }
            let addr = format!("127.0.0.1:{}", 6000 + i);
            if let Some(socket_addr) = addr.to_socket_addrs().unwrap().next() {
                let msg = format!("Announce:{}", self.id);
                let _ = self.send_message(msg.clone(), socket_addr);
                self.ack_manager.add(
                    Ack::new(socket_addr, msg.clone().to_string(), "Announce".to_string()),
                    Duration::from_secs(5),
                );
            }
        }
    }

    fn handle_announce(&mut self, msg: Announce) {
        Logger.log(
            LogLevel::Info,
            format!(
                "[Robot {}] Received announce message from Robot {}",
                self.id, msg.id
            )
            .as_str(),
        );
        let msg_ack = "Ack:Announce";
        let _ = self.send_message(msg_ack.to_string(), msg.addr);
        if !self.peers.contains(&msg.addr) {
            self.peers.push(msg.addr);
        }

        if let Some(leader_id) = self.leader_id {
            if leader_id == self.socket.local_addr().unwrap() {
                Logger.log(
                    LogLevel::LeaderInfo,
                    format!("Informing Robot {} that i`m the leader", msg.id).as_str(),
                );
                let msg_leader = format!("Leader:{}", self.id);
                let _ = self.send_message(msg_leader.clone(), msg.addr);
                self.ack_manager.add(
                    Ack::new(
                        msg.addr,
                        msg_leader.to_string().clone(),
                        "Leader".to_string(),
                    ),
                    Duration::from_secs(5),
                );
                self.leader_order_processing.send_work_to_robot(msg.addr);
            } else {
                let msg_hello = format!("Hello:{}", self.id);
                let _ = self.send_message(msg_hello.clone(), msg.addr);
                self.ack_manager.add(
                    Ack::new(msg.addr, msg_hello.to_string().clone(), "Hello".to_string()),
                    Duration::from_secs(5),
                );
            }
        } else {
            self.leader_id = Some(self.socket.local_addr().unwrap());
            self.im_leader = true;
            self.leader_order_processing.update_leader();
            Logger.log(
                LogLevel::LeaderInfo,
                format!("Informing Robot {} that i`m the leader", msg.id).as_str(),
            );
            let msg_leader = format!("Leader:{}", self.id);
            let _ = self.send_message(msg_leader.clone(), msg.addr);
            self.ack_manager.add(
                Ack::new(
                    msg.addr,
                    msg_leader.to_string().clone(),
                    "Leader".to_string(),
                ),
                Duration::from_secs(5),
            );
            self.leader_order_processing.send_work_to_robot(msg.addr);
            self.leader_order_processing
                .send_work_to_robot(self.socket.local_addr().unwrap());
        }
    }

    fn handle_hello(&mut self, msg: Hello) {
        Logger.log(
            LogLevel::Info,
            format!("[Robot {}] Received welcome from Robot {}", self.id, msg.id).as_str(),
        );
        if !self.peers.contains(&msg.addr) {
            self.peers.push(msg.addr);
        }
        let _ = self.send_message("Ack:Hello".to_string(), msg.addr);
    }

    fn prepare_order(&mut self, dto: &DTO) {
        let mount = dto.size_order as f64 / dto.ice_creams.len() as f64;

        Logger.log(
            LogLevel::ProcessingOrder,
            format!(
                "[Robot {}] Checking stock for ice cream {:?} with amount {}",
                self.id, dto.ice_creams, mount
            )
            .as_str(),
        );

        if let Some(addr_leader) = self.leader_id {
            let msg_use = format!("UseStock:{:?};{}", dto.ice_creams.join(","), mount);
            let _ = self.send_message(msg_use.clone(), addr_leader);
            self.ack_manager.add(
                Ack::new(
                    addr_leader,
                    msg_use.clone().to_string(),
                    "UseStock".to_string(),
                ),
                Duration::from_secs(5),
            );
        }

        Logger.log(LogLevel::ProcessingOrder, "Waiting for result checking...");
    }

    fn handle_work(&mut self, msg: WorkMessage) {
        Logger.log(
            LogLevel::Info,
            format!(
                "[Robot {}] Receiver working in order {}",
                self.id, msg.dto.id_order
            )
            .as_str(),
        );
        let _ = self.send_message("Ack:Work".to_string(), msg.addr);
        self.prepare_order(&msg.dto);
        self.current_order = Some(msg.dto.clone());
    }

    fn handle_availability(&mut self, msg: AvailabilityMessage) {
        Logger.log(
            LogLevel::Info,
            format!(
                "[Robot {}] received availability message from Robot {}",
                self.id, msg.addr
            )
            .as_str(),
        );
        self.leader_order_processing.finish_order(msg.addr);
        self.leader_order_processing.send_work_to_robot(msg.addr);
        let _ = self.send_message("Ack:Availability".to_string(), msg.addr);
    }

    fn handle_use_stock(&mut self, msg: UseStock) {
        Logger.log(
            LogLevel::LeaderInfo,
            format!(
                "Received use stock {:?} and mount {}",
                msg.ice_cream, msg.mount
            )
            .as_str(),
        );
        self.leader_order_processing
            .use_stock(&msg.ice_cream, msg.mount, msg.addr);
        let _ = self.send_message("Ack:UseStock".to_string(), msg.addr);
    }

    fn handle_stock_result(&mut self, msg: StockResult) {
        if let Some(order) = &self.current_order {
            let _ = self.send_message(format!("Ack:StockResult").to_string(), msg.addr);
            if msg.result {
                thread::sleep(std::time::Duration::from_secs(
                    rand::thread_rng().gen_range(2, 4),
                ));
                Logger.log(
                    LogLevel::Info,
                    format!(
                        "[Robot {}] Work complete for order {}",
                        self.id, order.id_order
                    )
                    .as_str(),
                );
            } else {
                thread::sleep(std::time::Duration::from_secs(
                    rand::thread_rng().gen_range(2, 3),
                ));
            }
            let order_result = msg.result;
            self.current_order_result = Some(order_result);
            self.send_result_interface(order.id_interface, order.id_order, msg.result);
        }
    }

    fn send_availability(&mut self) {
        if self.im_leader {
            self.handle_availability(AvailabilityMessage {
                addr: self.socket.local_addr().unwrap(),
            });
        } else if let Some(addr) = self.leader_id {
            let _ = self.send_message("Availability".to_string(), addr);
            self.ack_manager.add(
                Ack::new(addr, "Availability".to_string(), "Availability".to_string()),
                Duration::from_secs(5),
            );
        }
        self.current_order = None;
        self.current_order_result = None;
    }

    fn send_result_interface(&mut self, id_interface: usize, id_order: usize, result: bool) {
        let addr = format!("127.0.0.1:{}", 9000 + id_interface);
        let message = format!("Robot:{},{}", id_order, result);
        if let Some(addr_interface) = addr.to_socket_addrs().unwrap().next() {
            if self.send_message(message.clone(), addr_interface).is_ok() {
                self.ack_manager.add(
                    Ack::new(
                        addr_interface,
                        message.clone(),
                        "Result_Interface".to_string(),
                    ),
                    Duration::from_secs(5),
                );
            }
        }
    }

    fn handle_ack(&mut self, msg: AckRobot) {
        match msg.type_ack.as_str() {
            "Announce" => self.ack_manager.remove(msg.type_ack, msg.addr),
            "Availability" => self.ack_manager.remove(msg.type_ack, msg.addr),
            "Hello" => self.ack_manager.remove(msg.type_ack, msg.addr),
            "Leader" => self.ack_manager.remove(msg.type_ack, msg.addr),
            "Result_Interface" => {
                self.send_availability();
                self.ack_manager.remove(msg.type_ack, msg.addr);
            }
            "Resilience" => {
                Logger.log(LogLevel::Info, "Receive Ack Resilience Interface ");
                self.send_availability();
            }
            "UseStock" => self.ack_manager.remove(msg.type_ack, msg.addr),
            "StockResult" => self.ack_manager.remove(msg.type_ack, msg.addr),
            _ => Logger.log(LogLevel::Error, "Error Ack"),
        }
    }
}

impl Actor for Robot {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        Logger.log(
            LogLevel::Info,
            format!("Robot {} started", self.id).as_str(),
        );
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
        let id_robot = self.id;
        let actor_addr = _ctx.address();

        self.announce();

        actix::spawn(async move {
            let mut buffer = [0; 1024];
            loop {
                Logger.log(
                    LogLevel::Info,
                    format!("[Robot {}] Waiting for message", id_robot).as_str(),
                );
                socket.set_read_timeout(None).unwrap();

                match socket.recv_from(&mut buffer) {
                    Ok((size, sender)) => {
                        let content = String::from_utf8_lossy(&buffer[..size]).to_string();
                        actor_addr.send(Msg { content, sender }).await.unwrap();
                    }
                    Err(e) => {
                        Logger.log(
                            LogLevel::Error,
                            format!("[Robot {}] Error receiving message: {}", id_robot, e).as_str(),
                        );
                    }
                }
            }
        });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        Logger.log(
            LogLevel::Info,
            format!("[Robot {}] stopped", self.id).as_str(),
        );
    }
}

impl Handler<Msg> for Robot {
    type Result = ();

    fn handle(&mut self, msg: Msg, _ctx: &mut Self::Context) {
        if msg.content.starts_with("Announce:") {
            let id: usize = msg.content[9..].parse().unwrap();
            _ctx.address().do_send(Announce {
                id,
                addr: msg.sender,
            });
        } else if msg.content.starts_with("Election:") {
            let id: usize = msg.content[9..].parse().unwrap();
            _ctx.address().do_send(Election {
                id,
                sender: msg.sender,
            });
        } else if msg.content.starts_with("Leader:") {
            let id: usize = msg.content[7..].parse().unwrap();
            _ctx.address().do_send(Leader {
                leader_id: id,
                sender: msg.sender,
            });
        } else if msg.content.starts_with("Hello:") {
            let id: usize = msg.content[6..].parse().unwrap();
            _ctx.address().do_send(Hello {
                id,
                addr: msg.sender,
            });
        } else if msg.content.starts_with("Work:") {
            let content: String = msg.content[5..].parse().unwrap();
            let dto = serde_json::from_str::<DTO>(content.as_str()).unwrap();
            _ctx.address().do_send(WorkMessage {
                dto,
                addr: msg.sender,
            });
        } else if msg.content.starts_with("Availability") {
            _ctx.address()
                .do_send(AvailabilityMessage { addr: msg.sender });
        } else if msg.content.starts_with("UseStock:") {
            let content: Vec<&str> = msg.content[9..].split(';').collect();
            if content.len() == 2 {
                let ice_creams: Vec<&str> = content[0].split(',').collect(); // sabores
                let mount = content[1].parse::<f64>().unwrap(); //cantidad
                _ctx.address().do_send(UseStock {
                    ice_cream: ice_creams
                        .iter()
                        .map(|&s| s.trim_matches('"').to_string())
                        .collect(),
                    mount,
                    addr: msg.sender,
                });
            }
        } else if msg.content.starts_with("StockResult:") {
            let content: String = msg.content[12..].parse().unwrap();
            let result: bool = content == "true";
            _ctx.address().do_send(StockResult {
                result,
                addr: msg.sender,
            });
        } else if msg.content.starts_with("Ack:") {
            let type_ack: String = msg.content[4..].parse().unwrap();
            _ctx.address().do_send(AckRobot {
                addr: msg.sender,
                type_ack,
            });
        }
    }
}

impl Handler<Election> for Robot {
    type Result = ();

    fn handle(&mut self, msg: Election, _ctx: &mut Self::Context) {
        self.handle_election(msg)
    }
}

impl Handler<Leader> for Robot {
    type Result = ();

    fn handle(&mut self, msg: Leader, _ctx: &mut Self::Context) {
        self.handle_leader(msg)
    }
}

impl Handler<Announce> for Robot {
    type Result = ();

    fn handle(&mut self, msg: Announce, _ctx: &mut Self::Context) {
        self.handle_announce(msg)
    }
}

impl Handler<Hello> for Robot {
    type Result = ();

    fn handle(&mut self, msg: Hello, _ctx: &mut Self::Context) {
        self.handle_hello(msg)
    }
}

impl Handler<WorkMessage> for Robot {
    type Result = ();

    fn handle(&mut self, msg: WorkMessage, _ctx: &mut Self::Context) {
        self.handle_work(msg);
    }
}

impl Handler<AvailabilityMessage> for Robot {
    type Result = ();

    fn handle(&mut self, msg: AvailabilityMessage, _ctx: &mut Self::Context) {
        self.handle_availability(msg)
    }
}

impl Handler<UseStock> for Robot {
    type Result = ();

    fn handle(&mut self, msg: UseStock, _ctx: &mut Self::Context) -> Self::Result {
        self.handle_use_stock(msg);
    }
}

impl Handler<StockResult> for Robot {
    type Result = ();

    fn handle(&mut self, msg: StockResult, _ctx: &mut Self::Context) -> Self::Result {
        self.handle_stock_result(msg);
    }
}

impl Handler<AckRobot> for Robot {
    type Result = ();

    fn handle(&mut self, msg: AckRobot, _ctx: &mut Self::Context) -> Self::Result {
        self.handle_ack(msg);
    }
}
