//! Demo business with sample data, for trying Tillstone in five minutes
//! (PLAN §17) and for training. The data lives in `templates/demo.json`, so
//! a fork can ship its own demo by editing that file.

use crate::{BusinessSetup, Register, Result, template};

pub const DEMO_OWNER_PIN: &str = "1234";
pub const DEMO_CASHIER_PIN: &str = "1111";
/// The demo's owner passphrase. A demo isn't secret; real setups choose their own.
pub const DEMO_PASSPHRASE: &str = "demo passphrase";

fn template() -> template::Template {
    template::get("demo").expect("templates/demo.json is checked by the template tests")
}

pub fn setup() -> BusinessSetup {
    template().demo_setup().expect("templates/demo.json is checked by the template tests")
}

/// Fills a freshly created demo business with its menu, books and cashier.
pub fn seed(reg: &mut Register) -> Result<()> {
    template().apply(reg)
}

/// Who can sign in to the demo, and with which PIN, for the sign-in screen.
pub fn pins() -> Vec<(String, String)> {
    let t = template();
    let mut out = vec![];
    if let Some(d) = &t.demo {
        out.push((format!("{} (owner)", d.owner_name), d.owner_pin.clone()));
    }
    for s in &t.staff {
        let role = match s.role {
            template::Role::Cashier => "cashier",
            template::Role::Manager => "manager",
        };
        out.push((format!("{} ({role})", s.name), s.pin.clone()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_the_demo_template() {
        let pins = pins();
        assert!(pins.iter().any(|(_, p)| p == DEMO_OWNER_PIN));
        assert!(pins.iter().any(|(_, p)| p == DEMO_CASHIER_PIN));
        crate::owner_key::check_passphrase(DEMO_PASSPHRASE).unwrap();
    }
}
