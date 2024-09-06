use crate::structures::order::Order;
use crate::structures::record::RecordList;
use std::collections::HashMap;
use std::fs::{self};

pub fn read_file(path: &str) -> HashMap<usize, Order> {
    let reader = fs::read_to_string(path).expect("Failed to read file");
    let mut result = HashMap::new();
    let order_list: RecordList = serde_json::from_str(&reader).expect("Unable to parse JSON");

    // Itera sobre los pedidos
    for record in order_list.get_records() {
        let order = Order::new(
            record.get_id(),
            record.get_client_id(),
            record.get_ice_creams(),
            0,
            record.get_size_order(),
            record.get_cash_card(),
        );
        // let parse_id_terminal = record.get_id().to_string() + "_" + &id_t.to_string();
        result.insert(record.get_id(), order);
    }
    result
}
