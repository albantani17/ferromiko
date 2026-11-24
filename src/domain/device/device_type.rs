use std::str::FromStr;

use super::profile::{self, Profile};
use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    CiscoIos,
    Zte,
    ZteTelnet,
    Junos,
    Mikrotik,
    Linux,
}

impl DeviceType {
    pub fn profile(self) -> Profile {
        match self {
            DeviceType::CiscoIos => profile::CISCO_IOS,
            DeviceType::Zte => profile::ZTE,
            DeviceType::ZteTelnet => profile::ZTE_TELNET,
            DeviceType::Junos => profile::JUNOS,
            DeviceType::Mikrotik => profile::MIKROTIK,
            DeviceType::Linux => profile::LINUX,
        }
    }
}

/// Accepts netmiko's `device_type` names.
impl FromStr for DeviceType {
    type Err = Error;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "cisco_ios" | "cisco_xe" => Ok(DeviceType::CiscoIos),
            "zte_zxros" | "zte" => Ok(DeviceType::Zte),
            "zte_zxros_telnet" | "zte_telnet" => Ok(DeviceType::ZteTelnet),
            "juniper_junos" | "juniper" => Ok(DeviceType::Junos),
            "mikrotik_routeros" | "mikrotik" => Ok(DeviceType::Mikrotik),
            "linux" => Ok(DeviceType::Linux),
            other => Err(Error::UnknownDeviceType(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_netmiko_device_type_names() {
        assert_eq!("cisco_ios".parse::<DeviceType>().unwrap(), DeviceType::CiscoIos);
        assert_eq!("cisco_xe".parse::<DeviceType>().unwrap(), DeviceType::CiscoIos);
        assert_eq!("zte_zxros".parse::<DeviceType>().unwrap(), DeviceType::Zte);
        assert_eq!("zte_zxros_telnet".parse::<DeviceType>().unwrap(), DeviceType::ZteTelnet);
        assert_eq!("juniper_junos".parse::<DeviceType>().unwrap(), DeviceType::Junos);
        assert_eq!("mikrotik_routeros".parse::<DeviceType>().unwrap(), DeviceType::Mikrotik);
        assert_eq!("linux".parse::<DeviceType>().unwrap(), DeviceType::Linux);
    }

    #[test]
    fn rejects_unknown_device_type() {
        assert!(matches!("hp_comware".parse::<DeviceType>(), Err(Error::UnknownDeviceType(_))));
    }

    #[test]
    fn only_mikrotik_changes_the_login_name() {
        for device_type in [DeviceType::CiscoIos, DeviceType::Zte, DeviceType::ZteTelnet, DeviceType::Junos, DeviceType::Linux] {
            assert_eq!(device_type.profile().username_suffix, "", "{device_type:?}");
        }
        assert_eq!(DeviceType::Mikrotik.profile().username_suffix, "+ct511w4098h");
    }

    #[test]
    fn zte_telnet_uses_port_23_and_crlf_with_zte_commands() {
        let zte_telnet = DeviceType::ZteTelnet.profile();
        assert_eq!(zte_telnet.protocol.default_port(), 23);
        assert_eq!(zte_telnet.enter, "\r\n");
        assert_eq!(zte_telnet.save_command, DeviceType::Zte.profile().save_command);
    }

    #[test]
    fn zte_inherits_cisco_modes_but_saves_with_write() {
        let zte = DeviceType::Zte.profile();
        assert_eq!(zte.config_enter_command, Some("configure terminal"));
        assert_eq!(zte.save_command, Some("write"));
    }
}
