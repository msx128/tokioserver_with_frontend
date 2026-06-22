use std::path::Path;
use std::sync::Arc;

use regex::Regex;

use serde_json::Value;

use sqlx::MySqlPool;
use sqlx::mysql::MySqlPoolOptions;

use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

struct AppState {
    pool: MySqlPool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run().await?;
    Ok(())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect("mysql://root:123@localhost:3306/httpServer")
        .await?;

    let state = AppState { pool };
    let state = Arc::new(Mutex::new(state));

    println!("sever started");

    loop {
        let (mut socket, _) = listener.accept().await?;
        let state = state.clone();

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

            if let Err(e) = response(&buf[..bs], &mut socket, state).await {
                eprintln!("err={:?}", e)
            }
        });
    }
}

async fn response(
    buf: &[u8],
    socket: &mut TcpStream,
    state: Arc<Mutex<AppState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let address = str::from_utf8(&buf)?;

    let re = Regex::new(r"^(?<method>\S+)\s+(?<address>\S+)[\s\S]*?\r?\n\r?\n(?<body>[\s\S]*)$")?;
    let Some(caps) = re.captures(address) else {
        panic!()
    };

    match &caps["method"] {
        "GET" => Ok(get_handler(socket, match_path(&caps["address"])).await),
        "POST" => Ok(post_handler(socket, &caps["address"], &caps["body"], state).await),
        _ => panic!(),
    }
}

async fn post_handler(socket: &mut TcpStream, path: &str, body: &str, state: Arc<Mutex<AppState>>) {
    match path {
        "/click" => on_button_click(socket, body, state).await,
        _ => {
            let silly_message: &str = "horoshiy yazik";
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

async fn on_button_click(socket: &mut TcpStream, body: &str, state: Arc<Mutex<AppState>>) {
    let v: Value = match serde_json::from_str(body) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("problem with json parser; err={:?}", e);
            Value::Null
        }
    };

    let state = state.lock().await;

    let row = sqlx::query!("select click from clicks_counter")
        .fetch_one(&state.pool)
        .await;

    let counter: String = format!("{}", row.unwrap().click);
    let res: String = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        counter.len(),
        counter
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
