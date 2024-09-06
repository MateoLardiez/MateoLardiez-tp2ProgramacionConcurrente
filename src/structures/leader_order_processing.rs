use crate::common::log::{LogLevel, Logger};
use crate::common::protocol::DTO;
use crate::defines::ack::Ack;
use crate::structures::ack_manager::AckManager;
use crate::structures::ice_cream::IceCreamContainer;
use serde_json::to_string_pretty;
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct TimedItem {
    item_type: SocketAddr,
    n_order: usize,
    expiration: Instant,
}

impl TimedItem {
    fn new(item: SocketAddr, id: usize, duration: Duration) -> Self {
        TimedItem {
            item_type: item,
            n_order: id,
            expiration: Instant::now() + duration,
        }
    }
}

struct LeaderFlag {
    is_leader: Mutex<bool>,
    condvar: Condvar,
}

impl LeaderFlag {
    fn new() -> LeaderFlag {
        LeaderFlag {
            is_leader: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }

    fn wait_for_leader(&self) {
        let leader = self.is_leader.lock().unwrap();
        let _guard = self.condvar.wait_while(leader, |leader| !*leader).unwrap();
    }

    fn set_leader(&self) {
        let mut leader = self.is_leader.lock().unwrap();
        *leader = true;
        self.condvar.notify_all();
    }
}

pub struct LeaderOrderProcessing {
    orders_list: Arc<(Mutex<HashMap<usize, DTO>>, Condvar)>,
    leader_flag: Arc<LeaderFlag>,
    socket_leader: Arc<Mutex<Option<UdpSocket>>>,
    condvar_socket: Arc<Condvar>,
    working_list: Arc<Mutex<HashMap<SocketAddr, DTO>>>,
    stock: Arc<RwLock<HashMap<String, IceCreamContainer>>>,
    working_pending: Arc<(Mutex<Vec<TimedItem>>, Condvar)>,
    pending_send_works: Arc<(Mutex<Vec<SocketAddr>>, Condvar)>,
    ack_manager: Arc<Mutex<Option<AckManager>>>,
}

impl Default for LeaderOrderProcessing {
    fn default() -> Self {
        LeaderOrderProcessing::new()
    }
}

impl Clone for LeaderOrderProcessing {
    fn clone(&self) -> Self {
        LeaderOrderProcessing {
            orders_list: Arc::clone(&self.orders_list),
            leader_flag: Arc::clone(&self.leader_flag),
            socket_leader: Arc::clone(&self.socket_leader),
            condvar_socket: Arc::clone(&self.condvar_socket),
            working_list: Arc::clone(&self.working_list),
            stock: Arc::clone(&self.stock),
            working_pending: Arc::clone(&self.working_pending),
            pending_send_works: Arc::clone(&self.pending_send_works),
            ack_manager: Arc::clone(&self.ack_manager),
        }
    }
}

impl LeaderOrderProcessing {
    pub fn new() -> LeaderOrderProcessing {
        let stock: HashMap<String, IceCreamContainer> = [
            ("Chocolate".to_string(), IceCreamContainer::new(10.0)),
            ("Vainilla".to_string(), IceCreamContainer::new(10.0)),
            ("Crema Americana".to_string(), IceCreamContainer::new(10.0)),
            ("Dulce de Leche".to_string(), IceCreamContainer::new(10.0)),
            ("Frutilla".to_string(), IceCreamContainer::new(10.0)),
        ]
        .iter()
        .cloned()
        .collect();

        let ret = LeaderOrderProcessing {
            orders_list: Arc::new((Mutex::new(HashMap::new()), Condvar::new())),
            leader_flag: Arc::new(LeaderFlag::new()),
            socket_leader: Arc::new(Mutex::new(None)),
            condvar_socket: Arc::new(Condvar::new()),
            working_list: Arc::new(Mutex::new(HashMap::new())),
            stock: Arc::new(RwLock::new(stock)),
            working_pending: Arc::new((Mutex::new(Vec::new()), Condvar::new())),
            pending_send_works: Arc::new((Mutex::new(Vec::new()), Condvar::new())),
            ack_manager: Arc::new(Mutex::new(None)),
        };
        let mut clone = ret.clone();
        thread::spawn(move || clone.receiver());
        let mut clone_checking = ret.clone();
        thread::spawn(move || clone_checking.checking_work());
        let mut clone_send_work = ret.clone();
        thread::spawn(move || clone_send_work.sending_work());
        ret
    }

