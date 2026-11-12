//! M2 exit criteria (PLAN §18): retail, quick-service, full-service and
//! hybrid orders, simulated across devices and verified.

use pos_core::{
    book::{Book, Checkpoint, Ingested},
    catalog::{DiscountAmount, ModePreset, RevenueCenter, RoundingScope, TaxRate},
    events::*,
    ids::*,
    ledger::{self, Context, DeviceLog, Record},
    money::{Currency, Money, Rounding},
    order::{Problem, Status},
    permissions::Role,
    pricing::{DiscountSnapshot, ModifierSnapshot, PricingRules},
};

const OWNER: [u8; 32] = [42; 32];
const LOCATION: LocationId = LocationId::from_u128(1);
const CAFE: RevenueCenterId = RevenueCenterId::from_u128(10);
const SHOP: RevenueCenterId = RevenueCenterId::from_u128(11);
const DINING: RevenueCenterId = RevenueCenterId::from_u128(12);
const FOOD_TAX: TaxId = TaxId::from_u128(20);
const MERCH_TAX: TaxId = TaxId::from_u128(21);
const CASHIER_ROLE: RoleId = RoleId::from_u128(30);
const MANAGER_ROLE: RoleId = RoleId::from_u128(31);
const OWNER_ROLE: RoleId = RoleId::from_u128(32);
const OWNER_STAFF: StaffId = StaffId::from_u128(42);
const CASHIER: StaffId = StaffId::from_u128(40);
const MANAGER: StaffId = StaffId::from_u128(41);
const MUG: ItemId = ItemId::from_u128(50);
const LATTE: ItemId = ItemId::from_u128(51);
const STEAK: ItemId = ItemId::from_u128(52);
const WINE: ItemId = ItemId::from_u128(53);

fn usd(m: i64) -> Money {
    Money::from_minor(m, Currency::USD)
}

fn food_tax() -> TaxRate {
    TaxRate {
        id: FOOD_TAX,
        name: "Food tax".into(),
        rate_bp: 500,
        inclusive: false,
        compound: false,
        rounding: Rounding::HalfUp,
        scope: RoundingScope::PerOrder,
    }
}

fn merch_tax() -> TaxRate {
    TaxRate {
        id: MERCH_TAX,
        name: "Sales tax".into(),
        rate_bp: 825,
        scope: RoundingScope::PerLine,
        ..food_tax()
    }
}

/// A simulated device: its own log, its own book, and a store of every
/// record it has (what `pos-store` keeps on disk).
struct Device {
    log: DeviceLog,
    book: Book,
    store: Vec<Vec<u8>>,
    now: u64,
    staff: Option<StaffId>,
    training: bool,
    orders: u32,
    prefix: &'static str,
}

impl Device {
    fn new(secret: u8, prefix: &'static str) -> Self {
        Self {
            log: DeviceLog::new(&[secret; 32]),
            book: Book::new(),
            store: vec![],
            now: 1_800_000_000_000,
            staff: None,
            training: false,
            orders: 0,
            prefix,
        }
    }

    fn id(&self) -> DeviceId {
        self.log.device()
    }

    fn ctx(&self, approved_by: Option<StaffId>) -> Context {
        Context { location: LOCATION, staff: self.staff, approved_by, training: self.training }
    }

    /// What problems would this event cause? (Doesn't append.)
    fn try_event(
        &mut self,
        event: Event,
        approved_by: Option<StaffId>,
    ) -> Result<Record, Vec<Problem>> {
        self.now += 1_000;
        if let Some(seen) = self.book.latest_hlc() {
            self.log.clock().observe(seen, self.now);
        }
        let ctx = self.ctx(approved_by);
        let header = self.log.prepare(self.now, ctx, &event);
        let problems = self.book.check(&header, &event);
        if !problems.is_empty() {
            return Err(problems);
        }
        let record = self.log.seal(header, &event);
        self.store.push(record.wire.clone());
        match self.book.ingest_verified(record.clone()).expect("own record accepted") {
            Ingested::Applied { warnings, .. } => assert!(warnings.is_empty(), "{warnings:?}"),
            other => panic!("own record not applied: {other:?}"),
        }
        Ok(record)
    }

