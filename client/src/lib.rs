#![forbid(unsafe_code)]
use std::io::prelude::*;
use std::time::Duration;
use std::io::copy;
use std::net::{Shutdown, TcpStream, SocketAddr, SocketAddrV4, SocketAddrV6, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use std::{thread};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Version of socks
const SOCKS_VERSION: u8 = 0x05;

const RESERVED: u8 = 0x00;

#[derive(Clone,Debug, PartialEq)]
pub struct User {
    pub username: String,
    password: String
}


#[derive(Debug)]
/// Possible SOCKS5 Response Codes
enum ResponseCode {
    Success = 0x00,
}

/// DST.addr variant types
#[derive(PartialEq)]
enum AddrType {
    V4 = 0x01,
    Domain = 0x03,
    V6 = 0x04,
}

impl AddrType {
    /// Parse Byte to Command
    fn from(n: usize) -> Option<AddrType> {
        match n {
            1 => Some(AddrType::V4),
            3 => Some(AddrType::Domain),
            4 => Some(AddrType::V6),
            _ => None
        }
    }
}

/// SOCK5 CMD Type
#[derive(Debug)]
enum SockCommand {
    Connect = 0x01,
    Bind = 0x02,
    UdpAssosiate = 0x3
}

impl SockCommand {
    /// Parse Byte to Command
    fn from(n: usize) -> Option<SockCommand> {
        match n {
            1 => Some(SockCommand::Connect),
            2 => Some(SockCommand::Bind),
            3 => Some(SockCommand::UdpAssosiate),
            _ => None
        }
    }
}


/// Client Authentication Methods
pub enum AuthMethods {
    /// No Authentication
    NoAuth = 0x00,
    // GssApi = 0x01,
    /// Authenticate with a username / password
    UserPass = 0x02,
    /// Cannot authenticate
    NoMethods = 0xFF
}

pub struct Client {
    ip: String,
    port: u16,
    auth_methods: Vec<u8>
}

impl Client {
    pub fn new(port: u16,  ip: &str, auth_methods: Vec<u8>) -> Result<Self> {
        Ok( Client{
            ip: ip.to_string(),
            port,
            auth_methods,
        })
    }

    pub fn serve(&mut self) -> Result<()> {
        loop {
            match TcpStream::connect((&self.ip[..],self.port)){
                Ok(stream) => {
                    let mut client = SOCKClient::new(
                        stream,
                         self.auth_methods.clone()
                         );
                        println!("+");
                        match client.init() {
                            Ok(_) => {
                            }
                            Err(_) =>() 
                        };
                }
                _=> {
                    println!("-");
                    thread::sleep(Duration::from_millis(1000));
                }
            }

        }
    }
}

struct SOCKClient {
    stream: TcpStream,
    auth_nmethods: u8,
    auth_methods: Vec<u8>,
    socks_version: u8
}

impl SOCKClient {
    /// Create a new SOCKClient
    pub fn new(stream: TcpStream, auth_methods: Vec<u8>) -> Self {
        SOCKClient {
            stream,
            auth_nmethods: 0,
            socks_version: 0,
            auth_methods
        }
    }

    /// Shutdown a client
    pub fn shutdown(&mut self) -> Result<()> {
        self.stream.shutdown(Shutdown::Both)?;
        Ok(())
    }

    fn init(&mut self) -> Result<()> {
        // perform the preflight check 
        // this is handled by clients_server, client side proxy
        // code never sees this take place
        let mut preflight_buf = [0x22,0x44];
        self.stream.write_all(&mut preflight_buf)?;

        // Actual SOCKS prcedure beginss
        let mut header = [0u8; 2];
        // Read a byte from the stream and determine the version being requested
        self.stream.read_exact(&mut header)?;

        self.socks_version = header[0];
        self.auth_nmethods = header[1];

        //trace!("Version: {} Auth nmethods: {}", self.socks_version, self.auth_nmethods);

        // Handle SOCKS4 requests
        if header[0] != SOCKS_VERSION {
            //warn!("Init: Unsupported version: SOCKS{}", self.socks_version);
            self.shutdown()?;
        }
        // Valid SOCKS5
        else {
            // Authenticate w/ client
            self.auth()?;
            // Handle requests
            self.handle_client()?;
        }

        Ok(())
    }

    fn auth(&mut self) -> Result<()> {
        //debug!("Authenticating w/ {}", self.stream.peer_addr()?.ip());
        // Get valid auth methods
        let methods = self.get_avalible_methods()?;
        //trace!("methods: {:?}", methods);

        let mut response = [0u8; 2];

        // Set the version in the response
        response[0] = SOCKS_VERSION;
        
        if methods.contains(&(AuthMethods::NoAuth as u8)) {
            // set the default auth method (no auth)
            response[1] = AuthMethods::NoAuth as u8;
            self.stream.write_all(&response)?;
            Ok(())
        }
        else {
            response[1] = AuthMethods::NoMethods as u8;
            self.stream.write_all(&response)?;
            self.shutdown()?;
            Err(From::from(".."))
        }

    }

    /// Handles a client
    pub fn handle_client(&mut self) -> Result<()> {
            let req = SOCKSReq::from_stream(&mut self.stream)?;
            
            if req.addr_type == AddrType::V6 {
            }


            match req.command {
                SockCommand::Connect => {
                    let sock_addr = addr_to_socket(&req.addr_type, &req.addr, req.port)?;
                    let target = TcpStream::connect(&sock_addr[..])?;

                    self.stream.write_all(&[SOCKS_VERSION, ResponseCode::Success as u8, RESERVED, 1, 127, 0, 0, 1, 0, 0]).unwrap();

                    let mut outbound_in = target.try_clone()?;
                    let mut outbound_out = target.try_clone()?;
                    let mut inbound_in = self.stream.try_clone()?;
                    let mut inbound_out = self.stream.try_clone()?;


                    // Download Thread
                    thread::spawn(move || {
                        match copy(&mut outbound_in, &mut inbound_out){
                            Ok(_) => {
                                outbound_in.shutdown(Shutdown::Read).unwrap_or(());
                                inbound_out.shutdown(Shutdown::Write).unwrap_or(());
                            }
                            Err(_) => (),
                        }
                    });

                    // Upload Thread
                    thread::spawn(move || {
                        match copy(&mut inbound_in, &mut outbound_out){
                            Ok(_) =>{
                                inbound_in.shutdown(Shutdown::Read).unwrap_or(());
                                outbound_out.shutdown(Shutdown::Write).unwrap_or(());
                            }
                            Err(_) => (),
                        }
                    });


                },
                SockCommand::Bind => { },
                SockCommand::UdpAssosiate => { },
            }

        Ok(())
    }

    /// Return the avalible methods based on `self.auth_nmethods`
    fn get_avalible_methods(&mut self) -> Result<Vec<u8>> {
        let mut methods: Vec<u8> = Vec::with_capacity(self.auth_nmethods as usize);
        for _ in 0..self.auth_nmethods {
            let mut method = [0u8; 1];
            self.stream.read_exact(&mut method)?;
            if self.auth_methods.contains(&method[0]) {
                methods.append(&mut method.to_vec());
            }
        }
        Ok(methods)
    }
}

/// Convert an address and AddrType to a SocketAddr
fn addr_to_socket(addr_type: &AddrType, addr: &[u8], port: u16) -> Result<Vec<SocketAddr>> {
    match addr_type {
        AddrType::V6 => {
            let new_addr = (0..8).map(|x| {
                (u16::from(addr[(x * 2)]) << 8) | u16::from(addr[(x * 2) + 1])
            }).collect::<Vec<u16>>();


            Ok(vec![SocketAddr::from(
                SocketAddrV6::new(
                    Ipv6Addr::new(
                        new_addr[0], new_addr[1], new_addr[2], new_addr[3], new_addr[4], new_addr[5], new_addr[6], new_addr[7]), 
                    port, 0, 0)
            )])
        },
        AddrType::V4 => {
            Ok(vec![SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]), port))])
        },
        AddrType::Domain => {
            let mut domain = String::from_utf8_lossy(&addr[..]).to_string();
            domain.push_str(&":");
            domain.push_str(&port.to_string());

            Ok(domain.to_socket_addrs().unwrap().collect())
        }

    }
}


