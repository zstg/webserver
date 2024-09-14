#![allow(dead_code)]
#![allow(unused_variables)]

use std::fs::read_to_string;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::{spawn, sleep};
use std::time::{UNIX_EPOCH, SystemTime, Duration};
use std::sync::atomic::{AtomicI32};

/// Struct to hold global server state
struct GlobalServerState {
    listen_thread: Arc<AtomicI32>, // the thread that's currently listening (server-side)
    cli_thread: Arc<AtomicI32>, // the thread associated with the CLI
    session_array: Arc<Mutex<Vec<SessionState>>>, // stores session structs
    current_session_connections: Arc<Mutex<Vec<String>>>, // stores connection addresses
    history_buffer: Arc<Mutex<LogBuffer>> // past 1024 connections
}

/// Struct to represent a single connection log
struct Log {
    when_opened: i64, // time when a connection opened // i64 since UNIX_EPOCH requires it
    when_closed: i64, // time when it closed
    no_of_bytes_sent: i32, // number of bytes sent when the connection is active
    no_of_bytes_received: i32 // number of bytes received
}

/// Struct to hold a buffer of logs
struct LogBuffer {
    log_entries: Vec<Log>, // changed to Vec for dynamic size
    index: i32
}

/// Struct to hold session state information
#[derive(Debug, Clone)]
struct SessionState {
    timestamp: String,
    bytes_read: i32,
    bytes_written: i32
}

/// Struct to represent a network connection
pub struct Connection {
    pub stream: TcpStream,
    pub address: String,
    pub port: u16
}

impl Connection {
    /// Function to start the server and listen for incoming connections
    pub fn start_server(address: &str, port: u16) {
        let listener = TcpListener::bind(format!("{}:{}", address, port)).unwrap();
        let global_state = Arc::new(GlobalServerState {
            listen_thread: Arc::new(AtomicI32::new(0)),
            cli_thread: Arc::new(AtomicI32::new(0)),
            session_array: Arc::new(Mutex::new(vec![SessionState { timestamp: String::new(), bytes_read: 0, bytes_written: 0 }; 10  ])),
            current_session_connections: Arc::new(Mutex::new(vec![String::new(); 10])),
            history_buffer: Arc::new(Mutex::new(LogBuffer { log_entries: Vec::new(), index: 0 } ))
        });

        let accept_thread = {
            let global_state = Arc::clone(&global_state);
            spawn(move || Self::accept_connections(listener, global_state) )
        };

        // Join the accept_thread to ensure it runs continuously
        accept_thread.join().unwrap();
    }

    /// Function to accept incoming connections and spawn a thread for each connection
    fn accept_connections(listener: TcpListener, global_state: Arc<GlobalServerState>) {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let address = format!("{}", stream.peer_addr().unwrap());
                    let port = stream.peer_addr().unwrap().port();
                    let connection = Connection { stream, address, port };
                    let global_state = Arc::clone(&global_state);
                    spawn(move || Self::handle_connection(connection, global_state));
                }
                Err(e) => eprintln!("Failed to accept connection: {}", e),
            }
        }
    }

    /// Function to handle an individual connection
    fn handle_connection(mut connection: Connection, global_state: Arc<GlobalServerState>) {
        let mut reader = BufReader::new(connection.stream.try_clone().unwrap());
        let mut buffer = String::new();
        let mut log_entry = Log {
            when_opened: Self::get_current_time(),
            when_closed: 0,
            no_of_bytes_sent: 0,
            no_of_bytes_received: 0,
        };

        loop {
            buffer.clear();
            match reader.read_line(&mut buffer) {
                Ok(0) => break, // End of stream
                Ok(_) => {
                    let request_line = buffer.trim();
                    let (status_line, filename) = Response::determine_response(request_line);

                    let contents = match read_to_string(&filename) {
                        Ok(content) => content,
                        Err(_) => {
                            let error_response = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
                            let _ = connection.stream.write_all(error_response.as_bytes());
                            continue;
                        }
                    };

                    let length = contents.len();
                    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

                    // Write the response
                    if let Err(e) = connection.stream.write_all(response.as_bytes()) {
                        eprintln!("Failed to send response: {}", e);
                        break;
                    }
                    log_entry.no_of_bytes_sent += length as i32;
                    log_entry.no_of_bytes_received += buffer.len() as i32;
                }
                Err(e) => {
                //     eprintln!("Error reading line: {}", e);
                //     break;
                }
            }
        }

        log_entry.when_closed = Self::get_current_time();
        Self::update_global_state(log_entry, global_state);
    }

    /// Function to update the global server state with the connection log
    fn update_global_state(log_entry: Log, global_state: Arc<GlobalServerState>) {
        let mut history_buffer = global_state.history_buffer.lock().unwrap();
        if history_buffer.log_entries.len() >= 1024 {
            history_buffer.log_entries.remove(0);
        }
        history_buffer.log_entries.push(log_entry);
        history_buffer.index += 1;

        // Update session array and current session connections if needed
        // This can be done based on your specific logic for managing sessions
    }

    /// Function to get the current time in a suitable format
    fn get_current_time() -> i64 {
	(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64).try_into().unwrap()
    }
}

/// Struct to represent an HTTP request
pub struct Request {
    pub status: String
}

/// Struct to represent an HTTP response
pub struct Response {
    pub status: String
}

impl Response {
    fn determine_response(request_line: &str) -> (String, String) {
        match request_line {
            "GET / HTTP/1.1" => (
                "HTTP/1.1 200 OK".to_string(),
                "src/hello.html".to_string()
            ),
            "GET /slowpage HTTP/1.1" => {
                sleep(Duration::from_secs(5));
                (
                    "HTTP/1.1 200 OK".to_string(),
                    "src/slowpage.html".to_string()
                )
            },
            _ => (
                "HTTP/1.1 404 NOT FOUND".to_string(),
                "src/404.html".to_string()
            ),
        }
    }

    fn stats(status: &str) {
        println!("Status: {}", status);
    }
}

/// Main function that starts the server
fn main() {
    // Connection::start_server("192.168.0.94", 8080);
    Connection::start_server("localhost", 8080);
}
