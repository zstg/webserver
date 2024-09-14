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
    let accept_thread = spawn(move || accept_connections(listener));

    // Join the accept_thread to ensure it runs continuously
    accept_thread.join().unwrap();
}

/// Function to accept incoming connections and spawn a thread for each connection
fn accept_connections(listener: TcpListener) {
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        spawn(move || handle_connection(stream));
    }
}

/// Function to handle an individual connection
fn handle_connection(mut stream: TcpStream) {
    // Process requests in a loop
    loop {
        let request_line = match BufReader::new(&stream).lines().next() {
            Some(Ok(line)) => line, // lines() returns a Result and next() returns an Option
            _ => break, // Break the loop if there's an error (or there are no more lines)
        };
        println!("{}", request_line);

        let (status_line, filename) = determine_response(&request_line);

        let contents = match fs::read_to_string(filename) {
            Ok(content) => content,
            Err(_) => {
                // If there is an error reading the file, respond with a 500 Internal Server Error
                let error_response = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
                stream.write_all(error_response.as_bytes()).unwrap();
                continue; // Continue to the next request
            }
        };

        let length = contents.len();
        let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
	let sc = status_line; // NO &sc here, can't borrow
        spawn(move || stats(&sc));

        if let Err(e) = stream.write_all(response.as_bytes()) {
            eprintln!("Failed to send response: {}", e);
            break; // If sending the response fails, break the loop
        }
    }
}

/// Function to return server statistics
fn stats(_sl: &str) -> String {
    todo!()
}

/// Function to determine the response based on the request line
fn determine_response(request_line: &str) -> (String, String) {
    if request_line == "GET / HTTP/1.1" {
        ("HTTP/1.1 200 OK".to_string(), "/home/stig/Git/webserver/src/hello.html".to_string())
    } else if request_line == "GET /slowpage HTTP/1.1" {
        sleep(Duration::from_secs(5));
        ("HTTP/1.1 200 OK".to_string(), "/home/stig/Git/webserver/src/404.html".to_string()) // Could serve a heavier page here
    } else {
        ("HTTP/1.1 404 NOT FOUND".to_string(), "/home/stig/Git/webserver/src/404.html".to_string())
    }
}
