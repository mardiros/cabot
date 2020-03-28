extern crate cabot;

//use async_std;
use async_std::task;
use cabot::{Client, RequestBuilder};

#[async_std::main]
async fn main() {
    let response = task::spawn(async {
        let request = RequestBuilder::new("https://www.rust-lang.org")
            .build()
            .unwrap();
        let client = Client::new();
        let response = client.execute_box(&request).await;
        response
    });
    println!("{}", response.await.unwrap().body_as_string().unwrap());
}
