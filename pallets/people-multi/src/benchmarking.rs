// This file is part of Substrate.

// Copyright (C) 2020-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;

use alloc::vec;
use frame_benchmarking::{account, v2::*, BenchmarkError};
use frame_support::{assert_ok, pallet_prelude::Get};
use frame_system::RawOrigin as SystemOrigin;
use individuality_support::traits::RI_ZERO;
use sp_runtime::{traits::AppendZerosInput, BoundedVec};

const SEED: u32 = 0;

type SecretOf<T> = <<T as Config>::Crypto as GenerateVerifiable>::Secret;

#[allow(unused)]
type ProofOf<T> = <<T as Config>::Crypto as GenerateVerifiable>::Proof;

#[allow(unused)]
fn assert_last_event<T: Config + Send + Sync>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

fn new_member_from<T: Config + Send + Sync>(i: u32, seed: u32) -> (SecretOf<T>, MemberOf<T>) {
	let mut entropy = &(i, seed).encode()[..];
	let mut entropy = AppendZerosInput::new(&mut entropy);
	let secret = T::Crypto::new_secret(Decode::decode(&mut entropy).unwrap());
	let public = T::Crypto::member_from_secret(&secret);
	(secret, public)
}

fn generate_members_for_ring<T: Config + Send + Sync>(
	seed: u32,
) -> Vec<(SecretOf<T>, MemberOf<T>)> {
	(0..T::MaxRingSize::get())
		.map(|i| new_member_from::<T>(i, seed))
		.collect::<Vec<_>>()
}

fn generate_members<T: Config + Send + Sync>(
	seed: u32,
	start: u8,
	end: u8,
) -> Vec<(SecretOf<T>, MemberOf<T>)> {
	(start..end).map(|i| new_member_from::<T>(i as u32, seed)).collect::<Vec<_>>()
}

pub fn recognize_people<T: Config + Send + Sync>(
	members: &[(SecretOf<T>, MemberOf<T>)],
) -> Vec<(PersonalId, MemberOf<T>, SecretOf<T>)> {
	let mut people = Vec::new();
	for (secret, public) in members.iter() {
		let person = pallet::Pallet::<T>::reserve_new_id();
		pallet::Pallet::<T>::recognize_personhood(person, Some(public.clone())).unwrap();
		people.push((person, public.clone(), secret.clone()));
	}

	people
}

pub trait ContextToUseInPeopleBenchmarks {
	fn valid_account_context() -> Context;
}

#[benchmarks(
	where T: Config + core::marker::Send + core::marker::Sync + ContextToUseInPeopleBenchmarks,
)]
mod benches {
	use super::*;
	use frame_support::{
		dispatch::RawOrigin,
		traits::{Len, UnfilteredDispatchable},
	};
	use sp_runtime::{traits::ValidateUnsigned, transaction_validity::TransactionSource};

