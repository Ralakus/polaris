use clap::Parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

pub mod status;

#[derive(Clone, Debug)]
pub struct Response {
    pub status: status::Code,
    pub meta: String,
    pub body: String,
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}\r\n{}", self.status as u8, self.meta, self.body)
    }
}

impl Response {
    pub fn new(status: status::Code, meta: String, body: String) -> Self {
        Self { status, meta, body }
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Server address
    addr: String,

    /// TLS cert file
    #[clap(long, short = 'c')]
    cert: String,

    /// TLS key file
    #[clap(long, short = 'k')]
    key: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();
    let listener = TcpListener::bind(args.addr)
        .await
        .expect("Failed to start TCP listener");

    // Build TLS configuration.
    let tls_cfg = {
        // Load public certificate.
        let certs = load_certs(&args.cert);
        // Load private key.
        let key = load_private_key(&args.key);
        // Do not use client certificate authentication.
        let cfg = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .expect("failed to generate tls config");
        std::sync::Arc::new(cfg)
    };

    loop {
        let (socket, _) = listener.accept().await?;
        let mut acceptor = TlsAcceptor::from(tls_cfg.clone())
            .accept(socket)
            .await
            .expect("failed to accept tls connection");

        tokio::spawn(async move {
            let mut uri_buffer = [0; 2048];
            match acceptor.read(&mut uri_buffer).await {
                Ok(bytes_read) => {
                    if bytes_read > 1024 {
                        let response = Response {
                            status: status::Code::BadRequest,
                            meta: String::from("URI exceeds 1024 bytes"),
                            body: String::default(),
                        };
                        if let Err(e) = acceptor.write(response.to_string().as_bytes()).await {
                            eprintln!("Failed to send bad URI error to client : {}", e);
                        }
                        return;
                    }
                }
                Err(e) => {
                    let response = Response {
                        status: status::Code::BadRequest,
                        meta: format!("Failed to recieve URI : {}", e),
                        body: String::default(),
                    };
                    if let Err(e) = acceptor.write(response.to_string().as_bytes()).await {
                        eprintln!("Failed to send bad URI error to client : {}", e);
                    }
                    return;
                }
            }

            let uri_string = match std::str::from_utf8(&uri_buffer) {
                Ok(uri) => uri,
                Err(e) => {
                    let response = Response {
                        status: status::Code::BadRequest,
                        meta: format!("URI is not valid UTF-8 : {}", e),
                        body: String::default(),
                    };

                    if let Err(e) = acceptor.write(response.to_string().as_bytes()).await {
                        eprintln!("Failed to send non UTF-8 URI error to client : {}", e);
                    }

                    return;
                }
            };

            let uri = match url::Url::parse(uri_string) {
                Ok(uri) => uri,
                Err(e) => {
                    let response = Response {
                        status: status::Code::BadRequest,
                        meta: format!("URI is valid URI format : {}", e),
                        body: String::default(),
                    };

                    if let Err(e) = acceptor.write(response.to_string().as_bytes()).await {
                        eprintln!("Failed to send invalid URI error to client : {}", e);
                    }

                    return;
                }
            };

            let response = match uri.path_segments().map(|path| path.collect::<Vec<_>>()) {
                Some(path) if path.iter().next().map_or(false, |val| val == &"echo") => {
                    match uri.query() {
                        Some(query) => Response {
                            status: status::Code::Success,
                            meta: String::from("text/plain"),
                            body: format!(
                                "{}\r\n",
                                percent_encoding::percent_decode_str(query).decode_utf8_lossy()
                            ),
                        },
                        None => Response {
                            status: status::Code::Input,
                            meta: String::from("Please enter some text"),
                            body: String::default(),
                        },
                    }
                }
                _ => Response {
                    status: status::Code::Success,
                    meta: String::from("text/gemini"),
                    body: String::from("Please go to echo path for test.\n=> echo"),
                },
            };

            if let Err(e) = acceptor.write(response.to_string().as_bytes()).await {
                eprintln!("Failed to send response to client : {}", e);
            }
        });
    }
}

// Load public certificates from file.
fn load_certs(filename: &str) -> Vec<rustls::Certificate> {
    // Open certificate file.
    let certfile = std::fs::File::open(filename).unwrap();
    let mut reader = std::io::BufReader::new(certfile);

    // Load and return certificate.
    let certs = rustls_pemfile::certs(&mut reader).unwrap();
    certs.into_iter().map(rustls::Certificate).collect()
}

// Load private key from file.
fn load_private_key(filename: &str) -> rustls::PrivateKey {
    // Open keyfile.
    let keyfile = std::fs::File::open(filename).unwrap();
    let mut reader = std::io::BufReader::new(keyfile);

    // Load and return a single private key.
    let keys = rustls_pemfile::rsa_private_keys(&mut reader).unwrap();
    if keys.len() != 1 {
        panic!("expected a single private key");
    }

    rustls::PrivateKey(keys[0].clone())
}