    fn sending_work(&mut self) {
        loop {
            self.wait_for_sender_work_pendings();
            self.wait_for_orders();
            let mut list = self.pending_send_works.0.lock().unwrap();

            for i in 0..list.len() {
                let addr = list[i];
                self.send_work(addr);
                list.remove(i);
                // drop(list);
                break;
            }
        }
    }

    fn wait_for_sender_work_pendings(&self) {
        let (lock, cvar) = &self.pending_send_works.as_ref();
        let guard = lock.lock().unwrap();
        let _result = cvar.wait_while(guard, |guard| guard.is_empty()).unwrap();
    }

    fn checking_work(&mut self) {
        self.leader_flag.wait_for_leader();
        loop {
            self.wait_for_works_pendings();
            let mut pending = self.working_pending.0.lock().unwrap();
            let now = Instant::now();
            let mut i = 0;
            while i < pending.len() {
                if pending[i].expiration <= now {
                    let mut list = self.working_list.lock().unwrap();
                    let order_incomplete = list.get(&pending[i].item_type);
                    let mut order_list = self.orders_list.0.lock().unwrap();
                    let cvar = &self.orders_list.1;
                    if let Some(order) = order_incomplete {
                        if order.id_order == pending[i].n_order {
                            Logger.log(
                                LogLevel::LeaderInfo,
                                format!("Order {} is incomplete, return to orders list, Robot failure in {}", order.id_order, pending[i].item_type).as_str(),
                            );
                            order_list.insert(order.id_order, order.clone());
                            list.remove(&pending[i].item_type);
                            cvar.notify_all();
                        }
                    };
                    pending.remove(i);
                    break;
                } else {
                    i += 1;
                }
            }
        }
    }

    fn wait_for_works_pendings(&self) {
        let (lock, cvar) = &self.working_pending.as_ref();
        let guard = lock.lock().unwrap();
        let _result = cvar.wait_while(guard, |guard| guard.is_empty()).unwrap();
    }

    pub fn wait_for_socket(&mut self) -> UdpSocket {
        let (lock, cvar) = (&self.socket_leader, &self.condvar_socket);
        let sock_guard = lock.lock().unwrap();
        let _socket = cvar
            .wait_while(sock_guard, |sock_guard| sock_guard.is_none())
            .unwrap();
        _socket.as_ref().unwrap().try_clone().unwrap()
    }

    pub fn receiver(&mut self) {
        self.leader_flag.wait_for_leader();

        let socket = self.wait_for_socket();

        Logger.log(
            LogLevel::LeaderInfo,
            format!("Receiver socket {}", socket.local_addr().unwrap()).as_str(),
        );

        loop {
            let mut buffer = [0; 1024];
            match socket.recv_from(&mut buffer) {
                Ok((size, addr)) => {
                    let content = String::from_utf8_lossy(&buffer[..size]).to_string();
                    if content.starts_with("Order:") {
                        let msg: String = content[6..].parse().unwrap();
                        self.add_order(msg.clone(), addr);
                    } else if content.starts_with("Ack:") {
                        let msg: String = content[4..].parse().unwrap();
                        self.resolve_ack(msg.clone(), addr);
                    }
                }
                Err(e) => {
                    Logger.log(
                        LogLevel::Error,
                        format!("Error receive order {}", e).as_str(),
                    );
                    break;
                }
            }
        }
    }

    fn add_order(&mut self, data: String, addr: SocketAddr) {
        let dto = serde_json::from_str::<DTO>(data.as_str()).unwrap();
        Logger.log(
            LogLevel::LeaderInfo,
            format!(
                "Received from interface {} the order {}",
                dto.id_interface,
                to_string_pretty(&dto).unwrap()
            )
            .as_str(),
        );
        self.sender("Ack:Order", addr);
        let mut list = self.orders_list.0.lock().unwrap();
        list.insert(dto.id_order, dto);
        self.orders_list.1.notify_all();
    }

    fn resolve_ack(&mut self, msg: String, addr: SocketAddr) {
        if let Some(ack_manager) = self.ack_manager.lock().unwrap().as_mut() {
            match msg.as_str() {
                "Work" => ack_manager.remove(msg.clone(), addr),
                "StockResult" => ack_manager.remove(msg.clone(), addr),
                _ => {
                    // Handle other cases here
                }
            }
        }
    }

