use clap::Parser;
use percent_encoding::{AsciiSet, CONTROLS};
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

    /// Static file directory
    #[clap(long, short = 'd')]
    data: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();
    let listener = TcpListener::bind(args.addr)
        .await
        .expect("failed to start tcp listener");

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

    std::env::set_current_dir(args.data.clone()).expect("failed to set work dir");
    const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');

    loop {
        let (socket, _) = listener.accept().await?;
        let mut acceptor = match TlsAcceptor::from(tls_cfg.clone()).accept(socket).await {
            Ok(a) => a,
            Err(e) => {
                eprintln!("Failed to accept TLS connection : {}", e);
                continue;
            }
        };

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

            let path = format!(
                ".{}",
                percent_encoding::percent_decode_str(uri.path()).decode_utf8_lossy()
            );

            let response = match std::fs::metadata(path.clone()) {
                Ok(dir) if dir.is_dir() => match std::fs::read_dir(path.clone()) {
                    Ok(dir) => Response {
                        status: status::Code::Success,
                        meta: String::from("text/gemini"),
                        body: {
                            let mut links: Vec<String> = dir
                                .filter_map(|entry| {
                                    entry.map_or(None, |entry| {
                                        entry
                                            .path()
                                            .file_name()
                                            .map_or(None, |path| path.to_str())
                                            .map(|path| {
                                                (
                                                    entry.path().display().to_string(),
                                                    path.to_string(),
                                                )
                                            })
                                    })
                                })
                                .map(|(full, entry)| {
                                    format!(
                                        "=> /{} {}\n",
                                        percent_encoding::utf8_percent_encode(&full, FRAGMENT),
                                        entry
                                    )
                                })
                                .collect();

                            links.sort();

                            let body: String = links.iter().rev().map(String::from).collect();

                            format!(
                                "{}\n### Path: [ {} ]\n{}\n{}",
                                include_str!("header.gmi"),
                                path,
                                body,
                                include_str!("footer.gmi")
                            )
                        },
                    },
                    Err(e) => Response {
                        status: status::Code::CgiError,
                        meta: format!("Failed to generate directory list : {}", e),
                        body: String::default(),
                    },
                },
                Ok(file) if file.is_file() => match std::fs::read_to_string(path.clone()) {
                    Ok(body) => Response {
                        status: status::Code::Success,
                        meta: String::from("text/gemini"),
                        body: format!("{}\n{}", body, include_str!("footer.gmi")),
                    },
                    Err(e) => Response {
                        status: status::Code::CgiError,
                        meta: format!("Failed to read file : {}", e),
                        body: String::default(),
                    },
                },
                _ => Response {
                    status: status::Code::NotFound,
                    meta: String::from("Path not found"),
                    body: String::default(),
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
