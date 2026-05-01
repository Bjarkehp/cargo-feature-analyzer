pub mod feature_stats;
pub mod model_stats;
pub mod configuration_stats;
pub mod satisfiability;
pub mod line_count;
pub mod running_time_fca;
pub mod running_time_static;

pub trait Row {
    type Key: Ord;

    fn key(&self) -> Self::Key;
}

#[macro_export]
macro_rules! impl_key_crate_id {
    ($t:ty) => {
        impl $crate::result::Row for $t {
            type Key = CrateId;

            fn key(&self) -> Self::Key {
                self.crate_id.clone()
            }
        }
    };
}