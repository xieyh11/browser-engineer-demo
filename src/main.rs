use openssl::ssl::{SslConnector, SslMethod};
use std::{
    collections::HashMap,
    env,
    io::{BufRead, BufReader, Error, Read, Write},
    net::TcpStream,
};

fn request(url: &str) -> Result<Option<(HashMap<String, String>, String)>, Error> {
    let (schema, url) = url.split_once("://").unwrap();
    if schema != "https" && schema != "http" {
        return Ok(None);
    }
    let (url, path) = url.split_once("/").unwrap_or((url, "/"));
    let (host, url_port) = url.split_once(":").unwrap_or((url, ""));

    let port = match schema {
        "http" => 80,
        _ => 443,
    };
    let port = if url_port != "" {
        url_port.parse::<u16>().unwrap()
    } else {
        port
    };
    let url = format!("{}:{}", host, port);
    let mut s = TcpStream::connect(url).unwrap();
    match schema {
        "http" => online_access(s, host, path),
        _ => online_access(
            SslConnector::builder(SslMethod::tls())
                .unwrap()
                .build()
                .connect(host, s)
                .unwrap(),
            host,
            path,
        ),
    }
}

fn online_access<S: Read + Write>(
    mut s: S,
    host: &str,
    path: &str,
) -> Result<Option<(HashMap<String, String>, String)>, Error> {
    s.write_all(format!("GET {} HTTP/1.0\r\nHost: {}\r\n\r\n", path, host).as_bytes())?;
    let mut reader = BufReader::new(s);
    let mut buf = String::new();
    reader.read_line(&mut buf)?;
    let (version, status) = buf.split_once(" ").unwrap();
    let (status, explanation) = status.split_once(" ").unwrap();
    if status != "200" {
        return Ok(None);
    }
    let mut headers: HashMap<String, String> = HashMap::new();
    let mut body = String::new();
    for line in reader.by_ref().lines() {
        let line = line?;
        if line == "" {
            break;
        }
        let (header, value) = line.split_once(":").unwrap();
        headers.insert(header.to_lowercase(), value.trim().to_string());
    }
    for line in reader.lines() {
        body.push_str(&line?);
    }
    return Ok(Some((headers, body)));
}

fn show(body: &str) {
    let mut is_angle = 0;
    for c in body.chars() {
        is_angle = if c == '<' {
             is_angle+1
        } else if c == '>' {
           is_angle-1
        } else {
            is_angle
        };
        if is_angle == 0 {
            print!("{}", c);
        }
    }
}

fn load(url: &str) {
    let (_, body) = request(url).unwrap().unwrap();
    show(&body);
}

fn main() {
    load(&env::args().nth(1).unwrap());
}