    fn send_work(&self, addr: SocketAddr) {
        let dto_opt = self.get_next_order();
        if let Some(dto) = dto_opt {
            self.asign_work(addr, dto.clone());
            Logger.log(
                LogLevel::Work,
                format!("Send Order {} to Robot {} ", dto.id_order, addr).as_str(),
            );
            let msg = format!("Work:{}", dto.serialize().as_str());
            self.sender(msg.clone().as_str(), addr);
            if let Some(ack_manager) = self.ack_manager.lock().unwrap().as_mut() {
                ack_manager.add(
                    Ack::new(addr, msg.clone().to_string(), "Work".to_string()),
                    Duration::from_secs(5),
                );
            }
        }
    }

    fn sender(&self, msg: &str, addr: SocketAddr) {
        let socket_guard = self.socket_leader.lock().unwrap();
        if let Some(socket) = &*socket_guard {
            match socket.send_to(msg.as_bytes(), addr) {
                Ok(_) => Logger.log(
                    LogLevel::LeaderInfo,
                    format!("Message send to addr {}", addr).as_str(),
                ),
                Err(e) => Logger.log(
                    LogLevel::Error,
                    format!("Error send message to addr {} with error: {}", addr, e).as_str(),
                ),
            }
        } else {
            Logger.log(LogLevel::Error, "No hay socket para enviar orden");
        }
    }

    pub fn wait_for_orders(&self) {
        let (lock, cvar) = &self.orders_list.as_ref();
        let list = lock.lock().unwrap();
        let _list_empty = cvar.wait_while(list, |orders| orders.is_empty()).unwrap();
    }

    fn add_addr_sender_work(&mut self, addr: SocketAddr) {
        let (lock, cvar) = &self.pending_send_works.as_ref();
        let mut list_pending_send = lock.lock().unwrap();
        list_pending_send.push(addr);
        cvar.notify_all();
    }

    pub fn send_work_to_robot(&mut self, addr: SocketAddr) {
        // self.wait_for_orders();
        let (list_orders, _) = &self.orders_list.as_ref();
        if list_orders.lock().unwrap().is_empty() {
            self.add_addr_sender_work(addr);
        } else {
            self.send_work(addr);
        }
    }

    pub fn asign_work(&self, addr: SocketAddr, content: DTO) {
        let mut list = self.working_list.lock().unwrap();
        let id_order = content.id_order;
        list.insert(addr, content);
        let addr_work = addr;
        self.working_pending.0.lock().unwrap().push(TimedItem::new(
            addr_work,
            id_order,
            Duration::new(5, 0),
        ));
    }

    pub fn get_next_order(&self) -> Option<DTO> {
        let mut list = self.orders_list.0.lock().unwrap();
        if list.is_empty() {
            None
        } else {
            let k = *list.keys().next().unwrap();
            let dto = list.get(&k).cloned();
            list.remove(&k);
            dto
        }
    }

    pub fn finish_order(&mut self, addr: SocketAddr) {
        let mut list = self.working_list.lock().unwrap();
        Logger.log(
            LogLevel::LeaderInfo,
            format!("Robot in direction {} finish work", addr).as_str(),
        );
        let mut pending = self.working_pending.0.lock().unwrap();
        if let Some(order) = list.get(&addr) {
            let id_order = order.id_order;
            pending.retain(|item| item.item_type != addr && item.n_order != id_order);
            list.remove(&addr);
        }
    }

    pub fn update_leader(&mut self) {
        self.leader_flag.set_leader();
        let new_socket = UdpSocket::bind("127.0.0.1:5000").unwrap();
        let mut ack_manager = self.ack_manager.lock().unwrap();
        *ack_manager = Some(AckManager::new(new_socket.try_clone().unwrap()));
        let mut socket_guard = self.socket_leader.lock().unwrap();
        *socket_guard = Some(new_socket);
        self.condvar_socket.notify_all();
    }

    pub fn use_stock(&mut self, ice_creams: &[String], amount: f64, addr: SocketAddr) {
        let mut result = false;
        let mut stock = self.stock.write().unwrap();
        for ice_cream in ice_creams.iter() {
            if let Some(ice) = stock.get_mut(ice_cream) {
                result = ice.use_stock(amount);
                if !result {
                    Logger.log(
                        LogLevel::Error,
                        format!("Not enough stock for flavour {}", ice_cream).as_str(),
                    );
                    break;
                }
            }
        }
        self.sender(format!("StockResult:{}", result).as_str(), addr);
        self.ack_manager.lock().unwrap().as_mut().unwrap().add(
            Ack::new(addr, "StockResult".to_string(), "StockResult".to_string()),
            Duration::from_secs(5),
        );
    }
}
