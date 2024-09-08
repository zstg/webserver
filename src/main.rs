use std::fs;
use std::io::{BufRead,BufReader,Write};
use std::net::{TcpListener,TcpStream};
use std::thread::sleep;
use std::time::Duration;

#[allow(dead_code)]
fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let steam = stream.unwrap();
	handle_connection(steam);
    }
    
    fn handle_connection(mut stream: TcpStream) {
	let c = BufReader::new(&stream).lines().next(); // next returns a Some(OK()), so needs 2 unwraps
	let request_line = c.unwrap().unwrap();

	let (status_line, filename) =
	    if request_line == "GET / HTTP/1.1" {
		("HTTP/1.1 200 OK", "/home/stig/Git/webserver/src/hello.html")
	    }
            else if request_line == "GET /slowpage HTTP/1.1" {
		sleep(Duration::from_secs(15));
	        ("HTTP/1.1 200 OK", "/home/stig/Git/webserver/src/hello.html") // irl this can serve a different (typically) heavier page
	    }
	    else {
                ("HTTP/1.1 404 NOT FOUND", "/home/stig/Git/webserver/src/404.html")
            };

	let contents = fs::read_to_string(filename).unwrap();
	let length = contents.len();

	let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

	stream.write_all(response.as_bytes()).unwrap();
    }
}