    fn act(&mut self, event: Event) -> Record {
        let name = event.name();
        self.try_event(event, None).unwrap_or_else(|p| panic!("{name} refused: {p:?}"))
    }

    fn act_approved(&mut self, event: Event, by: StaffId) -> Record {
        self.try_event(event, Some(by)).expect("refused")
    }

    fn next_number(&mut self) -> String {
        self.orders += 1;
        format!("{}-{:04}", self.prefix, self.orders)
    }

    fn open(
        &mut self,
        center: RevenueCenterId,
        fulfillment: Fulfillment,
        table: Option<TableId>,
    ) -> OrderId {
        let order = OrderId::new();
        let number = self.next_number();
        self.act(Event::OrderOpened(OrderOpened {
            order,
            revenue_center: center,
            currency: Currency::USD,
            pricing_rules: PricingRules::CURRENT,
            number,
            fulfillment,
            table,
            guests: None,
            customer: None,
            tab_name: None,
        }));
        order
    }

    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        order: OrderId,
        item: ItemId,
        name: &str,
        center: RevenueCenterId,
        qty: i64,
        price: i64,
        tax: TaxRate,
        seat: Option<u8>,
    ) -> LineId {
        let line = LineId::new();
        self.act(Event::LineAdded(LineAdded {
            order,
            line,
            item,
            variant: None,
            name: name.into(),
            revenue_center: center,
            quantity: qty,
            unit_price: usd(price),
            modifiers: vec![],
            taxes: vec![tax],
            seat,
            course: None,
            track_stock: item == MUG,
        }));
        line
    }

    fn total(&self, order: OrderId) -> Money {
        self.book.order(order).unwrap().totals().unwrap().total
    }

    fn pay_cash_and_close(&mut self, order: OrderId, given: i64) -> Money {
        let total = self.total(order);
        self.act(Event::CashTendered(CashTendered {
            order,
            payment: PaymentId::new(),
            given: usd(given),
            applied: total,
        }));
        self.act(Event::OrderClosed(OrderClosed { order, total }));
        usd(given).checked_sub(total).unwrap()
    }
}

/// Sends every record each device has to the other.
fn sync(a: &mut Device, b: &mut Device) {
    for wire in a.store.clone() {
        deliver(b, &wire);
    }
    for wire in b.store.clone() {
        deliver(a, &wire);
    }
}

fn deliver(to: &mut Device, wire: &[u8]) {
    let outcome = to.book.ingest(wire).expect("peer record rejected");
    if !matches!(outcome, Ingested::Duplicate) && !to.store.iter().any(|w| w == wire) {
        to.store.push(wire.to_vec());
    }
}

fn device_added(founder_secret_device: DeviceId, role: DeviceRole, prefix: &str) -> Event {
    let mut e = DeviceAdded {
        device: founder_secret_device,
        name: format!("{prefix} device"),
        role,
        revenue_centers: vec![CAFE, SHOP, DINING],
        order_prefix: prefix.into(),
        owner_signature: [0; 64],
    };
    e.owner_signature = ledger::owner_sign(&OWNER, &e.signing_bytes());
    Event::DeviceAdded(e)
}

