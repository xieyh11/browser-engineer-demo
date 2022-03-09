use crate::http::request::Response;
use http::StatusCode;
use std::{borrow::Cow, error::Error};

pub fn show(resp: Response) -> Result<String, Box<dyn Error>> {
    if resp.status == StatusCode::OK {
        show_only_body(resp.body_to_string()?)
    } else {
        show_without_tag(resp.status, resp.body_to_string()?)
    }
}

fn show_without_tag(status: StatusCode, body: Cow<str>) -> Result<String, Box<dyn Error>> {
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

fn show_only_body(body: Cow<str>) -> Result<String, Box<dyn Error>> {
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
            if tag.starts_with("body") {
                meet_body = true;
            } else if tag.starts_with("/body") {
                leave_body = true;
            }
            tag.clear();
            is_angle -= 1;
        } else if is_angle == 0 && meet_body && !leave_body {
            only_body.push(c);
        } else if is_angle >= 1 {
            tag.push(c);
        }
    }
    Ok(only_body)
}
