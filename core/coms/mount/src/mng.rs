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

//!  The core logic of the mount subclass
use super::comm::MountUnitComm;
use super::rentry::MountState;
use core::error::*;
use core::rel::ReStation;
use core::unit::{UnitActiveState, UnitNotifyFlags};
use std::{cell::RefCell, rc::Rc};

impl MountState {
    fn mount_state_to_unit_state(&self) -> UnitActiveState {
        match *self {
            MountState::Dead => UnitActiveState::InActive,
            MountState::Mounted => UnitActiveState::Active,
        }
    }
}

pub(super) struct MountMng {
    comm: Rc<MountUnitComm>,
    state: RefCell<MountState>,
}

impl ReStation for MountMng {
    // no input, no compensate

    // data
    fn db_map(&self, _reload: bool) {
        if let Some(state) = self.comm.rentry_mng_get() {
            *self.state.borrow_mut() = state;
        }
    }

    fn db_insert(&self) {
        self.comm.rentry_mng_insert(self.state());
    }

    // reload: no external connections, no entry
}

impl MountMng {
    pub(super) fn new(_comm: &Rc<MountUnitComm>) -> Self {
        MountMng {
            comm: Rc::clone(_comm),
            state: RefCell::new(MountState::Dead),
        }
    }

    // process doesn't support manually mount/umount like systemd.
    // We only monitor the state of mountpoint.

    pub(super) fn enter_dead(&self, notify: bool) {
        self.set_state(MountState::Dead, notify);
    }

    pub(super) fn enter_mounted(&self, notify: bool) {
        self.set_state(MountState::Mounted, notify);
    }

    pub(super) fn start_check(&self) -> Result<bool> {
        let ret = self.comm.owner().map_or(false, |u| u.test_start_limit());
        if !ret {
            self.enter_dead(true);
            return Err(Error::UnitActionECanceled);
        }

        Ok(false)
    }

    pub fn get_state(&self) -> String {
        let state = *self.state.borrow();
        state.to_string()
    }

    fn set_state(&self, new_state: MountState, notify: bool) {
        let old_state = self.state();
        self.change_state(new_state);

        if notify {
            self.state_notify(new_state, old_state);
        }
    }

    fn state_notify(&self, new_state: MountState, old_state: MountState) {
        if new_state != old_state {
            log::debug!(
                "{} original state[{:?}] -> new state[{:?}]",
                self.comm.get_owner_id(),
                old_state,
                new_state,
            );
        }

        let old_unit_state = old_state.mount_state_to_unit_state();
        let new_unit_state = new_state.mount_state_to_unit_state();
        if let Some(u) = self.comm.owner() {
            u.notify(
                old_unit_state,
                new_unit_state,
                UnitNotifyFlags::RELOAD_FAILURE,
            )
        }

        self.db_update();
    }

    fn change_state(&self, new_state: MountState) {
        self.state.replace(new_state);
    }

    fn state(&self) -> MountState {
        *self.state.borrow()
    }

    pub(super) fn mount_state_to_unit_state(&self) -> UnitActiveState {
        self.state().mount_state_to_unit_state()
    }
}

#[cfg(test)]
mod tests {
    use super::MountMng;
    use super::MountState;
    use super::MountUnitComm;
    use std::rc::Rc;

    #[test]
    fn test_mount_set_state() {
        let _comm = Rc::new(MountUnitComm::new());
        let tm = MountMng::new(&_comm);
        tm.set_state(MountState::Mounted, false);
        assert_eq!(tm.state(), MountState::Mounted)
    }

    #[test]
    fn test_mount_enter_dead() {
        let _comm = Rc::new(MountUnitComm::new());
        let tm = MountMng::new(&_comm);
        tm.enter_dead(false);
        assert_eq!(tm.state(), MountState::Dead)
    }

    #[test]
    fn test_mount_enter_mounted() {
        let _comm = Rc::new(MountUnitComm::new());
        let tm = MountMng::new(&_comm);
        tm.enter_mounted(false);
        assert_eq!(tm.state(), MountState::Mounted)
    }
}
