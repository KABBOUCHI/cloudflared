# cloudflared

## Setup

```bash
cargo add cloudflared
```

## Usage

```rust
fn main() {
    let tunnel = cloudflared::Tunnel::builder()
        .url("http://localhost:8080")
        .build()
        .unwrap();

    println!("URL: {}", tunnel.url());

    std::thread::sleep(std::time::Duration::from_secs(42));
}
```