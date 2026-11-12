//! The Book: everything one device knows, built from verified records.
//!
//! - [`Book::ingest`] takes a record from anywhere (this device or a peer),
//!   verifies it, places it in its device's chain, and updates state.
//!   Problems are returned as warnings; the record is still kept (PLAN §6,
//!   "validation tolerant of different versions").
//! - [`Book::check`] is the strict version for the creating device: it
//!   reports every problem an event would cause, before it's signed.
//! - Records that arrive early (before earlier records from their device, or
//!   before the device was added) wait in a pending set rather than being
//!   rejected, because peers forward each other's logs in any order.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    catalog::{Discount, DiscountAmount, Item, ModifierGroup, RevenueCenter, TaxRate},
    events::{CashMovementKind, ConfigRecord, DeviceRole, Event, EventRef, Staff, Tender},
    hlc::Hlc,
    ids::{
        CustomerId, DeviceId, ItemId, OrderId, RevenueCenterId, RoleId, ShiftId, StaffId, TaxId,
    },
    ledger::{self, Accepted, ChainTracker, Header, LedgerError, Record},
    money::{Currency, Money},
    order::{Order, PaymentStatus, Problem, Status},
    permissions::{Permission, Role},
};

/// Percentage discounts above this need a manager's approval.
pub const LARGE_DISCOUNT_BP: i32 = 2000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Business {
    pub name: String,
    pub currency: Currency,
    pub owner_key: [u8; 32],
    /// The device that created the business; it's a member from the start.
    pub founder: DeviceId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub role: DeviceRole,
    pub revenue_centers: Vec<RevenueCenterId>,
    pub order_prefix: String,
    /// Events after this sequence number are not trusted.
    pub revoked_through: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub revenue_centers: BTreeMap<RevenueCenterId, RevenueCenter>,
    pub items: BTreeMap<ItemId, Item>,
    pub modifier_groups: BTreeMap<crate::ids::ModifierGroupId, ModifierGroup>,
    pub taxes: BTreeMap<TaxId, TaxRate>,
    pub discounts: BTreeMap<crate::ids::DiscountId, Discount>,
    pub roles: BTreeMap<RoleId, Role>,
    pub staff: BTreeMap<StaffId, Staff>,
    /// Last-writer-wins stamps per record, including deleted ones.
    stamps: BTreeMap<(String, uuid::Uuid), Hlc>,
}

impl Config {
    fn key(record: &ConfigRecord) -> (String, uuid::Uuid) {
        match record {
            ConfigRecord::RevenueCenter(r) => ("revenue_center".into(), r.id.0),
            ConfigRecord::Item(r) => ("item".into(), r.id.0),
            ConfigRecord::ModifierGroup(r) => ("modifier_group".into(), r.id.0),
            ConfigRecord::Tax(r) => ("tax".into(), r.id.0),
            ConfigRecord::Discount(r) => ("discount".into(), r.id.0),
            ConfigRecord::Role(r) => ("role".into(), r.id.0),
            ConfigRecord::Staff(r) => ("staff".into(), r.id.0),
        }
    }

    /// Applies a change if it's newer than what we have. Returns whether it won.
    fn upsert(&mut self, record: &ConfigRecord, at: Hlc) -> bool {
        let key = Self::key(record);
        if self.stamps.get(&key).is_some_and(|s| *s >= at) {
            return false;
        }
        self.stamps.insert(key, at);
        match record.clone() {
            ConfigRecord::RevenueCenter(r) => drop(self.revenue_centers.insert(r.id, r)),
            ConfigRecord::Item(r) => drop(self.items.insert(r.id, r)),
            ConfigRecord::ModifierGroup(r) => drop(self.modifier_groups.insert(r.id, r)),
            ConfigRecord::Tax(r) => drop(self.taxes.insert(r.id, r)),
            ConfigRecord::Discount(r) => drop(self.discounts.insert(r.id, r)),
            ConfigRecord::Role(r) => drop(self.roles.insert(r.id, r)),
            ConfigRecord::Staff(r) => drop(self.staff.insert(r.id, r)),
        }
        true
    }

