use core::fmt;
use std::{
    collections::HashMap,
    error::Error,
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender},
    },
    thread::JoinHandle,
};

use common::{HostSnapshot, SystemSnapshot};
use rocket::tokio::sync::AcquireError;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};

use crate::{
    collector,
    scheduler::{
        SchedulerConfigurationError::MissingSchedule,
        SchedulerStartError::{Configuration, ThreadSpawn},
    },
    system_snapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MonitoringMode {
    Background,
    Interactive,
    Incident,
}

pub enum SchedulerCommand {
    SetMode(MonitoringMode),
    ForceRefreshUsers,
    ForceRefreshHost,
    ForceRefreshProcesses,
    UpdateSystemSnapshot,
    UpdateHostSnapshot,
    Stop,
}

#[derive(Debug)]
pub enum SchedulerConfigurationError {
    MissingSchedule(MonitoringMode),
    InvalidHostInterval(MonitoringMode),
    InvalidUsersInterval(MonitoringMode),
    InvalidProcessesInterval(MonitoringMode),
}

impl std::fmt::Display for SchedulerConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
impl std::error::Error for SchedulerConfigurationError {}

#[derive(Debug)]
pub enum SchedulerStartError {
    Configuration(SchedulerConfigurationError),
    ThreadSpawn(std::io::Error),
}
impl std::error::Error for SchedulerStartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Configuration(err) => Some(err),
            Self::ThreadSpawn(err) => Some(err),
        }
    }
}
impl std::fmt::Display for SchedulerStartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl From<SchedulerConfigurationError> for SchedulerStartError {
    fn from(err: SchedulerConfigurationError) -> Self {
        Self::Configuration(err)
    }
}
impl From<std::io::Error> for SchedulerStartError {
    fn from(err: std::io::Error) -> Self {
        Self::ThreadSpawn(err)
    }
}

type SnapshotSlot<T> = Arc<Mutex<Option<Arc<T>>>>;

pub struct SchedulerHandle {
    pub tx: Sender<SchedulerCommand>,
    pub worker: JoinHandle<()>,
    pub system_snapshot: SnapshotSlot<common::SystemSnapshot>,
    pub host_snapshot: SnapshotSlot<common::HostSnapshot>,
}

#[derive(Debug, Clone, Copy)]
struct Schedule {
    host_update_interval: std::time::Duration,
    users_update_interval: std::time::Duration,
    processes_update_interval: std::time::Duration,

    process_refresh_kind: sysinfo::ProcessRefreshKind,
    remove_dead_processes: bool,
}

impl Default for Schedule {
    fn default() -> Schedule {
        Schedule {
            host_update_interval: std::time::Duration::from_secs(2),
            users_update_interval: std::time::Duration::from_mins(5),
            processes_update_interval: std::time::Duration::from_secs(30),

            process_refresh_kind: sysinfo::ProcessRefreshKind::nothing()
                .without_tasks()
                .with_cpu()
                .with_user(sysinfo::UpdateKind::Always)
                .with_memory()
                .with_cmd(sysinfo::UpdateKind::Always)
                .with_exe(sysinfo::UpdateKind::Always),
            remove_dead_processes: true,
        }
    }
}

impl Default for Schedules {
    fn default() -> Schedules {
        let mut hm = HashMap::new();
        hm.insert(MonitoringMode::Background, Schedule::default());
        hm.insert(
            MonitoringMode::Interactive,
            Schedule {
                host_update_interval: std::time::Duration::from_secs(1),
                users_update_interval: std::time::Duration::from_mins(5),
                processes_update_interval: std::time::Duration::from_secs(5),

                remove_dead_processes: true,
                ..Schedule::default()
            },
        );
        hm.insert(
            MonitoringMode::Incident,
            Schedule {
                host_update_interval: std::time::Duration::from_secs(1),
                users_update_interval: std::time::Duration::from_mins(1),
                processes_update_interval: std::time::Duration::from_secs(2),

                remove_dead_processes: false,
                ..Schedule::default()
            },
        );
        Schedules { behaviours: hm }
    }
}

