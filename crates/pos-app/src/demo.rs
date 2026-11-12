//! Demo business with sample data, for trying Tillstone in five minutes
//! (PLAN §17) and for training.

use pos_core::{
    catalog::{ItemKind, ModePreset, ModifierGroup},
    ids::ModifierGroupId,
    money::{Currency, Money},
};

use crate::{BusinessSetup, Register, Result, modifier, simple_item, variant};

pub const DEMO_OWNER_PIN: &str = "1234";
pub const DEMO_CASHIER_PIN: &str = "1111";

pub fn setup() -> BusinessSetup {
    BusinessSetup {
        business_name: "Corner Café & Books".into(),
        currency: Currency::USD,
        owner_name: "Alex".into(),
        owner_pin: DEMO_OWNER_PIN.into(),
        revenue_centers: vec![
            ("Café".into(), ModePreset::QuickService),
            ("Books".into(), ModePreset::Retail),
        ],
        tax: Some(("Sales tax".into(), 825, false)),
        order_prefix: "A".into(),
    }
}

/// Fills a freshly created demo business with a menu, books and a cashier.
pub fn seed(reg: &mut Register) -> Result<()> {
    let currency = reg.currency();
    let usd = |m: i64| Money::from_minor(m, currency);
    let centers = reg.revenue_centers();
    let cafe = centers.iter().find(|c| c.name == "Café").map(|c| c.id).expect("demo café");
    let books = centers.iter().find(|c| c.name == "Books").map(|c| c.id).expect("demo books");
    let tax = reg.taxes().first().map(|t| t.id).into_iter().collect::<Vec<_>>();

    let milk = ModifierGroup {
        id: ModifierGroupId::new(),
        name: "Milk".into(),
        min: 0,
        max: 1,
        modifiers: vec![
            modifier("Oat milk", usd(75)),
            modifier("Almond milk", usd(75)),
            modifier("Whole milk", usd(0)),
        ],
    };
    let extras = ModifierGroup {
        id: ModifierGroupId::new(),
        name: "Extras".into(),
        min: 0,
        max: 3,
        modifiers: vec![
            modifier("Extra shot", usd(75)),
            modifier("Vanilla", usd(50)),
            modifier("Decaf", usd(0)),
        ],
    };
    let (milk_id, extras_id) = (milk.id, extras.id);
    reg.save_modifier_group(milk)?;
    reg.save_modifier_group(extras)?;

    for (name, price, mods) in [
        ("Espresso", 300, vec![extras_id]),
        ("Americano", 350, vec![extras_id]),
        ("Latte", 475, vec![milk_id, extras_id]),
        ("Cappuccino", 450, vec![milk_id, extras_id]),
        ("Flat white", 450, vec![milk_id, extras_id]),
        ("Mocha", 525, vec![milk_id, extras_id]),
        ("Cold brew", 450, vec![milk_id]),
        ("Butter croissant", 350, vec![]),
        ("Blueberry muffin", 375, vec![]),
    ] {
        let mut item = simple_item(name, ItemKind::MenuItem, usd(price), cafe, tax.clone(), mods);
        if name == "Latte" {
            item.variants = vec![variant("Small", usd(425)), variant("Large", usd(525))];
        }
        reg.save_item(item)?;
    }
    for (name, price, stock) in [
        ("The Overstory", 1899, 12),
        ("Braiding Sweetgrass", 2000, 4),
        ("Piranesi", 1700, 9),
        ("Klara and the Sun", 1695, 1),
        ("Tote bag", 1400, 31),
        ("Enamel mug", 1200, 18),
    ] {
        let item =
            simple_item(name, ItemKind::PhysicalGood, usd(price), books, tax.clone(), vec![]);
        let id = item.id;
        reg.save_item(item)?;
        reg.receive_stock(id, stock)?;
    }
    reg.add_staff("Maya", DEMO_CASHIER_PIN, crate::StaffRole::Cashier)?;
    Ok(())
}
