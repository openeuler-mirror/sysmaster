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

//! usb_id builtin
//!

use crate::builtin::Builtin;
use crate::builtin::Netlink;
use crate::error::*;
use device::Device;
use snafu::OptionExt;
use snafu::ResultExt;
use sscanf::sscanf;
use std::cell::RefCell;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[repr(C, packed)]
#[allow(non_snake_case)]
struct UsbInterfaceDescriptor {
    bLength: u8,
    bDescriptorType: u8,
    bInterfaceNumber: u8,
    bAlternateSetting: u8,
    bNumEndpoints: u8,
    bInterfaceClass: u8,
    bInterfaceSubClass: u8,
    bInterfaceProtocol: u8,
    iInterface: u8,
}

#[derive(Default, Clone)]
struct UsbInfo {
    vendor: String,
    vendor_enc: String,
    vendor_id: String,
    model: String,
    model_enc: String,
    product_id: String,
    revision: String,
    serial: String,
    serial_short: String,
    type_str: String,
    instance: String,
    packed_if: String,
    ifnum: String,
    driver: String,
    protocol: i32,
    if_class: String,
}

/// usb_id builtin command
pub struct UsbId;

impl UsbId {
    const USB_IFTYPE_TABLE: [(i32, &'static str); 9] = [
        (1, "audio"),
        (3, "hid"),
        (5, "Physical"),
        (6, "media"),
        (7, "printer"),
        (8, "storage"),
        (9, "hub"),
        (0x0e, "video"),
        (0xff, "generic"), // fallback for undefined values
    ];

    fn usb_iftype(if_class_num: i32) -> Option<&'static str> {
        let result = Self::USB_IFTYPE_TABLE
            .iter()
            .find(|&&(class_num, _)| class_num == if_class_num);
        result.map(|&(_, name)| name)
    }

    const SUBTYPE_MAP: [(i32, &'static str); 6] = [
        (1, "rbc"),
        (2, "atapi"),
        (3, "tape"),
        (4, "floppy"),
        (6, "scsi"),
        (0, "generic"),
    ];

    fn usb_mass_storage_ifsubtype(from: &str, protocol: &mut i32) -> Option<&'static str> {
        *protocol = 0;
        if let Ok(num) = from.parse::<i32>() {
            for (n, s) in Self::SUBTYPE_MAP {
                if n == num {
                    *protocol = n;
                    return Some(s);
                }
            }
        }
        Some("generic")
    }

