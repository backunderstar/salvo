#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate novel;

use novel::{http::Status, response::content};

fn index() -> &'static str {
    "Hello, world!"
}

mod hello_world_tests {
    use super::*;

    use std::io::Read;

    use novel::Route;
    use novel::local::Client;
    use novel::http::{Status, ContentType};
    use novel::response::Body;

    fn routes() -> Vec<Route> {
        routes![index, empty, other]
    }

    #[test]
    fn user_head() {
        let client = Client::new(novel::ignite().mount("/", routes())).unwrap();
        let mut response = client.head("/other").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_empty_sized_body(response.body().unwrap(), 17);

        let content_type: Vec<_> = response.headers().get("Content-Type").collect();
        assert_eq!(content_type, vec![ContentType::JSON.to_string()]);
    }
}
