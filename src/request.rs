use http::StatusCode;
use openssl::ssl::{SslConnector, SslMethod};
use std::{
    collections::HashMap,
    error::Error,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    str::FromStr,
};

#[derive(Default)]
pub struct Response {
    pub status: StatusCode,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<Vec<u8>>,
}

fn request(url: &str) -> Result<Response, Box<dyn Error>> {
    if url.starts_with("data:") {
        return parse_data(url);
    }
    let (schema, url) = url.split_once("://").unwrap_or(("https", url));
    if schema == "file" {
        return Ok(Response {
            status: StatusCode::OK,
            headers: None,
            body: Some(fs::read(url)?),
        });
    }
    if schema != "https" && schema != "http" {
        return Err("Not Supoort Schema")?;
    }
    let (url, path) = url.split_once("/").unwrap_or((url, ""));
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
) -> Result<Response, Box<dyn Error>> {
    s.write_all(format!("GET /{} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: Browser-Demo/0.0.1\r\n\r\n", path, host).as_bytes())?;
    let mut reader = BufReader::new(s);
    let mut buf = String::new();
    reader.read_line(&mut buf)?;
    let (version, status) = buf.split_once(" ").unwrap();
    let (status, explanation) = status.split_once(" ").unwrap();
    if status != "200" {
        return Ok(Response {
            status: StatusCode::from_str(status)?,
            headers: None,
            body: Some(Vec::from(explanation)),
        });
    }
    let mut headers: HashMap<String, String> = HashMap::new();
    for line in reader.by_ref().lines() {
        let line = line?;
        if line == "" {
            break;
        }
        let (header, value) = line.split_once(":").unwrap();
        headers.insert(header.to_lowercase(), value.trim().to_string());
    }
    let mut body: Vec<u8> = Vec::new();
    reader.read_to_end(&mut body)?;
    return Ok(Response {
        status: StatusCode::from_str(status)?,
        headers: Some(headers),
        body: Some(body),
    });
}

fn show(resp: Response) -> Result<String, Box<dyn Error>> {
    if resp.status == StatusCode::OK {
        if let Some(body) = resp.body {
            show_only_body(body)
        } else {
            Ok(String::new())
        }
    } else {
        show_without_tag(resp.status, resp.body.unwrap())
    }
}

fn show_without_tag(status: StatusCode, body: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let body = String::from_utf8(body)?;
    let mut is_angle = 0;
    let mut body_without_tag = status.to_string();
    for c in body.chars() {
        if c == '<' {
            is_angle += 1;
        } else if c == '>' {
            is_angle -= 1;
        } else if is_angle == 0 {
            body_without_tag.push(c);
        }
    }
    Ok(body_without_tag)
}

fn parse_data(url: &str) -> Result<Response, Box<dyn Error>> {
    let url = &url["data:".len()..];
    let (metadata, body) = url.split_once(",").unwrap();
    match metadata {
        "text/html" => Ok(Response {
            status: StatusCode::OK,
            headers: None,
            body: Some(Vec::from(format!("<html><body>{}</body></html>", body))),
        }),
        _ => Err("Not Support Data Meta Type")?,
    }
}

fn show_only_body(body: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let body = String::from_utf8(body)?;
    let mut is_angle = 0;
    let mut meet_body = false;
    let mut leave_body = false;
    let mut tag = String::new();
    let mut only_body = String::new();
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
            only_body.push(c);
        } else if is_angle == 1 {
            tag.push(char::from(c));
        }
    }
    Ok(only_body)
}

pub fn load(url: &str) -> Result<String, Box<dyn Error>> {
    let resp = request(url)?;
    show(resp)
}
