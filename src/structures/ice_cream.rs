use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct IceCreamContainer {
    stock: Arc<Mutex<f64>>,
}

impl Clone for IceCreamContainer {
    fn clone(&self) -> Self {
        IceCreamContainer {
            stock: Arc::clone(&self.stock),
        }
    }
}

impl IceCreamContainer {
    pub fn new(stock: f64) -> IceCreamContainer {
        IceCreamContainer {
            stock: Arc::new(Mutex::new(stock)),
        }
    }

    pub fn use_stock(&mut self, amount: f64) -> bool {
        let mut stock = self.stock.lock().unwrap();
        let mut result = false;
        if *stock >= amount {
            *stock -= amount;
            result = true;
        }
        result
    }
}
