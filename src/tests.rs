#[cfg(test)]
use super::api;
#[cfg(test)]
use hyper::service::{make_service_fn, service_fn};
#[cfg(test)]
use reqwest::blocking::{multipart::Form, Client};
#[cfg(test)]
use std::{fs::read, path::Path};
#[cfg(test)]
use tokio::runtime::Runtime;

#[test]
fn get_img() {
    let _server = new_server(3000);
    let img = read(root().join("images").join("img_thumb.jpeg")).expect("read img");
    let mut resp = Client::new()
        .get("http://localhost:3000/images/img_thumb.jpeg")
        .send()
        .expect("request");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let mut got = Vec::new();
    resp.copy_to(&mut got).expect("copy bytes");
    assert_eq!(img, got);
}

#[test]
fn store_json_base64_img() {
    let port = 3001;
    let _server = new_server(port);
    let filename = "test_json_base64.jpeg";
    let img = read(root().join("images").join("img.png")).expect("read img");
    store_json_img(port, filename, api::ImageData::Base64(base64::encode(&img)));
    check_file(filename);
}

#[test]
fn store_json_remote_img() {
    let port = 3002;
    let _server = new_server(port);
    let filename = "test_json_remote.jpeg";
    let u = "https://s3.amazonaws.com/media-p.slid.es/uploads/nercury/images/1236480/logo-v2.png";
    store_json_img(port, filename, api::ImageData::URI(u.to_string()));
    check_file(filename);
}

#[test]
fn store_multipart_form_img() {
    let port = 3003;
    let _server = new_server(port);
    let filename = "test_multipart_form.jpeg";
    let img_path = root().join("images/img.png");
    let form = Form::new().file(filename, img_path).expect("form");
    let resp = Client::new()
        .post(&format!("http://localhost:{}/images", port))
        .multipart(form)
        .send()
        .expect("request");
    check_img_resp(filename, resp);
    check_file(filename);
}

#[cfg(test)]
fn store_json_img(port: u16, name: &str, data: api::ImageData) {
    let img = api::ImageRequest {
        filename: name.to_string(),
        data: data,
    };
    let req_body = api::StoreImgRequestBody(vec![img]);
    let json = serde_json::ser::to_vec(&req_body).expect("serialize request");

    let resp = Client::new()
        .post(&format!("http://localhost:{}/images", port))
        .header("Content-Type", "application/json")
        .body(json)
        .send()
        .expect("request");

    check_img_resp(name, resp);
}

#[cfg(test)]
fn check_img_resp(name: &str, resp: reqwest::blocking::Response) {
    assert_eq!(resp.status(), reqwest::StatusCode::CREATED);
    let resp_text = resp.text().expect("response text");
    let resp_body: api::StoreImgResponseBody =
        serde_json::de::from_str(&resp_text).expect("deserialize resp");
    let img = api::ImageResponse::new(name.to_string());
    let expected_resp_body = api::StoreImgResponseBody(vec![img]);
    assert_eq!(resp_body, expected_resp_body);
}

#[cfg(test)]
fn check_file(name: &str) {
    let expected = read(root().join("images/img_thumb.jpeg")).expect("read img");
    let got = read(root().join("images").join(name)).expect("read test img");
    assert_eq!(expected, got);
}

#[cfg(test)]
fn new_server(port: u16) -> Runtime {
    let rt = Runtime::new().expect("make runtime");
    rt.spawn(async move {
        let addr = ([0, 0, 0, 0], port).into();
        let svc = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(api::svc)) });
        hyper::Server::bind(&addr).serve(svc).await.expect("server");
    });
    rt
}

#[cfg(test)]
fn root<'a>() -> &'a Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
