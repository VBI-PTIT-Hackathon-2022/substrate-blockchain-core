use frame_support::traits::{Get, GetStorageVersion, PalletInfoAccess, StorageVersion};

use super::*;

/// Migrate the pallet storage to v1.
pub fn migrate_to_v1<T: Config<I>, I: 'static, P: GetStorageVersion + PalletInfoAccess>() -> frame_support::weights::Weight {
	let on_chain_storage_version = <P as GetStorageVersion>::on_chain_storage_version();
	log::info!(
		target: "runtime::uniques",
		"Running migration storage v1 for uniques with storage version {:?}",
		on_chain_storage_version,
	);

	if on_chain_storage_version < 1 {
		let mut count = 0;
		for (collection, detail) in Collection::<T, I>::iter() {
			CollectionAccount::<T, I>::insert(&detail.owner, &collection, ());
			count += 1;
		}
		StorageVersion::new(1).put::<P>();
		log::info!(
			target: "runtime::uniques",
			"Running migration storage v1 for uniques with storage version {:?} was complete",
			on_chain_storage_version,
		);
		// calculate and return migration weights
		T::DbWeight::get().reads_writes(count as u64 + 1, count as u64 + 1)
	} else {
		log::warn!(
			target: "runtime::uniques",
			"Attempted to apply migration to v1 but failed because storage version is {:?}",
			on_chain_storage_version,
		);
		T::DbWeight::get().reads(1)
	}
}