/// A business with a café, a gift shop and a dining room, a cashier and a
/// manager, and a register. Returns the register.
fn setup() -> Device {
    let mut reg = Device::new(1, "A");
    reg.act(Event::BusinessCreated(BusinessCreated {
        name: "Corner Café & Books".into(),
        currency: Currency::USD,
        owner_key: ledger::owner_public(&OWNER),
        first_location: LOCATION,
    }));
    let id = reg.id();
    reg.act(device_added(id, DeviceRole::Register, "A"));
    for record in [
        ConfigRecord::RevenueCenter(RevenueCenter::new(
            CAFE,
            LOCATION,
            "Café",
            ModePreset::QuickService,
        )),
        ConfigRecord::RevenueCenter(RevenueCenter::new(
            SHOP,
            LOCATION,
            "Gift shop",
            ModePreset::Retail,
        )),
        ConfigRecord::RevenueCenter(RevenueCenter::new(
            DINING,
            LOCATION,
            "Dining room",
            ModePreset::FullService,
        )),
        ConfigRecord::Tax(food_tax()),
        ConfigRecord::Tax(merch_tax()),
        ConfigRecord::Role(Role::cashier(CASHIER_ROLE)),
        ConfigRecord::Role(Role::manager(MANAGER_ROLE)),
        ConfigRecord::Role(Role::owner(OWNER_ROLE)),
        ConfigRecord::Staff(Staff {
            id: OWNER_STAFF,
            name: "Alex".into(),
            role: OWNER_ROLE,
            pin_hash: "x".into(),
            locations: vec![LOCATION],
            active: true,
        }),
    ] {
        reg.act(Event::ConfigChanged(ConfigChanged { record }));
    }
    // From now on, actions need a logged-in staff member with permission.
    // Only the owner may add staff.
    reg.staff = Some(OWNER_STAFF);
    for (id, name, role) in [(MANAGER, "Sam", MANAGER_ROLE), (CASHIER, "Maya", CASHIER_ROLE)] {
        reg.act(Event::ConfigChanged(ConfigChanged {
            record: ConfigRecord::Staff(Staff {
                id,
                name: name.into(),
                role,
                pin_hash: "x".into(),
                locations: vec![LOCATION],
                active: true,
            }),
        }));
    }
    reg.staff = Some(CASHIER);
    reg
}

fn add_handheld(reg: &mut Device, secret: u8, prefix: &'static str) -> Device {
    let mut hh = Device::new(secret, prefix);
    reg.staff = Some(MANAGER);
    reg.act(device_added(hh.id(), DeviceRole::Handheld, prefix));
    reg.staff = Some(CASHIER);
    sync(reg, &mut hh);
    hh.staff = Some(CASHIER);
    hh
}

#[test]
fn retail_sale_with_stock_and_refund() {
    let mut reg = setup();
    reg.staff = Some(MANAGER);
    reg.act(Event::InventoryAdjusted(InventoryAdjusted {
        item: MUG,
        delta: 10,
        reason: "received".into(),
    }));
    reg.staff = Some(CASHIER);

    let shift = ShiftId::new();
    reg.act(Event::ShiftOpened(ShiftOpened { shift, float: usd(10_000) }));

    let order = reg.open(SHOP, Fulfillment::Here, None);
    let mug = reg.add(order, MUG, "Mug", SHOP, 2, 1_200, merch_tax(), None);
    // 2 × 12.00 = 24.00 + 8.25 % = 1.98 → 25.98
    assert_eq!(reg.total(order), usd(2_598));
    let change = reg.pay_cash_and_close(order, 3_000);
    assert_eq!(change, usd(402));
    assert_eq!(reg.book.stock(MUG), 8);

    // Refund one mug: the cashier can't approve it alone.
    let refund = Event::RefundIssued(RefundIssued {
        order,
        refund: RefundId::new(),
        lines: vec![(mug, 1)],
        amount: usd(1_299),
        tender: Tender::Cash,
        reason: "chipped".into(),
    });
    assert!(
        matches!(reg.try_event(refund.clone(), None), Err(p) if p.contains(&Problem::NotPermitted(pos_core::permissions::Permission::RefundApprove)))
    );
    reg.act_approved(refund, MANAGER);
    assert_eq!(reg.book.stock(MUG), 9);

    // Blind close: counted 112.00, expected 100 + 25.98 − 12.99 = 112.99.
    reg.act(Event::ShiftClosed(ShiftClosed { shift, counted: usd(11_200) }));
    let s = reg.book.shift(shift).unwrap();
    assert_eq!(s.expected(), usd(11_299));
    assert_eq!(s.variance(), Some(usd(-99)));

    let report = reg.book.sales_report();
    assert_eq!(report.orders, 1);
    assert_eq!(report.refunds, usd(1_299));
    assert_eq!(report.by_tender["Cash"], usd(2_598));
}

