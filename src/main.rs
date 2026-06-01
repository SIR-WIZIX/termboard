use std::sync::Mutex;
#[macro_use]
extern crate rocket;

#[get("/hello/<name>")]
async fn greet(name: String) -> String {
    format!("Hello {name}!")
}

#[get("/status")]
async fn status() -> &'static str {
    "OK"
}

fn increase(a: &mut i32) {
    *a = *a + 1;
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![status])
        .mount("/", routes![greet])
}
