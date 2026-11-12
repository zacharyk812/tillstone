//! Roles and permissions (PLAN §9). Checked on the device that creates an
//! event and again, as warnings, on every peer that receives it.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::ids::RoleId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Permission {
    OrderCreate,
    OrderVoid,
    /// Void a line that was already sent to the kitchen.
    LineVoidAfterSend,
    DiscountApply,
    /// Discounts above the discount's approval threshold.
    DiscountLarge,
    RefundRequest,
    RefundApprove,
    PaymentTake,
    DrawerOpen,
    /// Open the drawer without a sale.
    DrawerNoSale,
    ShiftManage,
    ManageCatalog,
    ManageStaff,
    ReportsView,
    InventoryAdjust,
}

impl Permission {
    pub const ALL: [Self; 15] = [
        Self::OrderCreate,
        Self::OrderVoid,
        Self::LineVoidAfterSend,
        Self::DiscountApply,
        Self::DiscountLarge,
        Self::RefundRequest,
        Self::RefundApprove,
        Self::PaymentTake,
        Self::DrawerOpen,
        Self::DrawerNoSale,
        Self::ShiftManage,
        Self::ManageCatalog,
        Self::ManageStaff,
        Self::ReportsView,
        Self::InventoryAdjust,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub id: RoleId,
    pub name: String,
    pub permissions: BTreeSet<Permission>,
}

impl Role {
    pub fn allows(&self, p: Permission) -> bool {
        self.permissions.contains(&p)
    }

    /// Secure default roles (PLAN §9: strict permissions by default).
    pub fn cashier(id: RoleId) -> Self {
        use Permission::*;
        Self {
            id,
            name: "Cashier".into(),
            permissions: [OrderCreate, DiscountApply, RefundRequest, PaymentTake, DrawerOpen]
                .into(),
        }
    }

    pub fn manager(id: RoleId) -> Self {
        let mut role = Self::cashier(id);
        role.name = "Manager".into();
        role.permissions
            .extend(Permission::ALL.iter().copied().filter(|p| *p != Permission::ManageStaff));
        role
    }

    pub fn owner(id: RoleId) -> Self {
        Self { id, name: "Owner".into(), permissions: Permission::ALL.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_roles_are_strict() {
        let cashier = Role::cashier(RoleId::from_u128(1));
        assert!(cashier.allows(Permission::OrderCreate));
        assert!(!cashier.allows(Permission::RefundApprove));
        assert!(!cashier.allows(Permission::DrawerNoSale));
        assert!(!cashier.allows(Permission::LineVoidAfterSend));

        let manager = Role::manager(RoleId::from_u128(2));
        assert!(manager.allows(Permission::RefundApprove));
        assert!(!manager.allows(Permission::ManageStaff));

        assert!(Role::owner(RoleId::from_u128(3)).allows(Permission::ManageStaff));
    }
}