#[test]
fn quick_service_with_modifiers_discount_and_card_tip() {
    let mut reg = setup();
    let order = reg.open(CAFE, Fulfillment::ToGo, None);
    let line = LineId::new();
    reg.act(Event::LineAdded(LineAdded {
        order,
        line,
        item: LATTE,
        variant: None,
        name: "Latte".into(),
        revenue_center: CAFE,
        quantity: 1,
        unit_price: usd(475),
        modifiers: vec![
            ModifierSnapshot { name: "Oat milk".into(), price: usd(75) },
            ModifierSnapshot { name: "Extra shot".into(), price: usd(75) },
        ],
        taxes: vec![food_tax()],
        seat: None,
        course: None,
        track_stock: false,
    }));
    reg.act(Event::OrderSent(OrderSent { order, lines: vec![line] }));
    // A 10 % staff discount is within a cashier's limits.
    reg.act(Event::DiscountApplied(DiscountApplied {
        order,
        line: None,
        discount: DiscountSnapshot {
            name: "Staff 10%".into(),
            amount: DiscountAmount::Percent(1000),
        },
    }));
    // 6.25 − 0.63 = 5.62 + 5 % = 0.28 → 5.90
    let total = reg.total(order);
    assert_eq!(total, usd(590));

    // A 50 % discount needs a manager.
    let big = Event::DiscountApplied(DiscountApplied {
        order,
        line: None,
        discount: DiscountSnapshot {
            name: "Half off".into(),
            amount: DiscountAmount::Percent(5000),
        },
    });
    assert!(reg.try_event(big, None).is_err());

    let payment = PaymentId::new();
    reg.act(Event::PaymentStarted(PaymentStarted {
        order,
        payment,
        tender: Tender::Card { provider: "stripe".into(), reader: Some("front".into()) },
        amount: total,
    }));
    // Can't close until the card payment is captured.
    assert!(
        matches!(reg.try_event(Event::OrderClosed(OrderClosed { order, total }), None), Err(p) if matches!(p[0], Problem::Underpaid { .. }))
    );
    reg.act(Event::PaymentCaptured(PaymentCaptured {
        order,
        payment,
        provider_ref: Some("pi_123".into()),
        tip: usd(100),
    }));
    reg.act(Event::OrderClosed(OrderClosed { order, total }));

    let report = reg.book.sales_report();
    assert_eq!(report.tips, usd(100));
    assert_eq!(report.by_tender["Card (stripe)"], usd(590));
}