pub struct Schedules {
    behaviours: HashMap<MonitoringMode, Schedule>,
}
impl Schedules {
    fn validate(&self) -> Result<(), SchedulerConfigurationError> {
        for mode in [
            MonitoringMode::Background,
            MonitoringMode::Interactive,
            MonitoringMode::Incident,
        ] {
            let schedule = self.behaviours.get(&mode).ok_or(MissingSchedule(mode))?;
            if schedule.host_update_interval.is_zero() {
                return Err(SchedulerConfigurationError::InvalidHostInterval(mode));
            }
            if schedule.users_update_interval.is_zero() {
                return Err(SchedulerConfigurationError::InvalidUsersInterval(mode));
            }
            if schedule.processes_update_interval.is_zero() {
                return Err(SchedulerConfigurationError::InvalidProcessesInterval(mode));
            }
        }
        return Ok(());
    }
}

pub struct Scheduler {
    last_host_refresh_request: Option<std::time::Instant>,
    last_users_refresh_request: Option<std::time::Instant>,
    last_processes_refresh_request: Option<std::time::Instant>,
    last_host_refresh: Option<std::time::Instant>,
    last_users_refresh: Option<std::time::Instant>,
    last_processes_refresh: Option<std::time::Instant>,
    system_collector: collector::SystemCollector,

    operation_mode: MonitoringMode,

    schedules: Schedules,
}

impl Default for Scheduler {
    fn default() -> Self {
        Scheduler {
            last_host_refresh_request: None,
            last_users_refresh_request: None,
            last_processes_refresh_request: None,
            last_host_refresh: None,
            last_users_refresh: None,
            last_processes_refresh: None,
            system_collector: collector::SystemCollector::new(),

            operation_mode: MonitoringMode::Background,

            schedules: Schedules::default(),
        }
    }
}

impl Scheduler {
    pub fn new(collector: collector::SystemCollector, schedules: Schedules) -> Scheduler {
        Self {
            last_host_refresh_request: Option::None,
            last_users_refresh_request: Option::None,
            last_processes_refresh_request: Option::None,
            last_host_refresh: Option::None,
            last_users_refresh: Option::None,
            last_processes_refresh: Option::None,
            system_collector: collector,

            operation_mode: MonitoringMode::Background,

            schedules: schedules,
        }
    }

    fn run(
        &mut self,
        rx: Receiver<SchedulerCommand>,
        worker_system_snapshot_slot: SnapshotSlot<SystemSnapshot>,
        worker_host_snapshot_slot: SnapshotSlot<HostSnapshot>,
    ) {
        let mut timeout = self.tick();
        let mut next_due = std::time::Instant::now() + timeout;
        loop {
            match rx.recv_timeout(timeout) {
                Ok(SchedulerCommand::SetMode(mode)) => {
                    self.set_operation_mode(mode);
                    timeout = self.tick();
                    next_due = std::time::Instant::now() + timeout;
                }
                Ok(SchedulerCommand::Stop) => {
                    break;
                }
                Ok(SchedulerCommand::ForceRefreshUsers) => {
                    self.force_refresh_users();
                }
                Ok(SchedulerCommand::ForceRefreshHost) => {
                    self.force_refresh_host();
                }
                Ok(SchedulerCommand::ForceRefreshProcesses) => {
                    self.force_refresh_processes();
                }
                Ok(SchedulerCommand::UpdateSystemSnapshot) => {
                    *worker_system_snapshot_slot.lock().unwrap() =
                        Some(Arc::new(self.get_collector().full_snapshot()));
                }
                Ok(SchedulerCommand::UpdateHostSnapshot) => {
                    *worker_host_snapshot_slot.lock().unwrap() =
                        Some(Arc::new(self.get_collector().host_snapshot()));
                }
                Err(_) => {
                    timeout = self.tick();
                    next_due = std::time::Instant::now() + timeout;
                }
            }
            timeout = next_due - std::time::Instant::now();
            println!("{:?} - timeout at {:?}", timeout, std::time::Instant::now())
        }
    }

    pub fn start(mut self) -> Result<SchedulerHandle, SchedulerStartError> {
        self.schedules.validate()?;
        let system_snapshot_slot: SnapshotSlot<SystemSnapshot> =
            std::sync::Arc::new(Mutex::new(None));
        let host_snapshot_slot: SnapshotSlot<HostSnapshot> = std::sync::Arc::new(Mutex::new(None));
        let (tx, rx) = std::sync::mpsc::channel();

        let worker_system_snapshot_slot = Arc::clone(&system_snapshot_slot);
        let worker_host_snapshot_slot = Arc::clone(&host_snapshot_slot);

        let worker = std::thread::Builder::new()
            .name("termboard-scheduler".into())
            .spawn(move || self.run(rx, worker_system_snapshot_slot, worker_host_snapshot_slot))?;

        Ok(SchedulerHandle {
            tx: tx,
            worker: worker,
            system_snapshot: system_snapshot_slot,
            host_snapshot: host_snapshot_slot,
        })
    }

