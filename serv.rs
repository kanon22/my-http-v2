//use std::io::{self, BufRead, BufWriter, Read, Write, Error, ErrorKind};
use std::io::{self, BufRead, BufWriter, Read, Write, ErrorKind};
use std::io::Error as IoError;
use std::error::Error;
use std::fmt;
use std::str;
use std::string::String;
use std::num;
use std::fs::File;
use std::path::Path;
use std::ffi::OsStr;
use std::env;
use std::net::{TcpListener, TcpStream};

const BUF_SIZE: usize = 4;

/* 独自のエラー型を定義 */
#[derive(Debug)]
enum HTTPError {
    Io(io::Error),
    Parse(num::ParseIntError),
}

/* Display トレイトを実装 */
impl fmt::Display for HTTPError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            HTTPError::Io(ref err) => write!(f, "IO error: {}", err),
            HTTPError::Parse(ref err) => write!(f, "Parse error: {}", err),
        }
    }
}

/* Error トレイトを実装 */
impl Error for HTTPError {
    fn description(&self) -> &str {
        match *self {
            HTTPError::Io(ref err) => err.description(),
            HTTPError::Parse(ref err) => err.description(),
        }
    }
    // エラーが発生した lower-levelの原因をcauseに任意で実装できる
    //fn cause(&self) -> Option<&Error>
}

/* From トレイトを実装 */
impl From<io::Error> for HTTPError {
    fn from(err: io::Error) -> HTTPError {
        HTTPError::Io(err)
    }
}

impl From<num::ParseIntError> for HTTPError {
    fn from(err: num::ParseIntError) -> HTTPError {
        HTTPError::Parse(err)
    }
}

struct Response <'a> {
    resp_line: &'a [u8],
    resp_hdr: Option<Vec<&'a [u8]>>,
    msg_body: Option<MessageBody<'a>>, // MessageBodyのライフタイムを明示する
}

enum MessageBody <'a> {
    Bytes(&'a [u8]),
    Uri(&'a str),
}

struct RequestLine <'a> {
    method: &'a str,
    uri: &'a str,
    version: &'a str,
}

//fn http(stream: &TcpStream) -> Result<(), io::Error> { // 引数には参照を渡す
fn http(stream: &TcpStream) -> Result<(), HTTPError> { // 引数には参照を渡す
    let mut bufr = io::BufReader::new(stream);
    let mut bufw = io::BufWriter::new(stream);
    let rl: RequestLine;
    let status_code: u16;
    let resp400 = Response {
        resp_line: b"HTTP/1.1 400 Bad Request\r\n",
        resp_hdr: None,
        msg_body: None,
    };
    let resp405 = Response {
        resp_line: b"HTTP/1.1 405 Method Not Allowed\r\n",
        resp_hdr: Some(vec!(b"Allow: GET, HEAD")),
        msg_body: None,
    };
    let resp411 = Response {
        resp_line: b"HTTP/1.1 411 Length Required\r\n",
        resp_hdr: None,
        msg_body: None,
    };
    let resp501 = Response {
        resp_line: b"HTTP/1.1 501 Not Implementend\r\n",
        resp_hdr: None,
        msg_body: None,
    };

    /* リクエストライン・リクエストヘッダの読み込み */
    let mut line = String::new();
    let mut lines = Vec::new();
    while bufr.read_line(&mut line)? != 0 {
        if line == "\r\n" {
            break;
        }
        lines.push(line);
        line = String::new();
    }
    println!("{:?}", lines);

    /* リクエストラインのチェック */
    match check_req_line(&lines[0]) {
        Ok(x) => rl = x,
        Err(x) => {
            println!("{:?}", x);
            send_response(&mut bufw, resp400)?;
            return Ok(());
        },
    }

    /* ヘッダのチェック */
    let mut header: Vec<Vec<&str>> = Vec::new();
    for hdr in &lines[1..] {
        header.push(hdr.trim_end_matches("\r\n")
                    .splitn(2, ":").map(|x| x.trim()).collect());
    }
    //println!("{:?}", header);

    if rl.method == "POST" {
        if let Some(hdr) = header.iter().find(|hdr| hdr[0] == "Content-Length") {
            let content_length: usize = hdr[1].parse()?;
            println!("content_length: {}", content_length);

            /* TODO: メッセージボディの読み込み */
            let msg_body = bufr.fill_buf().unwrap();
            let bodylen: usize = msg_body.len();
            println!("msg_body: {:?}", msg_body);
            if bodylen != content_length {
                bufr.consume(bodylen);
                send_response(&mut bufw, resp400)?;
                return Ok(());
            }
            if let Ok(params) = str::from_utf8(msg_body) {
                println!("params: {}", params);
            }
            bufr.consume(bodylen);

        } else {
            send_response(&mut bufw, resp411)?;
            //status_code = 411;
            return Ok(());
        }
    }

    // method ごとに処理を呼び出す
    match rl.method {
        "GET" => {
            // ヘッダ生成まではHEADと同じ
            //status_code = head(&mut bufw, &rl.method, &rl.uri).expect("GET operaton error");
            match head(&mut bufw, &rl.method, &rl.uri) {
                Ok(x) => status_code = x,
                Err(x) => {
                    println!("{:?}", x);
                    return Ok(());
                },
            }
        },
        "HEAD" => {
            status_code = head(&mut bufw, &rl.method, &rl.uri).expect("HEAD operaton error");
        },
        "poyo" => {
            send_response(&mut bufw, resp400)?;
            status_code = 400;
        },
        "OPTIONS" | "POST" | "PUT" | "DELETE" | "TRACE" | "CONNECT" => {
            send_response(&mut bufw, resp501)?;
            status_code = 501;
        },
        _ => {
            send_response(&mut bufw, resp405)?;
            status_code = 405;
        },
    }
    println!("{}", status_code);
    Ok(())
}

