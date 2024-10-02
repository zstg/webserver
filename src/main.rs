#![allow(dead_code)]
#![allow(unused_variables)]

use std::fs::read_to_string;
use std::io::{BufRead, BufReader, Write, stdin};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::{spawn, sleep};
use std::time::{UNIX_EPOCH, SystemTime, Duration};
use std::sync::atomic::AtomicI32;
use std::convert::TryInto;

/// Struct to hold a circular buffer of logs
#[derive(Debug)]
struct CircularLogBuffer {
    log_entries: Vec<Log>,
    index: usize, // Index to insert the next log
    size: usize,  // Size of the buffer
}

impl CircularLogBuffer {
    /// Create a new CircularLogBuffer with a specified size
    fn new(size: usize) -> Self {
        CircularLogBuffer {
            log_entries: vec![Log {
                when_opened: 0,
                when_closed: 0,
                no_of_bytes_sent: 0,
                no_of_bytes_received: 0,
            }; size],
            index: 0,
            size,
        }
    }

    /// Add a new log entry to the circular buffer
    fn add_log(&mut self, log: Log) {
        self.log_entries[self.index] = log;
        self.index = (self.index + 1) % self.size; // Move to the next index in a circular manner
    }

    /// Get all log entries
    fn get_logs(&self) -> &[Log] {
        &self.log_entries
    }
}

/// Struct to hold global server state
#[derive(Debug)]
struct GlobalServerState {
    listen_thread: Arc<AtomicI32>,
    cli_thread: Arc<AtomicI32>,
    session_array: Arc<Mutex<Vec<SessionState>>>,
    current_session_connections: Arc<Mutex<Vec<String>>>,
    history_buffer: Arc<Mutex<CircularLogBuffer>>, // Changed to CircularLogBuffer
}

