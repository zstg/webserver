fn old(mut stream: TcpStream) {
	let buf_reader = BufReader::new(&stream);
	let http_request:Vec<_> = buf_reader.lines().map(|result| result.unwrap()).take_while(|line| !line.is_empty()).collect();

	let contents = fs::read_to_string("/home/stig/Git/webserver/src/hello.html").unwrap();
	let l = contents.len();


	// Write a HTTP response
	let response;
	if http_request[0] == "GET / HTTP/1.1" {
	    response = "HTTP/1.1 200 OK";
	}
	else {
	    response = "HTTP/1.1 404 NOT FOUND";
	}
	let formatted_response = format!("{response}\n\nContent-Length: {l}\n\n{contents}");
	println!("{}", formatted_response);
	let ans = stream.write(formatted_response.as_bytes()).unwrap(); // since response.as_bytes() returns a result of Ok type, we unwrap it.
	println!("{:?}",http_request);
	println!("{}",ans);

    }
