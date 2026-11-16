use chrono;
use common;
use std::sync::{Arc, Mutex};

mod collector;
mod scheduler;

#[macro_use]
extern crate rocket;

pub enum SchedulerCommand {
    SetMode(scheduler::MonitoringMode),
    ForceRefreshUsers,
    ForceRefreshHost,
    ForceRefreshProcesses,
    UpdateSystemSnapshot,
    UpdateHostSnapshot,
    Stop,
}

#[get("/hello/<name>")]
fn greet(name: String) -> String {
    format!("Hello {name}!")
}
#[cfg(feature = "debug-tools")]
#[get("/debug/snapshot")]
fn snapshot(collector: &rocket::State<Mutex<collector::SystemCollector>>) -> String {
    let mut system_collector = collector.lock().unwrap();

    let _ = system_collector.deref_mut().refresh_host();
    let _ = system_collector.deref_mut().refresh_users();
    let _ = system_collector.deref_mut().refresh_processes_default();
    let snapshot = system_collector.deref_mut().full_snapshot();
    std::mem::drop(system_collector);
    format!("{snapshot:#?}")
}
#[cfg(feature = "debug-tools")]
#[get("/debug/tick")]
fn tick() -> String {
    let mut scheduler = scheduler::Scheduler::default();

    let spare = scheduler.tick();

    let s1 = scheduler.get_collector().full_snapshot();

    sleep(spare);

    let _ = scheduler.tick();

    let s2 = scheduler.get_collector().full_snapshot();

    format!("{spare:?}\n{s1:#?} \n{s2:#?}")
}
#[cfg(feature = "debug-tools")]
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
    std::mem::drop(system_collector);
    format!(
        "full snapshot time - {0}\nrefresh host time - {1}\nrefresh users time - {2}\nrefresh processes time - {3}",
        full_snapshot_time.unwrap_or_default().as_nanos(),
        refresh_host_time.unwrap_or_default().as_nanos(),
        refresh_users_time.unwrap_or_default().as_nanos(),
        refresh_processes_time.unwrap_or_default().as_nanos(),
    )
}
#[cfg(feature = "debug-tools")]
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

#[cfg(feature = "debug-tools")]
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

#[get("/event")]
fn event(event: &rocket::State<Mutex<common::Event>>) -> String {
    let inner_event = event.lock().unwrap();
    let time = inner_event.time.to_string();
    let level = inner_event.level.to_string();
    let text = inner_event.text.clone();
    format!("{0} - level:{1} msg:{2}", time, level, text,)
}

#[launch]
fn rocket() -> _ {
    let event: common::Event = common::Event {
        time: chrono::Local::now(),
        text: "OK for now".to_string(),
        level: common::LogLevel::Info,
    };

    let mut scheduler = scheduler::Scheduler::default();
    let system_snapshot_slot =
        std::sync::Arc::new(Mutex::new(Arc::new(None::<common::SystemSnapshot>)));
    let host_snapshot_slot =
        std::sync::Arc::new(Mutex::new(Arc::new(None::<common::HostSnapshot>)));
    let (tx, rx) = std::sync::mpsc::channel();

    let worker_system_snapshot_slot = Arc::clone(&system_snapshot_slot);
    let worker_host_snapshot_slot = Arc::clone(&host_snapshot_slot);
    let worker = std::thread::spawn(move || {
        let mut timeout = scheduler.tick();
        loop {
            match rx.recv_timeout(timeout) {
                Ok(SchedulerCommand::SetMode(mode)) => {
                    scheduler.set_operation_mode(mode);
                    timeout = scheduler.tick();
                }
                Ok(SchedulerCommand::Stop) => {
                    break;
                }
                Ok(SchedulerCommand::ForceRefreshUsers) => {
                    scheduler.force_refresh_users();
                }
                Ok(SchedulerCommand::ForceRefreshHost) => {
                    scheduler.force_refresh_host();
                }
                Ok(SchedulerCommand::ForceRefreshProcesses) => {
                    scheduler.force_refresh_processes();
                }
                Ok(SchedulerCommand::UpdateSystemSnapshot) => {
                    *worker_system_snapshot_slot.lock().unwrap() =
                        Arc::new(Some(scheduler.get_collector().full_snapshot()));
                }
                Ok(SchedulerCommand::UpdateHostSnapshot) => {
                    *worker_host_snapshot_slot.lock().unwrap() =
                        Arc::new(Some(scheduler.get_collector().host_snapshot()));
                }
                Err(_) => {
                    timeout = scheduler.tick();
                }
            }
        }
    });

    //tx.send(SchedulerCommand::Stop);

    let collector = collector::SystemCollector::new();
    let rocket = rocket::build()
        .manage(Mutex::new(event))
        .manage(Mutex::new(collector))
        .manage(system_snapshot_slot)
        .manage(host_snapshot_slot)
        .manage(tx)
        .mount("/", routes![greet])
        .mount("/", routes![event]);

    #[cfg(feature = "debug-tools")]
    let rocket = rocket.mount(
        "/",
        routes![snapshot, benchmark, benchmark_series, benchmark_users, tick],
    );

    rocket
}