    fn scsi_type(from: &str) -> Option<&'static str> {
        let num = from.parse::<i32>().ok()?;
        Some(match num {
            0 | 0xE => "disk",
            1 => "tape",
            4 | 7 | 0xF => "optical",
            5 => "cd",
            _ => "generic",
        })
    }

    const USB_DT_INTERFACE: u8 = 0x04;
    fn dev_if_packed_info(dev: &Device, info: &mut UsbInfo) -> Result<()> {
        let syspath = dev.get_syspath().unwrap();
        let filename = PathBuf::from(syspath).join("descriptors");
        let mut file = File::open(filename).context(IoSnafu {
            filename: syspath.to_string(),
        })?;
        let mut buf = [0u8; 18 + 65535];
        let mut pos = 0;

        let size = file.read(&mut buf).context(IoSnafu {
            filename: syspath.to_string(),
        })?;
        if size < 18 {
            return Err(Error::ReadTooShort {
                filename: syspath.to_string(),
            });
        }

        while pos + std::mem::size_of::<UsbInterfaceDescriptor>() < size {
            let desc: UsbInterfaceDescriptor =
                unsafe { std::ptr::read_unaligned(buf.as_ptr().add(pos) as *const _) };
            if desc.bLength < 3 {
                break;
            }
            if desc.bLength > (size - std::mem::size_of::<UsbInterfaceDescriptor>()) as u8 {
                return Err(Error::CorruptData {
                    filename: syspath.to_string(),
                });
            }
            pos += desc.bLength as usize;

            if desc.bDescriptorType != Self::USB_DT_INTERFACE {
                continue;
            }

            let if_str = format!(
                ":{:02x}{:02x}{:02x}",
                desc.bInterfaceClass, desc.bInterfaceSubClass, desc.bInterfaceProtocol
            );

            if if_str.len() != 7 {
                continue;
            }

            if info.packed_if.contains(&if_str) {
                continue;
            }

            info.packed_if.push_str(&if_str);
        }

        if !info.packed_if.is_empty() {
            info.packed_if.push(':');
        }

        Ok(())
    }

    fn interface_directory(&self, device: &mut Device, info: &mut UsbInfo) -> Result<bool> {
        let dev_interface = device
            .get_parent_with_subsystem_devtype("usb", Some("usb_interface"))
            .context(FailToAccessSnafu {
                filename: "usb_interface".to_string(),
            })?;
        let mut dev_interface = dev_interface.lock().unwrap();

        let _interface_syspath = dev_interface.get_syspath().context(SysPathNotFoundSnafu)?;

        info.ifnum = dev_interface
            .get_sysattr_value("bInterfacceNumber".to_string())
            .unwrap_or_else(|_| Default::default());

        info.driver = dev_interface
            .get_sysattr_value("driver".to_string())
            .unwrap_or_else(|_| Default::default());

        info.if_class = dev_interface
            .get_sysattr_value("bInterfaceClass".to_string())
            .context(GetSysAttrSnafu)?;

        info.type_str = match info.if_class.parse::<i32>().context(ParseIntSnafu)? {
            8 => {
                let mut type_str = String::new();
                if let Ok(if_subclass) =
                    dev_interface.get_sysattr_value("bInterfaceSubClass".to_string())
                {
                    type_str = UsbId::usb_mass_storage_ifsubtype(&if_subclass, &mut info.protocol)
                        .unwrap()
                        .to_string();
                }
                type_str
            }
            i => UsbId::usb_iftype(i).unwrap().to_string(),
        };
        Ok(true)
    }

    fn mass_storage(&self, device: &mut Device, info: &mut UsbInfo) -> Result<bool> {
        if [2, 6].contains(&info.protocol) {
            let dev_scsi = device
                .get_parent_with_subsystem_devtype("scsi", Some("scsi_device"))
                .context(FailToAccessSnafu {
                    filename: "usb_interface".to_string(),
                })?;
            let mut dev_scsi = dev_scsi.lock().unwrap().to_owned();

            let scsi_sysname = dev_scsi.get_sysname().context(SysNameNotFoundSnafu)?;

            let (_host, _buss, target, lun) =
                sscanf!(&scsi_sysname, "{}:{}:{}:{}", i32, i32, i32, i32)
                    .context(FailToSscanfSnafu)?;

            let scsi_vendor = dev_scsi
                .get_sysattr_value("vendor".to_string())
                .context(GetSysAttrSnafu)?;
            // scsi_vendor to vendor
            crate::utils::encode_devnode_name(&scsi_vendor, &mut info.vendor_enc);
            info.vendor = crate::utils::replace_whitespace(&scsi_vendor);
            info.vendor = crate::utils::replace_chars(&info.vendor, "");

            let scsi_model = dev_scsi
                .get_sysattr_value("model".to_string())
                .context(GetSysAttrSnafu)?;
            // scsi_model to model
            crate::utils::encode_devnode_name(&scsi_model, &mut info.model_enc);
            info.model = crate::utils::replace_whitespace(&scsi_model);
            info.model = crate::utils::replace_chars(&info.model, "");

            let scsi_type_str = dev_scsi
                .get_sysattr_value("type".to_string())
                .context(GetSysAttrSnafu)?;

            // scsi_type_str to type_str
            if let Some(s) = UsbId::scsi_type(&scsi_type_str) {
                info.type_str = s.to_string();
            };

            let scsi_revision = dev_scsi
                .get_sysattr_value("rev".to_string())
                .context(GetSysAttrSnafu)?;

            // scsi_revision to revision, unimplemented!()
            info.revision = crate::utils::replace_whitespace(&scsi_revision);
            info.revision = crate::utils::replace_chars(&info.revision, "");

            info.instance = format!("{}:{}", target, lun);
        }
        Ok(true)
    }

    fn set_sysattr(&self, device: &mut Device, info: &mut UsbInfo) -> Result<bool> {
        info.vendor_id = device
            .get_sysattr_value("idVendor".to_string())
            .context(GetSysAttrSnafu)?;

        info.product_id = device
            .get_sysattr_value("idProduct".to_string())
            .context(GetSysAttrSnafu)?;

        if info.vendor.is_empty() {
            let usb_vendor = match device.get_sysattr_value("manufacturer".to_string()) {
                Ok(s) => s,
                Err(_) => info.vendor_id.clone(),
            };
            crate::utils::encode_devnode_name(&usb_vendor, &mut info.vendor_enc);
            info.vendor = crate::utils::replace_whitespace(&usb_vendor);
            info.vendor = crate::utils::replace_chars(&info.vendor, "");
        }

        if info.model.is_empty() {
            let usb_model = match device.get_sysattr_value("product".to_string()) {
                Ok(s) => s,
                Err(_) => info.product_id.clone(),
            };
            crate::utils::encode_devnode_name(&usb_model, &mut info.model_enc);
            info.model = crate::utils::replace_whitespace(&usb_model);
            info.model = crate::utils::replace_chars(&info.model, "");
        }

        if info.revision.is_empty() {
            if let Ok(usb_revision) = device.get_sysattr_value("bcdDevice".to_string()) {
                info.revision = crate::utils::replace_whitespace(&usb_revision);
                info.revision = crate::utils::replace_chars(&info.revision, "");
            }
        }

        if info.serial_short.is_empty() {
            if let Ok(mut usb_serial) = device.get_sysattr_value("serial".to_string()) {
                // usb_serial to serial
                for (_idx, byte) in usb_serial.bytes().enumerate() {
                    if !(0x20..=0x7f).contains(&byte) || byte == b',' {
                        usb_serial.clear();
                        break;
                    }
                }

                if !usb_serial.is_empty() {
                    info.serial_short = crate::utils::replace_whitespace(&usb_serial);
                    info.serial_short = crate::utils::replace_chars(&info.serial_short, "");
                }
            }

            info.serial = format!("{0}_{1}", info.vendor, info.model);
            if !info.serial_short.is_empty() {
                info.serial = format!("{0}_{1}", info.serial, info.serial_short);
            }

            if !info.instance.is_empty() {
                info.serial = format!("{0}-{1}", info.serial, info.instance);
            }
        }

        Ok(true)
    }

    fn add_all_property(
        &self,
        device: Arc<Mutex<Device>>,
        test: bool,
        info: UsbInfo,
    ) -> Result<bool> {
        match device.lock().unwrap().get_property_value("ID_BUS".to_string()) {
            Ok(_) => log::debug!("ID_BUS property is already set, setting only properties prefixed with \"ID_USB_\"."),
            Err(_) => {
                self.add_property(device.clone(), test, "ID_BUS".to_string(), "usb".to_string())?;
                self.add_property(device.clone(), test, "ID_MODEL".to_string(), info.model.clone())?;
                self.add_property(device.clone(), test, "ID_MODEL_ENC".to_string(), info.model_enc.clone())?;
                self.add_property(device.clone(), test, "ID_MODEL_ID".to_string(), info.product_id.clone())?;

                self.add_property(device.clone(), test, "ID_SERIAL".to_string(), info.serial.clone())?;
                if !info.serial_short.is_empty() {
                        self.add_property(device.clone(), test, "ID_SERIAL_SHORT".to_string(), info.serial_short.clone())?;
                }
                self.add_property(device.clone(), test, "ID_VENDOR".to_string(), info.vendor.clone())?;
                self.add_property(device.clone(), test, "ID_VENDOR_ENC".to_string(), info.vendor_enc.clone())?;
                self.add_property(device.clone(), test, "ID_VENDOR_ID".to_string(), info.vendor_id.clone())?;

                self.add_property(device.clone(), test, "ID_REVISION".to_string(), info.revision.clone())?;

                if !info.type_str.is_empty() {
                    self.add_property(device.clone(), test, "ID_TYPE".to_string(), info.type_str.clone())?;
                }
                if !info.instance.is_empty() {
                    self.add_property(device.clone(), test, "ID_INSTANCE".to_string(), info.instance.clone())?;
                }
            },
        }

        self.add_property(device.clone(), test, "ID_USB_MODEL".to_string(), info.model)?;
        self.add_property(
            device.clone(),
            test,
            "ID_USB_MODEL_ENC".to_string(),
            info.model_enc,
        )?;
        self.add_property(
            device.clone(),
            test,
            "ID_USB_MODEL_ID".to_string(),
            info.product_id,
        )?;
        self.add_property(
            device.clone(),
            test,
            "ID_USB_SERIAL".to_string(),
            info.serial,
        )?;
        if !info.serial_short.is_empty() {
            self.add_property(
                device.clone(),
                test,
                "ID_USB_SERIAL_SHORT".to_string(),
                info.serial_short,
            )?;
        }
        self.add_property(
            device.clone(),
            test,
            "ID_USB_VENDOR".to_string(),
            info.vendor,
        )?;
        self.add_property(
            device.clone(),
            test,
            "ID_USB_VENDOR_ENC".to_string(),
            info.vendor_enc,
        )?;
        self.add_property(
            device.clone(),
            test,
            "ID_USB_VENDOR_ID".to_string(),
            info.vendor_id,
        )?;
        self.add_property(
            device.clone(),
            test,
            "ID_USB_REVISION".to_string(),
            info.revision,
        )?;

        if !info.type_str.is_empty() {
            self.add_property(
                device.clone(),
                test,
                "ID_USB_TYPE".to_string(),
                info.type_str,
            )?;
        }

        if !info.instance.is_empty() {
            self.add_property(
                device.clone(),
                test,
                "ID_USB_INSTANCE".to_string(),
                info.instance,
            )?;
        }
        if !info.packed_if.is_empty() {
            self.add_property(
                device.clone(),
                test,
                "ID_USB_INTERFACES".to_string(),
                info.packed_if,
            )?;
        }
        if !info.ifnum.is_empty() {
            self.add_property(
                device.clone(),
                test,
                "ID_USB_INTERFACE_NUM".to_string(),
                info.ifnum,
            )?;
        }
        if !info.driver.is_empty() {
            self.add_property(device, test, "ID_USB_DRIVER".to_string(), info.driver)?;
        }
        Ok(true)
    }
}

