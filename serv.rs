use std::io::{self, BufRead, BufWriter, Read, Write};
use std::env;
use std::net::{TcpListener, TcpStream};

fn http(stream: &TcpStream) {
    println!("{:?}", stream);
    let mut bufr = io::BufReader::new(stream); // 参照を渡す
    let mut bufw = io::BufWriter::new(stream);
    let mut line = String::new();
    /*
    bufr.read_line(&mut line).expect("failed to read line");
    println!("{}", line);
    */
    /*
    let mut params = line.split_whitespace();
    let method = params.next().unwrap();
    let uri = params.next().unwrap();
    let version = params.next().unwrap();
    */
    //write!(bufw, "neko").expect("failed to write");
    /*
    bufw.write(b"neko").expect("failed to write");
    bufw.flush().expect("failed to flush"); // flushしないと送られない
    println!("wrote.");
    */
    // ここよくわからない
    /*
    loop{
        line = String::new();
        match bufr.read_line(&mut line).expect("failed to read line") {
            0 => break,
            x if x > 0 => print!("{}", line),
            _ => break,
        }
    }
    */
    line = String::new();
    let mut body = [0; 1024];
    bufr.read(&mut body).expect("failed to read");
    bufw.write(&body).expect("failed to write");
    bufw.flush().expect("failed to flush"); // flushしないと送られない
    //println!("{:?}", body);
}

fn main() {
    // コマンドライン引数の読込
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args); // {:?} で fmt::dbgの出力が得られる
    if args.len() != 2 {
        println!("usage: {} <portnumber>", args[0]);
        panic!("invalid arguments number");
    }
    let portnum: u16 = args[1].trim().parse().expect("invalid port number");

    // ソケットの作成
    let listener = TcpListener::bind(format!("127.0.0.1:{}", portnum))
        .expect("fail to generate TcpListener");

    // connection を accept して並列に処理する
    println!("{:?}", listener);
    let stream = listener.accept();
    println!("{:?}", stream);
    // stream: Ok((TcpStream { addr: V4(127.0.0.1:23456), peer: V4(127.0.0.1:49866), fd: 4 }, V4(127.0.0.1:49866)))
    // Okの中身はタプル
    http(&stream.expect("fail to accept stream").0);
    /*
    for stream in listener.incoming() {
        http(stream.expect("fail to accept stream"));
    }
    */
}