#[test]
fn full_service_shared_check_split_by_seat() {
    let mut reg = setup();
    let mut hh = add_handheld(&mut reg, 2, "B");
    let table = TableId::from_u128(7);

    let order = reg.open(DINING, Fulfillment::Here, Some(table));
    sync(&mut reg, &mut hh);

    // Both devices add to the same check at the same time, offline from each other.
    let steak = reg.add(order, STEAK, "Steak", DINING, 1, 3_200, food_tax(), Some(1));
    let wine = hh.add(order, WINE, "Wine", DINING, 2, 1_100, food_tax(), Some(2));
    sync(&mut reg, &mut hh);
    assert_eq!(reg.book.order(order), hh.book.order(order), "devices converge");
    assert_eq!(reg.book.order(order).unwrap().lines.len(), 2);

    reg.act(Event::OrderSent(OrderSent { order, lines: vec![steak, wine] }));
    sync(&mut reg, &mut hh);

    // The handheld can't split: the register opened the check and nobody holds a lease.
    let snapshot = |d: &Device, line: LineId| {
        d.book
            .order(order)
            .unwrap()
            .lines
            .iter()
            .find(|l| l.snapshot.line == line)
            .unwrap()
            .snapshot
            .clone()
    };
    let seat2 = OrderId::new();
    let split = Event::OrderSplit(OrderSplit {
        from: order,
        to: seat2,
        number: "B-0002".into(),
        revenue_center: DINING,
        currency: Currency::USD,
        pricing_rules: PricingRules::CURRENT,
        fulfillment: Fulfillment::Here,
        table: Some(table),
        lines: vec![snapshot(&hh, wine)],
    });
    assert!(
        matches!(hh.try_event(split.clone(), None), Err(p) if matches!(p[0], Problem::LeaseHeldByAnotherDevice { .. }))
    );

    // With the lease, it can.
    hh.act(Event::CheckLeaseTaken(CheckLeaseTaken { order, until_ms: hh.now + 120_000 }));
    hh.act(split);
    hh.act(Event::CheckLeaseReleased(CheckLeaseReleased { order }));
    sync(&mut reg, &mut hh);

    // Seat 1: steak 32.00 + 5 % = 33.60. Seat 2: wine 22.00 + 5 % = 23.10.
    assert_eq!(reg.total(order), usd(3_360));
    assert_eq!(reg.total(seat2), usd(2_310));
    reg.pay_cash_and_close(order, 4_000);
    hh.pay_cash_and_close(seat2, 2_310);
    sync(&mut reg, &mut hh);

    assert_eq!(reg.book.order(order).unwrap().status, Status::Closed);
    assert_eq!(hh.book.order(seat2).unwrap().status, Status::Closed);
    assert_eq!(reg.book.sales_report(), hh.book.sales_report());
    assert_eq!(reg.book.sales_report().total, usd(5_670));
}

#[test]
fn hybrid_gift_shop_item_on_a_dinner_check() {
    let mut reg = setup();
    let order = reg.open(DINING, Fulfillment::Here, Some(TableId::from_u128(3)));
    reg.add(order, STEAK, "Steak", DINING, 1, 3_200, food_tax(), None);
    reg.add(order, MUG, "Mug", SHOP, 1, 1_200, merch_tax(), None);
    // 32.00 + 1.60 food tax + 12.00 + 0.99 sales tax = 46.59
    assert_eq!(reg.total(order), usd(4_659));
    reg.pay_cash_and_close(order, 5_000);

    let report = reg.book.sales_report();
    assert_eq!(report.net_by_revenue_center[&DINING], usd(3_200));
    assert_eq!(
        report.net_by_revenue_center[&SHOP],
        usd(1_200),
        "the mug is credited to the gift shop"
    );
    assert_eq!(report.taxes[&FOOD_TAX], usd(160));
    assert_eq!(report.taxes[&MERCH_TAX], usd(99));
}

#[test]
fn modules_are_enforced() {
    let mut reg = setup();
    let open = |table| {
        Event::OrderOpened(OrderOpened {
            order: OrderId::new(),
            revenue_center: CAFE,
            currency: Currency::USD,
            pricing_rules: PricingRules::CURRENT,
            number: "A-9".into(),
            fulfillment: Fulfillment::Here,
            table,
            guests: None,
            customer: None,
            tab_name: None,
        })
    };
    // The café is quick service: no tables.
    let problems = reg.try_event(open(Some(TableId::from_u128(1))), None).unwrap_err();
    assert_eq!(problems, vec![Problem::ModuleDisabled("tables".into())]);
    assert!(reg.try_event(open(None), None).is_ok());
}

