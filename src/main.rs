use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

fn handle_connection(mut stream: TcpStream) {
    let mut buffer: [u8; 1024] = [0; 1024];

    stream.read(&mut buffer).unwrap();

    let request: std::borrow::Cow<'_, str> = String::from_utf8_lossy(&buffer);
    let first_line = request.lines().next().unwrap();
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    let path = Path::new("www/index.html");
    let display = path.display();

    let mut file = match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    };

    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(why) => panic!("couldn't read {}: {}", display, why),
        Ok(_) => print!("{} contains:\n{}", display, s),
    }

    let (method, path, version) = (parts[0], parts[1], parts[2]);

    println!("Method: {}\nPath: {}\nVersion: {}", method, path, version);

    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{}", s);

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn main() {
    let listener: TcpListener = TcpListener::bind("127.0.0.1:80").unwrap();

    println!("Ferrox running on http://127.0.0.1:80");

    for stream in listener.incoming() {
        let stream: TcpStream = stream.unwrap();
        handle_connection(stream);
    }
}
