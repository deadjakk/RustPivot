use std::error::Error;
use client::*;

// base socks code sourced from https://github.com/ajmwagar/merino,
// with modifications by @deadjakk

fn main() -> Result<(), Box<dyn Error>> {
    let ip = "localhost"; // change this to remote server 
    let port = 3030;             // ditto
    let mut auth_methods: Vec<u8> = Vec::new();
    // Allow unauthenticated connections
    auth_methods.push(client::AuthMethods::NoAuth as u8);
    let mut client = Client::new(port, ip, auth_methods)?;

    loop {
        match client.serve(){
            Ok(_) => (),
            Err(e) => println!("{:?}",e),
        };
    }
}