    fn delete(&mut self, kind: &str, id: uuid::Uuid, at: Hlc) -> bool {
        let key = (kind.to_string(), id);
        if self.stamps.get(&key).is_some_and(|s| *s >= at) {
            return false;
        }
        self.stamps.insert(key, at);
        match kind {
            "revenue_center" => drop(self.revenue_centers.remove(&RevenueCenterId(id))),
            "item" => drop(self.items.remove(&ItemId(id))),
            "modifier_group" => drop(self.modifier_groups.remove(&crate::ids::ModifierGroupId(id))),
            "tax" => drop(self.taxes.remove(&TaxId(id))),
            "discount" => drop(self.discounts.remove(&crate::ids::DiscountId(id))),
            "role" => drop(self.roles.remove(&RoleId(id))),
            "staff" => drop(self.staff.remove(&StaffId(id))),
            _ => {}
        }
        true
    }

    fn role_of(&self, staff: StaffId) -> Option<&Role> {
        self.staff.get(&staff).filter(|s| s.active).and_then(|s| self.roles.get(&s.role))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shift {
    pub id: ShiftId,
    pub device: DeviceId,
    pub staff: Option<StaffId>,
    pub float: Money,
    pub cash_sales: Money,
    pub cash_refunds: Money,
    pub paid_in: Money,
    pub paid_out: Money,
    pub no_sales: u32,
    pub counted: Option<Money>,
}

impl Shift {
    /// Cash that should be in the drawer.
    pub fn expected(&self) -> Money {
        let add = |a: Money, b: Money| a.checked_add(b).unwrap_or(a);
        let sub = |a: Money, b: Money| a.checked_sub(b).unwrap_or(a);
        sub(
            sub(add(add(self.float, self.cash_sales), self.paid_in), self.paid_out),
            self.cash_refunds,
        )
    }

    /// Counted minus expected (negative = short). `None` while still open.
    pub fn variance(&self) -> Option<Money> {
        self.counted.map(|c| c.checked_sub(self.expected()).unwrap_or(c))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warning {
    pub at: EventRef,
    pub event: String,
    pub problem: Problem,
}

/// Why a record was refused outright (it goes to quarantine for review).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    Ledger(LedgerError),
    /// From a revoked device, after its trusted cutoff.
    Revoked {
        device: DeviceId,
        seq: u64,
    },
    /// A membership change not signed by the owner authority.
    BadOwnerSignature,
}

impl std::fmt::Display for Rejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ledger(e) => write!(f, "{e}"),
            Self::Revoked { device, seq } => {
                write!(f, "device {device} was revoked before event {seq}")
            }
            Self::BadOwnerSignature => f.write_str("membership change not signed by the owner"),
        }
    }
}

/// Outcome of [`Book::ingest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ingested {
    /// Applied (possibly with warnings), plus any pending records it unblocked.
    Applied { warnings: Vec<Problem>, unblocked: usize },
    /// Already had it.
    Duplicate,
    /// Waiting for earlier records or for its device to be added.
    Pending,
}

#[derive(Debug, Default, Clone)]
pub struct Book {
    business: Option<Business>,
    devices: BTreeMap<DeviceId, DeviceInfo>,
    config: Config,
    chains: ChainTracker,
    order_records: HashMap<OrderId, Vec<Record>>,
    orders: HashMap<OrderId, Order>,
    order_problems: HashMap<OrderId, Vec<(EventRef, Problem)>>,
    shifts: BTreeMap<ShiftId, Shift>,
    open_shift: HashMap<DeviceId, ShiftId>,
    stock_adjustments: HashMap<ItemId, i64>,
    erased_customers: HashSet<CustomerId>,
    fiscal_outage: bool,
    warnings: Vec<Warning>,
    /// Records waiting for their predecessors, by (device, seq).
    pending: BTreeMap<(DeviceId, u64), Record>,
    /// Every accepted record, in acceptance order (stored by `pos-store`).
    log: Vec<EventRef>,
    activity: Vec<Activity>,
    latest_hlc: Option<Hlc>,
}

/// One staff action worth watching for loss prevention (PLAN §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    pub staff: Option<StaffId>,
    pub approved_by: Option<StaffId>,
    pub kind: ActivityKind,
    pub amount: Option<Money>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityKind {
    Void,
    LineVoid,
    Discount,
    Refund,
    NoSale,
}

