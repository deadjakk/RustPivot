use std::env;
use std::io::copy;
use std::time::Duration;
use std::io::prelude::*;
use std::sync::{mpsc::channel,mpsc::Receiver,mpsc::Sender};
use std::{thread};
use std::net::{Shutdown, TcpStream, TcpListener};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main(){

    // create channels in which to store stream
    let (streams_t, streams_r) : (Sender<TcpStream>, Receiver<TcpStream>) = channel();   
    
    // create two listeners, one for socks clients
    let frontend = env::args().nth(1)
        .expect("first arg not given, usage: <localaddr:port> <externaladdr:port>");
    let backend = env::args().nth(2)
        .expect("second arg not given, usage: <localaddr:port> <externaladdr:port>");
    println!("set socks5 proxy to {} to connect", frontend);

    thread::spawn(move || {
                               
        // create the listener for the connection from the connecting 
        let listener = TcpListener::bind(backend).unwrap();

        loop {
            match listener.accept(){
                Ok((stream,addr)) => {
                    println!("received reverse proxy connection from : {:?}",addr);
                    if let Err(e) = streams_t.send(stream) {
                        println!("error channeling socket :{:?}",e);
                        continue;
                    }
                } 
                _ => (),
            } 

        } 
    });

    let listener = TcpListener::bind(frontend).unwrap();
    loop {
        match listener.accept(){
            Ok((mut fstream,addr)) => {
                println!("received client connection from: {:?}",addr);
                
                match streams_r.recv_timeout(Duration::from_millis(1000)){
                    Ok(mut bstream) => {
                        // validate the socket is still alive before handing it off
                        match validate_stream(&mut bstream) {
                            Ok(_) => {
                                // stream is valid, move copy the fd
                                handle_streams(&mut fstream,&mut bstream);
                            }
                            Err(e) => {
                                // in case there are more in the channel
                                println!("error validating sock:{:?}",e);
                                if let Err(e) = bstream.shutdown(Shutdown::Both){
                                    println!("error, shutting backend socket: {:?}", e);
                                    continue; 
                                }
                            }
                        }
                    }
                    _=> { // no back end stream available, closing socket
                        if let Err(e) = fstream.shutdown(Shutdown::Both){
                            println!("error, shutting client socket: {:?}", e);
                        }
                    }
                }
            } 
            _ => (),
        } 

    } 

}

fn validate_stream(bstream: &mut TcpStream) -> Result<()> {
    let mut read_buf = [0u8,2];
    bstream.read_exact(&mut read_buf)?;
    match &read_buf {
        &[0x22,0x44] => {
            // 'preflight' check
            Ok(())
        }
        _ =>{ 
            Err(From::from("preflight message received was incorrect"))
        }
    }
}

fn handle_streams(fstream: &mut TcpStream, bstream: &mut TcpStream) {

    // Copy it all
    let mut outbound_in  = bstream.try_clone().expect("failed to clone socket");
    let mut outbound_out = bstream.try_clone().expect("failed to clone socket");
    let mut inbound_in   = fstream.try_clone().expect("failed to clone socket");
    let mut inbound_out  = fstream.try_clone().expect("failed to clone socket");

    // if alive, copy socks together in new threads
    thread::spawn(move || {
        match copy(&mut outbound_in, &mut inbound_out){
            Ok(_)=>{
                // these are GOING to throw errors, so just unwrapping
                outbound_in.shutdown(Shutdown::Read).unwrap_or(());
                inbound_out.shutdown(Shutdown::Write).unwrap_or(());
            }
            Err(_) => {
                 println!("failed to perform the copy on sockets.");
            }
        }
    });

    // Upload Thread
    thread::spawn(move || {
        match copy(&mut inbound_in, &mut outbound_out) {
            Ok(_) => {
                // these are GOING to throw errors, so just unwrapping
                inbound_in.shutdown(Shutdown::Read).unwrap_or(());
                outbound_out.shutdown(Shutdown::Write).unwrap_or(());
            }
            Err(_) => {
                 println!("failed to perform the copy on sockets..");
            }
        }
    });

}
