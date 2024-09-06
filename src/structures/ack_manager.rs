use crate::common::log::{LogLevel, Logger};
use crate::defines::ack::Ack;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::Mutex;
use std::sync::{Arc, Condvar};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct TimedItem {
    item_type: Ack,
    expiration: Instant,
}

impl TimedItem {
    fn new(item: Ack, duration: Duration) -> Self {
        TimedItem {
            item_type: item,
            expiration: Instant::now() + duration,
        }
    }
}

pub struct AckManager {
    acks: Arc<Mutex<Vec<TimedItem>>>,
    sender: Arc<UdpSocket>,
    condvar: Arc<Condvar>,
}

impl Clone for AckManager {
    fn clone(&self) -> Self {
        AckManager {
            acks: Arc::clone(&self.acks),
            sender: Arc::clone(&self.sender),
            condvar: Arc::clone(&self.condvar),
        }
    }
}

impl AckManager {
    pub fn new(socket: UdpSocket) -> Self {
        let ret = AckManager {
            acks: Arc::new(Mutex::new(Vec::new())),
            sender: Arc::new(socket),
            condvar: Arc::new(Condvar::new()),
        };
        let mut clone = ret.clone();
        thread::spawn(move || clone.start());
        ret
    }

    fn start(&mut self) {
        loop {
            self.wait_acks();
            thread::sleep(Duration::from_secs(1));
            // Logger.log(LogLevel::Error, "Tengo ack!!!!!!!");
            let mut items = self.acks.lock().unwrap();
            let now = Instant::now();
            let mut i = 0;
            while i < items.len() {
                if items[i].expiration <= now {
                    if items[i].item_type.get_num_tries() < 3 {
                        self.sender
                            .send_to(
                                &items[i].item_type.get_msg().as_bytes(),
                                &items[i].item_type.get_addr(),
                            )
                            .unwrap();
                        items[i].expiration = Instant::now() + Duration::from_secs(1);
                        items[i].item_type.increment_tries();
                    } else {
                        let ack = items[i].item_type.clone();
                        self.resolve_resilience(&ack);
                        Logger.log(
                            LogLevel::AckInfo,
                            format!("Se remueve un ACK {:?}", ack).as_str(),
                        );
                        let _ = items.remove(i);
                        drop(items);
                        break;
                    }
                } else {
                    i += 1;
                }
            }
        }
    }

    fn wait_acks(&self) {
        let lock = self.acks.lock().unwrap();
        let _condvar_lock = self
            .condvar
            .wait_while(lock, |lock| lock.is_empty())
            .unwrap();
    }

    pub fn add(&mut self, ack: Ack, duration: Duration) {
        let mut acks = self.acks.lock().unwrap();
        acks.push(TimedItem::new(ack, duration));
        self.condvar.notify_all();
    }

    pub fn remove(&mut self, ack: String, addr: SocketAddr) {
        let mut acks = self.acks.lock().unwrap();
        acks.retain(|x| !x.item_type.is_equal(addr, ack.clone()));
        // Logger.log(LogLevel::Error, "REMUEVO..............");
        self.condvar.notify_all();
    }

    fn resolve_resilience(&self, ack: &Ack) {
        match ack.get_type_msg().as_str() {
            "Result_Interface" => self.interface_resilience(ack),
            _ => {
                // Handle other cases here
            }
        }
    }

    fn interface_resilience(&self, ack: &Ack) {
        let addr = format!("{}", ack.get_addr());
        let id_str = &addr[(addr.len() - 2)..];
        let id_interface: usize = id_str.parse().unwrap();
        let msg_resilience: String = ack.get_msg()[6..].parse().unwrap();

        let addr_next = format!("127.0.0.1:{}", 9000 + id_interface + 1);
        //    Logger.log(LogLevel::Error, format!("Se envia a la siguiente interfaz: {:?}", addr_next).as_str());
        if let Some(addr_n) = addr_next.to_socket_addrs().unwrap().next() {
            let _ = self
                .sender
                .send_to(format!("Resilience:{}", msg_resilience).as_bytes(), addr_n);
        }

        let addr_prev = format!("127.0.0.1:{}", 9000 + id_interface - 1);
        // Logger.log(LogLevel::Error, format!("Se envia a la interfaz anterior: {:?}", addr_prev).as_str());
        if let Some(addr_p) = addr_prev.to_socket_addrs().unwrap().next() {
            let _ = self
                .sender
                .send_to(format!("Resilience:{}", msg_resilience).as_bytes(), addr_p);
        }
    }
}
