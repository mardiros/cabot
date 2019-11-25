extern crate cabot;

use async_std;
use cabot::{Client, RequestBuilder};

#[async_std::main]
async fn main() {
    let request = RequestBuilder::new("https://www.rust-lang.org/")
        .build()
        .unwrap();
    let client = Client::new();
    let response = client.execute(&request).await.unwrap();
    print!("{}", response.body_as_string().unwrap());
}
