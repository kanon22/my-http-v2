use std::io::{self, BufRead, BufWriter, Read, Write};
use std::str;
use std::fs::File;
use std::path::Path;
use std::ffi::OsStr;
use std::env;
use std::net::{TcpListener, TcpStream};

struct Response <'a> {
    resp_line: Option<&'a[u8]>,
    resp_hdr: Option<Vec<&'a[u8]>>,
    msg_body: Option<&'a[u8]>,
}

fn http(stream: &TcpStream) -> Result<(), io::Error> { // 引数には参照を渡す
    let mut bufr = io::BufReader::new(stream);
    let mut bufw = io::BufWriter::new(stream);
    let status_code: u16;
    let resp400 = Response {resp_line: Some(b"HTTP/1.1 400 Bad Request\r\n"), resp_hdr: None, msg_body: None};

    /* リクエストライン・リクエストヘッダの読み込み */
    let mut line = String::new();
    let mut header = Vec::new();
    while bufr.read_line(&mut line)? != 0 {
        if line == "\r\n" {
            break;
        }
        header.push(line);
        line = String::new();
    }
    println!("{:?}", header);

    /* リクエストラインのチェック */
    let mut params = header[0].split_whitespace();
    let method: &str;
    let uri: &str;
    let version: &str;
    if let Some(x) = params.next() {
        method = x;
    } else {
        response(&mut bufw, 400, resp400);
        return Ok(());
    }
    if let Some(x) = params.next() {
        uri = x;
    } else {
        response(&mut bufw, 400, resp400);
        return Ok(());
    }
    if let Some(x) = params.next() {
        version = x;
    } else {
        response(&mut bufw, 400, resp400);
        return Ok(());
    }

    /* TODO: メッセージボディの読み込み */

    // GET の処理
    match method {
        "GET" => {
            status_code = get(&mut bufw, &uri).expect("GET operaton error");
        },
        "HEAD" => {
            status_code = head(&mut bufw, &uri).expect("HEAD operaton error");
        },
        _ => {
            status_code = 501;
            println!("not implemented.");
        },
    }
    println!("{}", status_code);
    Ok(())
}

fn response(bufw: &mut BufWriter<&TcpStream>, status_code: u16, resp: Response) -> Option<()> {
    println!("{:?}", str::from_utf8(resp.resp_line.unwrap()));
    Some(())
}

fn get(bufw: &mut BufWriter<&TcpStream>, uri: &str) -> Result<u16, io::Error> {
    let status_line: &[u8];
    let status_code: u16;
    let file_uri: &str;

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
        bufw.write(b"\r\n")?;
        bufw.flush()?;
        return Ok(status_code);
    }
    let attr = path.metadata()?;
    if !attr.is_file() {
        /* 404 */
        status_line = b"HTTP/1.1 404 Not Found\r\n";
        status_code = 404;
    } else if attr.permissions().readonly() {
        /* 403 */
        status_line = b"HTTP/1.1 403 Forbidden\r\n";
        status_code = 403;
    } else {
        /* 200 */
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
            Some("txt") => content_type = b"Content-Type: text/plain\r\n",
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

    Ok(status_code)
}

fn head(bufw: &mut BufWriter<&TcpStream>, uri: &str) -> Result<u16, io::Error> {
    let status_line: &[u8];
    let status_code: u16;
    let file_uri: &str;

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
        return Ok(status_code);
    }
    let attr = path.metadata()?;
    if !attr.is_file() {
        /* 404 */
        status_line = b"HTTP/1.1 404 Not Found\r\n";
        status_code = 404;
    } else if attr.permissions().readonly() {
        /* 403 */
        status_line = b"HTTP/1.1 403 Forbidden\r\n";
        status_code = 403;
    } else {
        /* 200 */
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
            Some("txt") => content_type = b"Content-Type: text/plain\r\n",
            _ => content_type = b"Content-Type: application/octet-stream\r\n",
        }

        bufw.write(content_type)?;
        bufw.write(b"\r\n")?;
    }
    bufw.flush()?;

    Ok(status_code)
}

fn start_srv(portnum: u16) -> Result<(), io::Error> {
    /* ソケットの作成 */
    let listener = TcpListener::bind(format!("127.0.0.1:{}", portnum))?;

    /* connection を accept して並列に処理したい */
    println!("{:?}", listener);
    for stream in listener.incoming() {
        http(&stream?)?;
    }
    Ok(())
}

fn main() {
    /* コマンドライン引数の読込 */
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("usage: {} <portnumber>", args[0]);
        panic!("invalid arguments number");
    }
    let portnum: u16 = args[1].trim().parse().expect("invalid port number");

    start_srv(portnum).expect("error occured on srv");
}
