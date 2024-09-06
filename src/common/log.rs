use std::fmt;

// Enum para los diferentes tipos de mensajes
pub enum LogLevel {
    Error,
    Info,
    Connection,
    OrderRejected,
    OrderAproved,
    LeaderInfo,
    Work,
    GatewayPayment,
    StatusOrder,
    ProcessingOrder,
    AckInfo,
}

// Struct Logger que maneja los mensajes
pub struct Logger;

impl Logger {
    // Método genérico para imprimir mensajes
    pub fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Error => println!("\x1b[0;31m[Error]\x1b[0m {}", message), // Rojo
            LogLevel::Info => println!("\x1b[0;32m[Info]\x1b[0m {}", message),   // Verde
            LogLevel::Connection => println!("\x1b[0;34m[Connection]\x1b[0m {}", message), // Azul
            LogLevel::OrderAproved => println!("\x1b[0;32m[Order Aproved]\x1b[0m {}", message), // Verde
            LogLevel::OrderRejected => println!("\x1b[0;31m[Order Rejected]\x1b[0m {}", message), // Rojo
            LogLevel::LeaderInfo => println!("\x1b[0;35m[Leader Info]\x1b[0m {}", message), // magenta
            LogLevel::Work => println!("\x1b[0;33m[Work]\x1b[0m {}", message),              // Azul
            LogLevel::GatewayPayment => println!("\x1b[0;36m[Payment]\x1b[0m {}", message), // magenta
            LogLevel::StatusOrder => println!("\x1b[0;94m[Status Order]\x1b[0m {}", message), // light blue
            LogLevel::ProcessingOrder => {
                println!("\x1b[0;94m[Processing Order]\x1b[0m {}", message)
            } // light blue
            LogLevel::AckInfo => println!("\x1b[0;94m[Ack Info]\x1b[0m {}", message), // light blue
        }
    }
}

// Implementación de fmt::Display para LogLevel (opcional pero útil para formateo)
impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Error => write!(f, "Error"),
            LogLevel::Info => write!(f, "Info"),
            LogLevel::Connection => write!(f, "Connection"),
            LogLevel::OrderAproved => write!(f, "Order Aproved"),
            LogLevel::OrderRejected => write!(f, "Order Rejected"),
            LogLevel::LeaderInfo => write!(f, "Leader Info"),
            LogLevel::Work => write!(f, "Work"),
            LogLevel::GatewayPayment => write!(f, "Payment"),
            LogLevel::StatusOrder => write!(f, "Status Order"),
            LogLevel::ProcessingOrder => write!(f, "Processing Order"),
            LogLevel::AckInfo => write!(f, "Ack Info"),
        }
    }
}
