//! A day at a single register (M3): setup, sign-in, sales, cash, shifts,
//! restart, training mode and permissions.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use pos_app::{AppError, Register, StaffRole, demo};
use pos_core::{
    events::{CashMovementKind, Fulfillment},
    ids::RevenueCenterId,
    money::Money,
    order::{Problem, Status},
    permissions::Permission,
};

const DB_KEY: [u8; 32] = [5; 32];
const DEVICE: [u8; 32] = [6; 32];
const OWNER: [u8; 32] = [7; 32];

fn usd(m: i64) -> Money {
    Money::from_minor(m, pos_core::money::Currency::USD)
}

fn temp_db(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tillstone-app-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("register.db")
}

/// A controllable clock, advancing one second per reading.
fn clock() -> (Box<dyn Fn() -> u64 + Send>, Arc<AtomicU64>) {
    let t = Arc::new(AtomicU64::new(1_800_000_000_000));
    let t2 = t.clone();
    (Box::new(move || t2.fetch_add(1_000, Ordering::SeqCst)), t)
}

fn demo_register(path: &std::path::Path) -> (Register, Arc<AtomicU64>) {
    let (clock, t) = clock();
    let mut reg =
        Register::create_with_clock(path, &DB_KEY, &DEVICE, &OWNER, demo::setup(), clock).unwrap();
    demo::seed(&mut reg).unwrap();
    (reg, t)
}

fn cafe(reg: &Register) -> RevenueCenterId {
    reg.revenue_centers().into_iter().find(|c| c.name == "Café").unwrap().id
}

fn item(reg: &Register, center: RevenueCenterId, name: &str) -> pos_core::catalog::Item {
    reg.items(center).into_iter().find(|i| i.name == name).unwrap()
}

#[test]
fn a_quick_service_sale_with_modifiers_discount_and_cash() {
    let path = temp_db("sale");
    let (mut reg, _) = demo_register(&path);
    reg.sign_out();
    assert_eq!(reg.sign_in(demo::DEMO_CASHIER_PIN).unwrap().name, "Maya");

    let shift = reg.open_shift(usd(10_000)).unwrap();
    let cafe = cafe(&reg);
    let order = reg.open_order(cafe, Fulfillment::Here).unwrap();

    let latte = item(&reg, cafe, "Latte");
    let large = latte.variants.iter().find(|v| v.name == "Large").unwrap().id;
    let milk = reg.modifier_group(latte.modifier_groups[0]).unwrap();
    let oat = milk.modifiers.iter().find(|m| m.name == "Oat milk").unwrap().id;
    reg.add_item(order, latte.id, Some(large), &[oat], 1).unwrap();
    let croissant = item(&reg, cafe, "Butter croissant");
    let line = reg.add_item(order, croissant.id, None, &[], 1).unwrap();
    reg.set_quantity(order, line, 2).unwrap();

    // Large latte 5.25 + oat 0.75 = 6.00; croissants 7.00 → 13.00
    // 10 % staff discount → 11.70; tax 8.25 % per line → 0.45 + 0.52 = 0.97 → 12.67
    reg.discount_order(order, "Staff 10%", 1000, None).unwrap();
    let o = reg.order(order).unwrap();
    assert_eq!(o.number, "A1");
    assert_eq!(o.totals().unwrap().total, usd(1_267));

    let paid = reg.pay_cash(order, usd(2_000)).unwrap();
    assert_eq!(paid.change, usd(733));
    assert!(paid.closed);
    assert_eq!(reg.order(order).unwrap().status, Status::Closed);

    let summary = reg.close_shift(shift, usd(11_267)).unwrap();
    assert_eq!(summary.expected(), usd(11_267));
    assert_eq!(summary.variance(), Some(usd(0)));
    assert_eq!(reg.report().orders, 1);
}

#[test]
fn split_cash_payments_close_only_when_fully_paid() {
    let path = temp_db("split-cash");
    let (mut reg, _) = demo_register(&path);
    let cafe = cafe(&reg);
    let order = reg.open_order(cafe, Fulfillment::ToGo).unwrap();
    reg.add_item(order, item(&reg, cafe, "Mocha").id, None, &[], 2).unwrap(); // 10.50 + 0.87 = 11.37
    let first = reg.pay_cash(order, usd(500)).unwrap();
    assert!(!first.closed);
    assert_eq!(reg.order(order).unwrap().balance(), usd(637));
    let second = reg.pay_cash(order, usd(1_000)).unwrap();
    assert!(second.closed);
    assert_eq!(second.change, usd(363));
}

