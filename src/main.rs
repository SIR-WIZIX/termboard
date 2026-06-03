use std::{
    ops::{Deref, DerefMut},
    sync::Mutex,
};

use rocket::State;
#[macro_use]
extern crate rocket;

#[get("/hello/<name>")]
async fn greet(name: String) -> String {
    format!("Hello {name}!")
}

#[get("/status")]
async fn status(data: &rocket::State<MyData>) -> String {
    let out: i32;
    (data, out) = increase(*data);
    format!("OK attempt: {}", out)
}

fn increase(a: rocket::State<MyData>) -> (rocket::State<MyData>, i32) {
    let mut md = MyData {
        data: Mutex::new(a.data.into_inner().unwrap() + 1),
    };
    let num: i32 = *(md.data.get_mut()).unwrap();
    (md, num)
}

struct MyData {
    data: Mutex<i32>,
}

#[launch]
fn rocket() -> _ {
    let data: MyData = MyData {
        data: Mutex::new(0),
    };
    rocket::build()
        .manage(data)
        .mount("/", routes![status])
        .mount("/", routes![greet])
}
