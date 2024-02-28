use std::{thread::sleep, time::Duration};

use cloudflared::Tunnel;

fn main() {
    let tunnel = Tunnel::builder()
        .url("http://localhost:3333")
        .build()
        .unwrap();

    println!("URL: {}", tunnel.url());

    loop {
        sleep(Duration::from_millis(100));
    }
}
