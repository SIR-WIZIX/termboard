use std::sync::Mutex;
#[macro_use]
extern crate rocket;

static m: Mutex<i32> = Mutex::new(0);

#[get("/hello/<name>")]
async fn greet(name: String) -> String {
    format!("Hello {name}!")
}

#[get("/status")]
async fn status() -> String {
    format!("OK attempt: {}", increase(m.get_mut().unwrap()))
}

fn increase(a: &mut i32) -> &i32 {
    *a = *a + 1;
    a
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![status])
        .mount("/", routes![greet])
}
