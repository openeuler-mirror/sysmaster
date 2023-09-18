// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

#[cfg(feature = "plugin")]
use crate::base::PLUGIN_NAME;
#[cfg(feature = "plugin")]
use constants::LOG_FILE_PATH;

use super::comm::MountUmComm;
use super::rentry::{MountRe, MountReFrame};
use basic::mount_util::MountInfoParser;
use core::error::*;
use core::rel::{ReStation, ReliLastFrame, Reliability};
use core::unit::{UmIf, UnitActiveState, UnitManagerObj, UnitMngUtil, UnitType};
use event::{EventState, EventType, Events, Source};
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::{AsRawFd, RawFd};
use std::rc::Rc;
use std::sync::Arc;

struct MountManager {
    // owned objects
    // data
    comm: Arc<MountUmComm>,
    monitor: MountMonitor,
}

impl ReStation for MountManager {
    // input
    fn input_rebuild(&self) {
        self.monitor.defer_enable(true);
    }

    // compensate
    fn db_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        assert!(lunit.is_some());

        let frame = self.comm.rentry().last_frame();
        if frame.is_none() {
            // debug
            return;
        }

        let unit_id = lunit.unwrap();
        match frame.unwrap() {
            MountReFrame::Monitor => self.comm.um().rentry_trigger_merge(unit_id, true), // merge to trigger
        }
    }

    fn do_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        assert!(lunit.is_some());

        let frame = self.comm.rentry().last_frame();
        if frame.is_none() {
            // debug
            return;
        }

        let unit_id = lunit.unwrap();
        match frame.unwrap() {
            MountReFrame::Monitor => self.comm.um().trigger_unit(unit_id), // re-run
        }
    }

    // no data

    // reload
    fn register_ex(&self) {
        self.monitor.register_ex();
    }
}

impl UnitManagerObj for MountManager {}

// the declaration "pub(self)" is for identification only.
impl MountManager {
    pub fn new() -> MountManager {
        let _comm = MountUmComm::get_instance();
        MountManager {
            comm: Arc::clone(&_comm),
            monitor: MountMonitor::new(&_comm),
        }
    }

    fn _parser(&self) -> Result<i32> {
        let mut mount_data = String::new();
        File::open("/proc/self/mountinfo")
            .unwrap()
            .read_to_string(&mut mount_data)
            .unwrap();
        let parser = MountInfoParser::new(mount_data);
        for mount in parser {
            log::debug!(
                "[{}] mounted on [{}]",
                mount.mount_source,
                mount.mount_point,
            );
        }
        Ok(0)
    }

    fn register(&self) {
        self.monitor.register();
    }
}

struct MountMonitor {
    // owned objects
    data: Rc<MountMonitorData>,
    defer: Rc<MountMonitorDefer>,
    io: Rc<MountMonitorIo>,
}

// the declaration "pub(self)" is for identification only.
impl MountMonitor {
    pub fn new(commr: &Arc<MountUmComm>) -> MountMonitor {
        let filename = "/proc/self/mountinfo".to_string();
        let _data = Rc::new(MountMonitorData::new(commr, filename));
        let _defer = Rc::new(MountMonitorDefer::new(&_data));
        MountMonitor {
            data: Rc::clone(&_data),
            defer: Rc::clone(&_defer),
            io: Rc::new(MountMonitorIo::new(&_data, &_defer)),
        }
    }

    pub fn register_ex(&self) {
        let events = self.data.comm.um().events();

        // io
        let io = Rc::clone(&self.io);
        events.add_source(io.clone()).unwrap();
        events.set_enabled(io, EventState::On).unwrap();
    }

    pub fn defer_enable(&self, enable: bool) -> i32 {
        self.io.defer_enable(enable)
    }

    fn register(&self) {
        let events = self.data.comm.um().events();

        // defer
        let defer = Rc::clone(&self.defer);
        events.add_source(defer).unwrap();
    }
}

struct MountMonitorIo {
    // associated objects
    data: Rc<MountMonitorData>,
    defer: Rc<MountMonitorDefer>,
}

// the declaration "pub(self)" is for identification only.
impl MountMonitorIo {
    pub fn new(datar: &Rc<MountMonitorData>, deferr: &Rc<MountMonitorDefer>) -> MountMonitorIo {
        MountMonitorIo {
            data: Rc::clone(datar),
            defer: Rc::clone(deferr),
        }
    }

    pub fn defer_enable(&self, enable: bool) -> i32 {
        let source = Rc::clone(&self.defer);
        let state = match enable {
            true => EventState::OneShot,
            false => EventState::Off,
        };
        let events = self.data.comm.um().events();
        match events.set_enabled(source, state) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }

    fn reli(&self) -> Rc<Reliability> {
        self.data.comm.reli()
    }

    fn rentry(&self) -> Rc<MountRe> {
        self.data.comm.rentry()
    }
}

