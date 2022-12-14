#[cfg(test)]
pub(in crate::manager::unit) mod test_utils {
    use std::rc::Rc;

    use libutils::path_lookup::LookupPaths;

    use crate::{
        manager::{
            unit::{
                data::DataManager, uload_util::UnitFile, unit_entry::UnitX, unit_rentry::UnitRe,
            },
            UmIf, UnitType,
        },
        plugin::Plugin,
        Reliability,
    };

    pub(in crate::manager::unit) struct UmIfD;
    impl UmIf for UmIfD {}

    pub(in crate::manager::unit) fn create_unit_for_test_pub(
        dmr: &Rc<DataManager>,
        relir: &Rc<Reliability>,
        rentryr: &Rc<UnitRe>,
        name: &str,
    ) -> Rc<UnitX> {
        let mut l_path = LookupPaths::new();
        l_path.init_lookup_paths();
        let lookup_path = Rc::new(l_path);

        let file = Rc::new(UnitFile::new(&lookup_path));
        let unit_type = UnitType::UnitService;
        let umifd = Rc::new(UmIfD);
        let plugins = Plugin::get_instance();
        let subclass = plugins.create_unit_obj_with_um(unit_type, umifd).unwrap();
        subclass.attach_reli(Rc::clone(relir));
        Rc::new(UnitX::new(dmr, rentryr, &file, unit_type, name, subclass))
    }
}
