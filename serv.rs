use std::io::{self, BufRead, BufWriter, Read, Write};
use std::fs::{self, File};
use std::path::Path;
use std::ffi::OsStr;
use std::env;
use std::net::{TcpListener, TcpStream};

fn http(stream: &TcpStream) {
    println!("{:?}", stream);
    let mut bufr = io::BufReader::new(stream); // 参照を渡す
    let mut bufw = io::BufWriter::new(stream);
    let mut line = String::new();
    let mut body = [0; 1024];

    /* リクエストラインの読み込み */
    bufr.read_line(&mut line).expect("failed to read first line");
    let mut params = line.split_whitespace();
    let method = params.next().unwrap();
    let uri = params.next().unwrap();
    let version = params.next().unwrap();

    bufr.read(&mut body).expect("failed to read");
    // echo
    //bufw.write(&body).expect("failed to write");
    //bufw.flush().expect("failed to flush"); // flushしないと送られない

    // GET の処理
    match method {
        "GET" => get(&mut bufw, &uri).expect("GET operaton error"),
        _ => println!("not implemented."),
    }
}

fn get(bufw: &mut BufWriter<&TcpStream>, uri: &str) -> Result<(), io::Error> {
    let status_line: &[u8];
    let status_code: i32;
    let file_uri: &str;
    //bufw.write(b"seizon")?;

    if uri == "/" {
        file_uri = "index.html";
    }else{
        file_uri = uri.trim_start_matches("/");
    }

    /* check file existance */
    let path = Path::new(file_uri);
    if !path.exists() {
        status_line = b"HTTP/1.1 404 Not Found\r\n";
        status_code = 404;
        bufw.write(status_line)?;
        bufw.flush()?;
        return Ok(());
    }
    //let attr = fs::metadata(file_uri)?;
    let attr = path.metadata()?;
    if !attr.is_file() {
        // 404
        status_line = b"HTTP/1.1 404 Not Found\r\n";
        status_code = 404;
    } else if attr.permissions().readonly() {
        // 403
        status_line = b"HTTP/1.1 403 Forbidden\r\n";
        status_code = 403;
    } else {
        // 200
        status_line = b"HTTP/1.1 200 OK\r\n";
        status_code = 200;
    }

    bufw.write(status_line)?;

    if status_code == 200 {
        /* detect content-type */
        let content_type: &[u8];
        match path.extension().and_then(OsStr::to_str) {
            Some("html") => content_type = b"Content-Type: text/html\r\n",
            Some("png") | Some("ico") => content_type = b"Content-Type: image/png\r\n",
            Some("jpg") | Some("jpeg") => content_type = b"Content-Type: image/jpeg\r\n",
            _ => content_type = b"Content-Type: application/octet-stream\r\n",
        }

        bufw.write(content_type)?;

        bufw.write(b"\r\n")?;
        let mut buf = [0; 1024];
        let mut reader = io::BufReader::new(File::open(file_uri)?);
        loop {
            match reader.read(&mut buf)? {
                0 => break,
                n => {
                    let buf = &buf[..n];
                    bufw.write(&buf)?;
                }
            }
        }
    }
    bufw.flush()?;

    Ok(())
}

fn main() {
    // コマンドライン引数の読込
    let args: Vec<String> = env::args().collect();
    //println!("{:?}", args); // {:?} で fmt::dbgの出力が得られる
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
    // let stream = listener.accept();
    // stream: Ok((TcpStream { addr: V4(127.0.0.1:23456), peer: V4(127.0.0.1:49866), fd: 4 }, V4(127.0.0.1:49866)))
    // Okの中身はタプル
    // http(&stream.expect("fail to accept stream").0);
    for stream in listener.incoming() {
        http(&stream.expect("fail to accept stream"));
    }
}
