use chardetng::EncodingDetector;
use egui::TextBuffer;
use http::StatusCode;
use openssl::ssl::{SslConnector, SslMethod};
use std::{
    borrow::Cow,
    collections::HashMap,
    error::Error,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    str,
    str::FromStr,
};

const detect_step: usize = 100;
#[derive(Default)]
pub struct Response {
    pub status: StatusCode,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<Vec<u8>>,
}

impl Response {
    pub fn body_to_string(&self) -> Result<Cow<str>, Box<dyn Error>> {
        match &self.body {
            Some(body) => match str::from_utf8(body) {
                Ok(body) => Ok(Cow::Borrowed(body)),
                Err(_) => {
                    let mut guess_detector = EncodingDetector::new();
                    let mut body_iter = body.chunks(self::detect_step).peekable();
                    let mut decode_body: Option<Cow<str>> = None;
                    while let Some(part) = body_iter.next() {
                        let finished = body_iter.peek().is_none();
                        let meet_non_ascll = guess_detector.feed(part, finished);
                        if !meet_non_ascll {
                            continue;
                        }
                        let guess_charset = guess_detector.guess(None, false);
                        let (body_str, _, has_error) = guess_charset.decode(body);
                        if !has_error {
                            decode_body = Some(body_str);
                            break;
                        }
                    }

                    if let Some(body_str) = decode_body {
                        Ok(body_str)
                    } else {
                        Err("Cannot Guess Charset")?
                    }
                }
            },
            None => Ok(Cow::Borrowed("")),
        }
    }
}

pub fn request(url: &str) -> Result<Response, Box<dyn Error>> {
    if url.starts_with("data:") {
        return parse_data(url);
    }
    let (schema, url) = url.split_once("://").unwrap_or(("https", url));
    if schema == "file" {
        return Ok(Response {
            status: StatusCode::OK,
            headers: None,
            body: Some(fs::read(url)?),
            ..Default::default()
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
            ..Default::default()
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
        ..Default::default()
    });
}

fn parse_data(url: &str) -> Result<Response, Box<dyn Error>> {
    let url = &url["data:".len()..];
    let (metadata, body) = url.split_once(",").unwrap();
    match metadata {
        "text/html" => Ok(Response {
            status: StatusCode::OK,
            headers: None,
            body: Some(Vec::from(format!("<html><body>{}</body></html>", body))),
            ..Default::default()
        }),
        _ => Err("Not Support Data Meta Type")?,
    }
}
