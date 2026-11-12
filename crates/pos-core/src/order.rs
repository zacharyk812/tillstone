//! One order (a sale or a restaurant check), rebuilt from its events.
//!
//! [`Order::fold`] is deterministic and uses only data inside the events, so
//! every peer holding the same events reaches the same state, whatever order
//! they arrived in (PLAN §6). Events that don't make sense at their point in
//! the order's history are skipped and reported as [`Problem`]s.

use serde::{Deserialize, Serialize};

use crate::{
    events::{Event, Fulfillment, LineSnapshot, Tender, VoidReason},
    ids::{CustomerId, DeviceId, LineId, OrderId, PaymentId, RefundId, RevenueCenterId, TableId},
    ledger::Header,
    money::{Currency, Money},
    pricing::{self, DiscountSnapshot, LineInput, PricingRules, Totals},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Open,
    Closed,
    Voided,
    /// All lines moved into another order.
    MergedInto(OrderId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub snapshot: LineSnapshot,
    pub voided: Option<VoidReason>,
    /// Quantity refunded so far.
    pub refunded: i64,
}

impl Line {
    pub fn active(&self) -> bool {
        self.voided.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    Started,
    Captured,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payment {
    pub id: PaymentId,
    pub tender: Tender,
    pub amount: Money,
    pub tip: Money,
    pub status: PaymentStatus,
    pub provider_ref: Option<String>,
    /// Cash handed over (cash payments only).
    pub given: Option<Money>,
    pub device: DeviceId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refund {
    pub id: RefundId,
    pub amount: Money,
    pub tender: Tender,
    pub lines: Vec<(LineId, i64)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    pub device: DeviceId,
    pub until_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub number: String,
    pub revenue_center: RevenueCenterId,
    pub currency: Currency,
    pub pricing_rules: PricingRules,
    pub fulfillment: Fulfillment,
    pub table: Option<TableId>,
    pub guests: Option<u16>,
    pub customer: Option<CustomerId>,
    pub tab_name: Option<String>,
    /// The device that opened the order. It may restructure the check when
    /// no other device holds the lease (PLAN §6).
    pub opened_by: DeviceId,
    pub opened_at_ms: u64,
    pub training: bool,
    pub status: Status,
    pub lines: Vec<Line>,
    pub order_discounts: Vec<DiscountSnapshot>,
    pub payments: Vec<Payment>,
    pub refunds: Vec<Refund>,
    pub lease: Option<Lease>,
    pub fiscal: Option<Vec<(String, String)>>,
    /// The total the closing device reported (compared in audits).
    pub reported_total: Option<Money>,
}

/// Why an event was skipped, or what looks wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Problem {
    OrderNotOpen,
    AlreadyExists,
    UnknownLine(LineId),
    UnknownPayment(PaymentId),
    LineAlreadySent(LineId),
    LineNotSent(LineId),
    LineAlreadyVoided(LineId),
    NegativeQuantity,
    CurrencyMismatch,
    /// Restructuring (split, merge, payment, close) without the check lease.
    LeaseHeldByAnotherDevice {
        holder: DeviceId,
    },
    Underpaid {
        paid: Money,
        total: Money,
    },
    Overpaid {
        by: Money,
    },
    CashLessThanApplied,
    HasCapturedPayments,
    RefundExceedsPaid,
    RefundExceedsSold(LineId),
    NotClosed,
    Pricing(String),
    // Audits (warnings on receiving peers, errors on the creating device):
    TotalMismatch {
        reported: Money,
        computed: Money,
    },
    NotPermitted(crate::permissions::Permission),
    UnknownStaff,
    ModuleDisabled(String),
    FiscalSignatureMissing,
    NotAMember,
    UnknownRevenueCenter,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Order {
    /// Rebuilds an order from its events, which must be sorted by hybrid
    /// logical clock. Returns the order (if it was ever opened) and the
    /// problems found, each with the index of the event that caused it.
    pub fn fold<'a>(
        id: OrderId,
        events: impl IntoIterator<Item = (&'a Header, &'a Event)>,
    ) -> (Option<Order>, Vec<(usize, Problem)>) {
        let mut order: Option<Order> = None;
        let mut problems = Vec::new();
        for (i, (header, event)) in events.into_iter().enumerate() {
            let result = apply(&mut order, id, header, event);
            if let Err(p) = result {
                problems.push((i, p));
            } else if let Some(o) = &order {
                if let Ok(Some(soft)) = result {
                    problems.push((i, soft));
                }
                // Overpayment is kept (money is never discarded) but reported.
                if matches!(event, Event::OrderClosed(_))
                    && let Ok(over) = o.paid().checked_sub(o.total_unchecked())
                    && over.minor() > 0
                {
                    problems.push((i, Problem::Overpaid { by: over }));
                }
            }
        }
        (order, problems)
    }

    pub fn active_lines(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter().filter(|l| l.active())
    }

    pub fn totals(&self) -> Result<Totals, pricing::PricingError> {
        let inputs: Vec<LineInput> = self
            .active_lines()
            .map(|l| LineInput {
                line_id: l.snapshot.line,
                quantity: l.snapshot.quantity,
                unit_price: l.snapshot.unit_price,
                modifiers: l.snapshot.modifiers.clone(),
                taxes: l.snapshot.taxes.clone(),
                discounts: l.snapshot.discounts.clone(),
            })
            .collect();
        pricing::price(self.pricing_rules, self.currency, &inputs, &self.order_discounts)
    }

    fn total_unchecked(&self) -> Money {
        self.totals().map(|t| t.total).unwrap_or(Money::zero(self.currency))
    }

    /// Captured payments, excluding tips.
    pub fn paid(&self) -> Money {
        self.payments
            .iter()
            .filter(|p| p.status == PaymentStatus::Captured)
            .fold(Money::zero(self.currency), |acc, p| acc.checked_add(p.amount).unwrap_or(acc))
    }

    pub fn tips(&self) -> Money {
        self.payments
            .iter()
            .filter(|p| p.status == PaymentStatus::Captured)
            .fold(Money::zero(self.currency), |acc, p| acc.checked_add(p.tip).unwrap_or(acc))
    }

    pub fn refunded(&self) -> Money {
        self.refunds
            .iter()
            .fold(Money::zero(self.currency), |acc, r| acc.checked_add(r.amount).unwrap_or(acc))
    }

    /// Amount still owed.
    pub fn balance(&self) -> Money {
        self.total_unchecked().checked_sub(self.paid()).unwrap_or(Money::zero(self.currency))
    }

    fn line_mut(&mut self, id: LineId) -> Result<&mut Line, Problem> {
        self.lines.iter_mut().find(|l| l.snapshot.line == id).ok_or(Problem::UnknownLine(id))
    }

    /// Taking the lease: allowed unless another device holds it unexpired.
    fn check_lease_free(&self, header: &Header) -> Result<(), Problem> {
        match &self.lease {
            Some(l) if l.until_ms > header.hlc.wall_ms && l.device != header.device => {
                Err(Problem::LeaseHeldByAnotherDevice { holder: l.device })
            }
            _ => Ok(()),
        }
    }

    /// Splits, merges, payments and closing need the lease, or (when nobody
    /// holds an unexpired lease) must come from the device that opened it.
    fn check_lease(&self, header: &Header) -> Result<(), Problem> {
        let now = header.hlc.wall_ms;
        match &self.lease {
            Some(l) if l.until_ms > now && l.device != header.device => {
                Err(Problem::LeaseHeldByAnotherDevice { holder: l.device })
            }
            Some(l) if l.until_ms > now => Ok(()),
            _ if header.device == self.opened_by => Ok(()),
            _ => Err(Problem::LeaseHeldByAnotherDevice { holder: self.opened_by }),
        }
    }

    fn require_open(&self) -> Result<(), Problem> {
        if self.status == Status::Open { Ok(()) } else { Err(Problem::OrderNotOpen) }
    }
}

/// Applies one event. `Err` means it was skipped; `Ok(Some(problem))` means
/// it was applied anyway but looks wrong. Events that record real money are
/// never skipped for coordination reasons (PLAN §6 "two payments at once").
fn apply(
    slot: &mut Option<Order>,
    id: OrderId,
    header: &Header,
    event: &Event,
) -> Result<Option<Problem>, Problem> {
    let mut soft = None;
    // Events that can create the order.
    match event {
        Event::OrderOpened(e) => {
            if slot.is_some() {
                return Err(Problem::AlreadyExists);
            }
            *slot = Some(Order {
                id,
                number: e.number.clone(),
                revenue_center: e.revenue_center,
                currency: e.currency,
                pricing_rules: e.pricing_rules,
                fulfillment: e.fulfillment,
                table: e.table,
                guests: e.guests,
                customer: e.customer,
                tab_name: e.tab_name.clone(),
                opened_by: header.device,
                opened_at_ms: header.hlc.wall_ms,
                training: header.training,
                status: Status::Open,
                lines: vec![],
                order_discounts: vec![],
                payments: vec![],
                refunds: vec![],
                lease: None,
                fiscal: None,
                reported_total: None,
            });
            return Ok(None);
        }
        Event::OrderSplit(e) if e.to == id => {
            if slot.is_some() {
                return Err(Problem::AlreadyExists);
            }
            *slot = Some(Order {
                id,
                number: e.number.clone(),
                revenue_center: e.revenue_center,
                currency: e.currency,
                pricing_rules: e.pricing_rules,
                fulfillment: e.fulfillment,
                table: e.table,
                guests: None,
                customer: None,
                tab_name: None,
                opened_by: header.device,
                opened_at_ms: header.hlc.wall_ms,
                training: header.training,
                status: Status::Open,
                lines: e
                    .lines
                    .iter()
                    .map(|s| Line { snapshot: s.clone(), voided: None, refunded: 0 })
                    .collect(),
                order_discounts: vec![],
                payments: vec![],
                refunds: vec![],
                lease: None,
                fiscal: None,
                reported_total: None,
            });
            return Ok(None);
        }
        _ => {}
    }

    let order = slot.as_mut().ok_or(Problem::OrderNotOpen)?;
    match event {
        Event::LineAdded(e) => {
            order.require_open()?;
            if e.unit_price.currency() != order.currency {
                return Err(Problem::CurrencyMismatch);
            }
            if e.quantity < 0 {
                return Err(Problem::NegativeQuantity);
            }
            if order.lines.iter().any(|l| l.snapshot.line == e.line) {
                return Err(Problem::AlreadyExists);
            }
            order.lines.push(Line {
                snapshot: LineSnapshot {
                    line: e.line,
                    item: e.item,
                    variant: e.variant,
                    name: e.name.clone(),
                    revenue_center: e.revenue_center,
                    quantity: e.quantity,
                    unit_price: e.unit_price,
                    modifiers: e.modifiers.clone(),
                    taxes: e.taxes.clone(),
                    discounts: vec![],
                    seat: e.seat,
                    course: e.course,
                    sent: false,
                    track_stock: e.track_stock,
                },
                voided: None,
                refunded: 0,
            });
        }
        Event::LineQtyChanged(e) => {
            order.require_open()?;
            if e.quantity < 0 {
                return Err(Problem::NegativeQuantity);
            }
            let line = order.line_mut(e.line)?;
            if line.snapshot.sent {
                return Err(Problem::LineAlreadySent(e.line));
            }
            line.snapshot.quantity = e.quantity;
        }
        Event::LineRemoved(e) => {
            order.require_open()?;
            let line = order.line_mut(e.line)?;
            if line.snapshot.sent {
                return Err(Problem::LineAlreadySent(e.line));
            }
            order.lines.retain(|l| l.snapshot.line != e.line);
        }
        Event::OrderSent(e) => {
            order.require_open()?;
            for id in &e.lines {
                order.line_mut(*id)?;
            }
            for id in &e.lines {
                order.line_mut(*id)?.snapshot.sent = true;
            }
        }
        Event::LineVoidedAfterSend(e) => {
            order.require_open()?;
            let line = order.line_mut(e.line)?;
            if !line.snapshot.sent {
                return Err(Problem::LineNotSent(e.line));
            }
            if line.voided.is_some() {
                return Err(Problem::LineAlreadyVoided(e.line));
            }
            line.voided = Some(e.reason);
        }
        Event::DiscountApplied(e) => {
            order.require_open()?;
            match e.line {
                Some(id) => order.line_mut(id)?.snapshot.discounts.push(e.discount.clone()),
                None => order.order_discounts.push(e.discount.clone()),
            }
        }
        Event::TableAssigned(e) => {
            order.require_open()?;
            order.table = Some(e.table);
        }
        Event::CheckLeaseTaken(e) => {
            order.require_open()?;
            order.check_lease_free(header)?;
            order.lease = Some(Lease { device: header.device, until_ms: e.until_ms });
        }
        Event::CheckLeaseReleased(_) => {
            if order.lease.as_ref().is_some_and(|l| l.device == header.device) {
                order.lease = None;
            }
        }
        Event::OrderSplit(e) => {
            // This order is `from`.
            order.require_open()?;
            order.check_lease(header)?;
            for s in &e.lines {
                order.line_mut(s.line)?;
            }
            let moved: Vec<LineId> = e.lines.iter().map(|s| s.line).collect();
            order.lines.retain(|l| !moved.contains(&l.snapshot.line));
        }
        Event::OrderMerged(e) if e.from == id => {
            order.require_open()?;
            order.check_lease(header)?;
            if order.paid().minor() != 0 {
                return Err(Problem::HasCapturedPayments);
            }
            order.lines.clear();
            order.status = Status::MergedInto(e.into);
        }
        Event::OrderMerged(e) => {
            order.require_open()?;
            order.check_lease(header)?;
            for s in &e.lines {
                if order.lines.iter().any(|l| l.snapshot.line == s.line) {
                    return Err(Problem::AlreadyExists);
                }
            }
            order.lines.extend(e.lines.iter().map(|s| Line {
                snapshot: s.clone(),
                voided: None,
                refunded: 0,
            }));
        }
        Event::PaymentStarted(e) => {
            order.require_open()?;
            soft = order.check_lease(header).err();
            if e.amount.currency() != order.currency {
                return Err(Problem::CurrencyMismatch);
            }
            if order.payments.iter().any(|p| p.id == e.payment) {
                return Err(Problem::AlreadyExists);
            }
            order.payments.push(Payment {
                id: e.payment,
                tender: e.tender.clone(),
                amount: e.amount,
                tip: Money::zero(order.currency),
                status: PaymentStatus::Started,
                provider_ref: None,
                given: None,
                device: header.device,
            });
        }
        Event::PaymentCaptured(e) => {
            // Allowed after close: a capture confirmed late by the provider
            // must still be recorded (money is never discarded).
            let p = order
                .payments
                .iter_mut()
                .find(|p| p.id == e.payment)
                .ok_or(Problem::UnknownPayment(e.payment))?;
            p.status = PaymentStatus::Captured;
            p.tip = e.tip;
            p.provider_ref = e.provider_ref.clone();
        }
        Event::PaymentFailed(e) => {
            let p = order
                .payments
                .iter_mut()
                .find(|p| p.id == e.payment)
                .ok_or(Problem::UnknownPayment(e.payment))?;
            p.status = PaymentStatus::Failed;
        }
        Event::CashTendered(e) => {
            // Cash in hand is recorded even if the order closed meanwhile on
            // another device; it then shows as overpaid.
            if order.status != Status::Open {
                soft = Some(Problem::OrderNotOpen);
            } else {
                soft = order.check_lease(header).err();
            }
            if e.given.minor() < e.applied.minor() {
                return Err(Problem::CashLessThanApplied);
            }
            if order.payments.iter().any(|p| p.id == e.payment) {
                return Err(Problem::AlreadyExists);
            }
            order.payments.push(Payment {
                id: e.payment,
                tender: Tender::Cash,
                amount: e.applied,
                tip: Money::zero(order.currency),
                status: PaymentStatus::Captured,
                provider_ref: None,
                given: Some(e.given),
                device: header.device,
            });
        }
        Event::OrderClosed(e) => {
            order.require_open()?;
            order.check_lease(header)?;
            let total = order.totals().map_err(|err| Problem::Pricing(err.to_string()))?.total;
            let paid = order.paid();
            if paid.minor() < total.minor() {
                return Err(Problem::Underpaid { paid, total });
            }
            order.status = Status::Closed;
            order.reported_total = Some(e.total);
            order.lease = None;
        }
        Event::OrderVoided(_) => {
            order.require_open()?;
            if order.paid().minor() != 0 {
                return Err(Problem::HasCapturedPayments);
            }
            order.status = Status::Voided;
        }
        Event::RefundIssued(e) => {
            if order.status != Status::Closed {
                return Err(Problem::NotClosed);
            }
            let available = order.paid().checked_sub(order.refunded()).unwrap_or(e.amount);
            if e.amount.minor() > available.minor() {
                return Err(Problem::RefundExceedsPaid);
            }
            for (id, qty) in &e.lines {
                let line = order
                    .lines
                    .iter()
                    .find(|l| l.snapshot.line == *id)
                    .ok_or(Problem::UnknownLine(*id))?;
                if *qty < 0 || line.refunded + qty > line.snapshot.quantity {
                    return Err(Problem::RefundExceedsSold(*id));
                }
            }
            for (id, qty) in &e.lines {
                order.line_mut(*id)?.refunded += qty;
            }
            order.refunds.push(Refund {
                id: e.refund,
                amount: e.amount,
                tender: e.tender.clone(),
                lines: e.lines.clone(),
            });
        }
        Event::FiscalSigned(e) => {
            order.fiscal = Some(e.fields.clone());
        }
        _ => {}
    }
    Ok(soft)
}
