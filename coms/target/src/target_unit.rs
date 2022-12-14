//! TargetUnit is the entrance of the sub unit，implement the trait UnitObj,UnitMngUtil and UnitSubClass.
//! Trait UnitObj defines the behavior of the sub unit.
//! Trait UnitMngUtil is used to attach the Unitmanager to the sub unit.
//! Trait UnitSubClass implement the convert from sub unit to UnitObj.
use super::target_base::{LOG_LEVEL, PLUGIN_NAME};
use super::target_comm::TargetUnitComm;
use super::target_mng::TargetMng;
use libsysmaster::manager::Unit;
use libsysmaster::manager::{
    SubUnit, UmIf, UnitActiveState, UnitDependencyMask, UnitMngUtil, UnitRelationAtom,
    UnitRelations,
};
use libsysmaster::{ReStation, Reliability};
use libutils::logger;
use std::cell::RefCell;
use std::{path::PathBuf, rc::Rc};
struct Target {
    owner: RefCell<Option<Rc<Unit>>>,
    um: Rc<dyn UmIf>,
    comm: Rc<TargetUnitComm>,
    mng: Rc<TargetMng>,
}

impl ReStation for Target {
    // no input, no compensate

    // data
    fn db_map(&self) {
        self.mng.db_map();
    }

    fn db_insert(&self) {
        self.mng.db_insert();
    }

    // reload: entry-only
    fn entry_coldplug(&self) {
        // rebuild external connections, like: timer, ...
        // do nothing now
    }

    fn entry_clear(&self) {
        // release external connection, like: timer, ...
        // do nothing now
    }
}

impl Target {
    fn new(um_if: Rc<dyn UmIf>) -> Target {
        let _comm = Rc::new(TargetUnitComm::new());
        Target {
            owner: RefCell::new(None),
            um: Rc::clone(&um_if),
            comm: Rc::clone(&_comm),
            mng: Rc::new(TargetMng::new(&_comm)),
        }
    }

    pub(self) fn owner(&self) -> Option<Rc<Unit>> {
        if let Some(ref unit) = *self.owner.borrow() {
            Some(Rc::clone(unit))
        } else {
            None
        }
    }

    pub(self) fn add_default_dependencies(&self) {
        if let Some(u) = self.owner() {
            log::debug!("add default dependencies for target[{}]", u.id());
            if !u.default_dependencies() {
                return;
            }
            let um = Rc::clone(&self.um);
            let deps = um.get_dependency_list(
                u.id(),
                UnitRelationAtom::UnitAtomAddDefaultTargetDependencyQueue,
            );
            for _u in deps {
                if !_u.default_dependencies() {
                    continue;
                }

                if um.unit_has_dependecy(u.id(), UnitRelationAtom::UnitAtomBefore, _u.id()) {
                    continue;
                }

                let e = um.unit_add_dependency(
                    u.id(),
                    UnitRelations::UnitAfter,
                    _u.id(),
                    true,
                    UnitDependencyMask::UnitDependencyDefault,
                );
                if e.is_err() {
                    log::error!("add default dependencies error {:?}", e);
                    return;
                }
            }
        } else {
            return;
        }
    }
}

impl SubUnit for Target {
    fn load(&self, _conf_str: Vec<PathBuf>) -> libutils::Result<(), Box<dyn std::error::Error>> {
        //todo add default dependency funnction need add
        log::debug!("load for target");
        self.add_default_dependencies();
        Ok(())
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.to_unit_state()
    }

    fn attach_unit(&self, unit: Rc<libsysmaster::manager::Unit>) {
        self.comm.attach_unit(Rc::clone(&unit));
        self.owner.replace(Some(unit));
        self.db_insert();
    }

    fn init(&self) {}

    fn done(&self) {}

    fn dump(&self) {}

    fn start(&self) -> libutils::Result<(), libsysmaster::manager::UnitActionError> {
        //if current state is not valid, just return.
        self.mng.start_check()?;

        self.mng.start_action(true);
        Ok(())
    }

    fn stop(&self, force: bool) -> libutils::Result<(), libsysmaster::manager::UnitActionError> {
        if !force {
            self.mng.stop_check()?;
        }

        self.mng.stop_action(true);
        Ok(())
    }

    fn reload(&self) {}

    fn kill(&self) {}

    fn release_resources(&self) {}

    fn sigchld_events(
        &self,
        _pid: nix::unistd::Pid,
        _code: i32,
        _status: nix::sys::signal::Signal,
    ) {
    }

    fn reset_failed(&self) {}
}

impl UnitMngUtil for Target {
    fn attach_um(&self, _um: Rc<dyn UmIf>) {
        self.comm.attach_um(_um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

/*impl Default for Target {
    fn default() -> Self {
        Target::new()
    }
}*/

use libsysmaster::declure_unitobj_plugin_with_param;
declure_unitobj_plugin_with_param!(Target, Target::new, PLUGIN_NAME, LOG_LEVEL);
