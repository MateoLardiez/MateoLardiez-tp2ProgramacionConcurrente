use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum StatusOrder {
    Pending,
    Completed,
    Canceled,
}
