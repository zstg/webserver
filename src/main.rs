#[allow(dead_code)]
#[allow(unused_variables)]
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{sleep, spawn};
use std::time::Duration;

/// Main function that starts the server
fn main() {
    start_server("192.168.0.94:7878");
}

/// Function to start the server and listen for incoming connections
fn start_server(address_port: &str) {
    let listener = TcpListener::bind(address_port).unwrap();
    let accept_thread = spawn(move || {
        accept_connections(listener);
    });

    // Join the accept_thread to ensure it runs continuously
    accept_thread.join().unwrap();
}

/// Function to accept incoming connections and spawn a thread for each connection
fn accept_connections(listener: TcpListener) {
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        spawn(move || {
            handle_connection(stream);
        });
    }
}

/// Function to handle an individual connection
fn handle_connection(mut stream: TcpStream) {
    let c = BufReader::new(&stream).lines().next(); // next returns a Some(Ok()), so it needs 2 unwraps
    let request_line = c.unwrap().unwrap();

    let (status_line, filename) = determine_response(&request_line);

    let contents = fs::read_to_string(filename).unwrap();
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
    stream.write_all(response.as_bytes()).unwrap();
}

/// Function to determine the response based on the request line
fn determine_response(request_line: &str) -> (&str, &str) {
    if request_line == "GET / HTTP/1.1" {
        ("HTTP/1.1 200 OK", "/home/stig/Git/webserver/src/hello.html")
    } else if request_line == "GET /slowpage HTTP/1.1" {
        sleep(Duration::from_secs(5));
        ("HTTP/1.1 200 OK", "/home/stig/Git/webserver/src/404.html") // Could serve a heavier page here
    } else {
        ("HTTP/1.1 404 NOT FOUND", "/home/stig/Git/webserver/src/404.html")
    }
}
