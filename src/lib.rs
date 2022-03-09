mod http;
mod dom;

use std::error::Error;
use crate::http::request::request;
use crate::dom::show::show;

pub fn load(url: &str) -> Result<String, Box<dyn Error>> {
    let resp = request(url)?;
    show(resp)
}