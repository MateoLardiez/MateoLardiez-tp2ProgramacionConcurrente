use actix::prelude::*;
use tokio::net::TcpListener;
use tp2::structures::gateway::GatewayPayment;
use tp2::structures::handle_connection::HandleConnection;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // Inicia el actor del servidor
    let server_addr = GatewayPayment::new().start();

    // Escucha en la dirección local y el puerto 12345
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    while let Ok((stream, addr)) = listener.accept().await {
        // Envía un mensaje HandleConnection al actor HelloServer
        server_addr.do_send(HandleConnection::new(stream, addr));
    }

    Ok(())
}
