use crate::defines::status_order::StatusOrder;
use actix::Actor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    pub id: usize,
    pub id_client: usize,
    pub products: Vec<String>,
    pub amount: f32, // -> cantidad de productos por ejemplo 1/2kg
    pub total_price: usize,
    pub status: StatusOrder,
    pub card_cash: usize,
}

impl Default for Order {
    fn default() -> Self {
        Order {
            id: 0,
            id_client: 1,
            products: Vec::new(),
            amount: 0.0,
            total_price: 0,
            status: StatusOrder::Pending,
            card_cash: 0,
        }
    }
}

impl Order {
    pub fn new(
        id: usize,
        id_client: usize,
        products: Vec<String>,
        total_price: usize,
        amount: f32,
        card_cash: usize,
    ) -> Order {
        Order {
            id,
            id_client,
            products,
            amount,
            total_price,
            status: StatusOrder::Pending,
            card_cash,
        }
    }

    pub fn copy(&self) -> Order {
        Order {
            id: self.id,
            id_client: self.id_client,
            products: self.products.clone(),
            amount: self.amount,
            total_price: self.total_price,
            status: self.status,
            card_cash: self.card_cash,
        }
    }

    pub fn change_status(&mut self, status: StatusOrder) {
        self.status = status;
    }

    pub fn get_status(&self) -> StatusOrder {
        match self.status {
            StatusOrder::Canceled => StatusOrder::Canceled,
            StatusOrder::Pending => StatusOrder::Pending,
            StatusOrder::Completed => StatusOrder::Completed,
        }
    }
    pub fn get_total_price(&self) -> f32 {
        self.amount / self.products.len() as f32
    }
}

impl Actor for Order {
    type Context = actix::Context<Self>;
}
