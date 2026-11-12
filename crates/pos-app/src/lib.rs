//! Application services for a single register (PLAN §18, M3).
//!
//! [`Register`] ties the pieces together: every action is checked strictly
//! by the Book, signed into this device's log, written to encrypted storage,
//! and only then applied. If any step fails, nothing is recorded.
//!
//! This layer holds no business rules of its own; it turns user intentions
//! ("add a latte", "take $20 cash") into events.

use std::path::Path;

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use pos_core::{
    book::{Book, Ingested, SalesReport, Shift},
    catalog::{
        DiscountAmount, Item, ItemKind, ModePreset, Modifier, ModifierGroup, RevenueCenter,
        RoundingScope, TaxRate, Variant,
    },
    events::*,
    ids::*,
    ledger::{self, Context, DeviceLog},
    money::{Currency, Money, Rounding},
    order::{Order, Problem, Status},
    permissions::Role,
    pricing::{DiscountSnapshot, ModifierSnapshot, PricingRules},
};
use pos_store::{Store, StoreError};

pub mod demo;
pub mod owner_key;
pub mod template;

/// Wrong PINs allowed before sign-in locks (PLAN §9).
pub const PIN_ATTEMPTS: u32 = 5;
/// First lockout; doubles with each further failure.
pub const LOCKOUT_MS: u64 = 30_000;