    //TODO remove public setter in order for handeling possible data losses while switching from
    //Incident
    fn set_operation_mode(&mut self, operation_mode: MonitoringMode) {
        self.operation_mode = operation_mode;
    }

    //TODO same for removal
    pub fn set_schedules(&mut self, schedules: Schedules) {
        self.schedules = schedules;
    }

    pub fn tick(&mut self) -> std::time::Duration {
        let schedule = self.schedules.behaviours[&self.operation_mode];
        match self.last_host_refresh {
            Option::None => {
                self.refresh_host();
            }
            Option::Some(last_host_refresh) => {
                if last_host_refresh.elapsed() >= schedule.host_update_interval {
                    self.refresh_host();
                }
            }
        }

        match self.last_users_refresh {
            Option::None => {
                self.refresh_users();
            }
            Option::Some(last_users_refresh) => {
                if last_users_refresh.elapsed() >= schedule.users_update_interval {
                    self.refresh_users();
                }
            }
        }

        match self.last_processes_refresh {
            Option::None => {
                self.refresh_processes_specifics(
                    ProcessesToUpdate::All,
                    schedule.remove_dead_processes,
                    schedule.process_refresh_kind,
                );
            }
            Option::Some(last_processes_refresh) => {
                if last_processes_refresh.elapsed() >= schedule.processes_update_interval {
                    self.refresh_processes_specifics(
                        ProcessesToUpdate::All,
                        schedule.remove_dead_processes,
                        schedule.process_refresh_kind,
                    );
                }
            }
        }

        std::cmp::min(
            schedule.host_update_interval.saturating_sub(
                self.last_host_refresh
                    .unwrap_or(std::time::Instant::now())
                    .elapsed(),
            ),
            std::cmp::min(
                schedule.users_update_interval.saturating_sub(
                    self.last_users_refresh
                        .unwrap_or(std::time::Instant::now())
                        .elapsed(),
                ),
                schedule.processes_update_interval.saturating_sub(
                    self.last_processes_refresh
                        .unwrap_or(std::time::Instant::now())
                        .elapsed(),
                ),
            ),
        )
    }

    fn refresh_host(&mut self) {
        self.last_host_refresh_request = Option::Some(std::time::Instant::now());
        self.system_collector.refresh_host();
        self.last_host_refresh = Option::Some(std::time::Instant::now());
    }

    fn refresh_users(&mut self) {
        self.last_users_refresh_request = Option::Some(std::time::Instant::now());
        self.system_collector.refresh_users();
        self.last_users_refresh = Option::Some(std::time::Instant::now());
    }

    #[cfg(feature = "debug-tools")]
    fn refresh_processes_default(&mut self) {
        self.last_processes_refresh_request = Option::Some(std::time::Instant::now());
        self.system_collector.refresh_processes_default();
        self.last_processes_refresh = Option::Some(std::time::Instant::now());
    }

    fn refresh_processes_specifics(
        &mut self,
        processes: ProcessesToUpdate,
        remove_dead: bool,
        refresh_kind: ProcessRefreshKind,
    ) {
        self.last_processes_refresh_request = Option::Some(std::time::Instant::now());
        self.system_collector
            .refresh_processes_specifics(processes, remove_dead, refresh_kind);
        self.last_processes_refresh = Option::Some(std::time::Instant::now());
    }

    fn refresh_processes(&mut self) {
        let schedule = self.schedules.behaviours[&self.operation_mode];
        self.last_processes_refresh_request = Option::Some(std::time::Instant::now());
        self.system_collector.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            schedule.remove_dead_processes,
            schedule.process_refresh_kind,
        );
        self.last_processes_refresh = Option::Some(std::time::Instant::now());
    }

    pub fn force_refresh_processes(&mut self) {
        self.refresh_processes();
    }

    pub fn force_refresh_users(&mut self) {
        self.refresh_users();
    }
    pub fn force_refresh_host(&mut self) {
        self.refresh_host();
    }

    pub fn get_collector(&self) -> &collector::SystemCollector {
        &self.system_collector
    }
}
