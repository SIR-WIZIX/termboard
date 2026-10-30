use chrono;
use common;
use std::ops::{Deref, DerefMut};
use std::sync::Mutex;
mod collector;

#[macro_use]
extern crate rocket;

#[get("/hello/<name>")]
fn greet(name: String) -> String {
    format!("Hello {name}!")
}

#[get("/debug/snapshot")]
fn snapshot(collector: &rocket::State<Mutex<collector::SystemCollector>>) -> String {
    let mut system_collector = collector.lock().unwrap();

    let _ = system_collector.deref_mut().refresh_host();
    let _ = system_collector.deref_mut().refresh_users();
    let _ = system_collector.deref_mut().refresh_processes_default();
    let snapshot = system_collector.deref_mut().full_snapshot();
    format!("{snapshot:#?}")
}

#[get("/debug/bench")]
fn benchmark(collector: &rocket::State<Mutex<collector::SystemCollector>>) -> String {
    let mut system_collector = collector.lock().unwrap();

    let start_time = std::time::SystemTime::now();
    let _ = system_collector.deref_mut().refresh_host();
    let _ = system_collector.deref_mut().refresh_users();
    let _ = system_collector.deref_mut().refresh_processes_default();
    let _ = system_collector.deref_mut().full_snapshot();
    let full_snapshot_time = std::time::SystemTime::elapsed(&start_time);
    let start_time = std::time::SystemTime::now();
    let _ = system_collector.deref_mut().refresh_host();
    let refresh_host_time = std::time::SystemTime::elapsed(&start_time);
    let start_time = std::time::SystemTime::now();
    let _ = system_collector.deref_mut().refresh_users();
    let refresh_users_time = std::time::SystemTime::elapsed(&start_time);
    let start_time = std::time::SystemTime::now();
    let _ = system_collector.deref_mut().refresh_processes_default();
    let refresh_processes_time = std::time::SystemTime::elapsed(&start_time);
    format!(
        "full snapshot time - {0}\nrefresh host time - {1}\nrefresh users time - {2}\nrefresh processes time - {3}",
        full_snapshot_time.unwrap_or_default().as_nanos(),
        refresh_host_time.unwrap_or_default().as_nanos(),
        refresh_users_time.unwrap_or_default().as_nanos(),
        refresh_processes_time.unwrap_or_default().as_nanos(),
    )
}

#[get("/debug/bench/<times>")]
fn benchmark_series(
    times: u32,
    collector: &rocket::State<Mutex<collector::SystemCollector>>,
) -> String {
    let mut summary_refresh_processes_time = std::time::Duration::from_secs(0);
    let mut summary_full_snapshot_time = std::time::Duration::from_secs(0);
    let mut summary_refresh_host_time = std::time::Duration::from_secs(0);
    let mut summary_refresh_users_time = std::time::Duration::from_secs(0);
    let mut system_collector = collector.lock().unwrap();

    for _ in 0..times {
        let start_time = std::time::SystemTime::now();
        let _ = system_collector.deref_mut().refresh_host();
        let _ = system_collector.deref_mut().refresh_users();
        let _ = system_collector.deref_mut().refresh_processes_default();
        let _ = system_collector.deref_mut().full_snapshot();
        summary_full_snapshot_time += std::time::SystemTime::elapsed(&start_time).unwrap();
        let start_time = std::time::SystemTime::now();
        let _ = system_collector.deref_mut().refresh_host();
        summary_refresh_host_time += std::time::SystemTime::elapsed(&start_time).unwrap();
        let start_time = std::time::SystemTime::now();
        let _ = system_collector.deref_mut().refresh_users();
        summary_refresh_users_time += std::time::SystemTime::elapsed(&start_time).unwrap();
        let start_time = std::time::SystemTime::now();
        let _ = system_collector.deref_mut().refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::nothing()
                .without_tasks()
                .with_cpu()
                .with_user(sysinfo::UpdateKind::Always)
                .with_memory()
                .with_cmd(sysinfo::UpdateKind::Always)
                .with_exe(sysinfo::UpdateKind::Always),
        );

        summary_refresh_processes_time += std::time::SystemTime::elapsed(&start_time).unwrap();
    }
    format!(
        "full snapshot time - {0}\nrefresh host time - {1}\nrefresh users time - {2}\nrefresh processes time - {3}",
        summary_full_snapshot_time.as_nanos() / times as u128,
        summary_refresh_host_time.as_nanos() / times as u128,
        summary_refresh_users_time.as_nanos() / times as u128,
        summary_refresh_processes_time.as_nanos() / times as u128,
    )
}

#[get("/debug/bench/users/<times>")]
fn benchmark_users(
    times: u32,
    collector: &rocket::State<Mutex<collector::SystemCollector>>,
) -> String {
    let mut summary_refresh_users_time = std::time::Duration::from_secs(0);
    let mut system_collector = collector.lock().unwrap();

    for _ in 0..times {
        let start_time = std::time::SystemTime::now();
        let _ = system_collector.deref_mut().refresh_users();
        summary_refresh_users_time += std::time::SystemTime::elapsed(&start_time).unwrap();
    }
    format!(
        "users refresh time - {0}",
        summary_refresh_users_time.as_nanos() / times as u128,
    )
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
    format!("{0} - level:{1} msg:{2}", time, level, text,)
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

    let data: MyData = MyData { data: 0 };
    let collector = collector::SystemCollector::new();
    rocket::build()
        .manage(Mutex::new(data))
        .manage(Mutex::new(event))
        .manage(Mutex::new(collector))
        .mount("/", routes![status])
        .mount("/", routes![greet])
        .mount("/", routes![event])
        .mount("/", routes![snapshot])
        .mount("/", routes![benchmark])
        .mount("/", routes![benchmark_series])
        .mount("/", routes![benchmark_users])
}
