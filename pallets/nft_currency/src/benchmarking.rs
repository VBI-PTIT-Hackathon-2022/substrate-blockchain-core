//! Benchmarking setup for pallet-nft_currency

use super::*;

#[allow(unused)]
use crate::Pallet as NFTCurrency;
use frame_benchmarking::{account,benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
	mint_token {
		let caller: T::AccountId = whitelisted_caller();
		let account: T::AccountId = account("account",1,1);
	}: mint_to(RawOrigin::Signed(caller),account)
	verify {
		assert_eq!(TotalTokens::<T>::get(), 1);
	}

	transfer_token {
		let acc1: T::AccountId =  account("account1",0,0);
		let acc2 : T::AccountId = account("account2",1,1);
		NFTCurrency::<T>::mint_to(RawOrigin::Signed(acc1.clone()).into(),acc1.clone());
		let token_id = OwnerToken::<T>::get(acc1.clone())[0].to_vec();
	}: transfer_token(RawOrigin::Signed(acc1.clone()),acc2.clone(), token_id.to_vec())
	verify {
		assert_eq!(OwnerToken::<T>::get(acc1).len(), 0);
		assert_eq!(OwnerToken::<T>::get(acc2).len(), 1);
	}

	safe_transfer{
		let acc1: T::AccountId =  account("account1",0,0);
		let acc2 : T::AccountId = account("account2",1,1);
		NFTCurrency::<T>::mint_to(RawOrigin::Signed(acc1.clone()).into(),acc1.clone());
		let token_id = &OwnerToken::<T>::get(acc1.clone())[0].to_vec();
		NFTCurrency::<T>::approve_for_all(RawOrigin::Signed(acc1.clone()).into(),acc2.clone());
	}: safe_transfer(RawOrigin::Signed(acc2.clone()),acc1.clone(),acc2.clone(),token_id.clone())
	verify {
		assert_eq!(OwnerToken::<T>::get(acc1).len(), 0);
		assert_eq!(OwnerToken::<T>::get(acc2).len(), 1);
	}

	approve{
		let acc1: T::AccountId =  account("account1",0,0);
		let acc2 : T::AccountId = account("account2",1,1);
		NFTCurrency::<T>::mint_to(RawOrigin::Signed(acc1.clone()).into(),acc1.clone());
		let token_id = OwnerToken::<T>::get(acc1.clone())[0].to_vec();
	}: approve(RawOrigin::Signed(acc1.clone()), acc2.clone(), token_id.to_vec())
	verify{
		assert_eq!(TokenApproval::<T>::get(token_id.to_vec()).len(),1);
	}

	approve_for_all{
		let acc1: T::AccountId =  account("account1",0,0);
		let acc2 : T::AccountId = account("account2",1,1);
	}: approve_for_all(RawOrigin::Signed(acc1.clone()), acc2.clone())
	verify{
		assert_eq!(Approval::<T>::get((acc1,acc2)),Some(true));
	}

	set_token_uri{
		let acc1: T::AccountId =  account("account1",0,0);
		NFTCurrency::<T>::mint_to(RawOrigin::Signed(acc1.clone()).into(),acc1.clone());
		let token_id = OwnerToken::<T>::get(acc1.clone())[0].to_vec();
		let token_uri = "ipfs";
	}: set_token_uri(RawOrigin::Signed(acc1.clone()),token_id.clone(),token_uri.as_bytes().to_vec())
	verify{
		assert_eq!(TokenUri::<T>::get(token_id), Some(token_uri.as_bytes().to_vec()));
	}

	impl_benchmark_test_suite!(NFTCurrency, crate::mock::new_test_ext(), crate::mock::Test);
}
