use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DTO {
    pub id_order: usize,
    pub id_interface: usize,
    pub ice_creams: Vec<String>,
    pub size_order: f32,
    pub cash_card: usize,
    pub total_amount: usize,
}

fn vec_to_json(products: &[String]) -> Vec<serde_json::Value> {
    products
        .iter()
        .map(|product| serde_json::Value::String(product.clone()))
        .collect()
}

impl DTO {
    pub fn serialize(&self) -> String {
        let serialized = json!({
            "id_order": self.id_order,
            "id_interface": self.id_interface,
            "ice_creams": vec_to_json(&self.ice_creams),
            "size_order": self.size_order,
            "cash_card": self.cash_card,
            "total_amount": self.total_amount,
        });
        serialized.to_string()
    }
}
