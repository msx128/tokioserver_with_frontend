use std::path::Path;

use regex::Regex;

use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run().await?;
    Ok(())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    println!("sever started");

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            let bs = match socket.read(&mut buf).await {
                Ok(0) => return,
                Ok(bs) => bs,
                Err(e) => {
                    eprintln!("failed to read from buffer; err={:?}", e);
                    return;
                }
            };

            let request = match str::from_utf8(&buf[..bs]) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("failed to convert buf to str; err={:?}", e);
                    return;
                }
            };

            println!("{}", request);

            let path = match matching_path(&buf[..bs]) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("failed to convert buf to str; err={:?}", e);
                    return;
                }
            };

            handler(&mut socket, path).await;
        });
    }
}

fn matching_path(buf: &[u8]) -> Result<&str, Box<dyn std::error::Error>> {
    let address = str::from_utf8(&buf)?;

    let re = Regex::new(r"^GET (?<address>\S+)")?;
    let Some(caps) = re.captures(address) else {
        panic!()
    };

    let address = &caps["address"];

    match address {
        "/" => Ok("static/index.html"),
        "/css/index.css" => Ok("static/css/index.css"),
        "/css/404.css" => Ok("static/css/404.css"),
        "/meme" => Ok("static/meme.html"),
        "/css/meme.css" => Ok("static/css/meme.css"),
        "/images/meme.jpg" => Ok("static/images/meme.jpg"),
        "/button" => Ok("static/button.html"),
        "/css/button.css" => Ok("static/css/button.css"),
        "/js/button.js" => Ok("static/js/button.js"),
        _ => Ok("static/404.html"),
    }
}

async fn handler(socket: &mut TcpStream, path: &str) {
    let file = match fs::read(&path).await {
        Ok(content) => content,
        Err(e) => {
            eprintln!("can't read file; err={:?}", e);
            return;
        }
    };

    let mime = match Path::new(&path).extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("js") => "text/javascript; charset=utf-8",
        _ => "application/octet-stream",
    };

    let status_line = "HTTP/1.1 200 OK\r\n";
    let content_type = format!("Content-Type: {}\r\n", mime);
    let content_length = format!("Content-Length: {}\r\n", file.len());
    let headers = format!("{}{}{}\r\n", status_line, content_type, content_length);

    let mut response = headers.into_bytes();
    response.extend(file);

    if let Err(e) = socket.write_all(&response).await {
        eprintln!("failed to write to socket; err = {:?}", e);
        return;
    }
}
