use std::{io::{BufRead, BufReader}, net::{TcpListener, TcpStream}, result};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
	handle_connection(stream);
        println!("Connection established!");
    }

    fn handle_connection(stream: TcpStream) {
	let buf_reader = BufReader::new(stream);
	let http_request: Vec<_> = buf_reader.lines().map(|result| result.unwrap()).collect();
	println!("{:#?}",http_request);
    }
}