impl Builtin for UsbId {
    /// builtin command
    fn cmd(
        &self,
        device: Arc<Mutex<Device>>,
        _ret_rtnl: &mut RefCell<Option<Netlink>>,
        _argc: i32,
        _argv: Vec<String>,
        test: bool,
    ) -> Result<bool> {
        let mut info = UsbInfo::default();
        let mut usb_device = Device::default();

        let _syspath = device
            .lock()
            .unwrap()
            .get_syspath()
            .context(SysPathNotFoundSnafu)?;
        let _sysname = device
            .lock()
            .unwrap()
            .get_sysname()
            .context(SysNameNotFoundSnafu)?;
        let devtype = device
            .lock()
            .unwrap()
            .get_devtype()
            .context(FailToGetDevTypeSnafu)?;

        #[allow(clippy::never_loop)]
        loop {
            if devtype == "usb_device" {
                let _ = Self::dev_if_packed_info(&device.lock().unwrap(), &mut info);
                usb_device = device.lock().unwrap().to_owned();
                break;
            }

            match self.interface_directory(&mut device.lock().unwrap(), &mut info) {
                Ok(true) => (),
                Ok(false) => break,
                Err(e) => return Err(e),
            };

            log::debug!("if_class:{} protocol:{}", info.if_class, info.protocol);

            let dev_usb = device
                .lock()
                .unwrap()
                .get_parent_with_subsystem_devtype("usb", Some("usb_interface"))
                .context(FailToAccessSnafu {
                    filename: "usb_interface".to_string(),
                })?;

            let _ = Self::dev_if_packed_info(&dev_usb.lock().unwrap(), &mut info);

            match self.mass_storage(&mut device.lock().unwrap(), &mut info) {
                Ok(_) => (),
                Err(e) => {
                    log::error!("{:?}", e);
                }
            }

            usb_device = dev_usb.lock().unwrap().to_owned();
        }

        self.set_sysattr(&mut usb_device, &mut info)?;

        self.add_all_property(device, test, info)?;

        Ok(true)
    }

