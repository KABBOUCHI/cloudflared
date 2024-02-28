use std::{thread::sleep, time::Duration};

use cloudflared::Tunnel;

fn main() {
    let tunnel = Tunnel::builder()
        .url("http://localhost:8080")
        .build()
        .unwrap();

    println!("URL: {}", tunnel.url());

    sleep(Duration::from_secs(15));
}
