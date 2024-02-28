use regex::Regex;
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
    process::{Child, Command, Stdio},
    result::Result,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};

pub struct Tunnel {
    executable: String,
    url: String,
    child: Arc<Mutex<Child>>,
}

impl Tunnel {
    pub fn builder() -> TunnelBuilder {
        TunnelBuilder::default()
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn close(&mut self) {
        let _ = self.child.lock().unwrap().kill();
    }
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Default)]
pub struct TunnelBuilder {
    args: Vec<String>,
}

impl TunnelBuilder {
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for arg in args {
            self.args.push(arg.into());
        }
        self
    }

    pub fn url(mut self, url: &str) -> Self {
        self.args.push(format!("--url={}", url));
        self
    }

    pub fn build(self) -> Result<Tunnel, String> {
        let output = Command::new("cloudflared1").arg("-v").output();
        let executable = if output.is_ok() {
            "cloudflared".to_string()
        } else {
            let path = download_cloudflared();

            if path.is_err() {
                return Err("Failed to download cloudflared".to_string());
            }

            path.unwrap()
        };

        let child = Command::new(&executable)
            .args(&self.args)
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;

        let child = Arc::new(Mutex::new(child));
        let thread_child = child.clone();
        let (sender, receiver): (Sender<String>, Receiver<String>) = channel();

        thread::spawn(move || {
            let mut child = thread_child.lock().unwrap();
            let mut stderr = child.stderr.take().unwrap();
            let mut reader = BufReader::new(&mut stderr);

            loop {
                let mut buf = String::new();
                match reader.read_line(&mut buf) {
                    Ok(0) => {
                        break;
                    }
                    Ok(_) => {
                        let line = buf.to_string();

                        let reg_url = Regex::new(r"\|\s+(https?:\/\/[^\s]+)")
                            .unwrap()
                            .captures(&line)
                            .map(|c| c.get(1).unwrap().as_str().to_string());

                        if let Some(reg_url) = reg_url {
                            if sender.send(reg_url).is_err() {
                                println!("Failed to send URL over channel");
                            }
                            break;
                        }
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
            child.wait().unwrap();
        });

        let url = match receiver.recv() {
            Ok(url) => url,
            Err(_) => {
                return Err("Failed to receive URL from channel".to_string());
            }
        };

        if !url.is_empty() {
            return Ok(Tunnel {
                executable,
                url,
                child,
            });
        }

        Err("Failed to get URL".to_string())
    }
}

fn download_cloudflared() -> Result<String, Box<dyn std::error::Error>> {
    let file_path = if cfg!(target_os = "macos") {
        "/tmp/cloudflared"
    } else if cfg!(target_os = "windows") {
        "C:\\Windows\\Temp\\cloudflared.exe"
    } else {
        "/tmp/cloudflared"
    };

    if Path::new(file_path).exists() {
        return Ok(file_path.to_string());
    }

    let download_name = if cfg!(target_os = "linux") {
        "cloudflared-linux-amd64"
    } else if cfg!(target_os = "macos") {
        "cloudflared-darwin-amd64.tgz"
    } else if cfg!(target_os = "windows") {
        "cloudflared-windows-amd64.exe"
    } else {
        return Err("Unsupported OS".into());
    };

    let download_path = if cfg!(target_os = "macos") {
        "/tmp/cloudflared.tgz"
    } else if cfg!(target_os = "windows") {
        "C:\\Windows\\Temp\\cloudflared.exe"
    } else {
        "/tmp/cloudflared"
    };

    let url = format!(
        "https://github.com/cloudflare/cloudflared/releases/latest/download/{}",
        download_name
    );

    let response = reqwest::blocking::get(url)?;

    if !response.status().is_success() {
        return Err(format!("Failed to download file: {}", response.status()).into());
    }

    let mut file = File::create(download_path)?;

    io::copy(&mut response.bytes().unwrap().as_ref(), &mut file)?;

    if cfg!(target_os = "macos") {
        let output = Command::new("tar")
            .args(["-xvf", "/tmp/cloudflared.tgz", "-C", "/tmp"])
            .output()?;
        if !output.status.success() {
            return Err(format!("Failed to extract file: {:?}", output).into());
        }
    }

    Ok(file_path.to_string())
}
