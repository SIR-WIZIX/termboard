use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};

use crate::collector;

pub enum MachineState {
    IDLE,
    WATCHED,
    INCIDENT,
}

pub struct Scheduler {
    last_host_refresh_request: Option<std::time::Instant>,
    last_users_refresh_request: Option<std::time::Instant>,
    last_processes_refresh_request: Option<std::time::Instant>,
    last_host_refresh: Option<std::time::Instant>,
    last_users_refresh: Option<std::time::Instant>,
    last_processes_refresh: Option<std::time::Instant>,
    system_collector: collector::SystemCollector,

    host_update_interval: std::time::Duration,
    users_update_interval: std::time::Duration,
    processes_update_interval: std::time::Duration,

    operation_mode: MachineState,
}

impl Scheduler {
    pub fn new(
        collector: collector::SystemCollector,
        host_update_interval: std::time::Duration,
        users_update_interval: std::time::Duration,
        processes_update_interval: std::time::Duration,
    ) -> Scheduler {
        Self {
            last_host_refresh_request: Option::None,
            last_users_refresh_request: Option::None,
            last_processes_refresh_request: Option::None,
            last_host_refresh: Option::None,
            last_users_refresh: Option::None,
            last_processes_refresh: Option::None,
            system_collector: collector,
            host_update_interval: host_update_interval,
            users_update_interval: users_update_interval,
            processes_update_interval: processes_update_interval,

            operation_mode: MachineState::IDLE,
        }
    }

    pub fn set_operation_mode(&mut self, operation_mode: MachineState) {
        self.operation_mode = operation_mode;
    }

    fn tick(&self) {}

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