/* リクエストラインのチェック */
// ライフタイムは謎
fn check_req_line<'a>(req_line: &'a str) -> Result<RequestLine<'a>, io::Error> {
    let params: Vec<&str> = req_line.split_whitespace().collect();
    let req: RequestLine;

    if params.len() == 3 {
        req = RequestLine {
            method: params[0],
            uri: params[1],
            version: params[2],
        };
    } else {
        return Err(IoError::new(ErrorKind::Other, "invalid request line"));
    }
    Ok(req)
}

fn send_response(bufw: &mut BufWriter<&TcpStream>, resp: Response) -> Result<(), io::Error> {
    println!("{:?}", str::from_utf8(resp.resp_line));
    bufw.write(resp.resp_line)?;
    if let Some(vec) = resp.resp_hdr {
        for hdr in vec {
            bufw.write(hdr)?;
        }
    }
    bufw.write(b"\r\n")?;
    if let Some(MessageBody::Bytes(body)) = resp.msg_body {
        //bufw.write(b"\r\n")?;
        bufw.write(body)?;
    } else if let Some(MessageBody::Uri(uri)) = resp.msg_body {
        send_file(bufw, uri)?;
    }
    bufw.flush()?;
    Ok(())
}

fn send_file(bufw: &mut BufWriter<&TcpStream>, uri: &str) -> Result<(), io::Error> {
    let mut buf = [0; 1024];
    let mut reader = io::BufReader::new(File::open(uri)?);

    //bufw.write(b"\r\n")?;
    loop {
        match reader.read(&mut buf)? {
            0 => break,
            n => {
                let buf = &buf[..n];
                bufw.write(&buf)?;
            }
        }
    }
    bufw.flush()?;
    Ok(())
}

fn head(bufw: &mut BufWriter<&TcpStream>, method: &str, query_uri: &str) -> Result<u16, io::Error> {
    let mut resp = Response {
        resp_line: b"status line shold be here",
        resp_hdr: None,
        msg_body: None,
    };
    let mut header: Vec<&[u8]> = Vec::new();
    let status_code: u16;
    let file_uri: &str;
    let mut query: Vec<(&str, &str)> = Vec::new();

    let uq: Vec::<&str> = query_uri.split("?").collect();
    match uq.len() {
        1 => {
            let uri: &str = uq[0];
            if uri == "/" {
                file_uri = "index.html";
            } else {
                file_uri = uri.trim_start_matches("/");
            }
        },
        2 => {
            let (uri, qry): (&str, &str) = (uq[0], uq[1]);
            if uri == "/" {
                file_uri = "index.html";
            } else {
                file_uri = uri.trim_start_matches("/");
            }
            /* クエリの変数名と値を格納 */
            for pair in qry.split("&") {
                let q: Vec<&str> = pair.split("=").collect();
                if q.len() == 2 {
                    query.push((q[0], q[1]));
                } else {
                    resp.resp_line = b"HTTP/1.1 400 Bad Request\r\n";
                    send_response(bufw, resp)?;
                    //return Err(Error::new(ErrorKind::Other, "invalid query"));
                    return Err(IoError::new(ErrorKind::Other, "invalid query"));
                }
            }
        },
        _ => {
            resp.resp_line = b"HTTP/1.1 400 Bad Request\r\n";
            send_response(bufw, resp)?;
            return Err(IoError::new(ErrorKind::Other, "invalid query_uri"));
        },
    }
    println!("{:?}", query);

    /* check file existance */
    let path = Path::new(file_uri);
    if !path.exists() {
        resp.resp_line = b"HTTP/1.1 404 Not Found\r\n";
        status_code = 404;
        send_response(bufw, resp)?;
        return Ok(status_code);
    }
    let attr = path.metadata()?;
    if !attr.is_file() {
        /* 404 */
        resp.resp_line = b"HTTP/1.1 404 Not Found\r\n";
        status_code = 404;
    } else if attr.permissions().readonly() {
        /* 403 */
        resp.resp_line = b"HTTP/1.1 403 Forbidden\r\n";
        status_code = 403;
    } else {
        /* 200 */
        resp.resp_line = b"HTTP/1.1 200 OK\r\n";
        status_code = 200;
    }

    if status_code == 200 {
        /* detect content-type */
        let content_type: &[u8];
        match path.extension().and_then(OsStr::to_str) {
            Some("html") =>
                content_type = b"Content-Type: text/html\r\n",
            Some("png") | Some("ico") =>
                content_type = b"Content-Type: image/png\r\n",
            Some("jpg") | Some("jpeg") =>
                content_type = b"Content-Type: image/jpeg\r\n",
            Some("txt") =>
                content_type = b"Content-Type: text/plain\r\n",
            _ =>
                content_type = b"Content-Type: application/octet-stream\r\n",
        }
        header.push(content_type);
    }
    resp.resp_hdr = Some(header);

    /* status_codeが決まった後, method ごとに処理を呼び出す */
    match method {
        "GET" => {
            if status_code == 200 {
                resp.msg_body = Some(MessageBody::Uri(file_uri));
            }
            send_response(bufw, resp)?;
        },
        "HEAD" => send_response(bufw, resp)?,
        _ => return Err(IoError::new(ErrorKind::Other, "Unknown method")),
    }

    Ok(status_code)
}

//fn start_srv(portnum: u16) -> Result<(), io::Error> {
fn start_srv(portnum: u16) -> Result<(), HTTPError> {
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
