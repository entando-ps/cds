/*++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
+ Copyright (c) 2022 Entando SRL.                                                                 +
+ Permission is hereby granted, free of charge, to any person obtaining a copy of this software   +
+ and associated documentation files (the "Software"), to deal in the Software without            +
+ restriction, including without limitation the rights to use, copy, modify, merge, publish,      +
+ distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the   +
+ Software is furnished to do so, subject to the following conditions:                            +
+                                                                                                 +
+ The above copyright notice and this permission notice shall be included in all copies or        +
+ substantial portions of the Software.                                                           +
+                                                                                                 +
+ THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR                      +
+ IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,                        +
+ FITNESS FOR A PARTICULAR PURPOSE AND NON INFRINGEMENT. IN NO EVENT SHALL THE                    +
+ AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER                          +
+ LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,                   +
+ OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE                   +
+ SOFTWARE.                                                                                       +
++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++*/

extern crate core;

mod handlers;
mod utils;

use actix_cors::Cors;
use actix_web::http::{header, Method};
use actix_web::{http, middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use std::env;

use actix_web_middleware_keycloak_auth::{DecodingKey, KeycloakAuth};
use env_logger::Env;

use futures::future;

const INTERNAL_PORT: &str = "8080";
const PUBLIC_PORT: &str = "8081";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // load all env vars
    // uncomment to run locally and comment before creating the docker image.
    // dotenv::dotenv().unwrap();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("Internal sever listening on port: {}", INTERNAL_PORT);
    println!("Public server listening on port: {}", PUBLIC_PORT);

    let internal_server = HttpServer::new(move || {

        let keycloak_auth = KeycloakAuth::default_with_pk(
            DecodingKey::from_rsa_pem(env::var("KEYCLOAK_PUBLIC_KEY").as_ref().unwrap().as_bytes())
                .unwrap(),
        );

        App::new()
            .wrap(middleware::Logger::default())
            .wrap(keycloak_auth)
            .service(handlers::upload)
            .service(handlers::list)
            .service(handlers::delete)
            .service(utils::compress)
            .service(utils::decompress)
            .service(utils::disk_info)
            .service(handlers::index_protected)
    })
    .bind(format!("0.0.0.0:{}", INTERNAL_PORT))?
    .run();

    let public_server = HttpServer::new(move || {
        env::var("CORS_ALLOWED_ORIGIN").expect("$CORS_ALLOWED_ORIGIN is not defined.");
        env::var("CORS_ALLOWED_ORIGIN_END_WITH").expect("$CORS_ALLOWED_ORIGIN_END_WITH is not defined.");
        let cors = Cors::default()
            // .allow_any_origin()
            .allowed_origin(env::var("CORS_ALLOWED_ORIGIN").as_ref().unwrap())
            .send_wildcard()
            .allowed_origin_fn(|origin, _req_head| {
                origin.as_bytes().ends_with(env::var("CORS_ALLOWED_ORIGIN_END_WITH").as_ref().unwrap().as_bytes())
            })
            .allowed_methods(vec!["GET"])
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                header::CONTENT_TYPE,
            ])
            .max_age(3600);

        App::new()
            .wrap(middleware::Logger::exclude(
                middleware::Logger::default(),
                "/health/health_check",
            ))
            .service(
                web::scope("/health").route("/health_check", web::get().to(handlers::health_check)),
            )
            .wrap(cors)
            .service(handlers::index)
            .default_service(web::to(|req: HttpRequest| match *req.method() {
                Method::GET => HttpResponse::Ok(),
                Method::POST => HttpResponse::MethodNotAllowed(),
                _ => HttpResponse::NotFound(),
            }))
    })
    .bind(format!("0.0.0.0:{}", PUBLIC_PORT))?
    .run();

    future::try_join(internal_server, public_server).await?;
    Ok(())
}
