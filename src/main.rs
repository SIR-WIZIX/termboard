use std::ops::Deref;
use std::sync::Mutex;
mod common;

#[macro_use]
extern crate rocket;

#[get("/hello/<name>")]
fn greet(name: String) -> String {
    format!("Hello {name}!")
}

#[get("/status")]
fn status(state: &rocket::State<Mutex<MyData>>) -> String {
    let mut mutable_data = state.lock().unwrap();
    let out: i32;
    (*mutable_data, out) = increase(mutable_data.deref());
    print!(
        "status called on thread {:?}\r\n",
        std::thread::current().id()
    );
    format!(
        "OK attempt: {0} on thread: {1:?}\r\n",
        out,
        std::thread::current().id()
    )
}

fn increase(a: &MyData) -> (MyData, i32) {
    let md = MyData { data: a.data + 1 };
    let num: i32 = md.data;
    (md, num)
}

struct MyData {
    data: i32,
}

#[launch]
fn rocket() -> _ {
    let data: MyData = MyData { data: 0 };
    rocket::build()
        .manage(Mutex::new(data))
        .mount("/", routes![status])
        .mount("/", routes![greet])
}