    /// builtin init function
    fn init(&self) {}

    /// builtin exit function
    fn exit(&self) {}

    /// check whether builtin command should reload
    fn should_reload(&self) -> bool {
        false
    }

    /// the help of builtin command
    fn help(&self) -> String {
        "USB device properties".to_string()
    }

    /// whether the builtin command can only run once
    fn run_once(&self) -> bool {
        true
    }
}
#[cfg(test)]
mod test {
    use std::cell::RefCell;

    use device::device_enumerator::DeviceEnumerator;

    use crate::builtin::{usb_id::UsbId, Builtin, Netlink};

    #[test]
    fn test_usb_mass_storage_ifsubtype() {
        let mut protocol = 0;
        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("1", &mut protocol),
            Some("rbc")
        );
        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("2", &mut protocol),
            Some("atapi")
        );
        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("3", &mut protocol),
            Some("tape")
        );
        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("4", &mut protocol),
            Some("floppy")
        );

        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("6", &mut protocol),
            Some("scsi")
        );
        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("0", &mut protocol),
            Some("generic")
        );
        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("7", &mut protocol),
            Some("generic")
        );
    }

    #[test]
    fn test_scsi_type() {
        assert_eq!(UsbId::scsi_type("0"), Some("disk"));
        assert_eq!(UsbId::scsi_type("14"), Some("disk"));
        assert_eq!(UsbId::scsi_type("1"), Some("tape"));
        assert_eq!(UsbId::scsi_type("4"), Some("optical"));
        assert_eq!(UsbId::scsi_type("7"), Some("optical"));
        assert_eq!(UsbId::scsi_type("15"), Some("optical"));
        assert_eq!(UsbId::scsi_type("5"), Some("cd"));
        assert_eq!(UsbId::scsi_type("10"), Some("generic"));
    }

    #[test]
    fn test_usb_id() {
        let mut enumerator = DeviceEnumerator::new();

        for device in enumerator.iter_mut() {
            let mut rtnl = RefCell::<Option<Netlink>>::from(None);

            let builtin = UsbId {};
            if let Some(str) = device.lock().unwrap().get_devpath() {
                if !str.contains("usb") {
                    continue;
                }
            }
            println!("devpath:{:?}", device.lock().unwrap().get_devpath());
            if let Err(e) = builtin.cmd(device.clone(), &mut rtnl, 0, vec![], true) {
                println!("Builtin command path_id: fails:{:?}", e);
            }
        }
    }
}
