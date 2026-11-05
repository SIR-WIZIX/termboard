use std::collections::HashMap;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};

use crate::collector;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MonitoringMode {
    Background,
    Interactive,
    Incident,
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

    //TODO remove public setter in order for handeling possible data losses while switching from
    //Incident
    pub fn set_operation_mode(&mut self, operation_mode: MonitoringMode) {
        self.operation_mode = operation_mode;
    }

    //TODO same for removal
    pub fn set_schedules(&mut self, schedules: Schedules) {
        self.schedules = schedules;
    }

    fn tick(&mut self) -> std::time::Duration {
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
}
