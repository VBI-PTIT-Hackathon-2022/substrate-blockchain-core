// //! Benchmarking setup for pallet-nft_currency
//
// use super::*;
//
// #[allow(unused)]
// use crate::Pallet as Renting;
// use pallet_nft_currency as NftCurrency;
// use account_to_bytes;
// use frame_benchmarking::{account, benchmarks, whitelisted_caller};
// use frame_system::RawOrigin;
// benchmarks! {
// 	create_rental {
// 		let caller: T::AccountId = whitelisted_caller();
// 		let account: T::AccountId = account("account",1,1);
// 		let tokenId = NftCurrency::Pallet::mint_to()
// 		let orderLeft = Order{
// 			lender: caller,
// 			borrower:[0u8;32],
// 			fee:10000,
//
// 		}
// 		let uri = "linkUri";
// 	}: create_rental(RawOrigin::Signed(caller),account,caller,uri)
// 	verify {
// 		assert_eq!(TotalTokens::<T>::get(), 1);
// 		assert_eq!(TokenUri::<T>::get(), 1);
// 	}
//
//
//
// 	impl_benchmark_test_suite!(Renting, crate::mock::new_test_ext(), crate::mock::Test);
// }
