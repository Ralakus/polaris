use clap::Parser;
use percent_encoding::{AsciiSet, CONTROLS};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

pub mod response;
use response::Response;

/// URL percent encoding/decoding ascii set
const URL_PERCENT_ENCODING: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b':')
    //.add(b'/') Commented out due to wierd generated links and isn't a valid character for file names anyways
    .add(b'?')
    .add(b'#')
    .add(b'[')
    .add(b']')
    .add(b'@')
    .add(b'!')
    .add(b'$')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b';')
    .add(b'=');

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

    env_logger::init();

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

    loop {
        let (socket, addr) = listener.accept().await?;
        let tls_cfg = tls_cfg.clone();

        tokio::spawn(async move {
            let mut acceptor = match TlsAcceptor::from(tls_cfg).accept(socket).await {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Failed to accept TLS connection : {}", e);
                    return;
                }
            };

            let mut url_buffer = [0; 2048];
            let url_result = acceptor.read(&mut url_buffer).await;
            let closure = || async move {
                let byte_count;
                match url_result {
                    Ok(bytes_read) => {
                        byte_count = bytes_read;
                        if byte_count > 1024 {
                            return Response::BadRequest("url exceeds 1024 bytes".into());
                        }
                    }
                    Err(e) => return Response::BadRequest(format!("Failed to get url : {} ", e)),
                };

                let url_string = match std::str::from_utf8(&url_buffer[..byte_count]) {
                    Ok(url) => url,
                    Err(e) => {
                        return Response::BadRequest(format!("url is not valid UTF-8 : {}", e))
                    }
                };

                let url = match url::Url::parse(url_string) {
                    Ok(url) => url,
                    Err(e) => return Response::BadRequest(format!("url is valid : {}", e)),
                };

                log::info!(
                    "[{}] => {}",
                    addr,
                    url_string.strip_suffix("\r\n").unwrap_or(url_string).trim()
                );
                process_request(url).await
            };

            if let Err(e) = acceptor.write(&closure().await.as_bytes()).await {
                log::error!("Failed to send response to client : {}", e);
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

/// Server response code
async fn process_request(url: url::Url) -> Response {
    let path = match percent_encoding::percent_decode_str(url.path())
        .decode_utf8_lossy()
        .to_string()
    {
        path if path == "/" || path.is_empty() => ".".to_string(),
        path if path.starts_with('/') => path[1..].to_string(),
        path => path,
    };

    if path == "robots.txt" {
        return match std::fs::read(".robots.txt") {
            Ok(bytes) => Response::Success("text/plain".into(), bytes),
            Err(_) => Response::Success("text/plain".into(), "".into()),
        };
    }

    let header = std::fs::read_to_string(".header.gmi").unwrap_or_default();
    let footer = std::fs::read_to_string(".footer.gmi").unwrap_or_default();

    match std::fs::metadata(path.clone()) {
        Ok(dir) if dir.is_dir() => match std::fs::read_dir(path.clone()) {
            Ok(dir) => {
                let mut links: Vec<String> = dir
                    .filter_map(|dir_entry| dir_entry.ok())
                    .filter_map(|dir_entry| dir_entry.path().to_str().map(ToString::to_string))
                    .filter_map(|path| {
                        path.split('/')
                            .last()
                            .map(|path_ref| (path.clone(), path_ref.to_string()))
                    })
                    .filter_map(|(path, name)| {
                        if name.starts_with('.') {
                            None
                        } else {
                            Some((path, name))
                        }
                    })
                    .map(|(path, name)| {
                        format!(
                            "=> {} {}\n",
                            percent_encoding::utf8_percent_encode(&path, URL_PERCENT_ENCODING),
                            name
                        )
                    })
                    .collect();

                links.sort();

                let content: String = links.iter().rev().map(String::from).collect();
                let body = format!(
                    "{}\n### Path: [ {} ]\n{}\n{}",
                    header, path, content, footer,
                )
                .into_bytes();

                Response::Success("text/gemini".into(), body)
            }
            Err(e) => Response::CgiError(format!("Failed to generate directory list : {}", e)),
        },
        Ok(file) if file.is_file() => match std::fs::read(path.clone()) {
            Ok(bytes) => {
                let default_mime: mime::Mime = "text/gemini".parse().unwrap();
                let mime = mime_guess::from_path(path.clone())
                    .first()
                    .unwrap_or(default_mime.clone());

                Response::Success(format!("{}", mime), bytes)
            }
            Err(e) => Response::CgiError(format!("Failed to read file : {}", e)),
        },
        _ => Response::NotFound("Not found".into()),
    }
}
