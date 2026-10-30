use common::{self, ProcessSnapshot, SystemSnapshot};
use sysinfo;

pub struct SystemCollector {
    system: sysinfo::System,
    users: sysinfo::Users,
}

impl SystemCollector {
    pub fn new() -> SystemCollector {
        Self {
            system: sysinfo::System::new(),
            users: sysinfo::Users::new(),
        }
    }

    pub fn refresh_host(&mut self) {
        self.system.refresh_memory();
        self.system.refresh_cpu_all();
    }

    pub fn refresh_processes_specifics(
        &mut self,
        processes_to_update: sysinfo::ProcessesToUpdate,
        remove_dead: bool,
        process_refresh_kind: sysinfo::ProcessRefreshKind,
    ) {
        self.users.refresh();
        self.system.refresh_processes_specifics(
            processes_to_update,
            remove_dead,
            process_refresh_kind,
        );
    }

    pub fn processes_snapshot(&mut self) -> Vec<ProcessSnapshot> {
        self.refresh_processes_specifics(
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
        let processes_raw = self.system.processes();
        let mut processes: Vec<ProcessSnapshot> = Vec::new();

        for process in processes_raw {
            let user_id = process.1.user_id();
            let mut user_name: Option<String> = Option::None;
            match user_id {
                Some(user_id) => match sysinfo::Users::get_user_by_id(&self.users, user_id) {
                    Some(user) => user_name = Option::Some(String::from(user.name())),
                    None => {}
                },

                _ => {}
            }
            processes.push(ProcessSnapshot {
                pid: process.0.as_u32(),
                user_name: user_name,
                user_id: user_id.map(|uid| uid.to_string()),
                cpu_usage: process.1.cpu_usage(),
                cpu_time_ms: process.1.accumulated_cpu_time(),
                mem_used_bytes: process.1.memory(),
                executable_path: process.1.exe().map(|x| std::path::PathBuf::from(x)),
                process_name: std::ffi::OsString::from(process.1.name()),
                process_args: Vec::from(process.1.cmd()),
                active_time: std::time::Duration::from_secs(process.1.run_time()),
                start_time: std::time::SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(process.1.start_time()),
            });
        }

        processes
    }

    pub fn full_snapshot(&mut self) -> SystemSnapshot {
        self.refresh_host();
        return SystemSnapshot {
            cpu_usage: self.system.global_cpu_usage(),
            mem_used_bytes: self.system.used_memory(),
            mem_total_bytes: self.system.total_memory(),
            mem_available_bytes: self.system.available_memory(),
            swap_used_bytes: self.system.used_swap(),
            swap_total_bytes: self.system.total_swap(),
            swap_available_bytes: self.system.free_swap(),
            snapshot_time: std::time::SystemTime::now(),
            uptime: std::time::Duration::from_secs(sysinfo::System::uptime()),
            processes: self.processes_snapshot(),
        };
    }
}