#[test]
fn concurrent_payments_during_a_network_split_keep_the_money() {
    let mut reg = setup();
    let mut hh = add_handheld(&mut reg, 2, "B");
    let order = reg.open(DINING, Fulfillment::Here, None);
    reg.add(order, STEAK, "Steak", DINING, 1, 2_000, food_tax(), None);
    sync(&mut reg, &mut hh);
    let total = reg.total(order);

    // The network splits. The server at the handheld takes the lease and is
    // paid in cash; meanwhile the register (which opened the check and never
    // saw the lease) is paid in cash too.
    hh.act(Event::CheckLeaseTaken(CheckLeaseTaken { order, until_ms: hh.now + 120_000 }));
    hh.act(Event::CashTendered(CashTendered {
        order,
        payment: PaymentId::new(),
        given: total,
        applied: total,
    }));
    hh.act(Event::OrderClosed(OrderClosed { order, total }));
    reg.act(Event::CashTendered(CashTendered {
        order,
        payment: PaymentId::new(),
        given: total,
        applied: total,
    }));

    // When they reconnect, both payments are kept and the check shows as overpaid.
    sync(&mut reg, &mut hh);
    let o = reg.book.order(order).unwrap();
    assert_eq!(o.paid(), total.checked_mul(2).unwrap());
    assert_eq!(o.status, Status::Closed);
    assert_eq!(reg.book.order(order), hh.book.order(order));
    assert!(reg.book.order_problems(order).iter().any(|(_, p)| matches!(
        p,
        Problem::Overpaid { .. } | Problem::OrderNotOpen | Problem::LeaseHeldByAnotherDevice { .. }
    )));

    // But a device that *knows* about the lease refuses to take payment.
    let r = reg.try_event(
        Event::CashTendered(CashTendered {
            order,
            payment: PaymentId::new(),
            given: total,
            applied: total,
        }),
        None,
    );
    assert!(r.is_err());
}

#[test]
fn records_from_a_revoked_device_after_its_cutoff_are_refused() {
    let mut reg = setup();
    let mut hh = add_handheld(&mut reg, 2, "B");
    let order = hh.open(CAFE, Fulfillment::Here, None);
    sync(&mut reg, &mut hh);
    let cutoff = hh.log.device();
    let trusted = reg.book.version_vector().into_iter().find(|(d, _)| *d == cutoff).unwrap().1;

    // The handheld is stolen; the owner revokes it.
    reg.staff = Some(MANAGER);
    let mut revoke =
        DeviceRevoked { device: cutoff, trusted_through_seq: trusted, owner_signature: [0; 64] };
    revoke.owner_signature = ledger::owner_sign(&OWNER, &revoke.signing_bytes());
    reg.act(Event::DeviceRevoked(revoke));

    // Anything it writes afterwards is refused by the register.
    let late = hh.add(order, LATTE, "Latte", CAFE, 1, 475, food_tax(), None);
    let _ = late;
    let wire = hh.store.last().unwrap().clone();
    assert!(reg.book.ingest(&wire).is_err());
}

#[test]
fn membership_changes_need_the_owner() {
    let mut reg = setup();
    reg.staff = Some(MANAGER);
    let mut forged = DeviceAdded {
        device: DeviceId([9; 32]),
        name: "rogue".into(),
        role: DeviceRole::Register,
        revenue_centers: vec![],
        order_prefix: "Z".into(),
        owner_signature: [0; 64],
    };
    forged.owner_signature = ledger::owner_sign(&[1; 32], &forged.signing_bytes()); // wrong key
    let header = reg.log.prepare(reg.now + 1, reg.ctx(None), &Event::DeviceAdded(forged.clone()));
    let record = reg.log.seal(header, &Event::DeviceAdded(forged));
    assert!(reg.book.ingest(&record.wire).is_err());
}

#[test]
fn every_delivery_order_converges() {
    // Build a busy history on two devices, then deliver all records to a
    // third device in many different orders: it always ends the same.
    let mut reg = setup();
    let mut hh = add_handheld(&mut reg, 2, "B");
    for i in 0..3 {
        let o = reg.open(DINING, Fulfillment::Here, Some(TableId::from_u128(i)));
        sync(&mut reg, &mut hh);
        reg.add(o, STEAK, "Steak", DINING, 1, 3_000, food_tax(), Some(1));
        hh.add(o, WINE, "Wine", DINING, 1, 900, food_tax(), Some(2));
        sync(&mut reg, &mut hh);
        reg.pay_cash_and_close(o, 5_000);
    }
    sync(&mut reg, &mut hh);
    let expected = reg.book.sales_report();

    let all: Vec<Vec<u8>> = reg.store.clone();
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    for _ in 0..25 {
        let mut shuffled = all.clone();
        for i in (1..shuffled.len()).rev() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            shuffled.swap(i, (seed % (i as u64 + 1)) as usize);
        }
        let mut book = Book::new();
        for w in &shuffled {
            book.ingest(w).unwrap();
        }
        assert_eq!(book.pending_count(), 0);
        assert_eq!(book.sales_report(), expected);
    }
}

