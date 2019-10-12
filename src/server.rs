extern crate log;

use std::io::{
    Read,
    Write
};
use std::net;
use std::process;
use crate::clients;
use crate::config;
use crate::requests;
use crate::replies;

pub struct Server {
    pub clients: clients::ClientsMap,
    pub address: net::SocketAddrV4,
    pub key: String,
    pub password: String,
    pub drop_votes: u8,
    pub capacity: u16,
    pub list_size: u16,
    pub drop_verification: bool
}

impl Server {
    pub fn from_start_config(start_config: &config::StartConfig) -> Server {
        Server {
            clients: clients::ClientsMap::new(),
            address: start_config.address.clone(),
            key: start_config.key.clone(),
            password: start_config.password.clone(),
            drop_votes: start_config.drop_votes,
            capacity: start_config.capacity,
            list_size: start_config.list_size,
            drop_verification: start_config.drop_verification
        }
    }

    pub fn run(&mut self) {
        if let Ok(listener) = net::TcpListener::bind(self.address) {
            log::info!("I'm listening on {}", self.address);
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        if let Ok(peer_addr) = stream.peer_addr() {
                            log::debug!("New connection from {}", peer_addr);
                            let mut buffer = [0; 1024];
                            if let Ok(bytes_read) = stream.read(&mut buffer) {
                                let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
                                if let Some(request) = requests::Request::from(&request_str) {
                                    let request_type = format!("{}", request);
                                    match request {
                                        requests::Request::Admin(a_request) => {
                                            let mut reply: Option<String> = None;
                                            match peer_addr {
                                                net::SocketAddr::V4(admin_addr) => {
                                                    if self.address.ip() == admin_addr.ip() {                                                        
                                                        match a_request {
                                                            requests::AdminRequest::Drop { password, ip } => {
                                                                if self.key == password {
                                                                    reply = Some(replies::reply_admin_drop(&ip, &mut self.clients, &peer_addr))
                                                                } else { log::info!("The admin forgot the password"); }
                                                            },
                                                            requests::AdminRequest::GetByIndex { password, start_index, end_index } => {
                                                                if self.key == password {
                                                                    reply = Some(replies::reply_admin_getbyindex(start_index, end_index, &self.clients , &peer_addr));
                                                                } else { log::info!("The admin forgot the password"); }
                                                            },
                                                            requests::AdminRequest::GetByMac { password, mac } => {
                                                                if self.key == password {
                                                                    reply = Some(replies::reply_admin_getbymac(&mac, &self.clients, &peer_addr));
                                                                } else { log::info!("The admin forgot the password"); }
                                                            },
                                                            requests::AdminRequest::GetByUsername { password, username, start_index } => {
                                                                if self.key == password {
                                                                    reply = Some(replies::reply_admin_getbyusername(&username, &self.clients, self.list_size, start_index, &peer_addr));
                                                                } else { log::info!("The admin forgot the password"); }
                                                            },
                                                            requests::AdminRequest::SetCapacity { password, capacity } => {
                                                                if self.key == password {
                                                                    reply = Some(replies::reply_admin_setcapacity(capacity, &mut self.capacity, self.clients.len()));
                                                                } else { log::info!("The admin forgot the password"); }
                                                            },
                                                            requests::AdminRequest::SetDropVerification { password, drop_verification } => {
                                                                if self.key == password {
                                                                    reply = Some(replies::reply_admin_setdropverification(drop_verification, &mut self.drop_verification));
                                                                } else { log::info!("The admin forgot the password"); }
                                                            },
                                                            requests::AdminRequest::SetDropVotes { password, drop_votes } => {
                                                                if self.key == password {
                                                                    reply = Some(replies::reply_admin_setdropvotes(drop_votes, &mut self.drop_votes, &mut self.clients));
                                                                } else { log::info!("The admin forgot the password"); }
                                                            },
                                                            requests::AdminRequest::SetKey { password, key } => {
                                                                if self.key == password {
                                                                    reply = Some(replies::reply_admin_setkey(&key, &mut self.key));
                                                                } else { log::info!("The admin forgot the password"); }
                                                            },
                                                            requests::AdminRequest::SetListSize { password, list_size } => {
                                                                if self.key == password {
                                                                    reply = Some(replies::reply_admin_setlistsize(list_size, &mut self.list_size));
                                                                } else { log::info!("The admin forgot the password"); }
                                                            },
                                                            requests::AdminRequest::SetPassword { password, new_password } => {
                                                                if self.key == password {
                                                                    reply = Some(replies::reply_admin_setpassword(&new_password, &mut self.password));
                                                                } else { log::info!("The admin forgot the password"); }
                                                            }                                               
                                                        }
                                                    } else { log::warn!("A remote host tried to admin the sever ({})", admin_addr); }
                                                },
                                                net::SocketAddr::V6(admin_addr) => {
                                                    log::info!("I only support IPv4, admin {} doesn't know that", admin_addr)
                                                }
                                            }
                                                                            
                                            if let Some(reply) = reply {
                                                if let Ok(bytes_written) = stream.write(reply.as_bytes()) {
                                                    if bytes_written == reply.as_bytes().len() {
                                                        log::info!("{} Ok!", request_type);
                                                    } else {
                                                        log::warn!("{} Err! I couldn' write the entire reply", request_type);
                                                    }
                                                } else {
                                                    log::warn!("{} Err! I couldn' write anything", request_type);
                                                }                                                
                                            } else {
                                                let message = b"Only IPv4 is supported";
                                                if let Ok(bytes_written) = stream.write(message) {
                                                    if bytes_written == message.len() {
                                                        log::info!("Failure message sent to {}", peer_addr);
                                                    } else {
                                                        log::warn!("I could not write the entire failure message to admin {}", peer_addr);
                                                    }
                                                } else {
                                                    log::warn!("I couln't write to admin {}", peer_addr);
                                                }
                                            }
                                        },
                                        requests::Request::Client(c_request) => {    
                                            let mut reply: Option<String> = None;                                            
                                            match c_request {
                                                requests::ClientRequest::GetByMac { password: client_password, mac } => {
                                                    if self.password == client_password {
                                                        reply = Some(replies::reply_client_getbymac(&mac, &self.clients, &peer_addr));
                                                    } else { log::info!("The client {} doesn't know the password", peer_addr); }
                                                },
                                                requests::ClientRequest::GetByUsername { password: client_password, username, start_index } => {
                                                    if self.password == client_password {
                                                        reply = Some(replies::reply_client_getbyusername(&username, &self.clients, self.list_size, start_index, &peer_addr));
                                                    } else { log::info!("The client {} doesn't know the password", peer_addr); }
                                                },
                                                requests::ClientRequest::Drop { password: client_password, ip } => {
                                                    if self.password == client_password {
                                                        reply = Some(replies::reply_client_drop(&ip, &mut self.clients, self.drop_votes, &peer_addr));
                                                    } else { log::info!("The client {} doesn't know the password", peer_addr); }
                                                    log::debug!("Client's DB:\n{}", self.clients);
                                                },
                                                requests::ClientRequest::SignUp { password: client_password, username, mac, port, get_only_by_mac } => {
                                                    if self.password == client_password {
                                                        match peer_addr {
                                                            net::SocketAddr::V4(sock_addr) => {
                                                                reply = Some(replies::reply_client_signup(&mut self.clients, &username, &mac, &sock_addr.ip(), port, get_only_by_mac, self.capacity));
                                                            },
                                                            _ => log::info!("I only support IPv4, client {} doesn't know that", peer_addr)
                                                        }
                                                    } else { log::info!("The client {} doesn't know the password", peer_addr); }
                                                    log::debug!("Client's DB:\n{}", self.clients);
                                                }
                                            }

                                            if let Some(reply) = reply {
                                                if let Ok(bytes_written) = stream.write(reply.as_bytes()) {
                                                    if bytes_written == reply.as_bytes().len() {
                                                        log::info!("{} from {} Ok!", request_type, peer_addr);
                                                    } else {
                                                        log::warn!("{} from {} Err! I couldn' write the entire reply", request_type, peer_addr);
                                                    }
                                                } else {
                                                    log::warn!("{} from {} Err! I couldn' write anything", request_type, peer_addr);
                                                }                                                
                                            } else {
                                                let message = b"Only IPv4 is supported";
                                                if let Ok(bytes_written) = stream.write(message) {
                                                    if bytes_written == message.len() {
                                                        log::info!("Failure message sent to {}", peer_addr);
                                                    } else {
                                                        log::warn!("I could not write the entire failure message to client {}", peer_addr);
                                                    }
                                                } else {
                                                    log::warn!("I couln't write to client {}", peer_addr);
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    log::info!("I didn't understand the request of {}", peer_addr);
                                    let message = b"I don't understand you";
                                    if let Ok(bytes_written) = stream.write(message) {
                                        if bytes_written == message.len() {
                                            log::info!("Failure message sent to {}", peer_addr);
                                        } else {
                                            log::warn!("I could not write the entire failure message to client {}", peer_addr);
                                        }
                                    } else {
                                        log::warn!("I couln't write to client {}", peer_addr);
                                    }
                                }
                            } else {
                                log::error!("I couldn't read the message of {} :/", peer_addr);
                            }
                            if let Ok(()) = stream.shutdown(net::Shutdown::Both) {        
                            } else { log::error!("I couldn't shutdown the connection with {}", peer_addr) }
                        } else {
                            log::error!("I couldn't get to peer address of a client :/");
                        }
                    },
                    Err(e) => {
                        log::error!("{}", e);
                    }
                }
            }
        } else {
            log::error!("I couldn't bind to {} :/", self.address);
            process::exit(1);
        }
    }
}

pub fn is_valid_key(key: &str) -> bool {
    key.is_ascii() && key.len() < 33
}