/// Proxy User Request
struct SOCKSReq {
    pub version: u8,
    pub command: SockCommand,
    pub addr_type: AddrType,
    pub addr: Vec<u8>,
    pub port: u16
}

impl SOCKSReq {
    /// Parse a SOCKS Req from a TcpStream
    fn from_stream(stream: &mut TcpStream) -> Result<Self> {
        let mut packet = [0u8; 4];
        // Read a byte from the stream and determine the version being requested
        stream.read_exact(&mut packet)?;

        if packet[0] != SOCKS_VERSION {
            stream.shutdown(Shutdown::Both)?;
        }

        // Get command
        let mut command: SockCommand = SockCommand::Connect;
        match SockCommand::from(packet[1] as usize) {
            Some(com) => {
                command = com;
            },
            None => {
                stream.shutdown(Shutdown::Both)?;
            }
        };

        // DST.address
        let mut addr_type: AddrType = AddrType::V6;
        match AddrType::from(packet[3] as usize) {
            Some(addr) => {
                addr_type = addr;
            },
            None => {
                stream.shutdown(Shutdown::Both)?;
            }
        };

        // Get Addr from addr_type and stream
        let addr: Result<Vec<u8>> = match addr_type {
            AddrType::Domain => {
                let mut dlen = [0u8; 1];
                stream.read_exact(&mut dlen)?;

                let mut domain = vec![0u8; dlen[0] as usize];
                stream.read_exact(&mut domain)?;

                Ok(domain)
            },
            AddrType::V4 => {
                let mut addr = [0u8; 4];
                stream.read_exact(&mut addr)?;
                Ok(addr.to_vec())
            },
            AddrType::V6 => {
                let mut addr = [0u8; 16];
                stream.read_exact(&mut addr)?;
                Ok(addr.to_vec())
            }
        };

        let addr = addr?;

        // read DST.port
        let mut port = [0u8; 2];
        stream.read_exact(&mut port)?;

        // Merge two u8s into u16
        let port = (u16::from(port[0]) << 8) | u16::from(port[1]);

        // Return parsed request
        Ok(SOCKSReq {
            version: packet[0],
            command,
            addr_type,
            addr,
            port
        })
    }
}
