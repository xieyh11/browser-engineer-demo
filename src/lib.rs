mod http;
mod dom;

use std::error::Error;
use crate::http::request::*;
use crate::dom::show::show;

pub fn load(url: &str) -> Result<String, Box<dyn Error>> {
    let client = HttpClient::default();
    let mut req = Request::try_new(HttpMethod::GET, url)?;
    let resp = client.request(&mut req)?;
    show(resp)
}