/// Struct to represent a single connection log
#[derive(Debug, Clone)]
struct Log {
    when_opened: i64, // time when a connection opened // i64 since UNIX_EPOCH requires it
    when_closed: i64, // time when it closed
    no_of_bytes_sent: i32, // number of bytes sent when the connection is active
    no_of_bytes_received: i32 // number of bytes received
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
            session_array: Arc::new(Mutex::new(vec![SessionState { timestamp: String::new(), bytes_read: 0, bytes_written: 0 }; 10])),
            current_session_connections: Arc::new(Mutex::new(vec![String::new(); 10])),
            history_buffer: Arc::new(Mutex::new(CircularLogBuffer::new(1024))) // Initialized with a buffer size of 1024
        });
        let accept_thread = {
            let global_state = Arc::clone(&global_state);
            spawn(move || Self::accept_connections(listener, &global_state))
        };

        Self::prompt(global_state);
        // Join the accept_thread to ensure it runs continuously
        if let Err(e) = accept_thread.join() {
            eprintln!("Accept thread encountered an error: {:?}", e);
        }
    }

    /// Function to provide an administrative prompt
    fn prompt(global_state: Arc<GlobalServerState>) {
        println!("Welcome!");
        loop {
            let mut input = String::new();
            print!(r">>> ");
            Write::flush(&mut std::io::stdout()).unwrap();
            if stdin().read_line(&mut input).is_err() {
                eprintln!("Error reading input.");
                continue;
            }
            Self::process_inp(input, global_state.clone());
        }
    }

    fn process_inp(inp: String, global_state: Arc<GlobalServerState>) {
        match inp.as_str().trim() {
            "/exit" | "exit" => std::process::exit(1),
            "/status" | "status" => Self::print_status(global_state),
            "/hist" | "hist" => Self::print_history(global_state),
            _ => eprintln!("Unknown command: {}", inp.trim()),
        }
    }  

    /// Function to print history of connections
    fn print_history(global_state: Arc<GlobalServerState>) {
        let history_buffer = global_state.history_buffer.lock().unwrap();
        let logs = history_buffer.get_logs();

        // Print the history buffer (up to 1024 logs)
        println!("Connection History (last 1024 logs):");
        for log in logs {
            println!(
                "Connection opened: {}, closed: {}, bytes sent: {}, bytes received: {}",
                log.when_opened,
                log.when_closed,
                log.no_of_bytes_sent,
                log.no_of_bytes_received
            );
        }
    }

    /// Function to print status of active connections
    fn print_status(global_state: Arc<GlobalServerState>) {
        let session_array = global_state.session_array.lock().unwrap();
        let current_connections = global_state.current_session_connections.lock().unwrap();

        // Calculate the total number of active sessions
        let active_sessions = current_connections.iter().filter(|conn| !conn.is_empty()).count();

        // Print the server status
        println!("Server Status:");
        println!("Active Sessions: {}", active_sessions);
        
        let total_bytes_read: i32 = session_array.iter().map(|session| session.bytes_read).sum();
        let total_bytes_written: i32 = session_array.iter().map(|session| session.bytes_written).sum();

        println!("Total Bytes Sent: {}", total_bytes_written);
        println!("Total Bytes Received: {}", total_bytes_read);

        println!("{:?}", current_connections);
    }
    
    /// Function to accept incoming connections and spawn a thread for each connection
    fn accept_connections(listener: TcpListener, global_state: &Arc<GlobalServerState>) {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let address = format!("{}", stream.peer_addr().unwrap());
                    let port = stream.peer_addr().unwrap().port();
                    let connection = Connection { stream, address, port };
                    let global_state = global_state.clone();
                    spawn(move || {
                        if let Err(e) = Self::handle_connection(connection, global_state) {
                            eprintln!("Error handling connection: {:?}", e);
                        }
                    });
                }
                Err(e) => eprintln!("Failed to accept connection: {}", e),
            }
        }
    }

    /// Function to handle an individual connection
    fn handle_connection(mut connection: Connection, global_state: Arc<GlobalServerState>) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = BufReader::new(connection.stream.try_clone()?);
        let mut buffer = String::new();
        let mut log_entry = Log {
            when_opened: Self::get_current_time(),
            when_closed: 0,
            no_of_bytes_sent: 0,
            no_of_bytes_received: 0,
        };

        // Find an available slot in current_session_connections
        let mut session_index = None;
        {
            let mut current_connections = global_state.current_session_connections.lock().unwrap();
            if let Some(index) = current_connections.iter().position(|conn| conn.is_empty()) {
                current_connections[index] = connection.address.clone();
                session_index = Some(index);
            }
        }

        if session_index.is_none() {
            // No available slots for connections
            eprintln!("No available session slots for connection {}", connection.address);
            return Ok(()); // No need to return an error
        }

        let session_index = session_index.unwrap();

        // Main connection handling loop
        loop {
            buffer.clear();
            match reader.read_line(&mut buffer) {
                Ok(0) => break, // End of stream
                Ok(_) => {
                    let request = Request::new(buffer.trim());
                    let response = Response::generate_response(&request);

                    // Write the response
                    if let Err(e) = connection.stream.write_all(response.as_bytes()) {
                        eprintln!("Failed to send response: {}", e);
                        break;
                    }

                    // Update bytes sent and received for this session
                    log_entry.no_of_bytes_sent += response.len() as i32;
                    log_entry.no_of_bytes_received += buffer.len() as i32;

                    // Update session bytes read and written
                    {
                        let mut session_array = global_state.session_array.lock().unwrap();
                        let session = &mut session_array[session_index];
                        session.bytes_read += buffer.len() as i32;
                        session.bytes_written += response.len() as i32;
                    }
                }
                Err(e) => {
                    // eprintln!("Error reading line: {}", e);
                    break;
                }
            }
        }

        log_entry.when_closed = Self::get_current_time();
        Self::update_global_state(log_entry, &global_state);

        // Clear the session once the connection is closed
        {
            let mut current_connections = global_state.current_session_connections.lock().unwrap();
            current_connections[session_index] = String::new();
        }

        Ok(())
    }

    /// Function to update the global server state with the connection log
    fn update_global_state(log_entry: Log, global_state: &Arc<GlobalServerState>) {
        let mut history_buffer = global_state.history_buffer.lock().unwrap();
        history_buffer.add_log(log_entry);
        
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

impl Request {
    /// Create a new Request from the request line
    pub fn new(request_line: &str) -> Self {
        Self {
            status: request_line.to_string()
        }
    }
}

/// Struct to represent an HTTP response
pub struct Response {
    pub status: String
}

impl Response {
    /// Generate the HTTP response based on the request
    fn generate_response(request: &Request) -> String {
        match request.status.as_str() {
            "GET / HTTP/1.1" => {
                let filename = "src/hello.html";
                let contents = match read_to_string(filename) {
                    Ok(content) => content,
                    Err(_) => return "HTTP/1.1 500 Internal Server Error\r\n\r\n".to_string(),
                };
                let length = contents.len();
                format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", length, contents)
            },
            "GET /slowpage HTTP/1.1" => {
                sleep(Duration::from_secs(15));
                let filename = "src/slowpage.html";
                let contents = match read_to_string(filename) {
                    Ok(content) => content,
                    Err(_) => return "HTTP/1.1 500 Internal Server Error\r\n\r\n".to_string(),
                };
                let length = contents.len();
                format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", length, contents)
            },
            _ => {
                let filename = "src/404.html";
                let contents = match read_to_string(filename) {
                    Ok(content) => content,
                    Err(_) => "HTTP/1.1 500 Internal Server Error\r\n\r\n".to_string(),
                };
                let length = contents.len();
                format!("HTTP/1.1 404 NOT FOUND\r\nContent-Length: {}\r\n\r\n{}", length, contents)
            }
        }
    }
}

fn shell(cmd:&str) -> String {
    let op = std::process::Command::new("sh")
             .arg("-c")
             .arg(cmd)
             .output()
             .expect("Error");

    // convert the raw otuput into meaningful ascii
    let v: String = op.stdout
                   .into_iter()
                   .map(|c| c as char)
                   .filter(|&c| c != '\n')
                   .collect();
    v
}


/// Main function that starts the server
fn main() {
    // let ip = shell("curl ifconfig.me"); // shell("curl ipinfo.io/ip");
    Connection::start_server("0.0.0.0", 80);
}
