mod request;

use std::env;
use crate::request::load;
fn main() {
    load(&env::args().nth(1).unwrap());
}