impl Book {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn business(&self) -> Option<&Business> {
        self.business.as_ref()
    }

    pub fn devices(&self) -> &BTreeMap<DeviceId, DeviceInfo> {
        &self.devices
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn order(&self, id: OrderId) -> Option<&Order> {
        self.orders.get(&id)
    }

    pub fn orders(&self) -> impl Iterator<Item = &Order> {
        self.orders.values()
    }

    pub fn order_problems(&self, id: OrderId) -> &[(EventRef, Problem)] {
        self.order_problems.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn shift(&self, id: ShiftId) -> Option<&Shift> {
        self.shifts.get(&id)
    }

    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn version_vector(&self) -> Vec<(DeviceId, u64)> {
        self.chains.version_vector()
    }

    pub fn is_erased(&self, customer: CustomerId) -> bool {
        self.erased_customers.contains(&customer)
    }

    /// The latest timestamp of any accepted record. A device must merge this
    /// into its clock (`HlcClock::observe`) before creating events, so its
    /// events sort after everything it has seen (PLAN §6).
    pub fn latest_hlc(&self) -> Option<Hlc> {
        self.latest_hlc
    }

    pub fn activity(&self) -> &[Activity] {
        &self.activity
    }

    /// Is `device` allowed to write event `seq`?
    fn membership(&self, header: &Header, event: &Event) -> Result<bool, Rejection> {
        if let Event::BusinessCreated(_) = event {
            return Ok(self.business.is_none() && header.seq == 1);
        }
        let Some(business) = &self.business else { return Ok(false) };
        if header.device == business.founder {
            return Ok(true);
        }
        match self.devices.get(&header.device) {
            None => Ok(false),
            Some(d) => match d.revoked_through {
                Some(cutoff) if header.seq > cutoff => {
                    Err(Rejection::Revoked { device: header.device, seq: header.seq })
                }
                _ => Ok(true),
            },
        }
    }

    /// Verifies and applies a record from anywhere.
    pub fn ingest(&mut self, wire: &[u8]) -> Result<Ingested, Rejection> {
        let record = ledger::verify_record(wire).map_err(Rejection::Ledger)?;
        self.ingest_verified(record)
    }

    /// Applies a record whose signature was already verified (e.g. one this
    /// device just sealed, or one loaded from local encrypted storage).
    pub fn ingest_verified(&mut self, record: Record) -> Result<Ingested, Rejection> {
        let outcome = self.try_accept(record)?;
        if let Ingested::Applied { warnings, .. } = outcome {
            let unblocked = self.drain_pending();
            return Ok(Ingested::Applied { warnings, unblocked });
        }
        Ok(outcome)
    }

    fn try_accept(&mut self, record: Record) -> Result<Ingested, Rejection> {
        let key = (record.header.device, record.header.seq);
        if !self.membership(&record.header, &record.event)? {
            self.pending.insert(key, record);
            return Ok(Ingested::Pending);
        }
        match self.chains.accept(&record) {
            Ok(Accepted::Duplicate) => return Ok(Ingested::Duplicate),
            Ok(Accepted::New) => {}
            Err(LedgerError::Gap { .. }) => {
                self.pending.insert(key, record);
                return Ok(Ingested::Pending);
            }
            Err(e) => return Err(Rejection::Ledger(e)),
        }
        let warnings = self.apply(record)?;
        Ok(Ingested::Applied { warnings, unblocked: 0 })
    }

    /// Retries pending records until none make progress.
    fn drain_pending(&mut self) -> usize {
        let mut unblocked = 0;
        loop {
            let ready: Vec<(DeviceId, u64)> = self
                .pending
                .iter()
                .filter(|((device, seq), r)| {
                    let next = self.chains.head(device).map_or(1, |(s, _)| s + 1);
                    *seq == next && matches!(self.membership(&r.header, &r.event), Ok(true))
                })
                .map(|(k, _)| *k)
                .collect();
            if ready.is_empty() {
                // Drop pending records that can never apply (revoked).
                let dead: Vec<(DeviceId, u64)> = self
                    .pending
                    .iter()
                    .filter(|(_, r)| self.membership(&r.header, &r.event).is_err())
                    .map(|(k, _)| *k)
                    .collect();
                for k in dead {
                    self.pending.remove(&k);
                }
                return unblocked;
            }
            for key in ready {
                if let Some(record) = self.pending.remove(&key)
                    && matches!(self.try_accept(record), Ok(Ingested::Applied { .. }))
                {
                    unblocked += 1;
                }
            }
        }
    }

    fn apply(&mut self, record: Record) -> Result<Vec<Problem>, Rejection> {
        let at = EventRef { device: record.header.device, seq: record.header.seq };
        if self.latest_hlc.is_none_or(|l| l < record.header.hlc) {
            self.latest_hlc = Some(record.header.hlc);
        }
        let mut problems = self.audit(&record.header, &record.event);
        let h = record.header.clone();

        match &record.event {
            Event::BusinessCreated(e) => {
                self.business = Some(Business {
                    name: e.name.clone(),
                    currency: e.currency,
                    owner_key: e.owner_key,
                    founder: h.device,
                });
            }
            Event::DeviceAdded(e) => {
                let owner = self.business.as_ref().map(|b| b.owner_key).unwrap_or_default();
                if !ledger::owner_verify(&owner, &e.signing_bytes(), &e.owner_signature) {
                    return Err(Rejection::BadOwnerSignature);
                }
                self.devices.insert(
                    e.device,
                    DeviceInfo {
                        name: e.name.clone(),
                        role: e.role,
                        revenue_centers: e.revenue_centers.clone(),
                        order_prefix: e.order_prefix.clone(),
                        revoked_through: None,
                    },
                );
            }
            Event::DeviceRevoked(e) => {
                let owner = self.business.as_ref().map(|b| b.owner_key).unwrap_or_default();
                if !ledger::owner_verify(&owner, &e.signing_bytes(), &e.owner_signature) {
                    return Err(Rejection::BadOwnerSignature);
                }
                if let Some(d) = self.devices.get_mut(&e.device) {
                    // A later revocation can only tighten the cutoff.
                    let cutoff = d
                        .revoked_through
                        .map_or(e.trusted_through_seq, |c| c.min(e.trusted_through_seq));
                    d.revoked_through = Some(cutoff);
                }
            }
            Event::ConfigChanged(e) => {
                self.config.upsert(&e.record, h.hlc);
            }
            Event::ConfigDeleted(e) => {
                self.config.delete(&e.kind, e.id, h.hlc);
            }
            Event::ShiftOpened(e) => {
                let zero = Money::zero(e.float.currency());
                self.shifts.insert(
                    e.shift,
                    Shift {
                        id: e.shift,
                        device: h.device,
                        staff: h.staff,
                        float: e.float,
                        cash_sales: zero,
                        cash_refunds: zero,
                        paid_in: zero,
                        paid_out: zero,
                        no_sales: 0,
                        counted: None,
                    },
                );
                self.open_shift.insert(h.device, e.shift);
            }
            Event::ShiftClosed(e) => {
                if let Some(s) = self.shifts.get_mut(&e.shift) {
                    s.counted = Some(e.counted);
                }
                self.open_shift.remove(&h.device);
            }
            Event::CashMovement(e) => {
                if let Some(s) = self.shifts.get_mut(&e.shift) {
                    match e.kind {
                        CashMovementKind::PaidIn => {
                            s.paid_in = s.paid_in.checked_add(e.amount).unwrap_or(s.paid_in)
                        }
                        CashMovementKind::PaidOut => {
                            s.paid_out = s.paid_out.checked_add(e.amount).unwrap_or(s.paid_out)
                        }
                        CashMovementKind::NoSale => s.no_sales += 1,
                    }
                }
                if e.kind == CashMovementKind::NoSale {
                    self.activity.push(Activity {
                        staff: h.staff,
                        approved_by: h.approved_by,
                        kind: ActivityKind::NoSale,
                        amount: None,
                    });
                }
            }
            Event::InventoryAdjusted(e) => {
                *self.stock_adjustments.entry(e.item).or_default() += e.delta;
            }
            Event::FiscalOutage(e) => self.fiscal_outage = e.started,
            Event::CustomerErased(e) => {
                self.erased_customers.insert(e.customer);
            }
            _ => {}
        }

        // Cash in or out of this device's open shift.
        if !h.training
            && let Some(shift) =
                self.open_shift.get(&h.device).and_then(|id| self.shifts.get_mut(id))
        {
            match &record.event {
                Event::CashTendered(e) => {
                    shift.cash_sales =
                        shift.cash_sales.checked_add(e.applied).unwrap_or(shift.cash_sales)
                }
                Event::RefundIssued(e) if e.tender == Tender::Cash => {
                    shift.cash_refunds =
                        shift.cash_refunds.checked_add(e.amount).unwrap_or(shift.cash_refunds)
                }
                _ => {}
            }
        }

        // Loss-prevention activity.
        let kind = match &record.event {
            Event::OrderVoided(_) => Some((ActivityKind::Void, None)),
            Event::LineVoidedAfterSend(_) => Some((ActivityKind::LineVoid, None)),
            Event::DiscountApplied(_) => Some((ActivityKind::Discount, None)),
            Event::RefundIssued(e) => Some((ActivityKind::Refund, Some(e.amount))),
            _ => None,
        };
        if let Some((kind, amount)) = kind {
            self.activity.push(Activity {
                staff: h.staff,
                approved_by: h.approved_by,
                kind,
                amount,
            });
        }

        // Orders: insert in clock order and rebuild each affected order.
        let affected = record.event.orders();
        let closes = matches!(record.event, Event::OrderClosed(_));
        let reported_total =
            if let Event::OrderClosed(e) = &record.event { Some(e.total) } else { None };
        self.log.push(at);
        for id in &affected {
            let list = self.order_records.entry(*id).or_default();
            let pos = list.partition_point(|r| sort_key(&r.header) < sort_key(&h));
            list.insert(pos, record.clone());
        }
        for id in &affected {
            let new_problems = self.refold(*id, at);
            problems.extend(new_problems);
        }
        if closes
            && let (Some(id), Some(reported)) = (affected.first(), reported_total)
            && let Some(order) = self.orders.get(id)
            && let Ok(t) = order.totals()
            && t.total != reported
        {
            problems.push(Problem::TotalMismatch { reported, computed: t.total });
        }

        for p in &problems {
            self.warnings.push(Warning {
                at,
                event: record.event.name().to_string(),
                problem: p.clone(),
            });
        }
        Ok(problems)
    }

    /// Rebuilds one order; returns the problems caused by the record `at`.
    fn refold(&mut self, id: OrderId, at: EventRef) -> Vec<Problem> {
        let records = self.order_records.get(&id).map(Vec::as_slice).unwrap_or(&[]);
        let (order, problems) = Order::fold(id, records.iter().map(|r| (&r.header, &r.event)));
        let located: Vec<(EventRef, Problem)> = problems
            .into_iter()
            .map(|(i, p)| {
                (EventRef { device: records[i].header.device, seq: records[i].header.seq }, p)
            })
            .collect();
        let mine = located.iter().filter(|(r, _)| *r == at).map(|(_, p)| p.clone()).collect();
        match order {
            Some(o) => drop(self.orders.insert(id, o)),
            None => drop(self.orders.remove(&id)),
        }
        self.order_problems.insert(id, located);
        mine
    }

    /// Checks that depend on configuration, so they can differ between peers.
    /// Warnings on receivers; errors on the creating device.
    fn audit(&self, h: &Header, event: &Event) -> Vec<Problem> {
        let mut out = Vec::new();
        let needs = |p: Permission, out: &mut Vec<Problem>| {
            // Before any staff exist (initial setup), the founder may configure.
            if self.config.staff.is_empty() {
                return;
            }
            let ok = |s: Option<StaffId>| {
                s.and_then(|s| self.config.role_of(s)).is_some_and(|r| r.allows(p))
            };
            match h.staff {
                None => out.push(Problem::UnknownStaff),
                Some(s) if self.config.role_of(s).is_none() => out.push(Problem::UnknownStaff),
                Some(_) if ok(h.staff) || ok(h.approved_by) => {}
                Some(_) => out.push(Problem::NotPermitted(p)),
            }
        };
        let order_center = |id: OrderId| self.orders.get(&id).map(|o| o.revenue_center);
        let modules = |center: Option<RevenueCenterId>| {
            center.and_then(|c| self.config.revenue_centers.get(&c)).map(|c| c.modules)
        };

        match event {
            Event::OrderOpened(e) => {
                needs(Permission::OrderCreate, &mut out);
                match self.config.revenue_centers.get(&e.revenue_center) {
                    None if !self.config.revenue_centers.is_empty() => {
                        out.push(Problem::UnknownRevenueCenter)
                    }
                    Some(c) => {
                        if e.table.is_some() && !c.modules.tables {
                            out.push(Problem::ModuleDisabled("tables".into()));
                        }
                        if e.tab_name.is_some() && !c.modules.tabs {
                            out.push(Problem::ModuleDisabled("tabs".into()));
                        }
                    }
                    None => {}
                }
            }
            Event::LineAdded(e) => {
                if (e.seat.is_some() || e.course.is_some())
                    && modules(order_center(e.order)).is_some_and(|m| !m.seats_and_courses)
                {
                    out.push(Problem::ModuleDisabled("seats_and_courses".into()));
                }
            }
            Event::TableAssigned(e) => {
                if modules(order_center(e.order)).is_some_and(|m| !m.tables) {
                    out.push(Problem::ModuleDisabled("tables".into()));
                }
            }
            Event::OrderSplit(e) => {
                if modules(order_center(e.from)).is_some_and(|m| !m.split_merge_transfer) {
                    out.push(Problem::ModuleDisabled("split_merge_transfer".into()));
                }
            }
            Event::OrderMerged(e) => {
                if modules(order_center(e.into)).is_some_and(|m| !m.split_merge_transfer) {
                    out.push(Problem::ModuleDisabled("split_merge_transfer".into()));
                }
            }
            Event::PaymentCaptured(e) => {
                if e.tip.minor() > 0 && modules(order_center(e.order)).is_some_and(|m| !m.tips) {
                    out.push(Problem::ModuleDisabled("tips".into()));
                }
            }
            Event::LineVoidedAfterSend(_) => needs(Permission::LineVoidAfterSend, &mut out),
            Event::OrderVoided(_) => needs(Permission::OrderVoid, &mut out),
            Event::DiscountApplied(e) => {
                needs(Permission::DiscountApply, &mut out);
                if matches!(e.discount.amount, DiscountAmount::Percent(bp) if bp > LARGE_DISCOUNT_BP)
                {
                    needs(Permission::DiscountLarge, &mut out);
                }
            }
            Event::RefundIssued(_) => needs(Permission::RefundApprove, &mut out),
            Event::PaymentStarted(_) | Event::CashTendered(_) => {
                needs(Permission::PaymentTake, &mut out)
            }
            Event::CashMovement(e) => match e.kind {
                CashMovementKind::NoSale => needs(Permission::DrawerNoSale, &mut out),
                _ => needs(Permission::DrawerOpen, &mut out),
            },
            Event::ShiftOpened(_) | Event::ShiftClosed(_) => {
                needs(Permission::DrawerOpen, &mut out)
            }
            Event::ConfigChanged(e) => match e.record {
                ConfigRecord::Staff(_) | ConfigRecord::Role(_) => {
                    needs(Permission::ManageStaff, &mut out)
                }
                _ => needs(Permission::ManageCatalog, &mut out),
            },
            Event::ConfigDeleted(e) => match e.kind.as_str() {
                "staff" | "role" => needs(Permission::ManageStaff, &mut out),
                _ => needs(Permission::ManageCatalog, &mut out),
            },
            Event::InventoryAdjusted(_) => needs(Permission::InventoryAdjust, &mut out),
            Event::OrderClosed(e) => {
                let fiscal = order_center(e.order)
                    .and_then(|c| self.config.revenue_centers.get(&c))
                    .is_some_and(|c| c.fiscal_mode);
                let signed = self.orders.get(&e.order).is_some_and(|o| o.fiscal.is_some());
                if fiscal && !signed && !self.fiscal_outage {
                    out.push(Problem::FiscalSignatureMissing);
                }
            }
            _ => {}
        }
        out
    }

    /// Every problem `event` would cause if this device appended it now.
    /// Empty means it's safe to seal. Doesn't change the book.
    pub fn check(&self, header: &Header, event: &Event) -> Vec<Problem> {
        match self.membership(header, event) {
            Ok(true) => {}
            _ => return vec![Problem::NotAMember],
        }
        let mut problems = self.audit(header, event);
        let pending =
            Record { header: header.clone(), event: event.clone(), hash: [0; 32], wire: vec![] };
        for id in event.orders() {
            let mut records: Vec<&Record> =
                self.order_records.get(&id).map(|v| v.iter().collect()).unwrap_or_default();
            let pos = records.partition_point(|r| sort_key(&r.header) < sort_key(header));
            records.insert(pos, &pending);
            let (order, found) = Order::fold(id, records.iter().map(|r| (&r.header, &r.event)));
            problems.extend(found.into_iter().filter(|(i, _)| *i == pos).map(|(_, p)| p));
            if let (Event::OrderClosed(e), Some(order)) = (event, order)
                && let Ok(t) = order.totals()
                && t.total != e.total
            {
                problems.push(Problem::TotalMismatch { reported: e.total, computed: t.total });
            }
        }
        problems
    }

    /// Stock on hand: adjustments minus what closed orders sold (net of refunds).
    pub fn stock(&self, item: ItemId) -> i64 {
        let sold: i64 = self
            .orders
            .values()
            .filter(|o| o.status == Status::Closed && !o.training)
            .flat_map(|o| o.active_lines())
            .filter(|l| l.snapshot.item == item && l.snapshot.track_stock)
            .map(|l| l.snapshot.quantity - l.refunded)
            .sum();
        self.stock_adjustments.get(&item).copied().unwrap_or(0) - sold
    }

    /// Sales report over closed, non-training orders.
    pub fn sales_report(&self) -> SalesReport {
        let currency = self.business.as_ref().map_or(Currency::USD, |b| b.currency);
        let zero = Money::zero(currency);
        let add = |a: &mut Money, b: Money| *a = a.checked_add(b).unwrap_or(*a);
        let mut r = SalesReport {
            orders: 0,
            net_by_revenue_center: BTreeMap::new(),
            taxes: BTreeMap::new(),
            total: zero,
            tips: zero,
            refunds: zero,
            by_tender: BTreeMap::new(),
        };
        for o in self.orders.values().filter(|o| o.status == Status::Closed && !o.training) {
            let Ok(t) = o.totals() else { continue };
            r.orders += 1;
            add(&mut r.total, t.total);
            add(&mut r.tips, o.tips());
            add(&mut r.refunds, o.refunded());
            for (line, lt) in o.active_lines().zip(&t.lines) {
                add(
                    r.net_by_revenue_center.entry(line.snapshot.revenue_center).or_insert(zero),
                    lt.net,
                );
            }
            for tax in &t.taxes {
                add(r.taxes.entry(tax.tax).or_insert(zero), tax.amount);
            }
            for p in o.payments.iter().filter(|p| p.status == PaymentStatus::Captured) {
                let name = match &p.tender {
                    Tender::Cash => "Cash".to_string(),
                    Tender::Card { provider, .. } => format!("Card ({provider})"),
                    Tender::Other { name } => name.clone(),
                };
                add(r.by_tender.entry(name).or_insert(zero), p.amount);
            }
        }
        r
    }

    /// Who did how many voids, discounts, refunds and no-sales.
    pub fn loss_prevention(&self) -> BTreeMap<Option<StaffId>, BTreeMap<String, u32>> {
        let mut out: BTreeMap<Option<StaffId>, BTreeMap<String, u32>> = BTreeMap::new();
        for a in &self.activity {
            *out.entry(a.staff).or_default().entry(format!("{:?}", a.kind)).or_default() += 1;
        }
        out
    }

    /// Every accepted record's position, in the order it was accepted.
    pub fn log(&self) -> &[EventRef] {
        &self.log
    }

    /// Serializable state for fast startup (PLAN §6 "checkpoints").
    pub fn checkpoint(&self) -> Checkpoint {
        let mut records: Vec<&Record> = self.order_records.values().flatten().collect();
        records.sort_by_key(|r| (r.header.device, r.header.seq));
        records.dedup_by_key(|r| (r.header.device, r.header.seq));
        Checkpoint {
            version: 1,
            business: self.business.clone(),
            devices: self.devices.clone(),
            config: self.config.clone(),
            heads: self
                .chains
                .version_vector()
                .into_iter()
                .map(|(d, s)| (d, s, self.chains.head(&d).map(|(_, h)| h).unwrap_or_default()))
                .collect(),
            order_records: records.into_iter().map(|r| r.wire.clone()).collect(),
            shifts: self.shifts.clone(),
            open_shift: self.open_shift.iter().map(|(d, s)| (*d, *s)).collect(),
            stock_adjustments: self.stock_adjustments.iter().map(|(i, n)| (*i, *n)).collect(),
            erased_customers: self.erased_customers.iter().copied().collect(),
            fiscal_outage: self.fiscal_outage,
            warnings: self.warnings.clone(),
            activity: self.activity.clone(),
            log: self.log.clone(),
            latest_hlc: self.latest_hlc,
        }
    }

    /// Restores a book from a checkpoint written by this device. Order
    /// records are decoded without re-checking signatures: they were verified
    /// when first received, and the checkpoint lives in encrypted storage.
    pub fn restore(cp: Checkpoint) -> Result<Self, LedgerError> {
        let mut book = Self {
            business: cp.business,
            devices: cp.devices,
            config: cp.config,
            shifts: cp.shifts,
            open_shift: cp.open_shift.into_iter().collect(),
            stock_adjustments: cp.stock_adjustments.into_iter().collect(),
            erased_customers: cp.erased_customers.into_iter().collect(),
            fiscal_outage: cp.fiscal_outage,
            warnings: cp.warnings,
            activity: cp.activity,
            log: cp.log,
            latest_hlc: cp.latest_hlc,
            ..Self::default()
        };
        book.chains = ChainTracker::from_heads(cp.heads);
        for wire in &cp.order_records {
            let record = ledger::decode_trusted(wire)?;
            for id in record.event.orders() {
                book.order_records.entry(id).or_default().push(record.clone());
            }
        }
        let ids: Vec<OrderId> = book.order_records.keys().copied().collect();
        for id in ids {
            book.order_records.get_mut(&id).expect("present").sort_by_key(|a| sort_key(&a.header));
            let none = EventRef { device: DeviceId([0; 32]), seq: 0 };
            book.refold(id, none);
        }
        Ok(book)
    }
}

fn sort_key(h: &Header) -> (Hlc, DeviceId, u64) {
    (h.hlc, h.device, h.seq)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesReport {
    pub orders: u32,
    pub net_by_revenue_center: BTreeMap<RevenueCenterId, Money>,
    pub taxes: BTreeMap<TaxId, Money>,
    pub total: Money,
    pub tips: Money,
    pub refunds: Money,
    pub by_tender: BTreeMap<String, Money>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub version: u16,
    pub business: Option<Business>,
    pub devices: BTreeMap<DeviceId, DeviceInfo>,
    pub config: Config,
    pub heads: Vec<(DeviceId, u64, ledger::Hash)>,
    pub order_records: Vec<Vec<u8>>,
    pub shifts: BTreeMap<ShiftId, Shift>,
    pub open_shift: Vec<(DeviceId, ShiftId)>,
    pub stock_adjustments: Vec<(ItemId, i64)>,
    pub erased_customers: Vec<CustomerId>,
    pub fiscal_outage: bool,
    pub warnings: Vec<Warning>,
    pub activity: Vec<Activity>,
    pub log: Vec<EventRef>,
    pub latest_hlc: Option<Hlc>,
}

impl Checkpoint {
    pub fn encode(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("in-memory encoding")
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        postcard::from_bytes(bytes).ok()
    }
}

/// Re-exported so tests and the bridge can name the set type.
pub type Set<T> = BTreeSet<T>;