	#[benchmark]
	fn as_personal_identity() -> Result<(), BenchmarkError> {
		// Generate people and build a ring
		let caller: T::AccountId = whitelisted_caller();
		let members = generate_members_for_ring::<T>(SEED);
		let recognized_people = recognize_people::<T>(&members);
		assert_ok!(pallet::Pallet::<T>::onboard_people(SystemOrigin::None.into()));
		assert_ok!(pallet::Pallet::<T>::build_ring(SystemOrigin::None.into(), RI_ZERO, None));

		// Select one of the generated people's information
		let (personal_id, _, secret): &(PersonalId, MemberOf<T>, SecretOf<T>) =
			&recognized_people[0];

		// A simple call to benchmark with
		let call = frame_system::Call::<T>::remark { remark: vec![] };
		let boxed_call: Box<<T as frame_system::Config>::RuntimeCall> = Box::new(call.into());
		let signature = boxed_call.using_encoded(|msg| {
			<T::Crypto as GenerateVerifiable>::sign(secret, msg)
				.expect("failed to create signature")
		});

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), *personal_id, boxed_call, signature);

		Ok(())
	}

	#[benchmark]
	fn as_personal_alias() -> Result<(), BenchmarkError> {
		// Generate people and build a ring
		let caller: T::AccountId = whitelisted_caller();
		let members = generate_members_for_ring::<T>(SEED);
		recognize_people::<T>(&members);
		assert_ok!(pallet::Pallet::<T>::onboard_people(SystemOrigin::None.into()));
		assert_ok!(pallet::Pallet::<T>::build_ring(SystemOrigin::None.into(), RI_ZERO, None));

		// A simple call to benchmark with
		let call = frame_system::Call::<T>::remark { remark: vec![] };
		let boxed_call: Box<<T as frame_system::Config>::RuntimeCall> = Box::new(call.into());

		let context = <T as ContextToUseInPeopleBenchmarks>::valid_account_context();

		// Generate a valid proof
		let proof = boxed_call.using_encoded(|msg| {
			let (secret, member) = &members[0];
			T::Crypto::create(
				T::Crypto::open(member, members.iter().map(|(_, m)| m.clone())).unwrap(),
				secret,
				&context[..],
				msg,
			)
			.map(|(p, _)| p)
			.expect("should create proof")
		});

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), context, boxed_call, proof, RI_ZERO);

		Ok(())
	}

	#[benchmark]
	fn under_alias() -> Result<(), BenchmarkError> {
		// Generate people and build a ring
		let members = generate_members_for_ring::<T>(SEED);
		recognize_people::<T>(&members);
		assert_ok!(pallet::Pallet::<T>::onboard_people(SystemOrigin::None.into()));
		assert_ok!(pallet::Pallet::<T>::build_ring(SystemOrigin::None.into(), RI_ZERO, None));

		// Create account and alias
		let account: T::AccountId = whitelisted_caller();
		let context = <T as ContextToUseInPeopleBenchmarks>::valid_account_context();
		let alias_value: Alias = [0u8; 32];
		let ra = RevisedContextualAlias {
			revision: 0,
			ring: RI_ZERO,
			ca: ContextualAlias { context, alias: alias_value },
		};

		// Set up alias account association
		let block_number = frame_system::Pallet::<T>::block_number();
		assert_ok!(pallet::Pallet::<T>::set_alias_account(
			Origin::PersonalAlias(ra.clone()).into(),
			account.clone(),
			block_number
		));
		assert!(AccountToAlias::<T>::contains_key(&account));
		assert!(AliasToAccount::<T>::contains_key(&ra.ca));

		// A simple call to benchmark with
		let call = frame_system::Call::<T>::remark { remark: vec![] };
		let boxed_call = Box::new(call.into());

		#[extrinsic_call]
		_(RawOrigin::Signed(account), boxed_call);

		Ok(())
	}

	#[benchmark]
	fn set_alias_account() -> Result<(), BenchmarkError> {
		// Generate people and build a ring
		let members = generate_members_for_ring::<T>(SEED);
		recognize_people::<T>(&members);
		assert_ok!(pallet::Pallet::<T>::onboard_people(SystemOrigin::None.into()));
		assert_ok!(pallet::Pallet::<T>::build_ring(SystemOrigin::None.into(), RI_ZERO, None));

		let block_number = frame_system::Pallet::<T>::block_number();

		let alias_value: Alias = [0u8; 32];
		let alias = RevisedContextualAlias {
			ca: ContextualAlias {
				context: <T as ContextToUseInPeopleBenchmarks>::valid_account_context(),
				alias: alias_value,
			},
			revision: 0,
			ring: 0,
		};

		// An account had already been assigned to this alias
		let old_account: T::AccountId = account("test_old", 0, SEED);
		assert_ok!(pallet::Pallet::<T>::set_alias_account(
			Origin::PersonalAlias(alias.clone()).into(),
			old_account.clone(),
			block_number
		));
		assert!(AccountToAlias::<T>::contains_key(&old_account));
		assert!(AliasToAccount::<T>::contains_key(&alias.ca));

		let account: T::AccountId = account("test", 0, SEED);

		#[extrinsic_call]
		_(Origin::PersonalAlias(alias.clone()), account.clone(), block_number);

		assert!(!AccountToAlias::<T>::contains_key(&old_account));
		assert!(AccountToAlias::<T>::contains_key(&account));
		assert!(AliasToAccount::<T>::contains_key(&alias.ca));
		assert_eq!(AliasToAccount::<T>::get(&alias.ca), Some(account));

		Ok(())
	}

	#[benchmark]
	fn unset_alias_account() -> Result<(), BenchmarkError> {
		// Generate people and build a ring
		let members = generate_members_for_ring::<T>(SEED);
		recognize_people::<T>(&members);
		assert_ok!(pallet::Pallet::<T>::onboard_people(SystemOrigin::None.into()));
		assert_ok!(pallet::Pallet::<T>::build_ring(SystemOrigin::None.into(), RI_ZERO, None));

		let account: T::AccountId = account("test", 0, SEED);
		let block_number = frame_system::Pallet::<T>::block_number();

		let alias_value: Alias = [0u8; 32];
		let alias = RevisedContextualAlias {
			ca: ContextualAlias {
				context: <T as ContextToUseInPeopleBenchmarks>::valid_account_context(),
				alias: alias_value,
			},
			revision: 0,
			ring: 0,
		};

		assert_ok!(pallet::Pallet::<T>::set_alias_account(
			Origin::PersonalAlias(alias.clone()).into(),
			account.clone(),
			block_number
		));
		assert!(AccountToAlias::<T>::contains_key(&account));
		assert!(AliasToAccount::<T>::contains_key(&alias.ca));

		#[extrinsic_call]
		_(Origin::PersonalAlias(alias.clone()));

		assert!(!AccountToAlias::<T>::contains_key(&account));
		assert!(!AliasToAccount::<T>::contains_key(&alias.ca));

		Ok(())
	}

	#[benchmark]
	fn force_recognize_personhood() -> Result<(), BenchmarkError> {
		let members = generate_members_for_ring::<T>(SEED);

		#[extrinsic_call]
		_(SystemOrigin::Root, members.iter().map(|(_, m)| m.clone()).collect::<Vec<_>>());

		for person in members {
			assert!(pallet::Keys::<T>::get(person.1).is_some());
		}

		Ok(())
	}

	#[benchmark]
	fn set_personal_id_account() -> Result<(), BenchmarkError> {
		// Generate people and build a ring
		let members = generate_members_for_ring::<T>(SEED);
		let people = recognize_people::<T>(&members);
		assert_ok!(pallet::Pallet::<T>::onboard_people(SystemOrigin::None.into()));
		assert_ok!(pallet::Pallet::<T>::build_ring(SystemOrigin::None.into(), RI_ZERO, None));

		// Get one of the generated people's information
		let (personal_id, _, _): &(PersonalId, MemberOf<T>, SecretOf<T>) = &people[0];

		let account: T::AccountId = account("test", 0, SEED);
		let block_number = frame_system::Pallet::<T>::block_number();

		// An account had already been assigned to this personal id
		let old_account: T::AccountId = frame_benchmarking::account("test_old", 0, SEED);
		assert_ok!(pallet::Pallet::<T>::set_personal_id_account(
			Origin::PersonalIdentity(*personal_id).into(),
			old_account.clone(),
			block_number
		));

		#[extrinsic_call]
		_(Origin::PersonalIdentity(*personal_id), account.clone(), block_number);

		assert_eq!(AccountToPersonalId::<T>::get(&old_account), None);
		assert_eq!(AccountToPersonalId::<T>::get(&account), Some(*personal_id));
		assert!(People::<T>::get(personal_id).is_some());
		assert_eq!(People::<T>::get(personal_id).unwrap().account, Some(account));

		Ok(())
	}

	#[benchmark]
	fn unset_personal_id_account() -> Result<(), BenchmarkError> {
		// Generate people and build a ring
		let members = generate_members_for_ring::<T>(SEED);
		let people = recognize_people::<T>(&members);
		assert_ok!(pallet::Pallet::<T>::onboard_people(SystemOrigin::None.into()));
		assert_ok!(pallet::Pallet::<T>::build_ring(SystemOrigin::None.into(), RI_ZERO, None));

		// Get one of the generated people's information
		let (personal_id, _, _): &(PersonalId, MemberOf<T>, SecretOf<T>) = &people[0];

		let account: T::AccountId = account("test", 0, SEED);
		let block_number = frame_system::Pallet::<T>::block_number();

		// An account had already been assigned to this personal id
		let old_account: T::AccountId = frame_benchmarking::account("test_old", 0, SEED);
		assert_ok!(pallet::Pallet::<T>::set_personal_id_account(
			Origin::PersonalIdentity(*personal_id).into(),
			old_account.clone(),
			block_number
		));

		#[extrinsic_call]
		_(Origin::PersonalIdentity(*personal_id));

		assert_eq!(AccountToPersonalId::<T>::get(&old_account), None);
		assert_eq!(AccountToPersonalId::<T>::get(&account), None);
		assert!(People::<T>::get(personal_id).is_some());
		assert_eq!(People::<T>::get(personal_id).unwrap().account, None);

		Ok(())
	}

	#[benchmark]
	fn set_onboarding_size() -> Result<(), BenchmarkError> {
		#[extrinsic_call]
		_(SystemOrigin::Root, u32::MAX);

		assert_eq!(OnboardingSize::<T>::get(), u32::MAX);

		Ok(())
	}

	#[benchmark]
	fn merge_rings() -> Result<(), BenchmarkError> {
		// Two rings exist
		let ring_size: u32 = <T as Config>::MaxRingSize::get();
		let members = generate_members::<T>(SEED, 0, ring_size as u8 * 2);
		recognize_people::<T>(&members);
		assert_ok!(pallet::Pallet::<T>::onboard_people(SystemOrigin::None.into()));
		assert_ok!(pallet::Pallet::<T>::onboard_people(SystemOrigin::None.into()));
		assert_ok!(pallet::Pallet::<T>::build_ring(SystemOrigin::None.into(), RI_ZERO, None));
		assert_ok!(pallet::Pallet::<T>::build_ring(SystemOrigin::None.into(), 1, None));

		// Suspend and remove more than half of the people in both rings
		assert_ok!(pallet::Pallet::<T>::start_people_set_mutation_session());
		let suspensions: Vec<PersonalId> = (1..ring_size / 2 + 2)
			.chain(ring_size + 1..ring_size * 3 / 2 + 2)
			.map(|i| i as PersonalId)
			.collect();
		assert_ok!(pallet::Pallet::<T>::suspend_personhood(&suspensions));
		assert_ok!(pallet::Pallet::<T>::end_people_set_mutation_session());

		let suspended_indices: BoundedVec<u32, _> =
			(1..ring_size / 2 + 2).collect::<Vec<_>>().try_into().unwrap();
		assert_ok!(pallet::Pallet::<T>::remove_suspended_people(
			SystemOrigin::None.into(),
			RI_ZERO,
			suspended_indices.clone()
		));
		assert_ok!(pallet::Pallet::<T>::remove_suspended_people(
			SystemOrigin::None.into(),
			1,
			suspended_indices
		));

		// The current ring has to have a higher index than the ones being merged
		CurrentRingIndex::<T>::set(14);

		#[extrinsic_call]
		_(SystemOrigin::None, RI_ZERO, 1);

		assert_eq!(RingKeys::<T>::get(RI_ZERO).len(), 8);
		assert_eq!(RingKeysStatus::<T>::get(RI_ZERO).total, 8);
		assert!(Root::<T>::get(RI_ZERO).is_some());
		assert!(Root::<T>::get(1).is_none());

		Ok(())
	}

	#[benchmark]
	fn validate_unsigned_with_build_ring(
		n: Linear<1, { T::MaxRingSize::get() }>,
	) -> Result<(), BenchmarkError> {
		// One full queue page of people awaiting
		let queue_page_size: u32 = <T as Config>::OnboardingQueuePageSize::get();
		let members = generate_members::<T>(SEED, 0, queue_page_size as u8);
		recognize_people::<T>(&members);
		assert_ok!(pallet::Pallet::<T>::onboard_people(SystemOrigin::None.into()));

		// No ring built but people onboarded successfully
		assert!(Root::<T>::get(RI_ZERO).is_none());
		assert_eq!(RingKeys::<T>::get(RI_ZERO).len(), 10);
		assert_eq!(RingKeysStatus::<T>::get(RI_ZERO), RingStatus { total: 10, included: 0 });

		#[block]
		{
			let call = Call::build_ring { ring_index: RI_ZERO, limit: Some(n) };
			<pallet::Pallet<T> as ValidateUnsigned>::validate_unsigned(
				TransactionSource::Local,
				&call,
			)
			.map_err(|e| -> &'static str { e.into() })?;
			call.dispatch_bypass_filter(RawOrigin::None.into())?;
		}

		// The ring becomes built
		assert!(Root::<T>::get(RI_ZERO).is_some());
		assert_eq!(RingKeys::<T>::get(RI_ZERO).len(), 10);
		assert_eq!(RingKeysStatus::<T>::get(RI_ZERO), RingStatus { total: 10, included: n });

		Ok(())
	}

	#[benchmark]
	fn validate_unsigned_with_onboard_people() -> Result<(), BenchmarkError> {
		// One full ring exists
		let ring_size: u32 = <T as Config>::MaxRingSize::get();
		let members = generate_members::<T>(SEED, 0, ring_size as u8);
		recognize_people::<T>(&members);
		assert_ok!(pallet::Pallet::<T>::onboard_people(SystemOrigin::None.into()));
		assert_ok!(pallet::Pallet::<T>::build_ring(SystemOrigin::None.into(), RI_ZERO, None));
		assert_eq!(RingKeys::<T>::get(RI_ZERO).len(), 10);
		assert_eq!(RingKeysStatus::<T>::get(RI_ZERO), RingStatus { total: 10, included: 10 });

		assert_eq!(QueuePageIndices::<T>::get(), (0, 0));
		assert!(OnboardingQueue::<T>::get(0).is_empty());

		// 1st onboarding page with fewer people than open slots
		let keys_len: u8 = Keys::<T>::iter().collect::<Vec<_>>().len().try_into().unwrap();
		let members = generate_members::<T>(SEED, keys_len, keys_len + ring_size as u8 / 2);
		recognize_people::<T>(&members);
		assert_eq!(OnboardingQueue::<T>::get(0).len(), (ring_size as u8 / 2) as usize);

		// To stop adding keys to the first page and start filling the next one
		QueuePageIndices::<T>::put((0, 1));
		assert!(OnboardingQueue::<T>::get(1).is_empty());

		// 2nd onboarding page full
		let keys_len: u8 = Keys::<T>::iter().collect::<Vec<_>>().len().try_into().unwrap();
		assert_eq!(keys_len, (ring_size + ring_size / 2) as u8);
		let queue_page_size: u32 = <T as Config>::OnboardingQueuePageSize::get();
		let members = generate_members::<T>(SEED, keys_len, keys_len + queue_page_size as u8);
		recognize_people::<T>(&members);

		assert_eq!(QueuePageIndices::<T>::get(), (0, 1));
		assert_eq!(OnboardingQueue::<T>::get(0).len(), (ring_size as u8 / 2) as usize);
		assert!(OnboardingQueue::<T>::get(1).is_full());

		assert_eq!(RingKeys::<T>::get(1).len(), 0);

		#[block]
		{
			let call = Call::onboard_people {};
			<pallet::Pallet<T> as ValidateUnsigned>::validate_unsigned(
				TransactionSource::Local,
				&call,
			)
			.map_err(|e| -> &'static str { e.into() })?;
			call.dispatch_bypass_filter(RawOrigin::None.into())?;
		}

		assert_eq!(RingKeys::<T>::get(1).len(), 10);
		assert_eq!(RingKeysStatus::<T>::get(1), RingStatus { total: 10, included: 0 });

		Ok(())
	}

	#[benchmark]
	fn validate_unsigned_with_remove_suspended_people(
		n: Linear<1, { T::MaxRingSize::get() }>,
	) -> Result<(), BenchmarkError> {
		// Generate people and build a ring
		let members = generate_members_for_ring::<T>(SEED);
		recognize_people::<T>(&members);
		assert_ok!(pallet::Pallet::<T>::onboard_people(SystemOrigin::None.into()));
		assert_ok!(pallet::Pallet::<T>::build_ring(SystemOrigin::None.into(), RI_ZERO, None));

		// For later verification
		let initial_root = Root::<T>::get(RI_ZERO).unwrap();

		// Suspend 'n' number of people in the ring
		assert_ok!(pallet::Pallet::<T>::start_people_set_mutation_session());
		let suspensions: Vec<PersonalId> = (0..n as PersonalId).collect();
		assert_ok!(pallet::Pallet::<T>::suspend_personhood(&suspensions));
		assert_ok!(pallet::Pallet::<T>::end_people_set_mutation_session());

		// To make sure they are indeed pending suspension
		assert_eq!(PendingSuspensions::<T>::get(RI_ZERO), n as u32);

		let suspended_indices: BoundedVec<u32, _> =
			(0..n as u32).collect::<Vec<_>>().try_into().unwrap();

		#[block]
		{
			let call = Call::remove_suspended_people { ring_index: RI_ZERO, suspended_indices };
			<pallet::Pallet<T> as ValidateUnsigned>::validate_unsigned(
				TransactionSource::Local,
				&call,
			)
			.map_err(|e| -> &'static str { e.into() })?;
			call.dispatch_bypass_filter(RawOrigin::None.into())?;
		}

		// Pending suspensions are cleared for the ring
		assert_eq!(PendingSuspensions::<T>::get(RI_ZERO), 0);

		// Ring data becomes modified
		let ring_size: u32 = <T as Config>::MaxRingSize::get();
		assert_eq!(
			RingKeysStatus::<T>::get(RI_ZERO),
			RingStatus { included: 0, total: ring_size - n as u32 }
		);
		assert_eq!(RingKeys::<T>::get(RI_ZERO).len(), (ring_size - n as u32) as usize);
		assert_ne!(Root::<T>::get(RI_ZERO).unwrap().intermediate, initial_root.intermediate);

		Ok(())
	}

	#[benchmark]
	fn validate_unsigned_with_merge_queue_pages() -> Result<(), BenchmarkError> {
		// Two pages exists: first is full, the second contains one member
		let queue_page_size: u32 = <T as Config>::OnboardingQueuePageSize::get();
		let members = generate_members::<T>(SEED, 0, queue_page_size as u8 + 1);
		recognize_people::<T>(&members);

		assert_eq!(QueuePageIndices::<T>::get(), (0, 1));
		assert!(OnboardingQueue::<T>::get(0).is_full());
		assert_eq!(OnboardingQueue::<T>::get(1).len(), 1);

		// One key is removed from the first page
		OnboardingQueue::<T>::mutate(0, |keys| {
			keys.pop();
		});
		assert_eq!(OnboardingQueue::<T>::get(0).len(), queue_page_size as usize - 1);

		#[block]
		{
			let call = Call::merge_queue_pages {};
			<pallet::Pallet<T> as ValidateUnsigned>::validate_unsigned(
				TransactionSource::Local,
				&call,
			)
			.map_err(|e| -> &'static str { e.into() })?;
			call.dispatch_bypass_filter(RawOrigin::None.into())?;
		}

		// The queue pages have changed
		assert_eq!(QueuePageIndices::<T>::get(), (1, 1));
		assert!(OnboardingQueue::<T>::get(0).is_empty());
		assert!(OnboardingQueue::<T>::get(1).is_full());

		Ok(())
	}

	// Implements a test for each benchmark. Execute with:
	// `cargo test -p pallet-people --features runtime-benchmarks`.
	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
