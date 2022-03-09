use chardetng::EncodingDetector;
use http::StatusCode;
use openssl::ssl::{SslConnector, SslMethod};
use std::{
    borrow::Cow,
    collections::HashMap,
    error::Error,
    fmt, fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    str,
    str::FromStr,
};

const DETECT_STEP: usize = 100;
const HTTP_SEP: &str = "\r\n";
#[derive(Default)]
pub struct Response {
    pub version: String,
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
                    let mut body_iter = body.chunks(self::DETECT_STEP).peekable();
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

pub enum HttpVersion {
    V1_0,
    V1_1,
}

impl fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HttpVersion::V1_0 => write!(f, "HTTP/1.0"),
            HttpVersion::V1_1 => write!(f, "HTTP/1.1"),
        }
    }
}

pub enum HttpMethod {
    GET,
}

impl std::default::Default for HttpMethod {
    fn default() -> Self {
        HttpMethod::GET
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HttpMethod::GET => write!(f, "GET"),
        }
    }
}

pub enum HttpSchema {
    DATA(String),
    FILE,
    HTTPS,
    HTTP,
}

impl std::default::Default for HttpSchema {
    fn default() -> Self {
        HttpSchema::HTTPS
    }
}

impl fmt::Display for HttpSchema {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HttpSchema::DATA(t) => write!(f, "data:{}", t),
            HttpSchema::FILE => write!(f, "file"),
            HttpSchema::HTTP => write!(f, "http"),
            HttpSchema::HTTPS => write!(f, "https"),
        }
    }
}

impl TryFrom<&str> for HttpSchema {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "file" => Ok(HttpSchema::FILE),
            "http" => Ok(HttpSchema::HTTP),
            "https" => Ok(HttpSchema::HTTPS),
            t => Err(format!("Not Support Schema {}", t)),
        }
    }
}

#[derive(Default)]
pub struct Request {
    method: HttpMethod,
    schema: HttpSchema,
    host: String,
    port: u16,
    path: String,
    body: Vec<u8>,
    headers: HashMap<String, String>,
}

impl Request {
    pub fn try_new(method: HttpMethod, url: &str) -> Result<Self, Box<dyn Error>> {
        if url.starts_with("data:") {
            let url = &url["data:".len()..];
            let (metadata, body) = url.split_once(",").unwrap_or(("text/html", url));
            Ok(Request {
                method: HttpMethod::GET,
                schema: HttpSchema::DATA(metadata.to_string()),
                body: Vec::from(body),
                ..Default::default()
            })
        } else {
            let (schema, url) = url.split_once("://").unwrap_or(("https", url));
            let schema = HttpSchema::try_from(schema)?;
            match &schema {
                HttpSchema::FILE => Ok(Request {
                    method: HttpMethod::GET,
                    schema: schema,
                    path: url.to_string(),
                    ..Default::default()
                }),
                other => {
                    let (url, path) = url.split_once("/").unwrap_or((url, ""));
                    let (host, url_port) = url.split_once(":").unwrap_or((url, ""));
                    let port = if url_port != "" {
                        url_port.parse::<u16>().unwrap_or(match other {
                            HttpSchema::HTTP => 80,
                            _ => 443,
                        })
                    } else {
                        match other {
                            HttpSchema::HTTP => 80,
                            _ => 443,
                        }
                    };
                    Ok(Request {
                        method: method,
                        schema: schema,
                        host: host.to_string(),
                        port: port,
                        path: format!("/{}", path),
                        ..Default::default()
                    })
                }
            }
        }
    }

    fn get_url(&self) -> String {
        format!("{}:{}", &self.host, &self.port)
    }

    fn set_header(&mut self, k: &str, v: &str) {
        self.headers
            .insert(k.to_owned().to_lowercase(), v.to_owned());
    }
}

pub struct HttpOpt {
    keep_alive: bool,
}

impl std::default::Default for HttpOpt {
    fn default() -> Self {
        HttpOpt { keep_alive: false }
    }
}

pub struct HttpClient {
    version: HttpVersion,
    opt: HttpOpt,
    user_agent: String,
}

impl std::default::Default for HttpClient {
    fn default() -> Self {
        HttpClient {
            version: HttpVersion::V1_1,
            opt: HttpOpt::default(),
            user_agent: "Browser-Demo/0.0.1".to_owned(),
        }
    }
}

impl HttpClient {
    pub fn request(&self, req: &Request) -> Result<Response, Box<dyn Error>> {
        match &req.schema {
            HttpSchema::DATA(t) => parse_data(req, t),
            HttpSchema::FILE => Ok(Response {
                status: StatusCode::OK,
                headers: None,
                body: Some(fs::read(&req.path)?),
                ..Default::default()
            }),
            _ => self.request_http(req),
        }
    }

    fn request_http(&self, req: &Request) -> Result<Response, Box<dyn Error>> {
        let s = TcpStream::connect(req.get_url()).unwrap();
        match &req.schema {
            HttpSchema::HTTP => self.online_access(s, req),
            _ => self.online_access(
                SslConnector::builder(SslMethod::tls())
                    .unwrap()
                    .build()
                    .connect(&req.host, s)
                    .unwrap(),
                req,
            ),
        }
    }

    fn online_access<S: Read + Write>(
        &self,
        mut s: S,
        req: &Request,
    ) -> Result<Response, Box<dyn Error>> {
        s.write_all(self.construct_request(req).as_bytes())?;
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
            version: version.to_owned(),
            status: StatusCode::from_str(status)?,
            headers: Some(headers),
            body: Some(body),
            ..Default::default()
        });
    }

    fn construct_request(&self, req: &Request) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("{} {} {}", &req.method, &req.path, &self.version));
        lines.push(format!("Host: {}", &req.host));
        match &self.version {
            HttpVersion::V1_0 => {}
            HttpVersion::V1_1 => {
                if !self.opt.keep_alive {
                    lines.push("Connection: close".to_owned());
                }
            }
        };
        lines.push(format!("User-Agent: {}", &self.user_agent));
        for (k, v) in &req.headers {
            lines.push(format!("{}: {}", k, v));
        }
        format!("{}{}{}", lines.join(HTTP_SEP), HTTP_SEP, HTTP_SEP)
    }
}

fn parse_data(req: &Request, metadata: &str) -> Result<Response, Box<dyn Error>> {
    match metadata {
        "text/html" => Ok(Response {
            status: StatusCode::OK,
            headers: None,
            body: Some(Vec::from(format!(
                "<html><body>{}</body></html>",
                String::from_utf8(req.body.clone())?
            ))),
            ..Default::default()
        }),
        _ => Err("Not Support Data Meta Type")?,
    }
}