#[derive(Debug)]
pub enum AppError {
    Store(StoreError),
    /// The action was refused; nothing was recorded.
    Refused(Vec<Problem>),
    WrongPin {
        attempts_left: u32,
    },
    LockedOut {
        retry_in_ms: u64,
    },
    NotSignedIn,
    NotFound(&'static str),
    Invalid(&'static str),
    Rejected(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(e) => write!(f, "{e}"),
            Self::Refused(p) => write!(f, "not allowed: {p:?}"),
            Self::WrongPin { attempts_left } => {
                write!(f, "wrong PIN ({attempts_left} attempts left)")
            }
            Self::LockedOut { retry_in_ms } => {
                write!(f, "too many wrong PINs; try again in {}s", retry_in_ms / 1000 + 1)
            }
            Self::NotSignedIn => f.write_str("sign in first"),
            Self::NotFound(what) => write!(f, "{what} not found"),
            Self::Invalid(why) => f.write_str(why),
            Self::Rejected(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for AppError {}

impl From<StoreError> for AppError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

pub type Result<T, E = AppError> = std::result::Result<T, E>;

/// What the setup wizard collects (PLAN §11 "setup wizard").
#[derive(Debug, Clone)]
pub struct BusinessSetup {
    pub business_name: String,
    pub currency: Currency,
    pub owner_name: String,
    pub owner_pin: String,
    /// Selling areas, e.g. ("Café", QuickService), ("Books", Retail).
    pub revenue_centers: Vec<(String, ModePreset)>,
    /// Default tax: name, rate in basis points, whether prices include it.
    pub tax: Option<(String, i32, bool)>,
    /// Prefix for this device's order numbers ("A").
    pub order_prefix: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaffRole {
    Cashier,
    Manager,
    Owner,
}

/// A signed-in staff member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedIn {
    pub id: StaffId,
    pub name: String,
    pub role: String,
}

/// Result of a cash payment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CashResult {
    pub change: Money,
    /// The order is fully paid and was closed.
    pub closed: bool,
}

const ROLE_CASHIER: RoleId = RoleId::from_u128(1);
const ROLE_MANAGER: RoleId = RoleId::from_u128(2);
const ROLE_OWNER: RoleId = RoleId::from_u128(3);

pub struct Register {
    store: Store,
    book: Book,
    log: DeviceLog,
    location: LocationId,
    prefix: String,
    signed_in: Option<StaffId>,
    training: bool,
    failed_pins: u32,
    locked_until_ms: u64,
    clock: Box<dyn Fn() -> u64 + Send>,
}

fn system_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn hash_pin(pin: &str) -> Result<String> {
    hash_pin_check(pin)?;
    Argon2::default()
        .hash_password(pin.as_bytes())
        .map(|h| h.to_string())
        .map_err(|_| AppError::Invalid("couldn't hash PIN"))
}

/// Checks a PIN's format without hashing it.
pub fn hash_pin_check(pin: &str) -> Result<()> {
    if pin.len() < 4 || pin.len() > 8 || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::Invalid("PINs are 4 to 8 digits"));
    }
    Ok(())
}

fn pin_matches(pin: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .is_ok_and(|h| Argon2::default().verify_password(pin.as_bytes(), &h).is_ok())
}

impl Register {
    /// Creates a new business on this device (the founder) and signs the
    /// owner in. `owner_secret` is the owner authority key (PLAN §6); the
    /// caller keeps it encrypted with the owner passphrase.
    pub fn create(
        db_path: impl AsRef<Path>,
        db_key: &[u8; 32],
        device_secret: &[u8; 32],
        owner_secret: &[u8; 32],
        setup: BusinessSetup,
    ) -> Result<Self> {
        Self::create_with_clock(
            db_path,
            db_key,
            device_secret,
            owner_secret,
            setup,
            Box::new(system_now),
        )
    }

    pub fn create_with_clock(
        db_path: impl AsRef<Path>,
        db_key: &[u8; 32],
        device_secret: &[u8; 32],
        owner_secret: &[u8; 32],
        setup: BusinessSetup,
        clock: Box<dyn Fn() -> u64 + Send>,
    ) -> Result<Self> {
        let store = Store::open(db_path, db_key)?;
        if store.record_count()? > 0 {
            return Err(AppError::Invalid("this device already has a business"));
        }
        let location = LocationId::new();
        let mut reg = Self {
            store,
            book: Book::new(),
            log: DeviceLog::new(device_secret),
            location,
            prefix: setup.order_prefix.clone(),
            signed_in: None,
            training: false,
            failed_pins: 0,
            locked_until_ms: 0,
            clock,
        };
        reg.act(Event::BusinessCreated(BusinessCreated {
            name: setup.business_name.clone(),
            currency: setup.currency,
            owner_key: ledger::owner_public(owner_secret),
            first_location: location,
        }))?;
        let mut added = DeviceAdded {
            device: reg.log.device(),
            name: "Front register".into(),
            role: DeviceRole::Register,
            revenue_centers: vec![],
            order_prefix: setup.order_prefix.clone(),
            owner_signature: [0; 64],
        };
        added.owner_signature = ledger::owner_sign(owner_secret, &added.signing_bytes());
        reg.act(Event::DeviceAdded(added))?;

        // Configuration before any staff exist (initial setup).
        let mut records = vec![
            ConfigRecord::Role(Role::cashier(ROLE_CASHIER)),
            ConfigRecord::Role(Role::manager(ROLE_MANAGER)),
            ConfigRecord::Role(Role::owner(ROLE_OWNER)),
        ];
        for (name, mode) in &setup.revenue_centers {
            records.push(ConfigRecord::RevenueCenter(RevenueCenter::new(
                RevenueCenterId::new(),
                location,
                name,
                *mode,
            )));
        }
        if let Some((name, rate_bp, inclusive)) = &setup.tax {
            records.push(ConfigRecord::Tax(TaxRate {
                id: TaxId::new(),
                name: name.clone(),
                rate_bp: *rate_bp,
                inclusive: *inclusive,
                compound: false,
                rounding: Rounding::HalfUp,
                scope: RoundingScope::PerLine,
            }));
        }
        let owner = StaffId::new();
        records.push(ConfigRecord::Staff(Staff {
            id: owner,
            name: setup.owner_name.clone(),
            role: ROLE_OWNER,
            pin_hash: hash_pin(&setup.owner_pin)?,
            locations: vec![location],
            active: true,
        }));
        for record in records {
            reg.act(Event::ConfigChanged(ConfigChanged { record }))?;
        }
        reg.signed_in = Some(owner);
        Ok(reg)
    }

    /// Opens an existing register after a restart.
    pub fn open(
        db_path: impl AsRef<Path>,
        db_key: &[u8; 32],
        device_secret: &[u8; 32],
    ) -> Result<Self> {
        Self::open_with_clock(db_path, db_key, device_secret, Box::new(system_now))
    }

    pub fn open_with_clock(
        db_path: impl AsRef<Path>,
        db_key: &[u8; 32],
        device_secret: &[u8; 32],
        clock: Box<dyn Fn() -> u64 + Send>,
    ) -> Result<Self> {
        let store = Store::open(db_path, db_key)?;
        let book = store.load_book()?;
        let fresh = DeviceLog::new(device_secret);
        let device = fresh.device();
        let log = match store.last_record_of(device)? {
            Some(last) => DeviceLog::resume(device_secret, &last.header, last.hash),
            None => fresh,
        };
        book.business().ok_or(AppError::NotFound("business"))?;
        let location = book
            .config()
            .revenue_centers
            .values()
            .next()
            .map(|c| c.location)
            .or_else(|| {
                book.config().staff.values().flat_map(|s| s.locations.first()).next().copied()
            })
            .ok_or(AppError::NotFound("location"))?;
        let prefix = book
            .devices()
            .get(&device)
            .map(|d| d.order_prefix.clone())
            .unwrap_or_else(|| "A".into());
        Ok(Self {
            store,
            book,
            log,
            location,
            prefix,
            signed_in: None,
            training: false,
            failed_pins: 0,
            locked_until_ms: 0,
            clock,
        })
    }

    pub fn book(&self) -> &Book {
        &self.book
    }

    pub fn device(&self) -> DeviceId {
        self.log.device()
    }

    /// Whether `secret` is this business's owner authority key, e.g. after
    /// unsealing it with the passphrase or reading a recovery code.
    pub fn is_owner_secret(&self, secret: &[u8; 32]) -> bool {
        self.book.business().is_some_and(|b| b.owner_key == ledger::owner_public(secret))
    }

    pub fn currency(&self) -> Currency {
        self.book.business().map_or(Currency::USD, |b| b.currency)
    }

    pub fn signed_in(&self) -> Option<SignedIn> {
        let id = self.signed_in?;
        let staff = self.book.config().staff.get(&id)?;
        let role =
            self.book.config().roles.get(&staff.role).map_or_else(String::new, |r| r.name.clone());
        Some(SignedIn { id, name: staff.name.clone(), role })
    }

    pub fn training(&self) -> bool {
        self.training
    }

    /// Training mode: everything works, nothing counts (PLAN §20).
    pub fn set_training(&mut self, on: bool) {
        self.training = on;
    }

    /// Checks, signs, stores and applies one event.
    fn act(&mut self, event: Event) -> Result<ledger::Record> {
        self.act_approved(event, None)
    }

    fn act_approved(
        &mut self,
        event: Event,
        approved_by: Option<StaffId>,
    ) -> Result<ledger::Record> {
        let now = (self.clock)();
        if let Some(seen) = self.book.latest_hlc() {
            self.log.clock().observe(seen, now);
        }
        let ctx = Context {
            location: self.location,
            staff: self.signed_in,
            approved_by,
            training: self.training,
        };
        let header = self.log.prepare(now, ctx, &event);
        let problems = self.book.check(&header, &event);
        if !problems.is_empty() {
            return Err(AppError::Refused(problems));
        }
        let record = self.log.seal(header, &event);
        // Durable on disk before it counts (PLAN §13).
        self.store.append(&record)?;
        match self.book.ingest_verified(record.clone()) {
            Ok(Ingested::Applied { .. }) => Ok(record),
            Ok(other) => Err(AppError::Rejected(format!("own record not applied: {other:?}"))),
            Err(e) => Err(AppError::Rejected(e.to_string())),
        }
    }

    // ---- Staff -------------------------------------------------------------

    /// Signs in by PIN alone (PINs are unique per business). Works offline.
    pub fn sign_in(&mut self, pin: &str) -> Result<SignedIn> {
        self.sign_in_inner(None, pin)
    }

    /// Signs in a chosen staff member: checks one PIN hash instead of all of
    /// them, so it stays fast with large teams.
    pub fn sign_in_as(&mut self, staff: StaffId, pin: &str) -> Result<SignedIn> {
        self.sign_in_inner(Some(staff), pin)
    }

    /// Active staff, by name, for a "tap your name" sign-in screen.
    pub fn staff_list(&self) -> Vec<(StaffId, String)> {
        let mut v: Vec<(StaffId, String)> = self
            .book
            .config()
            .staff
            .values()
            .filter(|s| s.active)
            .map(|s| (s.id, s.name.clone()))
            .collect();
        v.sort_by(|a, b| a.1.cmp(&b.1));
        v
    }

    fn sign_in_inner(&mut self, who: Option<StaffId>, pin: &str) -> Result<SignedIn> {
        let now = (self.clock)();
        if now < self.locked_until_ms {
            return Err(AppError::LockedOut { retry_in_ms: self.locked_until_ms - now });
        }
        let found = self
            .book
            .config()
            .staff
            .values()
            .filter(|s| who.is_none_or(|w| w == s.id))
            .find(|s| s.active && pin_matches(pin, &s.pin_hash))
            .map(|s| s.id);
        match found {
            Some(id) => {
                self.failed_pins = 0;
                self.signed_in = Some(id);
                Ok(self.signed_in().expect("just signed in"))
            }
            None => {
                self.failed_pins += 1;
                if self.failed_pins >= PIN_ATTEMPTS {
                    let doublings = (self.failed_pins - PIN_ATTEMPTS).min(10);
                    self.locked_until_ms = now + (LOCKOUT_MS << doublings);
                    return Err(AppError::LockedOut { retry_in_ms: LOCKOUT_MS << doublings });
                }
                Err(AppError::WrongPin { attempts_left: PIN_ATTEMPTS - self.failed_pins })
            }
        }
    }

    pub fn sign_out(&mut self) {
        self.signed_in = None;
    }

    fn require_signed_in(&self) -> Result<StaffId> {
        self.signed_in.ok_or(AppError::NotSignedIn)
    }

    pub fn add_staff(&mut self, name: &str, pin: &str, role: StaffRole) -> Result<StaffId> {
        self.require_signed_in()?;
        let hash = hash_pin(pin)?;
        if self.book.config().staff.values().any(|s| s.active && pin_matches(pin, &s.pin_hash)) {
            return Err(AppError::Invalid("that PIN is already in use"));
        }
        let id = StaffId::new();
        let role = match role {
            StaffRole::Cashier => ROLE_CASHIER,
            StaffRole::Manager => ROLE_MANAGER,
            StaffRole::Owner => ROLE_OWNER,
        };
        self.act(Event::ConfigChanged(ConfigChanged {
            record: ConfigRecord::Staff(Staff {
                id,
                name: name.into(),
                role,
                pin_hash: hash,
                locations: vec![self.location],
                active: true,
            }),
        }))?;
        Ok(id)
    }

    // ---- Catalog -----------------------------------------------------------

    pub fn revenue_centers(&self) -> Vec<RevenueCenter> {
        let mut v: Vec<RevenueCenter> =
            self.book.config().revenue_centers.values().cloned().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    pub fn taxes(&self) -> Vec<TaxRate> {
        self.book.config().taxes.values().cloned().collect()
    }

    pub fn items(&self, center: RevenueCenterId) -> Vec<Item> {
        let mut v: Vec<Item> = self
            .book
            .config()
            .items
            .values()
            .filter(|i| i.revenue_center == center)
            .cloned()
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    pub fn item(&self, id: ItemId) -> Option<&Item> {
        self.book.config().items.get(&id)
    }

    pub fn modifier_group(&self, id: ModifierGroupId) -> Option<&ModifierGroup> {
        self.book.config().modifier_groups.get(&id)
    }

    pub fn save_item(&mut self, item: Item) -> Result<()> {
        self.act(Event::ConfigChanged(ConfigChanged { record: ConfigRecord::Item(item) }))?;
        Ok(())
    }

    pub fn save_modifier_group(&mut self, group: ModifierGroup) -> Result<()> {
        self.act(Event::ConfigChanged(ConfigChanged {
            record: ConfigRecord::ModifierGroup(group),
        }))?;
        Ok(())
    }

    /// Stock on hand for a tracked item.
    pub fn stock(&self, item: ItemId) -> i64 {
        self.book.stock(item)
    }

    pub fn receive_stock(&mut self, item: ItemId, quantity: i64) -> Result<()> {
        self.act(Event::InventoryAdjusted(InventoryAdjusted {
            item,
            delta: quantity,
            reason: "received".into(),
        }))?;
        Ok(())
    }

    // ---- Orders ------------------------------------------------------------

    pub fn order(&self, id: OrderId) -> Option<&Order> {
        self.book.order(id)
    }

    /// Open orders started on this device, newest first.
    pub fn open_orders(&self) -> Vec<&Order> {
        let me = self.device();
        let mut v: Vec<&Order> =
            self.book.orders().filter(|o| o.status == Status::Open && o.opened_by == me).collect();
        v.sort_by_key(|o| std::cmp::Reverse(o.opened_at_ms));
        v
    }

    fn next_number(&self) -> String {
        let me = self.device();
        let mine = self.book.orders().filter(|o| o.opened_by == me).count();
        format!("{}{}", self.prefix, mine + 1)
    }

    pub fn open_order(
        &mut self,
        center: RevenueCenterId,
        fulfillment: Fulfillment,
    ) -> Result<OrderId> {
        self.require_signed_in()?;
        let order = OrderId::new();
        self.act(Event::OrderOpened(OrderOpened {
            order,
            revenue_center: center,
            currency: self.currency(),
            pricing_rules: PricingRules::CURRENT,
            number: self.next_number(),
            fulfillment,
            table: None,
            guests: None,
            customer: None,
            tab_name: None,
        }))?;
        Ok(order)
    }

    /// Adds an item. Price, modifiers and taxes are snapshotted from the
    /// catalog now, so later catalog changes never alter this sale.
    pub fn add_item(
        &mut self,
        order: OrderId,
        item: ItemId,
        variant: Option<VariantId>,
        modifiers: &[ModifierId],
        quantity: i64,
    ) -> Result<LineId> {
        let item = self.item(item).ok_or(AppError::NotFound("item"))?.clone();
        let (name, unit_price) = match variant {
            Some(v) => {
                let v = item
                    .variants
                    .iter()
                    .find(|x| x.id == v)
                    .ok_or(AppError::NotFound("variant"))?;
                (format!("{} ({})", item.name, v.name), v.price)
            }
            None => (item.name.clone(), item.price),
        };
        let mut snapshots = Vec::new();
        for id in modifiers {
            let m = item
                .modifier_groups
                .iter()
                .filter_map(|g| self.modifier_group(*g))
                .flat_map(|g| g.modifiers.iter())
                .find(|m| m.id == *id)
                .ok_or(AppError::NotFound("modifier"))?;
            snapshots.push(ModifierSnapshot { name: m.name.clone(), price: m.price });
        }
        let taxes =
            item.taxes.iter().filter_map(|t| self.book.config().taxes.get(t)).cloned().collect();
        let line = LineId::new();
        self.act(Event::LineAdded(LineAdded {
            order,
            line,
            item: item.id,
            variant,
            name,
            revenue_center: item.revenue_center,
            quantity,
            unit_price,
            modifiers: snapshots,
            taxes,
            seat: None,
            course: None,
            track_stock: item.track_stock,
        }))?;
        Ok(line)
    }

    pub fn set_quantity(&mut self, order: OrderId, line: LineId, quantity: i64) -> Result<()> {
        if quantity == 0 {
            return self.remove_line(order, line);
        }
        self.act(Event::LineQtyChanged(LineQtyChanged { order, line, quantity }))?;
        Ok(())
    }

    pub fn remove_line(&mut self, order: OrderId, line: LineId) -> Result<()> {
        self.act(Event::LineRemoved(LineRemoved { order, line }))?;
        Ok(())
    }

    /// A percentage discount on the whole order. Large ones need a manager's
    /// PIN (`approver_pin`).
    pub fn discount_order(
        &mut self,
        order: OrderId,
        name: &str,
        basis_points: i32,
        approver_pin: Option<&str>,
    ) -> Result<()> {
        let approved_by = approver_pin.and_then(|pin| {
            self.book
                .config()
                .staff
                .values()
                .find(|s| s.active && pin_matches(pin, &s.pin_hash))
                .map(|s| s.id)
        });
        self.act_approved(
            Event::DiscountApplied(DiscountApplied {
                order,
                line: None,
                discount: DiscountSnapshot {
                    name: name.into(),
                    amount: DiscountAmount::Percent(basis_points),
                },
            }),
            approved_by,
        )?;
        Ok(())
    }

    /// Sends unsent lines to the kitchen.
    pub fn send(&mut self, order: OrderId) -> Result<()> {
        let o = self.order(order).ok_or(AppError::NotFound("order"))?;
        let lines: Vec<LineId> =
            o.active_lines().filter(|l| !l.snapshot.sent).map(|l| l.snapshot.line).collect();
        if lines.is_empty() {
            return Ok(());
        }
        self.act(Event::OrderSent(OrderSent { order, lines }))?;
        Ok(())
    }

    /// Takes cash. Applies at most the balance; the rest is change. Closes
    /// the order when it's fully paid.
    pub fn pay_cash(&mut self, order: OrderId, given: Money) -> Result<CashResult> {
        let o = self.order(order).ok_or(AppError::NotFound("order"))?;
        let balance = o.balance();
        if given.minor() <= 0 {
            return Err(AppError::Invalid("enter the cash amount"));
        }
        let applied = if given.minor() > balance.minor() { balance } else { given };
        self.act(Event::CashTendered(CashTendered {
            order,
            payment: PaymentId::new(),
            given,
            applied,
        }))?;
        let change = given.checked_sub(applied).map_err(|_| AppError::Invalid("currency"))?;
        let closed = self.close_if_paid(order)?;
        Ok(CashResult { change, closed })
    }

    fn close_if_paid(&mut self, order: OrderId) -> Result<bool> {
        let o = self.order(order).ok_or(AppError::NotFound("order"))?;
        if o.status != Status::Open || o.balance().minor() > 0 {
            return Ok(false);
        }
        let total = o.totals().map_err(|e| AppError::Rejected(e.to_string()))?.total;
        self.act(Event::OrderClosed(OrderClosed { order, total }))?;
        Ok(true)
    }

    pub fn void_order(&mut self, order: OrderId, reason: &str) -> Result<()> {
        self.act(Event::OrderVoided(OrderVoided { order, reason: reason.into() }))?;
        Ok(())
    }

    // ---- Shifts and reports --------------------------------------------------

    pub fn open_shift(&mut self, float: Money) -> Result<ShiftId> {
        let shift = ShiftId::new();
        self.act(Event::ShiftOpened(ShiftOpened { shift, float }))?;
        Ok(shift)
    }

    /// Closes a shift with a blind count and returns the summary, including
    /// the variance (PLAN §9).
    pub fn close_shift(&mut self, shift: ShiftId, counted: Money) -> Result<Shift> {
        self.act(Event::ShiftClosed(ShiftClosed { shift, counted }))?;
        self.book.shift(shift).cloned().ok_or(AppError::NotFound("shift"))
    }

    pub fn cash_movement(
        &mut self,
        shift: ShiftId,
        kind: CashMovementKind,
        amount: Money,
        reason: &str,
    ) -> Result<()> {
        self.act(Event::CashMovement(CashMovement { shift, kind, amount, reason: reason.into() }))?;
        Ok(())
    }

    /// X report: a snapshot without closing anything. (Z reports are the same
    /// figures, taken when the day is closed.)
    pub fn report(&self) -> SalesReport {
        self.book.sales_report()
    }

    /// Saves a checkpoint for fast startup.
    pub fn checkpoint(&self) -> Result<()> {
        self.store.save_checkpoint(&self.book)?;
        Ok(())
    }
}

/// Helpers for building catalog entries.
pub fn simple_item(
    name: &str,
    kind: ItemKind,
    price: Money,
    center: RevenueCenterId,
    taxes: Vec<TaxId>,
    modifier_groups: Vec<ModifierGroupId>,
) -> Item {
    Item {
        id: ItemId::new(),
        name: name.into(),
        kind,
        price,
        variants: vec![],
        modifier_groups,
        taxes,
        revenue_center: center,
        track_stock: kind == ItemKind::PhysicalGood,
        age_restricted: false,
    }
}

pub fn modifier(name: &str, price: Money) -> Modifier {
    Modifier { id: ModifierId::new(), name: name.into(), price }
}

pub fn variant(name: &str, price: Money) -> Variant {
    Variant { id: VariantId::new(), name: name.into(), price }
}
