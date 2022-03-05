use openssl::ssl::{SslConnector, SslMethod};
use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Error, Read, Write},
    net::TcpStream,
};

fn request(url: &str) -> Result<Option<(HashMap<String, String>, String)>, Error> {
    if url.starts_with("data:") {
        return parse_data(url);
    }
    let (schema, url) = url.split_once("://").unwrap_or(("https", url));
    if schema == "file" {
        return Ok(Some((HashMap::new(), fs::read_to_string(url)?)));
    }
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
        url_port.parse::<u16>().unwrap_or(port)
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
    s.write_all(format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: Browser-Demo/0.0.1\r\n\r\n", path, host).as_bytes())?;
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
        if c == '<' {
            is_angle += 1;
        } else if c == '>' {
            is_angle -= 1;
        } else if is_angle == 0 {
            print!("{}", c);
        }
    }
}

fn parse_data(url: &str) -> Result<Option<(HashMap<String, String>, String)>, Error> {
    let url = &url["data:".len()..];
    let (metadata, body) = url.split_once(",").unwrap();
    match metadata {
        "text/html" => Ok(Some((
            HashMap::new(),
            format!("<html><body>{}</body></html>", body),
        ))),
        _ => Ok(None),
    }
}

fn show_only_body(body: &str) {
    let mut is_angle = 0;
    let mut meet_body = false;
    let mut leave_body = false;
    let mut tag = String::new();
    for c in body.chars() {
        if c == '<' {
            tag.clear();
            is_angle += 1;
        } else if c == '>' {
            if tag == "body" {
                meet_body = true;
            } else if tag == "/body" {
                leave_body = true;
            }
            tag.clear();
            is_angle -= 1;
        } else if is_angle == 0 && meet_body && !leave_body {
            print!("{}", c);
        } else if is_angle == 1 {
            tag.push(c);
        }
    }
}
pub fn load(url: &str) {
    let (_, body) = request(url).unwrap().unwrap();
    show_only_body(&body);
}
