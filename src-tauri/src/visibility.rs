//! Silinmiş varlıklara ait geçmiş yalnız Yönetici'ye, etiketli olarak gösterilir.

use stok_core::movement::MovementView;
use stok_core::report::{FirmDeliveryRow, display_firm_name, display_name};
use stok_core::user::{Permission, Role, can};

/// Liste/rapor sorgularında silinmiş satırlar dahil mi? İstemciden alınmaz, rolden belirlenir.
pub fn include_deleted(role: Role) -> bool {
    can(role, Permission::ViewDeleted)
}

pub fn label_movement(mut v: MovementView) -> MovementView {
    v.movement.product_name = display_name(&v.movement.product_name, v.entity_deleted);
    if let Some(f) = &v.movement.firm_name {
        v.movement.firm_name = Some(display_firm_name(f, v.firm_deleted));
    }
    v
}

pub fn label_delivery(mut r: FirmDeliveryRow) -> FirmDeliveryRow {
    r.product_name = display_name(&r.product_name, r.entity_deleted);
    r.firm_name = display_firm_name(&r.firm_name, r.firm_deleted);
    r
}
