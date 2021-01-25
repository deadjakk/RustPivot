# RustPivot Server

This should be compiled and run on the server that is
receiving the connection from the RustPivot client.

Build:
`cargo b --bin server --release`

Usage: 
`./target/release/server 127.0.0.1:2020 0.0.0.0:3030`

There is no interaction with this server, just information
regarding the state of the socks communication.

After the client has successfully connected back to the server,
place `socks5 127.0.0.1 2020` in `/etc/proxychains.conf`  
and proxy your traffic using `proxychains <command>` or use via browser, etc.  
