use russh::client::{Config, GexParams};
use russh::keys::Algorithm;
use russh::{Preferred, cipher, kex, mac};

// Not legacy, but russh leaves ECDH out of its defaults (libssh2 offered it) and some IOS/VRP images offer nothing newer.
const ECDH_KEX: &[kex::Name] = &[kex::ECDH_SHA2_NISTP256, kex::ECDH_SHA2_NISTP384, kex::ECDH_SHA2_NISTP521];
const LEGACY_KEX: &[kex::Name] = &[
    kex::DH_G14_SHA1,
    kex::DH_GEX_SHA1,
    kex::DH_G1_SHA1,
];
const LEGACY_HOST_KEYS: &[Algorithm] = &[Algorithm::Dsa];
const LEGACY_CIPHERS: &[cipher::Name] =
    &[cipher::AES_256_CBC, cipher::AES_192_CBC, cipher::AES_128_CBC, cipher::TRIPLE_DES_CBC];
const LEGACY_MACS: &[mac::Name] = &[mac::HMAC_SHA1_ETM, mac::HMAC_SHA1];
// russh asks for 3072..8192-bit groups; old devices often top out at 2048 (russh's floor), and 8192 is slow.
const LEGACY_GEX_BITS: (usize, usize, usize) = (2048, 2048, 8192);

/// Client config with russh defaults, widened for old devices when `include_legacy` is set.
pub fn client_config(include_legacy: bool) -> Config {
    let mut config = Config { preferred: preferred_algorithms(include_legacy), ..Default::default() };
    if include_legacy {
        let (min, preferred, max) = LEGACY_GEX_BITS;
        config.gex = GexParams::new(min, preferred, max).unwrap_or_default();
    }
    config
}

/// russh's modern defaults plus ECDH, followed by legacy algorithms when `include_legacy` is set.
fn preferred_algorithms(include_legacy: bool) -> Preferred {
    let defaults = Preferred::default();
    let modern = Preferred { kex: [&defaults.kex[..], ECDH_KEX].concat().into(), ..defaults };
    if !include_legacy {
        return modern;
    }
    // ssh-rsa (SHA-1) host keys are already in russh's defaults.
    Preferred {
        kex: [&modern.kex[..], LEGACY_KEX].concat().into(),
        key: [&modern.key[..], LEGACY_HOST_KEYS].concat().into(),
        cipher: [&modern.cipher[..], LEGACY_CIPHERS].concat().into(),
        mac: [&modern.mac[..], LEGACY_MACS].concat().into(),
        ..modern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names<T: AsRef<str>>(list: &[T]) -> Vec<&str> {
        list.iter().map(AsRef::as_ref).collect()
    }

    #[test]
    fn keeps_russh_defaults_plus_ecdh_when_legacy_is_off() {
        let preferred = preferred_algorithms(false);
        let defaults = Preferred::default();
        assert_eq!(names(&preferred.kex[..defaults.kex.len()]), names(&defaults.kex));
        assert_eq!(names(&preferred.kex[defaults.kex.len()..]), names(ECDH_KEX));
        assert!(!names(&preferred.cipher).contains(&"3des-cbc"));
    }

    #[test]
    fn appends_legacy_algorithms_after_modern_defaults() {
        let modern = preferred_algorithms(false);
        let preferred = preferred_algorithms(true);
        assert_eq!(names(&preferred.kex[..modern.kex.len()]), names(&modern.kex));
        assert_eq!(names(&preferred.kex[modern.kex.len()..]), names(LEGACY_KEX));
        assert_eq!(names(&preferred.cipher[modern.cipher.len()..]), names(LEGACY_CIPHERS));
        assert_eq!(names(&preferred.mac[modern.mac.len()..]), names(LEGACY_MACS));
        assert_eq!(preferred.key.last(), Some(&Algorithm::Dsa));
    }

    #[test]
    fn accepts_2048_bit_group_exchange_only_in_legacy_mode() {
        assert_eq!(client_config(false).gex.min_group_size(), GexParams::default().min_group_size());
        let legacy = client_config(true);
        assert_eq!(legacy.gex.min_group_size(), 2048);
        assert_eq!(legacy.gex.preferred_group_size(), 2048);
        assert_eq!(names(&legacy.preferred.cipher), names(&preferred_algorithms(true).cipher));
    }

    #[test]
    fn offers_the_algorithms_old_routers_need() {
        let preferred = preferred_algorithms(true);
        let kex = names(&preferred.kex);
        for name in ["diffie-hellman-group14-sha1", "diffie-hellman-group1-sha1", "diffie-hellman-group-exchange-sha1"] {
            assert!(kex.contains(&name), "{name}");
        }
        assert!(names(&preferred.cipher).contains(&"3des-cbc"));
        assert!(names(&preferred.mac).contains(&"hmac-sha1"));
        assert!(preferred.key.contains(&Algorithm::Rsa { hash: None }));
    }
}
