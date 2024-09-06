use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Record {
    id: usize,
    client_id: usize,
    ice_creams: Vec<String>,
    size_order: f32,
    cash_card: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecordList {
    orders: Vec<Record>,
}

impl Record {
    pub fn get_id(&self) -> usize {
        self.id
    }
    pub fn get_client_id(&self) -> usize {
        self.client_id
    }
    pub fn get_ice_creams(&self) -> Vec<String> {
        self.ice_creams.clone()
    }
    pub fn get_size_order(&self) -> f32 {
        self.size_order
    }
    pub fn get_cash_card(&self) -> usize {
        self.cash_card
    }
}

impl RecordList {
    pub fn get_records(&self) -> Vec<Record> {
        self.orders.clone()
    }
}
