#[macro_use]
extern crate lazy_static;
extern crate regex;

use regex::Regex;

use std::collections::HashMap;

use std::io;
use std::io::BufReader;
use std::io::BufRead;

use std::fs::File;
use std::env;

use std::sync::{Arc, Mutex};
use std::thread;

use std::net::TcpStream;

// Start port.
const START: i32 = 1;
// End port.
const END:   i32 = 65536;

lazy_static! {
    // Port number to service name mapping, ports are strings which is a bit stupid.
    static ref SERVICES:HashMap<String, String> = { load_services().unwrap_or( HashMap::new() ) };
}

fn main() {
    // Not enough CLI arguments, help the user out.
    if env::args().len() < 3 {
        println!( "Usage: \n\t{} HOST THREADS", env::args().nth(0).unwrap() );
        std::process::exit(1);
    }

    // Host to scan.
    let host         = env::args().nth(1).unwrap();
    // Amount of threads to spawn.
    let thread_count = env::args().nth(2).unwrap().parse::<i32>().unwrap();

    if thread_count <= 0 {
        println!( "Invalid thread count '{}', must be >= 1.", thread_count );
        std::process::exit(1);
    }

    // Keeps tracks of thread handles.
    let mut threads: Vec<_> = vec![];
    // Synchronized access to the discovered open ports.
    let open_ports = Arc::new( Mutex::new( vec![] ) );

    // Size of each port range segment for each thread.
    let step = END / thread_count;

    // First port range segment, for the first thread.
    let mut start  = START;
    let mut finish = START + step;

    // Spawn the specified amount of threads.
    for _ in 0..thread_count {

        // We'll be moving into threads so make some copies of the data we need to access.
        let host       = host.clone();
        let open_ports = open_ports.clone();

        threads.push( thread::spawn( move || {
            for port in get_open_ports( &host, start, finish ) {
                // Yay, open port, add it to the list.
                open_ports.lock().unwrap().push( port );
            }
        }));

        // Move to the next segment of the port range for the next thread.
        start   = finish;
        finish += step;
    }

    // Wait for all threads to finish.
    for thread in threads {
        // Not sure what to do with the return value, ignore it I guess.
        let _ = thread.join();
    }

    // Get the list of open ports and sort it.
    let mut ports = open_ports.lock().unwrap();
    ports.sort();

    // No dice.
    if ports.is_empty() {
        println!( "No open ports" );
        return
    }

    println!( "\nNUMBER \t| SERVICE" );
    println!( "---------------------------------------" );
    for port in ports.iter() {
        println!( "{} \t| {}", port, get_service_name( port ) );
        println!( "---------------------------------------" )
    }
}

fn load_services() -> Result<HashMap<String, String>, io::Error> {
    let f = try!( File::open( "/etc/services" ) );
    let file = BufReader::new(&f);

    let mut s = HashMap::new();

    let regexp = Regex::new( r"(\w+)\s+(\d+)" ).unwrap();

    for line in file.lines() {
        let l = line.unwrap();

        let captures = regexp.captures( &l );
        if !captures.is_some() { continue }

        let ccaptures = captures.unwrap();

        let name = ccaptures.at(1).unwrap_or( "" );
        let port = ccaptures.at(2).unwrap_or( "" );

        if port.is_empty() || name.is_empty() { continue }

        s.insert( port.to_string(), name.to_string() );
    }

    Result::Ok( s )
}

fn get_service_name( port: &i32 ) -> String {
    SERVICES.get( &port.to_string() ).unwrap_or( &String::new() ).to_string()
}

fn is_open( host: &String, port: i32 ) -> bool {
    TcpStream::connect( &*format!( "{}:{}", host, port ) ).is_ok()
}

fn get_open_ports( host: &String, start: i32, finish: i32 ) -> Vec<i32> {
    let mut open_ports: Vec<i32> = vec![];

    for port in start..finish {
        if !is_open( host, port ) { continue; }
        open_ports.push( port )
    }

    open_ports
}