#[test]
fn checkpoints_restore_the_same_state() {
    let mut reg = setup();
    let order = reg.open(CAFE, Fulfillment::Here, None);
    reg.add(order, LATTE, "Latte", CAFE, 2, 475, food_tax(), None);
    let open = reg.open(CAFE, Fulfillment::ToGo, None);
    reg.add(open, LATTE, "Latte", CAFE, 1, 475, food_tax(), None);
    reg.pay_cash_and_close(order, 2_000);

    let bytes = reg.book.checkpoint().encode();
    let restored = Book::restore(Checkpoint::decode(&bytes).unwrap()).unwrap();
    assert_eq!(restored.sales_report(), reg.book.sales_report());
    assert_eq!(restored.order(open), reg.book.order(open));
    assert_eq!(restored.version_vector(), reg.book.version_vector());
    // And it keeps accepting new records afterwards.
    let mut reg2 = reg;
    reg2.book = restored;
    reg2.pay_cash_and_close(open, 1_000);
}

#[test]
fn fiscal_mode_requires_a_signature() {
    let mut reg = setup();
    reg.staff = Some(MANAGER);
    let mut cafe = RevenueCenter::new(CAFE, LOCATION, "Café", ModePreset::QuickService);
    cafe.fiscal_mode = true;
    reg.act(Event::ConfigChanged(ConfigChanged { record: ConfigRecord::RevenueCenter(cafe) }));
    let order = reg.open(CAFE, Fulfillment::Here, None);
    reg.add(order, LATTE, "Latte", CAFE, 1, 475, food_tax(), None);
    let total = reg.total(order);
    reg.act(Event::CashTendered(CashTendered {
        order,
        payment: PaymentId::new(),
        given: total,
        applied: total,
    }));
    let close = Event::OrderClosed(OrderClosed { order, total });
    assert_eq!(
        reg.try_event(close.clone(), None).unwrap_err(),
        vec![Problem::FiscalSignatureMissing]
    );
    reg.act(Event::FiscalSigned(FiscalSigned {
        order,
        adapter: "test".into(),
        fields: vec![("sig".into(), "abc".into())],
    }));
    reg.act(close);
}

#[test]
fn training_orders_never_count() {
    let mut reg = setup();
    reg.training = true;
    let order = reg.open(CAFE, Fulfillment::Here, None);
    reg.add(order, LATTE, "Latte", CAFE, 1, 475, food_tax(), None);
    reg.pay_cash_and_close(order, 500);
    assert_eq!(reg.book.sales_report().orders, 0);
}

#[test]
fn a_wrong_total_from_another_device_is_a_warning_not_a_rejection() {
    let mut reg = setup();
    let mut hh = add_handheld(&mut reg, 2, "B");
    let order = hh.open(CAFE, Fulfillment::Here, None);
    hh.add(order, LATTE, "Latte", CAFE, 1, 475, food_tax(), None);
    let total = hh.total(order);
    hh.act(Event::CashTendered(CashTendered {
        order,
        payment: PaymentId::new(),
        given: total,
        applied: total,
    }));
    // A buggy device reports the wrong total (bypassing its own checks).
    let bad = Event::OrderClosed(OrderClosed { order, total: usd(1) });
    hh.log.clock().observe(hh.book.latest_hlc().unwrap(), hh.now);
    let header = hh.log.prepare(hh.now + 1, hh.ctx(None), &bad);
    let record = hh.log.seal(header, &bad);
    hh.store.push(record.wire.clone());
    sync(&mut reg, &mut hh);
    assert_eq!(reg.book.order(order).unwrap().status, Status::Closed);
    assert!(reg.book.warnings().iter().any(|w| matches!(w.problem, Problem::TotalMismatch { .. })));
}
