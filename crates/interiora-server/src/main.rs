use std::env;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let port: u16 = match env::var("PORT") {
        Ok(value) => value.parse().expect("PORT must be a port number"),
        Err(_) => 3000,
    };
    let bind = format!("0.0.0.0:{port}");
    println!("interiora-server listening on {bind}");
    interiora_server::serve(&bind).await
}
