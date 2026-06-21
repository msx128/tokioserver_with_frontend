use std::path::Path;

use regex::Regex;

use sqlx::mysql::MySqlPoolOptions;

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

            if let Err(e) = response(&buf, &mut socket).await {
                eprintln!("err={:?}", e)
            }
        });
    }
}

async fn response(buf: &[u8], socket: &mut TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let address = str::from_utf8(&buf)?;

    let re = Regex::new(r"^(?<method>\S+) (?<address>\S+)")?;
    let Some(caps) = re.captures(address) else {
        panic!()
    };

    match &caps["method"] {
        "GET" => Ok(get_handler(socket, match_path(&caps["address"])).await),
        "POST" => Ok(post_handler(socket, match_path(&caps["address"])).await),
        _ => panic!(),
    }
}

async fn post_handler(socket: &mut TcpStream, path: String) {
    match &path[..] {
        "/click" => on_button_click(socket).await,
        _ => {
            let silly_message: &str = "horoshiy yazik programmirovania";
            let res: String = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                silly_message.len(),
                silly_message
            );
            if let Err(e) = socket.write_all(res.as_bytes()).await {
                eprintln!("could not write socket; err={:?}", e);
            }
        }
    };
}

async fn on_button_click(socket: &mut TcpStream) {
    let silly_message: &str = "horoshiy yazik programmirovania";
    let res: String = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        silly_message.len(),
        silly_message
    );
    if let Err(e) = socket.write_all(res.as_bytes()).await {
        eprintln!("could not write socket; err={:?}", e);
    }
}

fn match_path(address: &str) -> String {
    match address {
        "/" => "static/index.html".to_string(),
        "/css/index.css" => "static/css/index.css".to_string(),
        "/css/404.css" => "static/css/404.css".to_string(),
        "/meme" => "static/meme.html".to_string(),
        "/css/meme.css" => "static/css/meme.css".to_string(),
        "/images/meme.jpg" => "static/images/meme.jpg".to_string(),
        "/button" => "static/button.html".to_string(),
        "/css/button.css" => "static/css/button.css".to_string(),
        "/js/button.js" => "static/js/button.js".to_string(),
        _ => "static/404.html".to_string(),
    }
}

async fn get_handler(socket: &mut TcpStream, path: String) {
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
