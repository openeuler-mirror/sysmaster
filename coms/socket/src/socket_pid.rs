//! socket_pid implement the management of pid。
//!

use super::socket_comm::SocketUnitComm;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) struct SocketPid {
    comm: Rc<SocketUnitComm>,
    data: RefCell<SocketPidData>,
}

impl SocketPid {
    pub(super) fn new(comm: &Rc<SocketUnitComm>) -> SocketPid {
        SocketPid {
            comm: comm.clone(),
            data: RefCell::new(SocketPidData::new()),
        }
    }

    pub(super) fn unwatch_control(&self) {
        if let Some(pid) = self.control() {
            self.comm.um().child_unwatch_pid(self.comm.unit().id(), pid);
            self.reset_control();
        }
    }

    pub(super) fn control(&self) -> Option<Pid> {
        self.data.borrow().control()
    }

    pub(super) fn set_control(&self, pid: Pid) {
        self.data.borrow_mut().set_control(pid)
    }

    pub(super) fn update_control(&self, pid: Option<Pid>) {
        if let Some(id) = pid {
            self.set_control(id);
        } else {
            self.reset_control();
        }
    }

    fn reset_control(&self) {
        self.data.borrow_mut().reset_control()
    }
}

struct SocketPidData {
    control: Option<Pid>,
}

impl SocketPidData {
    pub(self) fn new() -> Self {
        SocketPidData { control: None }
    }

    pub(self) fn control(&self) -> Option<Pid> {
        self.control.as_ref().cloned()
    }

    pub(self) fn reset_control(&mut self) {
        self.control = None;
    }

    pub(self) fn set_control(&mut self, pid: Pid) {
        self.control = Some(pid);
    }
}

#[cfg(test)]
mod tests {
    use super::SocketPid;
    use crate::socket_comm::SocketUnitComm;
    use nix::unistd::Pid;
    use std::rc::Rc;

    #[test]
    fn test_socket_pid() {
        let _comm = Rc::new(SocketUnitComm::new());
        let pid = Rc::new(SocketPid::new(&_comm));

        pid.set_control(Pid::from_raw(5));
        assert_eq!(pid.control().unwrap(), Pid::from_raw(5));
    }
}
