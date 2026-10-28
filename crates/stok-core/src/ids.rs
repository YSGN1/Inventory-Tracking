use std::{fmt, str::FromStr};
use uuid::Uuid;

macro_rules! id_type {
    ($($name:ident),* $(,)?) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
                 serde::Serialize, serde::Deserialize, ts_rs::TS)]
        #[ts(export, type = "string")]
        pub struct $name(pub Uuid);

        impl $name {
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self { Self(Uuid::now_v7()) }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
        }
        impl FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> { Uuid::parse_str(s).map(Self) }
        }
    )*};
}

id_type!(TenantId, UserId, ProductId, VariantId, FirmId, MovementId);
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn ids_are_time_ordered() {
        let a = ProductId::new(); let b = ProductId::new();
        assert!(a.0 < b.0);
        assert_eq!(a.to_string().parse::<ProductId>().unwrap(), a);
    }
}
