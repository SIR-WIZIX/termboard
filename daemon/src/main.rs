use std::ops::Deref;
use std::sync::Mutex;
use chrono;
use common;

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

#[get("/event")]
fn event(event: &rocket::State<Mutex<common::Event>>) -> String {
    let inner_event = event.lock().unwrap();
    let time = inner_event.time.to_string();
    let level = inner_event.level.to_string();
    let text = inner_event.text.clone();
    format!("{0} - level:{1} msg:{2}",
    time,
    level,
    text,
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
    let event: common::Event = common::Event {
    time: chrono::Local::now(),
    text: "OK for now".to_string(),
    level: common::LogLevel::Info,
    };

    let data: MyData = MyData { data: 0, };
    rocket::build()
        .manage(Mutex::new(data))
        .manage(Mutex::new(event))
        .mount("/", routes![status])
        .mount("/", routes![greet])
        .mount("/", routes![event])
}