impl Source for MountMonitorIo {
    fn event_type(&self) -> EventType {
        EventType::Io
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn dispatch(&self, _e: &Events) -> i32 {
        drain_out(self.data.epfd);

        self.reli()
            .set_last_frame2(ReliLastFrame::SubManager as u32, UnitType::UnitMount as u32);
        self.rentry().set_last_frame(MountReFrame::Monitor);
        let ret = self.data.dispatch_mountinfo();
        self.rentry().clear_last_frame();
        self.reli().clear_last_frame();

        if let Err(e) = ret {
            log::error!("Failed to dispatch mountinfo, ignoring: {:?}", e);
        }

        self.defer_enable(false)
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn fd(&self) -> RawFd {
        self.data.epfd
    }

    fn priority(&self) -> i8 {
        -10i8
    }
}

struct MountMonitorDefer {
    // associated objects
    data: Rc<MountMonitorData>,
}

impl MountMonitorDefer {
    pub fn new(datar: &Rc<MountMonitorData>) -> MountMonitorDefer {
        MountMonitorDefer {
            data: Rc::clone(datar),
        }
    }

    fn reli(&self) -> Rc<Reliability> {
        self.data.comm.reli()
    }

    fn rentry(&self) -> Rc<MountRe> {
        self.data.comm.rentry()
    }
}

impl Source for MountMonitorDefer {
    fn event_type(&self) -> EventType {
        EventType::Defer
    }

    fn epoll_event(&self) -> u32 {
        0
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn dispatch(&self, _event: &Events) -> i32 {
        self.reli()
            .set_last_frame2(ReliLastFrame::SubManager as u32, UnitType::UnitMount as u32);
        self.rentry().set_last_frame(MountReFrame::Monitor);
        let ret = self.data.dispatch_mountinfo();
        self.rentry().clear_last_frame();
        self.reli().clear_last_frame();

        match ret {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }

    fn priority(&self) -> i8 {
        0i8
    }
}

struct MountMonitorData {
    // associated objects
    comm: Arc<MountUmComm>,

    // owned objects
    _file: File,
    epfd: i32,
    _epevent: epoll::Event,
    _events: Vec<epoll::Event>,
}

impl MountMonitorData {
    pub fn new(commr: &Arc<MountUmComm>, filename: String) -> Self {
        let me_file = match File::open(&filename) {
            Ok(me_file) => me_file,
            Err(why) => {
                panic!("Failed to open {}: {}", &filename, why);
            }
        };

        let me_fd = me_file.as_raw_fd();
        let me_epfd = epoll::create(true).unwrap();
        let event = epoll::Events::EPOLLET | epoll::Events::EPOLLIN;
        let me_epevent = epoll::Event::new(event, me_fd as u64);
        let op = epoll::ControlOptions::EPOLL_CTL_ADD;
        drain_out(me_epfd);
        epoll::ctl(me_epfd, op, me_fd, me_epevent).unwrap();

        log::debug!("Monitoring {} by epool fd {}", filename, me_epfd);
        MountMonitorData {
            comm: Arc::clone(commr),
            _file: me_file,
            epfd: me_epfd,
            _epevent: me_epevent,
            _events: Vec::new(),
        }
    }

    pub fn dispatch_mountinfo(&self) -> Result<()> {
        // First mark all active mount point we have as dead.
        let mut dead_mount_set: HashSet<String> = HashSet::new();
        let unit_type = Some(UnitType::UnitMount);
        for unit in self.comm.um().units_get_all(unit_type).iter() {
            if self.comm.um().current_active_state(unit) == UnitActiveState::Active {
                dead_mount_set.insert(String::from(unit));
            }
        }

        // Then start mount point we don't know.
        let mut mountinfo_content = String::new();
        File::open("/proc/self/mountinfo")
            .unwrap()
            .read_to_string(&mut mountinfo_content)
            .unwrap();
        let parser = MountInfoParser::new(mountinfo_content);
        for mount in parser {
            // pop
            // We don't process autofs for now, because it is not
            // .mount but .automount in systemd.
            if mount.fstype == "autofs" {
                continue;
            }
            let unit_name = mount_point_to_unit_name(&mount.mount_point);
            if dead_mount_set.contains(unit_name.as_str()) {
                dead_mount_set.remove(unit_name.as_str());
            } else if self.comm.um().load_unit_success(unit_name.as_str()) {
                // record + action
                self.comm.reli().set_last_unit(&unit_name);
                let start_ok = self.comm.um().unit_start_directly(&unit_name).is_ok();
                self.comm.reli().clear_last_unit();

                if start_ok {
                    log::debug!("{} change to mounted.", unit_name);
                } else {
                    log::error!("Failed to start {}", unit_name);
                }
            }
        }

        // Finally stop mount point in dead_mount_set.
        for unit_name in dead_mount_set.into_iter() {
            // pop
            // record + action
            self.comm.reli().set_last_unit(&unit_name);
            let ret = self.comm.um().unit_stop(&unit_name, false);
            self.comm.reli().clear_last_unit();

            if ret.is_ok() {
                log::debug!("{} change to dead.", unit_name);
            } else {
                log::error!("Failed to stop {}.", unit_name);
            }
        }
        Ok(())
    }
}

fn drain_out(epfd: i32) {
    // drain out all events.
    let mut me_events: Vec<epoll::Event> = vec![epoll::Event::new(epoll::Events::empty(), 0)];
    while epoll::wait(epfd, 0, &mut me_events).unwrap() > 0 {}
}

fn mount_point_to_unit_name(mount_point: &str) -> String {
    let mut res = String::from(mount_point).replace('/', "-") + ".mount";
    if res != "-.mount" {
        res = String::from(&res[1..])
    }
    res
}

impl UnitMngUtil for MountManager {
    fn attach_um(&self, um: Rc<dyn UmIf>) {
        self.comm.attach_um(um);
        self.register();
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

impl Default for MountManager {
    fn default() -> Self {
        MountManager::new()
    }
}

use core::declare_umobj_plugin;
declare_umobj_plugin!(MountManager, MountManager::default);

#[cfg(test)]
mod test {
    //use super::*;

    #[test]
    fn test_monitor() {
        //let event = Rc::new(Events::new().unwrap());
        //let reli = Rc::new(Reliability::new());
        //let comm = Rc::new(MountComm::new());
        //let mount_source = Rc::new(MountMonitor::new(&event, &reli, &comm));

        //event.add_source(mount_source.clone()).unwrap();
        //event.set_enabled(mount_source.clone(), EventState::On).unwrap();

        //event.run(10).unwrap();
    }
}
