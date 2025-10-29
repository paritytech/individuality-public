// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Individuality.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.

// Individuality is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Individuality.  If not, see <http://www.gnu.org/licenses/>.

//! People lite pallet benchmarks

#![allow(unused)]

use super::*;
use crate::Pallet as Resources;
use core::time::Duration;
use frame_benchmarking::v2::{benchmarks, *};
use frame_support::{
	assert_ok,
	dispatch::{DispatchInfo, PostDispatchInfo},
	traits::EnsureOrigin,
	BoundedVec,
};
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin as SystemOrigin};
use sp_core::Get;
use sp_runtime::traits::{
	AsTransactionAuthorizedOrigin, DispatchTransaction, Dispatchable, Verify, Zero,
};

/// Benchmark helper trait.
pub trait BenchmarkHelper<T: Config> {
	/// Sets a time in seconds since the UNIX epoch for benchmarks.
	fn set_time(now: Duration);
	/// Sign a message.
	fn sign_message(message: &[u8]) -> (T::AccountId, T::OffchainSignature);
}

// --- Helpers

fn assert_last_event<T: Config>(generic_event: <T as frame_system::Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

#[benchmarks(
	where T: Config + core::marker::Send + core::marker::Sync,
)]
mod benches {
	use super::*;

	#[benchmark]
	fn register_lite_person() -> Result<(), BenchmarkError> {
		let origin =
			T::EnsureLitePerson::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let Ok(account) = T::EnsureLitePerson::try_origin(origin.clone()) else {
			panic!("origin was created with `try_successful_origin`; qed");
		};
		let identifier_key = [0u8; 65];
		let username = Username::try_from(b"validusername.12".to_vec()).unwrap();
		let reserved = Username::try_from(b"validreserved".to_vec()).unwrap();
		T::BenchmarkHelper::set_time(Duration::from_secs(1000));

		#[extrinsic_call]
		_(origin.clone(), [0u8; 65], username, Some(reserved.clone()));

		assert_last_event::<T>(Event::LitePersonRegistered { account }.into());
		Ok(())
	}

	#[benchmark]
	fn register_person() -> Result<(), BenchmarkError> {
		let identifier_key = [0u8; 65];
		let username = Username::try_from(b"validusername.12".to_vec()).unwrap();
		let reserved = Username::try_from(b"validreserved".to_vec()).unwrap();
		T::BenchmarkHelper::set_time(Duration::from_secs(1000));

		let (account, _) = T::BenchmarkHelper::sign_message(b"mock");
		let reservation =
			UsernameReservation { owner: account.clone(), since: T::Clock::now().as_secs() };
		ReservedUsernames::<T>::insert(&reserved, reservation);
		UsernameOwnerOf::<T>::insert(&username, &account);
		let info = ConsumerInfo {
			identifier_key,
			full_username: None,
			lite_username: username,
			credibility: Credibility::Lite,
		};
		Consumers::<T>::insert(&account, info);
		frame_system::Pallet::<T>::inc_sufficients(&account);

		let origin = T::EnsurePerson::try_successful_origin(&RESOURCES_CONTEXT)
			.map_err(|_| BenchmarkError::Weightless)?;
		let Ok(alias) = T::EnsurePerson::try_origin(origin.clone(), &RESOURCES_CONTEXT) else {
			panic!("origin was created with `try_successful_origin`; qed");
		};
		let username_choice = PersonalUsernameChoice::Reservation(reserved);
		let (_, proof) = T::BenchmarkHelper::sign_message(&alias[..]);

		#[extrinsic_call]
		_(origin.clone(), account.clone(), proof, username_choice);

		assert_last_event::<T>(Event::PersonRegistered { account, alias }.into());
		Ok(())
	}

	#[benchmark]
	fn touch_person_authorization() -> Result<(), BenchmarkError> {
		let identifier_key = [0u8; 65];
		let username = Username::try_from(b"validusername.12".to_vec()).unwrap();
		let reserved = Username::try_from(b"validreserved".to_vec()).unwrap();
		T::BenchmarkHelper::set_time(Duration::from_secs(1000));

		let (account, _) = T::BenchmarkHelper::sign_message(b"mock");
		let reservation =
			UsernameReservation { owner: account.clone(), since: T::Clock::now().as_secs() };
		ReservedUsernames::<T>::insert(&reserved, reservation);
		UsernameOwnerOf::<T>::insert(&username, &account);
		let info = ConsumerInfo {
			identifier_key,
			full_username: None,
			lite_username: username,
			credibility: Credibility::Lite,
		};
		Consumers::<T>::insert(&account, info);
		frame_system::Pallet::<T>::inc_sufficients(&account);

		let origin = T::EnsurePerson::try_successful_origin(&RESOURCES_CONTEXT)
			.map_err(|_| BenchmarkError::Weightless)?;
		let Ok(alias) = T::EnsurePerson::try_origin(origin.clone(), &RESOURCES_CONTEXT) else {
			panic!("origin was created with `try_successful_origin`; qed");
		};
		let username_choice = PersonalUsernameChoice::Reservation(reserved);
		let (_, proof) = T::BenchmarkHelper::sign_message(&alias[..]);
		Pallet::<T>::register_person(origin.clone(), account.clone(), proof, username_choice);

		T::BenchmarkHelper::set_time(Duration::from_secs(
			1000 + T::MinPersonAuthUpdateInterval::get() as u64 + 1,
		));

		#[extrinsic_call]
		_(origin);

		let now = T::Clock::now().as_secs();
		assert!(matches!(
			Consumers::<T>::get(&account).unwrap().credibility,
			Credibility::Person { last_update: now, .. }
		));

		Ok(())
	}

	#[benchmark]
	fn remove_expired_username_reservation() -> Result<(), BenchmarkError> {
		let origin =
			T::EnsureLitePerson::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let identifier_key = [0u8; 65];
		let username = Username::try_from(b"validusername.12".to_vec()).unwrap();
		let reserved = Username::try_from(b"validreserved".to_vec()).unwrap();
		T::BenchmarkHelper::set_time(Duration::from_secs(1000));

		assert_ok!(Pallet::<T>::register_lite_person(
			origin.clone(),
			[0u8; 65],
			username,
			Some(reserved.clone())
		));
		assert_eq!(ReservedUsernames::<T>::get(&reserved).unwrap().since, 1000);

		T::BenchmarkHelper::set_time(Duration::from_secs(
			1000 + T::UsernameReservationDuration::get() as u64 + 1,
		));

		#[extrinsic_call]
		_(origin, reserved.clone());

		assert!(!ReservedUsernames::<T>::contains_key(&reserved));

		Ok(())
	}

	#[benchmark]
	fn update_identifier_key() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let username = Username::try_from(b"validusername.12".to_vec()).unwrap();
		let consumer_info = ConsumerInfo {
			identifier_key: [0u8; 65],
			full_username: None,
			lite_username: username,
			credibility: Credibility::Lite,
		};
		Consumers::<T>::insert(&caller, consumer_info);

		let new_key = [1u8; 65];

		#[extrinsic_call]
		_(SystemOrigin::Signed(caller.clone()), new_key);

		assert_eq!(Consumers::<T>::get(caller).unwrap().identifier_key, new_key);

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