#[test]
fn everything_survives_a_restart() {
    let path = temp_db("restart");
    let (mut reg, t) = demo_register(&path);
    let cafe_id = cafe(&reg);
    let order = reg.open_order(cafe_id, Fulfillment::Here).unwrap();
    reg.add_item(order, item(&reg, cafe_id, "Espresso").id, None, &[], 1).unwrap();
    let before = reg.order(order).cloned();
    drop(reg); // the app is killed

    let t2 = t.clone();
    let clock: Box<dyn Fn() -> u64 + Send> =
        Box::new(move || t2.fetch_add(1_000, Ordering::SeqCst));
    let mut reg = Register::open_with_clock(&path, &DB_KEY, &DEVICE, clock).unwrap();
    assert_eq!(reg.order(order).cloned(), before);
    assert_eq!(reg.open_orders().len(), 1);
    // The log continues where it left off: new actions chain correctly.
    reg.sign_in(demo::DEMO_CASHIER_PIN).unwrap();
    reg.add_item(order, item(&reg, cafe_id, "Americano").id, None, &[], 1).unwrap();
    reg.pay_cash(order, usd(1_000)).unwrap();
    reg.checkpoint().unwrap();
    drop(reg);

    let clock: Box<dyn Fn() -> u64 + Send> = Box::new(move || t.fetch_add(1_000, Ordering::SeqCst));
    let reg = Register::open_with_clock(&path, &DB_KEY, &DEVICE, clock).unwrap();
    assert_eq!(reg.order(order).unwrap().status, Status::Closed);
    // 3.00 + 3.50 = 6.50 + 0.25 + 0.29 tax = 7.04
    assert_eq!(reg.report().total, usd(704));
}

#[test]
fn wrong_pins_lock_sign_in() {
    let path = temp_db("lockout");
    let (mut reg, t) = demo_register(&path);
    reg.sign_out();
    for left in (1..pos_app::PIN_ATTEMPTS).rev() {
        assert!(
            matches!(reg.sign_in("9999"), Err(AppError::WrongPin { attempts_left }) if attempts_left == left)
        );
    }
    assert!(matches!(reg.sign_in("9999"), Err(AppError::LockedOut { .. })));
    // Even the right PIN is refused while locked.
    assert!(matches!(reg.sign_in(demo::DEMO_CASHIER_PIN), Err(AppError::LockedOut { .. })));
    t.fetch_add(pos_app::LOCKOUT_MS + 1, Ordering::SeqCst);
    assert!(reg.sign_in(demo::DEMO_CASHIER_PIN).is_ok());
}

#[test]
fn cashiers_have_limits() {
    let path = temp_db("limits");
    let (mut reg, _) = demo_register(&path);
    reg.sign_out();
    reg.sign_in(demo::DEMO_CASHIER_PIN).unwrap();

    // Can't add staff.
    let err = reg.add_staff("Eve", "4321", StaffRole::Owner).unwrap_err();
    assert!(
        matches!(err, AppError::Refused(p) if p.contains(&Problem::NotPermitted(Permission::ManageStaff)))
    );

    // A 50 % discount needs a manager's or owner's PIN.
    let cafe = cafe(&reg);
    let order = reg.open_order(cafe, Fulfillment::Here).unwrap();
    reg.add_item(order, item(&reg, cafe, "Latte").id, None, &[], 1).unwrap();
    assert!(reg.discount_order(order, "Half off", 5000, None).is_err());
    assert!(reg.discount_order(order, "Half off", 5000, Some("0000")).is_err());
    reg.discount_order(order, "Half off", 5000, Some(demo::DEMO_OWNER_PIN)).unwrap();

    // No-sale drawer opens need permission.
    let shift = reg.open_shift(usd(5_000)).unwrap();
    assert!(
        reg.cash_movement(shift, CashMovementKind::NoSale, usd(0), "change for a twenty").is_err()
    );
}

#[test]
fn training_mode_sales_never_count() {
    let path = temp_db("training");
    let (mut reg, _) = demo_register(&path);
    reg.set_training(true);
    let cafe = cafe(&reg);
    let order = reg.open_order(cafe, Fulfillment::Here).unwrap();
    reg.add_item(order, item(&reg, cafe, "Espresso").id, None, &[], 1).unwrap();
    reg.pay_cash(order, usd(500)).unwrap();
    assert_eq!(reg.report().orders, 0);
}

#[test]
fn selling_books_reduces_stock() {
    let path = temp_db("stock");
    let (mut reg, _) = demo_register(&path);
    let books = reg.revenue_centers().into_iter().find(|c| c.name == "Books").unwrap().id;
    let book = item(&reg, books, "Piranesi");
    assert_eq!(reg.stock(book.id), 9);
    let order = reg.open_order(books, Fulfillment::Here).unwrap();
    reg.add_item(order, book.id, None, &[], 2).unwrap();
    reg.pay_cash(order, usd(5_000)).unwrap();
    assert_eq!(reg.stock(book.id), 7);
}

#[test]
fn a_device_can_only_create_one_business() {
    let path = temp_db("twice");
    let (reg, _) = demo_register(&path);
    drop(reg);
    let (clock, _) = clock();
    assert!(
        Register::create_with_clock(&path, &DB_KEY, &DEVICE, &OWNER, demo::setup(), clock).is_err()
    );
}
