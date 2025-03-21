// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
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

use crate::{
	extension::{AsPerson, AsPersonInfo},
	mock::*,
	pallet::{AccountToPersonalId, Origin as PeopleOrigin},
	*,
};
use frame_support::{
	assert_noop, assert_ok,
	dispatch::Pays,
	traits::{ExtrinsicCall, Get},
	BoundedVec,
};
use individuality_support::traits::RI_ZERO;
use sp_runtime::transaction_validity::{InvalidTransaction, InvalidTransaction::BadSigner};
use verifiable::demo_impls::Simple;

fn generate_people_with_index(
	start: u8,
	end: u8,
) -> Vec<(PersonalId, MemberOf<Test>, SecretOf<Test>)> {
	let mut people = Vec::new();
	for i in start..=end {
		let person = PeoplePallet::reserve_new_id();
		let secret = Simple::new_secret([i; 32]);
		let public = Simple::member_from_secret(&secret);

		PeoplePallet::recognize_personhood(person, Some(public)).unwrap();
		people.push((person, public, secret));
	}

	people
}

fn suspended_indices_list(ring_index: u32) -> BoundedVec<u32, <Test as Config>::MaxRingSize> {
	let suspended_indices = PendingSuspensions::<Test>::get(ring_index);
	assert!(&suspended_indices[..].windows(2).all(|pair| pair[0] < pair[1]));
	suspended_indices
}

#[test]
fn build_ring_works() {
	TestExt::new().execute_with(|| {
		PeoplePallet::set_onboarding_size(RuntimeOrigin::root(), 5).unwrap();
		// No one to onboard.
		assert_noop!(
			PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None),
			Error::<Test>::StillFresh
		);

		// Not enough for a queue
		generate_people_with_index(0, 3);

		// People are recognized but not onboarded yet, the ring has 0 members, same as initial
		// value.
		assert_eq!(Keys::<Test>::count(), 4);
		assert_noop!(
			PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None),
			Error::<Test>::StillFresh
		);

		// Onboard people
		assert_noop!(
			PeoplePallet::onboard_people(RuntimeOrigin::none()),
			Error::<Test>::Incomplete
		);

		// Now we have enough to build one.
		generate_people_with_index(4, 4);
		assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));

		// There isn't a root yet.
		assert!(!Root::<Test>::contains_key(0));
		assert_eq!(RingKeysStatus::<Test>::get(0), RingStatus { total: 5, included: 0 });
		// Build the root.
		assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));
		assert!(Root::<Test>::contains_key(0));

		// We can add 5 more people
		generate_people_with_index(5, 9);

		assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
		assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

		// We can add 26 more people
		generate_people_with_index(10, 35);

		assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
		assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), 1, None));

		assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
		assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), 2, None));

		// Can't build 3, because then there are only 4 spots left which is less than onboarding
		// size of 5
		assert_noop!(
			PeoplePallet::onboard_people(RuntimeOrigin::none()),
			Error::<Test>::Incomplete
		);
	});
}

// Basic End to End scenario:
// 1. People are "recognized" from an outside source.
// 2. The root is refreshed which begins the root building process.
// 3. They are pushed into a building ring.
// 4. Once all people are pushed in, we can bake the ring.
// 5. After the expiration time, the ring can be refreshed again.
// x. And taking core actions outside of this flow will error.
// #[test]
// fn basic_end_to_end_works() {
// 	TestExt::new().execute_with(|| {
// 		// Step 1: Here we create a group of people and recognize all of them.
// 		let people = generate_people(10);

// 		// Cannot bake root before it starts building.
// 		assert_noop!(
// 			PeoplePallet::bake_root(RuntimeOrigin::none(), RI_ZERO),
// 			Error::<Test>::NotBuilding
// 		);
// 		// Step 2: So let's refresh the root and start the process.
// 		assert_ok!(PeoplePallet::refresh_root(RuntimeOrigin::none()));
// 		// Can't start the process again until the process is done.
// 		assert_noop!(
// 			PeoplePallet::refresh_root(RuntimeOrigin::none()),
// 			Error::<Test>::AlreadyBuilding
// 		);
// 		// Cannot bake root before until the ring is populated.
// 		assert_noop!(
// 			PeoplePallet::bake_root(RuntimeOrigin::none(), RI_ZERO),
// 			Error::<Test>::NotBuilding
// 		);

// 		// Step 3: Let's push people into the ring.
// 		for person in &people {
// 			assert_ok!(PeoplePallet::push_member(RuntimeOrigin::none(), person.0));
// 		}

// 		// Step 4: Now we can bake the ring.
// 		assert_ok!(PeoplePallet::bake_root(RuntimeOrigin::none(), RI_ZERO));
// 		// The ring does not expire instantly.
// 		assert_noop!(
// 			PeoplePallet::refresh_root(RuntimeOrigin::none()),
// 			Error::<Test>::StillFreshEnough
// 		);
// 		// Cannot take other actions when the root is not building.
// 		assert_noop!(
// 			PeoplePallet::push_member(RuntimeOrigin::none(), people[0].0),
// 			Error::<Test>::NotBuilding
// 		);
// 		assert_noop!(
// 			PeoplePallet::bake_root(RuntimeOrigin::none(), RI_ZERO),
// 			Error::<Test>::NotBuilding
// 		);

// 		// 5. Let's go to the expiration date, and then we should be able to refresh the ring.
// 		System::set_block_number(<Test as Config>::RootLifespan::get());
// 		assert_ok!(PeoplePallet::refresh_root(RuntimeOrigin::none()));

// 		// And the process starts again...
// 	});
// }

// The `PeopleTrait` as a method to `suspend_index` in addition to `recognize_personhood`.
// This API can be called at any point in time, even during the middle of a ring building process.
// This test verifies that the `suspend_index` API works during the different stages of ring
// building.
//
// Scenarios:
// 1. While a ring is not being built, `suspend_index` should:
// 	- directly remove people, as if they were never recognized to begin with.
// 2. While the ring is being built, if they have not been "pushed", `suspend_index` should:
// 	- directly remove people, as if they were never recognized to begin with.
// 3. While the ring is being built, if they have already been "pushed", it should:
// 	- Not block the ring building process.
// 	- Not allow their key to be validated by the ring.
// 4. While the ring is being built, if they have already been "pushed":
// 	- We should be able to `suspend_index`, then `recognize_person` again, and the ring should still
//    build.

// This test checks scenario 1, which is the behavior of `suspend_index` when the root is not being
// built. In this case, suspending an index should act as if they were never recognized.
// #[test]
// fn suspend_index_works_scenario_1() {
// 	TestExt::new().execute_with(|| {
// 		// Setup: Here we create a group of people and recognize all of them.
// 		let mut people = generate_people(10);

// 		// Scenario 1: There is no root being built.
// 		assert_noop!(
// 			PeoplePallet::bake_root(RuntimeOrigin::none(), RI_ZERO),
// 			Error::<Test>::NotBuilding
// 		);
// 		// Lets suspend the first, middle, and last person
// 		let mut suspended = Vec::new();
// 		for i in [9, 5, 0] {
// 			assert_ok!(PeoplePallet::suspend_index(people[i].0));
// 			suspended.push(people.remove(i));
// 		}

// 		// Ring creation process should work as if the suspended people were never recognized.
// 		assert_ok!(PeoplePallet::refresh_root(RuntimeOrigin::none()));

// 		// We cannot push a ring with the suspended people...
// 		for person in &suspended {
// 			assert_noop!(
// 				PeoplePallet::push_member(RuntimeOrigin::none(), person.0),
// 				Error::<Test>::NotPerson
// 			);
// 		}

// 		// Recognized people are okay though.
// 		for person in &people {
// 			assert_ok!(PeoplePallet::push_member(RuntimeOrigin::none(), person.0));
// 		}

// 		// Let's inspect some low level implementation details.
// 		if let Some(Builder::Building { count, zombies, failed, .. }) =
// 			RootBuilder::<Test>::get(RI_ZERO)
// 		{
// 			assert_eq!(count as usize, people.len());
// 			assert_eq!(zombies, 0);
// 			assert_eq!(failed, 0);
// 		} else {
// 			panic!("expected root builder to be building")
// 		}

// 		// We can successfully bake the root once all recognized people are pushed.
// 		assert_ok!(PeoplePallet::bake_root(RuntimeOrigin::none(), RI_ZERO));
// 	});
// }

// This test checks scenario 2, which is the behavior of `suspend_index` when the root is being
// built, but the people who are being suspended have not been pushed yet. In this case, suspending
// an index should act as if they were never recognized.
// #[test]
// fn suspend_index_works_scenario_2() {
// 	TestExt::new().execute_with(|| {
// 		// Setup: Lets immediately start the root building process, and recognize some people.
// 		assert_ok!(PeoplePallet::refresh_root(RuntimeOrigin::none()));
// 		let mut people = generate_people(10);
// 		// Cannot bake root until everyone is pushed.
// 		assert_noop!(
// 			PeoplePallet::bake_root(RuntimeOrigin::none(), RI_ZERO),
// 			Error::<Test>::NotBuilding
// 		);

// 		// Scenario 2: The root is being built...
// 		// Let's introduce the first group of people to start with.
// 		for person in people.iter().take(7) {
// 			assert_ok!(PeoplePallet::push_member(RuntimeOrigin::none(), person.0));
// 		}

// 		// Then let's suspend the last of them.
// 		let mut suspended = Vec::new();
// 		for i in [9, 8, 7] {
// 			assert_ok!(PeoplePallet::suspend_index(people[i].0));
// 			suspended.push(people.remove(i));
// 		}

// 		// We cannot push to the ring with the suspended people...
// 		for person in &suspended {
// 			assert_noop!(
// 				PeoplePallet::push_member(RuntimeOrigin::none(), person.0),
// 				Error::<Test>::NotPerson
// 			);
// 		}

// 		// Let's inspect some low level implementation details.
// 		if let Some(Builder::Building { count, zombies, failed, .. }) =
// 			RootBuilder::<Test>::get(RI_ZERO)
// 		{
// 			assert_eq!(count as usize, people.len());
// 			assert_eq!(zombies, 0);
// 			assert_eq!(failed, 0);
// 		} else {
// 			panic!("expected root builder to be building")
// 		}

// 		// We can successfully bake the root once all recognized people are pushed.
// 		assert_ok!(PeoplePallet::bake_root(RuntimeOrigin::none(), RI_ZERO));
// 	});
// }

// This test checks scenario 3, which is the behavior of `suspend_index` when the root is being
// built, and the people who are being suspended have already been pushed. In this case, suspending
// an index:
// 1. There will be remnants of the suspended members in the ring.
// 2. But it should not stop the building and baking process.
// 3. Also, they should not be able to use the baked ring for anything.
// #[test]
// fn suspend_index_works_scenario_3() {
// 	TestExt::new().execute_with(|| {
// 		// Setup: Lets immediately start the root building process, and recognize some people.
// 		assert_ok!(PeoplePallet::refresh_root(RuntimeOrigin::none()));
// 		let mut people = generate_people(10);

// 		// Scenario 2: The root is being built, and we push everyone.
// 		for person in &people {
// 			assert_ok!(PeoplePallet::push_member(RuntimeOrigin::none(), person.0));
// 		}

// 		// Now after they have been pushed, let's suspend them.
// 		let mut suspended = Vec::new();
// 		for i in [9, 5, 0] {
// 			assert_ok!(PeoplePallet::suspend_index(people[i].0));
// 			suspended.push(people.remove(i));
// 		}

// 		// 1. Let's inspect some low level implementation details.
// 		if let Some(Builder::Building { count, zombies, failed, .. }) =
// 			RootBuilder::<Test>::get(RI_ZERO)
// 		{
// 			assert_eq!(count as usize, people.len() + suspended.len());
// 			assert_eq!(zombies as usize, suspended.len());
// 			assert_eq!(failed, 0);
// 		} else {
// 			panic!("expected root builder to be building")
// 		}

// 		// 2. We can successfully bake the root once all recognized people are pushed.
// 		assert_ok!(PeoplePallet::bake_root(RuntimeOrigin::none(), RI_ZERO));

// 		// TODO: Active people can have their signatures verified.
// 		// let active_person = &people[0];
// 		// let msg = "hello world".as_bytes();
// 		// TODO: Sign is not implemented for `Simple`
// 		// let signature =
// 		// 	<<Test as Config>::Crypto as GenerateVerifiable>::sign(&active_person.2, msg).unwrap();
// 		// assert!(PeoplePallet::verify_signature(active_person.0, msg, &signature));

// 		// 3. These suspended members are in the ring, but cannot use it.
// 		for person in &suspended {
// 			// Here we make the assumption that in the pallet, you must be in the `People` list to
// 			// access any features. Thus, if they are not in this group, we can safely say that
// 			// they do not have access.
// 			assert!(People::<Test>::get(person.0).is_none());
// 		}
// 	});
// }

// This test checks scenario 4, which is the behavior of `suspend_index` and then calling
// `recognize_personhood` again, while the root is being built, and and see that the ring still
// builds. This specifically checks the behavior of `zombies`.
// 1. Implementation should track `zombies` and `count` correctly after this process.
// 2. We can bake the root even after all the suspends and re-recognizes.
// 3. People that we expect will continue to be people, and suspended will not.
// #[test]
// fn suspend_index_works_scenario_4() {
// 	TestExt::new().execute_with(|| {
// 		// Setup: Lets immediately start the root building process, and recognize some people.
// 		assert_ok!(PeoplePallet::refresh_root(RuntimeOrigin::none()));
// 		let mut people = generate_people(10);

// 		// Scenario 2: The root is being built, and we push everyone.
// 		for person in &people {
// 			assert_ok!(PeoplePallet::push_member(RuntimeOrigin::none(), person.0));
// 		}

// 		// Now after they have been pushed, let's suspend them.
// 		let mut suspended = Vec::new();
// 		for i in [9, 5, 0] {
// 			assert_ok!(PeoplePallet::suspend_index(people[i].0));
// 			suspended.push(people.remove(i));
// 		}

// 		// Cannot re-push a suspended member
// 		for person in &suspended {
// 			assert_noop!(
// 				PeoplePallet::push_member(RuntimeOrigin::none(), person.0),
// 				Error::<Test>::NotPerson
// 			);
// 		}

// 		// We can recognize them again to restore their personhood.
// 		let mut re_people = Vec::new();
// 		for person in suspended.drain(0..2) {
// 			assert_ok!(PeoplePallet::recognize_personhood(person.0, Some(person.1)));
// 			re_people.push(person);
// 		}

// 		// Cannot re-push an already added member
// 		for person in &re_people {
// 			assert_noop!(
// 				PeoplePallet::push_member(RuntimeOrigin::none(), person.0),
// 				Error::<Test>::AlreadyPushed
// 			);
// 		}

// 		// 1. Let's inspect some low level implementation details.
// 		if let Some(Builder::Building { count, zombies, failed, .. }) =
// 			RootBuilder::<Test>::get(RI_ZERO)
// 		{
// 			assert_eq!(suspended.len(), 1);
// 			assert_eq!(re_people.len(), 2);
// 			assert_eq!(people.len(), 7);
// 			assert_eq!(count as usize, people.len() + suspended.len() + re_people.len());
// 			assert_eq!(zombies as usize, suspended.len());
// 			assert_eq!(failed, 0);
// 		} else {
// 			panic!("expected root builder to be building")
// 		}

// 		// 2. We can successfully bake the root once all recognized people are pushed.
// 		assert_ok!(PeoplePallet::bake_root(RuntimeOrigin::none(), RI_ZERO));

// 		// 3. These suspended members are in the ring, but cannot use it.
// 		for person in &suspended {
// 			// Here we make the assumption that in the pallet, you must be in the `People` list to
// 			// access any features. Thus, if they are not in this group, we can safely say that
// 			// they do not have access.
// 			assert!(People::<Test>::get(person.0).is_none());
// 		}

// 		// Everyone else is a person
// 		for person in people.iter().chain(re_people.iter()) {
// 			// Here we make the assumption that in the pallet, you must be in the `People` list to
// 			// access any features. Thus, if they are not in this group, we can safely say that
// 			// they do not have access.
// 			assert!(People::<Test>::get(person.0).is_some());
// 		}
// 	});
// }

#[test]
fn recognize_person_with_duplicate_key() {
	TestExt::new().execute_with(|| {
		// Recognize person A with a key.
		let person_a = PeoplePallet::reserve_new_id();
		let secret_a = Simple::new_secret([1; 32]);
		let key_a = Simple::member_from_secret(&secret_a);
		PeoplePallet::recognize_personhood(person_a, Some(key_a)).unwrap();

		// Recognize person B with the same key.
		let person_b = PeoplePallet::reserve_new_id();
		assert_noop!(
			PeoplePallet::recognize_personhood(person_b, Some(key_a)),
			Error::<Test>::KeyAlreadyInUse
		);
	});
}

// #[test]
// fn multiple_rings_works() {
// 	TestExt::new().execute_with(|| {
// 		// At the start of creating a new ring, you refresh the root.
// 		assert_ok!(PeoplePallet::refresh_root(RuntimeOrigin::none()));

// 		// Let's create some people...
// 		let max_ring_size: u32 = <Test as crate::Config>::MaxRingSize::get();
// 		// Generate enough people to fill 2 rings, and use a third.
// 		let people = generate_people((max_ring_size * 3 - 1) as u8);

// 		// Alice and two friends are placed into ring 0.
// 		for person in &people {
// 			assert_ok!(PeoplePallet::push_member(RuntimeOrigin::none(), person.0));
// 		}

// 		assert_ok!(PeoplePallet::bake_root(RuntimeOrigin::none(), 0));
// 		assert_ok!(PeoplePallet::bake_root(RuntimeOrigin::none(), 1));
// 		assert_ok!(PeoplePallet::bake_root(RuntimeOrigin::none(), 2));
// 		assert_noop!(PeoplePallet::bake_root(RuntimeOrigin::none(), 3), Error::<Test>::NotBuilding);
// 	});
// }

#[test]
fn recognize_same_person_2_times() {
	TestExt::new().execute_with(|| {
		let person_a = PeoplePallet::reserve_new_id();
		let secret_a = Simple::new_secret([1; 32]);
		let key_a = Simple::member_from_secret(&secret_a);
		assert_ok!(PeoplePallet::recognize_personhood(person_a, Some(key_a)));
		assert!(People::<Test>::get(person_a).is_some());
		assert_noop!(
			PeoplePallet::recognize_personhood(person_a, Some(key_a)),
			Error::<Test>::KeyAlreadyInUse,
		);
		assert_noop!(
			PeoplePallet::renew_id_reservation(person_a),
			Error::<Test>::PersonalIdReservationCannotRenew,
		);
	});
}

// TODO George: migrate key test

#[test]
fn recognize_person_with_duplicate_key_after_suspend() {
	TestExt::new().execute_with(|| {
		OnboardingSize::<Test>::set(1);

		let person_a = PeoplePallet::reserve_new_id();
		let person_b = PeoplePallet::reserve_new_id();
		let person_c = PeoplePallet::reserve_new_id();
		let secret_a = Simple::new_secret([1; 32]);
		let secret_b = Simple::new_secret([2; 32]);
		let secret_c = Simple::new_secret([3; 32]);
		let key_a = Simple::member_from_secret(&secret_a);
		let key_b = Simple::member_from_secret(&secret_b);
		let key_c = Simple::member_from_secret(&secret_c);
		// Recognize person A and B
		assert_ok!(PeoplePallet::recognize_personhood(person_a, Some(key_a)));
		// Onboard A so that they become part of a ring.
		assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
		// B will be part of the onboarding queue.
		assert_ok!(PeoplePallet::recognize_personhood(person_b, Some(key_b)));

		assert_eq!(
			People::<Test>::get(person_a).unwrap().position,
			RingPosition::Included {
				ring_index: 0,
				ring_position: 0,
				scheduled_for_removal: false
			}
		);
		assert_eq!(
			People::<Test>::get(person_b).unwrap().position,
			RingPosition::Onboarding { queue_page: 0 }
		);

		// Start suspensions.
		assert_ok!(PeoplePallet::start_people_set_mutation_session());

		// Suspend person A and B
		assert_ok!(PeoplePallet::suspend_personhood(&[person_a, person_b]));

		// End suspensions.
		assert_ok!(PeoplePallet::end_people_set_mutation_session());
		assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));
		assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::signed(0), 0));

		// Make sure both A and B are suspended.
		assert_eq!(People::<Test>::get(person_a).unwrap().position, RingPosition::Suspended);
		assert_eq!(People::<Test>::get(person_b).unwrap().position, RingPosition::Suspended);

		// Recognize person C with same key as A
		assert_noop!(
			PeoplePallet::recognize_personhood(person_c, Some(key_a)),
			Error::<Test>::KeyAlreadyInUse
		);

		// Recognize person C with a different key
		assert_ok!(PeoplePallet::recognize_personhood(person_c, Some(key_c)));

		// Resume personhood for A and B.
		assert_ok!(PeoplePallet::recognize_personhood(person_a, None));
		assert_ok!(PeoplePallet::recognize_personhood(person_b, None));
		// Both A and B kept their keys.
		assert_eq!(Keys::<Test>::get(key_a), Some(person_a));
		assert_eq!(Keys::<Test>::get(key_b), Some(person_b));
	});
}

#[test]
fn id_reservation_works() {
	TestExt::new().execute_with(|| {
		// Initially, no personal IDs are reserved or recognized.
		assert_eq!(NextPersonalId::<Test>::get(), 0);
		assert!(!ReservedPersonalId::<Test>::contains_key(0));
		assert!(People::<Test>::get(0).is_none());

		assert_noop!(
			PeoplePallet::renew_id_reservation(0),
			Error::<Test>::PersonalIdReservationCannotRenew,
		);

		// Reserve a new ID. This should create a reservation at ID=0.
		assert_eq!(PeoplePallet::reserve_new_id(), 0);
		assert_eq!(NextPersonalId::<Test>::get(), 1);
		assert!(ReservedPersonalId::<Test>::contains_key(0));
		assert!(People::<Test>::get(0).is_none());

		assert_noop!(
			PeoplePallet::renew_id_reservation(0),
			Error::<Test>::PersonalIdReservationCannotRenew,
		);

		// Reserve another new ID. This should create a reservation at ID=1.
		assert_eq!(PeoplePallet::reserve_new_id(), 1);
		assert_eq!(NextPersonalId::<Test>::get(), 2);
		assert!(ReservedPersonalId::<Test>::contains_key(1));
		assert!(People::<Test>::get(1).is_none());

		// Cancel the reservation for ID=0.
		assert_ok!(PeoplePallet::cancel_id_reservation(0));
		assert!(!ReservedPersonalId::<Test>::contains_key(0));
		assert!(People::<Test>::get(0).is_none());

		// Reserve a new ID again. This should create a reservation at ID=2.
		assert_eq!(PeoplePallet::reserve_new_id(), 2);
		assert_eq!(NextPersonalId::<Test>::get(), 3);
		assert!(ReservedPersonalId::<Test>::contains_key(2));
		assert!(People::<Test>::get(2).is_none());

		// Renew the reservation for ID=0.
		assert_ok!(PeoplePallet::renew_id_reservation(0));
		assert!(ReservedPersonalId::<Test>::contains_key(0));
		assert!(People::<Test>::get(0).is_none());

		assert_noop!(
			PeoplePallet::renew_id_reservation(0),
			Error::<Test>::PersonalIdReservationCannotRenew,
		);

		// Recognize personhood for ID=0 with a dummy key.
		assert_ok!(PeoplePallet::recognize_personhood(0, Some([0; 32])));
		assert!(People::<Test>::get(0).is_some());
		assert!(!ReservedPersonalId::<Test>::contains_key(0));

		assert_noop!(
			PeoplePallet::renew_id_reservation(0),
			Error::<Test>::PersonalIdReservationCannotRenew,
		);
	});
}

#[test]
fn force_recognize_personhood_works() {
	TestExt::new().execute_with(|| {
		use verifiable::demo_impls::Simple;

		// We'll create 5 new people to recognize.
		let num_people = 5;
		let mut keys = Vec::new();
		for i in 0..num_people {
			let secret = Simple::new_secret([i as u8; 32]);
			let public_key = Simple::member_from_secret(&secret);
			keys.push(public_key);
		}

		// Initially, no one is recognized.
		for id in 0..num_people {
			assert!(People::<Test>::get(id).is_none());
			assert!(!ReservedPersonalId::<Test>::contains_key(id));
		}
		assert_eq!(NextPersonalId::<Test>::get(), 0);

		// Using the root origin, force recognize these people.
		assert_ok!(PeoplePallet::force_recognize_personhood(RuntimeOrigin::root(), keys.clone()));

		// After recognition, each person should now exist in storage.
		for (i, key) in keys.clone().into_iter().enumerate() {
			let who = i as PersonalId;
			let record = People::<Test>::get(who).expect("Person should be recognized");
			assert_eq!(record.key, key);
			assert!(!ReservedPersonalId::<Test>::contains_key(who));
		}

		// NextPersonalId should now point to the next free ID after recognizing `num_people`.
		assert_eq!(NextPersonalId::<Test>::get(), num_people);

		// Any further IDs not used yet should be empty.
		assert!(People::<Test>::get(num_people).is_none());

		// Fails for non-root origin.
		assert_noop!(
			PeoplePallet::force_recognize_personhood(RuntimeOrigin::signed(0), keys.clone()),
			sp_runtime::DispatchError::BadOrigin
		);

		// Fails for duplicate keys.
		let another_key = {
			let secret = Simple::new_secret([233; 32]);
			Simple::member_from_secret(&secret)
		};
		assert_noop!(
			PeoplePallet::force_recognize_personhood(
				RuntimeOrigin::root(),
				vec![another_key, another_key]
			),
			Error::<Test>::KeyAlreadyInUse
		);
	});
}

#[test]
fn cannot_renew_future_id() {
	TestExt::new().execute_with(|| {
		// Initially, NextPersonalId should be 0.
		assert_eq!(NextPersonalId::<Test>::get(), 0);

		// Id 0 is not reserved, can't renew.
		assert_noop!(
			PeoplePallet::renew_id_reservation(0),
			Error::<Test>::PersonalIdReservationCannotRenew
		);

		// Id 1 is not reserved, can't renew.
		assert_noop!(
			PeoplePallet::renew_id_reservation(1),
			Error::<Test>::PersonalIdReservationCannotRenew
		);

		// Reserve a new personal ID. This will be ID 0, and NextPersonalId should now become 1.
		let first_id = PeoplePallet::reserve_new_id();
		assert_eq!(first_id, 0);
		assert_eq!(NextPersonalId::<Test>::get(), 1);

		// Id 0 is reserved, can't renew.
		assert_noop!(
			PeoplePallet::renew_id_reservation(0),
			Error::<Test>::PersonalIdReservationCannotRenew
		);

		// Id 1 is future, can't renew.
		assert_noop!(
			PeoplePallet::renew_id_reservation(1),
			Error::<Test>::PersonalIdReservationCannotRenew
		);

		// Cancel the reservation for ID=0.
		assert_ok!(PeoplePallet::cancel_id_reservation(0));

		// Id 0 is not reserved, can renew.
		assert_ok!(PeoplePallet::renew_id_reservation(0));

		// Id 1 is future, can't renew.
		assert_noop!(
			PeoplePallet::renew_id_reservation(1),
			Error::<Test>::PersonalIdReservationCannotRenew
		);
	});
}

#[test]
fn test_set_personal_id_account() {
	TestExt::new().execute_with(|| {
		generate_people_with_index(0, 3);

		// (In our test, we treat PersonalId as a simple u64.)
		// Verify that there are no mappings for personal id 1 and account 42.
		assert!(AccountToPersonalId::<Test>::get(42).is_none());
		assert!(People::<Test>::get(1).unwrap().account.is_none());

		// Create an origin that represents a personal identity.
		// (Recall that your pallet’s Origin enum has a variant PersonalIdentity.)
		let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(1));
		// Call the extrinsic to set personal id account.
		assert_ok!(PeoplePallet::set_personal_id_account(origin, 42, 0), Pays::No.into());

		// Check that the mapping is now present.
		assert_eq!(AccountToPersonalId::<Test>::get(42), Some(1));
		assert_eq!(People::<Test>::get(1).unwrap().account, Some(42));

		// Now update the mapping by calling the extrinsic again.
		let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(1));
		// Here we change the account to 43.
		assert_ok!(PeoplePallet::set_personal_id_account(origin, 43, 0), Pays::Yes.into());
		// The old mapping for account 42 should be removed.
		assert!(AccountToPersonalId::<Test>::get(42).is_none());
		assert_eq!(AccountToPersonalId::<Test>::get(43), Some(1));
		assert_eq!(People::<Test>::get(1).unwrap().account, Some(43));

		// Test that a non-personal identity origin (for example, a Signed origin)
		// does not work (the call should error with BadOrigin).
		let origin = RuntimeOrigin::signed(44);
		assert_noop!(
			PeoplePallet::set_personal_id_account(origin, 44, 0),
			sp_runtime::DispatchError::BadOrigin
		);

		// Test that trying to use an account that is already in use fails.
		// First, set a mapping for personal id 2 using account 45.
		let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(2));
		assert_ok!(PeoplePallet::set_personal_id_account(origin, 45, 0), Pays::No.into());
		// Then try to set personal id 3 to use the same account 45.
		let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(3));
		assert_noop!(
			PeoplePallet::set_personal_id_account(origin, 45, 0),
			Error::<Test>::AccountInUse
		);
	});
}

#[test]
fn test_unset_personal_id_account() {
	TestExt::new().execute_with(|| {
		generate_people_with_index(0, 1);

		// First, set a mapping for personal id 1 to account 50.
		let id_origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(1));
		assert_ok!(
			PeoplePallet::set_personal_id_account(id_origin.clone(), 50, 0),
			Pays::No.into()
		);
		assert_eq!(AccountToPersonalId::<Test>::get(50), Some(1));

		// Now call the unset extrinsic.
		assert_ok!(PeoplePallet::unset_personal_id_account(id_origin.clone()), Pays::Yes.into());
		// Verify that the mappings have been removed.
		assert!(AccountToPersonalId::<Test>::get(50).is_none());
		assert!(People::<Test>::get(1).unwrap().account.is_none());

		// Calling unset again on the same account should fail.
		assert_noop!(
			PeoplePallet::unset_personal_id_account(id_origin.clone()),
			Error::<Test>::InvalidAccount
		);
	});
}

#[test]
fn test_as_personal_identity_with_account_check_and_nonce() {
	// Use our test externalities.
	new_test_ext().execute_with(|| {
		let dummy_call = frame_system::Call::<Test>::remark { remark: vec![] };
		let account: u64 = 42;

		// 0: transaction fails because there signer is wrong, no associated personal id.
		let nonce: u64 = 0;
		let tx_ext = (
			AsPerson::<Test>::new(Some(AsPersonInfo::AsPersonalIdentityWithAccount(nonce))),
			frame_system::CheckNonce::<Test>::from(nonce),
		);
		assert_noop!(
			exec_tx(Some(account), tx_ext, dummy_call.clone()),
			InvalidTransaction::BadSigner
		);

		// Add a person and an associated account ---
		let personal_id = generate_people_with_index(0, 0).pop().unwrap().0;
		AccountToPersonalId::<Test>::insert(account, personal_id);
		System::inc_sufficients(&account);

		// 1: a successful transaction
		let nonce: u64 = 0;
		let tx_ext = (
			AsPerson::<Test>::new(Some(AsPersonInfo::AsPersonalIdentityWithAccount(nonce))),
			frame_system::CheckNonce::<Test>::from(nonce),
		);
		assert_ok!(exec_tx(Some(account), tx_ext, dummy_call.clone()));
		assert_eq!(frame_system::Pallet::<Test>::account_nonce(account), 1);

		// 2: another successful transaction
		let nonce: u64 = 1;
		let tx_ext = (
			AsPerson::<Test>::new(Some(AsPersonInfo::AsPersonalIdentityWithAccount(nonce))),
			frame_system::CheckNonce::<Test>::from(nonce),
		);
		assert_ok!(exec_tx(Some(account), tx_ext, dummy_call.clone()));
		assert_eq!(frame_system::Pallet::<Test>::account_nonce(account), 2);

		// 3: transaction fails because the nonce is wrong
		let nonce: u64 = 1;
		let tx_ext = (
			AsPerson::<Test>::new(Some(AsPersonInfo::AsPersonalIdentityWithAccount(nonce))),
			frame_system::CheckNonce::<Test>::from(nonce),
		);
		assert_noop!(exec_tx(Some(account), tx_ext, dummy_call), InvalidTransaction::Stale);
		assert_eq!(frame_system::Pallet::<Test>::account_nonce(account), 2);
	});
}

mod offchain_worker {
	use super::*;
	use frame_support::{pallet_prelude::Get, traits::OffchainWorker, BoundedVec};
	use sp_core::offchain::{
		testing::{TestOffchainExt, TestTransactionPoolExt},
		OffchainDbExt, OffchainWorkerExt, TransactionPoolExt,
	};

	#[test]
	fn submits_build_ring_transaction_only_at_interval_ticks() {
		let mut ext = new_test_ext();
		let (offchain, _state) = TestOffchainExt::new();
		let (pool, state) = TestTransactionPoolExt::new();
		ext.register_extension(OffchainDbExt::new(offchain.clone()));
		ext.register_extension(OffchainWorkerExt::new(offchain));
		ext.register_extension(TransactionPoolExt::new(pool));

		ext.execute_with(|| {
			// Only one new member is required to build the ring
			OnboardingSize::<Test>::set(1);

			// 5 members already exist in the ring and none of them is included
			let member_keys = (0..5)
				.map(|i| Simple::member_from_secret(&Simple::new_secret([i as u8; 32])))
				.collect::<Vec<_>>();
			let keys: BoundedVec<MemberOf<Test>, <Test as Config>::MaxRingSize> =
				BoundedVec::try_from(member_keys).expect("failed to init members");
			let ring_status = RingStatus { total: keys.len().saturated_into(), included: 0 };
			RingKeys::<Test>::insert(0, keys.clone());
			RingKeysStatus::<Test>::insert(0, ring_status);

			// Offchain worker should not submit the transaction
			// if the block numbers are in between the interval so
			// starting from block number 1 to T::RingBakingInterval - 1
			let interval: u64 = <Test as Config>::RingBakingInterval::get();
			let mut block = 1;
			while block < interval {
				System::set_block_number(block);
				PeoplePallet::offchain_worker(block);
				assert_eq!(state.read().transactions.len(), 0);
				block += 1;
			}

			// At T::RingBakingInterval the offchain worker should submit the transaction
			System::set_block_number(block);
			PeoplePallet::offchain_worker(block);
			assert_eq!(state.read().transactions.len(), 1);

			// and the transaction should be build_ring call
			let transaction = state.write().transactions.pop().unwrap();
			let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
			let ring_index = match ex.function {
				crate::mock::RuntimeCall::PeoplePallet(crate::Call::build_ring {
					ring_index,
					..
				}) => ring_index,
				e => panic!("Unexpected call: {:?}", e),
			};
			assert_eq!(ring_index, 0);
		});
	}

	#[test]
	fn no_transaction_submitted_if_ring_doesnt_exist() {
		let mut ext = new_test_ext();
		let (offchain, _state) = TestOffchainExt::new();
		let (pool, state) = TestTransactionPoolExt::new();
		ext.register_extension(OffchainDbExt::new(offchain.clone()));
		ext.register_extension(OffchainWorkerExt::new(offchain));
		ext.register_extension(TransactionPoolExt::new(pool));

		ext.execute_with(|| {
			// A ring exists with some suspensions
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let suspensions: &[PersonalId] = &[1];
			assert_ok!(PeoplePallet::suspend_personhood(suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());

			// Offchain worker is called
			PeoplePallet::offchain_worker(0);

			// 1 transaction is submitted
			assert_eq!(state.read().transactions.len(), 1);

			// and the transaction should be migrate_keys call
			let transaction = state.write().transactions.pop().unwrap();
			let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
			assert!(matches!(
				ex.function,
				crate::mock::RuntimeCall::PeoplePallet(crate::Call::migrate_keys { limit: _ },)
			));
			// The call gets dispatched
			assert_ok!(ex.call().clone().dispatch(RuntimeOrigin::none()));
			assert!(RingsState::<Test>::get().append_only());

			// Mutation session is ongoing
			assert_ok!(PeoplePallet::start_people_set_mutation_session());

			// Offchain worker is called
			PeoplePallet::offchain_worker(0);

			// No transactions are submitted
			assert_eq!(state.read().transactions.len(), 0);
		});
	}

	#[test]
	fn one_ring_and_no_suspensions() {
		TestExt::new().execute_with(|| {
			let mut ext = new_test_ext();
			let (offchain, _state) = TestOffchainExt::new();
			let (pool, state) = TestTransactionPoolExt::new();
			ext.register_extension(OffchainDbExt::new(offchain.clone()));
			ext.register_extension(OffchainWorkerExt::new(offchain));
			ext.register_extension(TransactionPoolExt::new(pool));

			ext.execute_with(|| {
				// A ring exists but no-one in it is suspended
				generate_people_with_index(0, 9);
				assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
				assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

				// Offchain worker is called
				PeoplePallet::offchain_worker(0);

				// No transactions are submitted
				assert_eq!(state.read().transactions.len(), 0);
			});
		});
	}

	#[test]
	fn no_rings_and_empty_queue() {
		TestExt::new().execute_with(|| {
			let mut ext = new_test_ext();
			let (offchain, _state) = TestOffchainExt::new();
			let (pool, state) = TestTransactionPoolExt::new();
			ext.register_extension(OffchainDbExt::new(offchain.clone()));
			ext.register_extension(OffchainWorkerExt::new(offchain));
			ext.register_extension(TransactionPoolExt::new(pool));

			ext.execute_with(|| {
				// Offchain worker is called
				PeoplePallet::offchain_worker(0);
				// No transactions are submitted
				assert_eq!(state.read().transactions.len(), 0);
			});
		});
	}

	#[test]
	fn no_rings_and_not_enough_people_in_the_queue() {
		TestExt::new().execute_with(|| {
			let mut ext = new_test_ext();
			let (offchain, _state) = TestOffchainExt::new();
			let (pool, state) = TestTransactionPoolExt::new();
			ext.register_extension(OffchainDbExt::new(offchain.clone()));
			ext.register_extension(OffchainWorkerExt::new(offchain));
			ext.register_extension(TransactionPoolExt::new(pool));

			ext.execute_with(|| {
				// Several people are awaiting in the queue
				generate_people_with_index(0, 3);

				// Offchain worker is called
				PeoplePallet::offchain_worker(0);

				// No transactions are submitted
				assert_eq!(state.read().transactions.len(), 0);
			});
		});
	}

	// TODO: move to onboarding offchain worker
	// #[test]
	// fn no_transaction_submitted_if_not_included_too_small() {
	// 	let mut ext = new_test_ext();
	// 	let (offchain, _state) = TestOffchainExt::new();
	// 	let (pool, state) = TestTransactionPoolExt::new();
	// 	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	// 	ext.register_extension(OffchainWorkerExt::new(offchain));
	// 	ext.register_extension(TransactionPoolExt::new(pool));

	// 	ext.execute_with(|| {
	// 		// Only one new member is required to build the ring
	// 		OnboardingSize::<Test>::set(2);

	// 		// 1 member exists in the ring and is not included
	// 		let member_keys = vec![Simple::member_from_secret(&Simple::new_secret([0u8; 32]))];
	// 		let keys: BoundedVec<MemberOf<Test>, <Test as Config>::MaxRingSize> =
	// 			BoundedVec::try_from(member_keys).expect("failed to init members");
	// 		let ring_status =
	// 			RingStatus { total: keys.len().saturated_into(), included: 0, suspended: 0 };
	// 		RingKeys::<Test>::insert(0, keys.clone());
	// 		RingKeysStatus::<Test>::insert(0, ring_status);

	// 		let block = 0;
	// 		System::set_block_number(block);
	// 		PeoplePallet::offchain_worker(block);
	// 		assert_eq!(state.read().transactions.len(), 0);
	// 	});
	// }
	#[test]
	fn no_rings_and_some_people_awaiting_onboarding() {
		TestExt::new().execute_with(|| {
			let mut ext = new_test_ext();
			let (offchain, _state) = TestOffchainExt::new();
			let (pool, state) = TestTransactionPoolExt::new();
			ext.register_extension(OffchainDbExt::new(offchain.clone()));
			ext.register_extension(OffchainWorkerExt::new(offchain));
			ext.register_extension(TransactionPoolExt::new(pool));

			ext.execute_with(|| {
				// Several people are awaiting in the queue
				generate_people_with_index(0, 3);

				// Mutation session is ongoing
				assert_ok!(PeoplePallet::start_people_set_mutation_session());

				// Offchain worker is called
				PeoplePallet::offchain_worker(0);

				// No transactions are submitted
				assert_eq!(state.read().transactions.len(), 0);
			});
		});
	}

	mod suspensions {
		use super::*;

		#[test]
		fn one_ring_with_suspensions() {
			TestExt::new().execute_with(|| {
				let mut ext = new_test_ext();
				let (offchain, _state) = TestOffchainExt::new();
				let (pool, state) = TestTransactionPoolExt::new();
				ext.register_extension(OffchainDbExt::new(offchain.clone()));
				ext.register_extension(OffchainWorkerExt::new(offchain));
				ext.register_extension(TransactionPoolExt::new(pool));

				ext.execute_with(|| {
					// A ring exists with some suspensions
					generate_people_with_index(0, 9);
					assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
					assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

					assert_ok!(PeoplePallet::start_people_set_mutation_session());
					let suspensions: &[PersonalId] = &[1];
					assert_ok!(PeoplePallet::suspend_personhood(suspensions));
					assert_ok!(PeoplePallet::end_people_set_mutation_session());

					// Offchain worker is called
					PeoplePallet::offchain_worker(0);

					// 1 transaction is submitted
					assert_eq!(state.read().transactions.len(), 1);

					// and the transaction should be migrate_keys call
					let transaction = state.write().transactions.pop().unwrap();
					let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
					assert!(matches!(
						ex.function,
						crate::mock::RuntimeCall::PeoplePallet(crate::Call::migrate_keys {
							limit: _
						},)
					));
					// The call gets dispatched
					assert_ok!(ex.call().clone().dispatch(RuntimeOrigin::none()));
					assert!(RingsState::<Test>::get().append_only());

					// Offchain worker is called
					PeoplePallet::offchain_worker(0);

					// 1 transaction is submitted
					assert_eq!(state.read().transactions.len(), 1);

					// and the transaction should be remove_suspended_keys call
					let transaction = state.write().transactions.pop().unwrap();
					let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
					let (ring_index, suspended_indices) = match ex.function {
						crate::mock::RuntimeCall::PeoplePallet(
							crate::Call::remove_suspended_keys { ring_index },
						) => (ring_index, suspended_indices_list(ring_index)),
						e => panic!("Unexpected call: {:?}", e),
					};
					assert_eq!(ring_index, 0);
					assert_eq!(suspended_indices.len(), 1);
					assert_eq!(suspended_indices[0], 1);
				});
			});
		}

		#[test]
		fn multiple_rings_with_suspensions() {
			TestExt::new().execute_with(|| {
				let mut ext = new_test_ext();
				let (offchain, _state) = TestOffchainExt::new();
				let (pool, state) = TestTransactionPoolExt::new();
				ext.register_extension(OffchainDbExt::new(offchain.clone()));
				ext.register_extension(OffchainWorkerExt::new(offchain));
				ext.register_extension(TransactionPoolExt::new(pool));

				ext.execute_with(|| {
					//

					// Two rings exist with some suspensions
					generate_people_with_index(0, 19);
					assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
					assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
					assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));
					assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), 1, None));

					assert_ok!(PeoplePallet::start_people_set_mutation_session());
					let suspensions: &[PersonalId] = &[1, 11];
					assert_ok!(PeoplePallet::suspend_personhood(suspensions));
					assert_ok!(PeoplePallet::end_people_set_mutation_session());

					// Offchain worker is called
					PeoplePallet::offchain_worker(0);

					// 1 transaction is submitted
					assert_eq!(state.read().transactions.len(), 1);

					// and the transaction should be migrate_keys call
					let transaction = state.write().transactions.pop().unwrap();
					let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
					assert!(matches!(
						ex.function,
						crate::mock::RuntimeCall::PeoplePallet(crate::Call::migrate_keys {
							limit: _
						},)
					));
					// The call gets dispatched
					assert_ok!(ex.call().clone().dispatch(RuntimeOrigin::none()));
					assert!(RingsState::<Test>::get().append_only());

					// Offchain worker is called
					PeoplePallet::offchain_worker(0);

					// 1 transaction is submitted
					assert_eq!(state.read().transactions.len(), 1);

					// and the transaction should be remove_suspended_keys call
					let transaction = state.write().transactions.pop().unwrap();
					let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
					let (ring_index, suspended_indices) = match ex.function {
						crate::mock::RuntimeCall::PeoplePallet(
							crate::Call::remove_suspended_keys { ring_index },
						) => (ring_index, suspended_indices_list(ring_index)),
						e => panic!("Unexpected call: {:?}", e),
					};

					// and it should be for the ring with index 1
					assert_eq!(ring_index, 1);
					assert_eq!(suspended_indices.len(), 1);
					assert_eq!(suspended_indices[0], 1);

					// The call gets dispatched
					assert_ok!(ex.call().clone().dispatch(RuntimeOrigin::none()));

					// Offchain worker is called again
					PeoplePallet::offchain_worker(0);

					// 2 transactions is submitted
					assert_eq!(state.read().transactions.len(), 2);

					// and the 1st one should be build_ring call for the ring where the suspensions
					// were removed
					let transaction = state.write().transactions.pop().unwrap();
					let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
					let ring_index = match ex.function {
						crate::mock::RuntimeCall::PeoplePallet(crate::Call::build_ring {
							ring_index,
							..
						}) => ring_index,
						e => panic!("Unexpected call: {:?}", e),
					};
					assert_eq!(ring_index, 1);

					// and the 2nd one should be remove_suspended_keys call for the other ring
					let transaction = state.write().transactions.pop().unwrap();
					let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
					let (ring_index, suspended_indices) = match ex.function {
						crate::mock::RuntimeCall::PeoplePallet(
							crate::Call::remove_suspended_keys { ring_index },
						) => (ring_index, suspended_indices_list(RI_ZERO)),
						e => panic!("Unexpected call: {:?}", e),
					};

					// and it should be for the ring with index 0
					assert_eq!(ring_index, 0);
					assert_eq!(suspended_indices.len(), 1);
					assert_eq!(suspended_indices[0], 1);
				});
			});
		}
	}

	mod onboarding {
		use super::*;

		#[test]
		fn sends_tx_if_all_conditions_met() {
			TestExt::new().execute_with(|| {
				let mut ext = new_test_ext();
				let (offchain, _state) = TestOffchainExt::new();
				let (pool, state) = TestTransactionPoolExt::new();
				ext.register_extension(OffchainDbExt::new(offchain.clone()));
				ext.register_extension(OffchainWorkerExt::new(offchain));
				ext.register_extension(TransactionPoolExt::new(pool));

				ext.execute_with(|| {
					PeoplePallet::set_onboarding_size(RuntimeOrigin::root(), 3).unwrap();
					OnboardingSize::<Test>::set(3);

					// Several people are awaiting in the queue
					generate_people_with_index(0, 3);

					// Offchain worker is called
					PeoplePallet::offchain_worker(0);

					// 1 transaction is submitted
					assert_eq!(state.read().transactions.len(), 1);

					// and the transaction should be onboard_people call
					let transaction = state.write().transactions.pop().unwrap();
					let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
					assert_eq!(
						ex.function,
						crate::mock::RuntimeCall::PeoplePallet(crate::Call::onboard_people {})
					);
				});
			});
		}
	}
}

mod validate_unsigned {
	use super::*;
	use frame_support::BoundedVec;
	use sp_runtime::{
		traits::ValidateUnsigned,
		transaction_validity::{InvalidTransaction, TransactionSource},
	};

	#[test]
	fn works_for_build_ring() {
		TestExt::new().execute_with(|| {
			// Set-up to make the checks validating the need of ring build pass
			OnboardingSize::<Test>::set(1);
			let member_key = Simple::member_from_secret(&Simple::new_secret([0u8; 32]));
			let keys: BoundedVec<MemberOf<Test>, <Test as Config>::MaxRingSize> =
				BoundedVec::try_from(vec![member_key]).expect("failed to init members");
			let ring_status = RingStatus { total: keys.len().saturated_into(), included: 0 };
			RingKeys::<Test>::insert(RI_ZERO, keys);
			RingKeysStatus::<Test>::insert(RI_ZERO, ring_status);

			// build_ring call should succeed
			let build_ring_call = Call::<Test>::build_ring {
				ring_index: 0,
				limit: Some(OnboardingSize::<Test>::get()),
			};
			assert_ok!(PeoplePallet::validate_unsigned(TransactionSource::Local, &build_ring_call));
		});
	}

	#[test]
	fn works_for_onboard_people() {
		TestExt::new().execute_with(|| {
			// Set-up needed to make the call pass
			generate_people_with_index(0, 19);

			// onboard_people call should succeed
			let onboard_people_call = Call::<Test>::onboard_people {};
			assert_ok!(PeoplePallet::validate_unsigned(
				TransactionSource::Local,
				&onboard_people_call
			));
		});
	}

	#[test]
	fn works_for_remove_suspended_keys() {
		TestExt::new().execute_with(|| {
			// Set-up needed to make the call pass
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			// Suspend and remove more than half of the people in both rings
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let suspensions: &[PersonalId] = &[1];
			assert_ok!(PeoplePallet::suspend_personhood(suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// remove_suspended_keys call should succeed
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![1]);
			let remove_suspended_keys_call =
				Call::<Test>::remove_suspended_keys { ring_index: RI_ZERO };
			assert_ok!(PeoplePallet::validate_unsigned(
				TransactionSource::Local,
				&remove_suspended_keys_call
			));
		});
	}

	#[test]
	fn fails_for_other_calls() {
		TestExt::new().execute_with(|| {
			let force_recognize_call = Call::<Test>::force_recognize_personhood { people: vec![] };
			assert_eq!(
				PeoplePallet::validate_unsigned(TransactionSource::Local, &force_recognize_call),
				InvalidTransaction::Call.into()
			);

			let set_onboarding_size_call = Call::<Test>::set_onboarding_size { onboarding_size: 1 };
			assert_eq!(
				PeoplePallet::validate_unsigned(
					TransactionSource::Local,
					&set_onboarding_size_call
				),
				InvalidTransaction::Call.into()
			);
		});
	}

	#[test]
	fn checks_the_need_to_build_the_ring() {
		TestExt::new().execute_with(|| {
			let valid_call = Call::<Test>::build_ring {
				ring_index: 0,
				limit: Some(OnboardingSize::<Test>::get()),
			};
			assert_eq!(
				PeoplePallet::validate_unsigned(TransactionSource::Local, &valid_call),
				InvalidTransaction::Stale.into()
			);
		});
	}
}

mod chunks {
	use super::*;
	use frame_support::traits::Get;
	use sp_runtime::BoundedVec;

	#[test]
	#[should_panic]
	fn no_chunks_first_page() {
		new_test_ext().execute_with(|| {
			Chunks::<Test>::remove(0);
			assert!(PeoplePallet::fetch_chunks(0..5).is_err());
		})
	}

	#[test]
	#[should_panic]
	fn no_chunks_second_page() {
		new_test_ext().execute_with(|| {
			let chunk_page_size: u32 = <Test as Config>::ChunkPageSize::get();
			Chunks::<Test>::remove(1);
			assert!(PeoplePallet::fetch_chunks(
				(chunk_page_size - 1) as usize..(chunk_page_size * 2) as usize
			)
			.is_err());
		})
	}

	#[test]
	#[should_panic]
	fn insufficient_chunks() {
		new_test_ext().execute_with(|| {
			let drain = Chunks::<Test>::drain();
			for _ in drain {}
			let chunk_page_size: u32 = <Test as Config>::ChunkPageSize::get();
			let chunks: BoundedVec<(), _> = [(); 4096]
				.into_iter()
				.take(chunk_page_size as usize - 1)
				.collect::<Vec<_>>()
				.try_into()
				.unwrap();
			Chunks::<Test>::insert(0, chunks);
			assert!(PeoplePallet::fetch_chunks(0..(chunk_page_size as usize + 1)).is_err());
		})
	}

	#[test]
	fn single_page_chunks() {
		new_test_ext().execute_with(|| {
			let chunk_page_size: u32 = <Test as Config>::ChunkPageSize::get();
			assert_eq!(
				PeoplePallet::fetch_chunks(0..(chunk_page_size - 1) as usize).unwrap().len(),
				chunk_page_size as usize - 1
			);
			assert_eq!(
				PeoplePallet::fetch_chunks(1..chunk_page_size as usize).unwrap().len(),
				chunk_page_size as usize - 1
			);
			assert_eq!(
				PeoplePallet::fetch_chunks(1..(chunk_page_size - 1) as usize).unwrap().len(),
				chunk_page_size as usize - 2
			);
			assert_eq!(
				PeoplePallet::fetch_chunks(0..chunk_page_size as usize).unwrap().len(),
				chunk_page_size as usize
			);
		})
	}

	#[test]
	fn multi_page_chunks() {
		new_test_ext().execute_with(|| {
			let chunk_page_size: u32 = <Test as Config>::ChunkPageSize::get();
			let chunks: BoundedVec<(), _> = [(); 4096]
				.into_iter()
				.take(chunk_page_size as usize)
				.collect::<Vec<_>>()
				.try_into()
				.unwrap();
			Chunks::<Test>::insert(0, &chunks);
			Chunks::<Test>::insert(1, chunks);
			assert_eq!(
				PeoplePallet::fetch_chunks(0..(2 * chunk_page_size - 1) as usize).unwrap().len(),
				2 * chunk_page_size as usize - 1
			);
			assert_eq!(
				PeoplePallet::fetch_chunks(1..2 * chunk_page_size as usize).unwrap().len(),
				2 * chunk_page_size as usize - 1
			);
			assert_eq!(
				PeoplePallet::fetch_chunks(1..(2 * chunk_page_size - 1) as usize).unwrap().len(),
				2 * chunk_page_size as usize - 2
			);
			assert_eq!(
				PeoplePallet::fetch_chunks(0..2 * chunk_page_size as usize).unwrap().len(),
				2 * chunk_page_size as usize
			);
		});
	}
}

mod merge_rings {
	use super::*;

	#[test]
	fn fails_if_suspension_session_is_in_progress() {
		TestExt::new().execute_with(|| {
			RingsState::<Test>::set(RingMembersState::Mutating(1));
			assert_noop!(
				PeoplePallet::merge_rings(RuntimeOrigin::none(), 1, 2),
				Error::<Test>::SuspensionSessionInProgress
			);
		});
	}

	#[test]
	fn fails_given_rings_with_the_same_id() {
		TestExt::new().execute_with(|| {
			assert_noop!(
				PeoplePallet::merge_rings(RuntimeOrigin::none(), 1, 1),
				Error::<Test>::InvalidRing
			);
		});
	}

	#[test]
	fn fails_given_current_ring_id() {
		TestExt::new().execute_with(|| {
			CurrentRingIndex::<Test>::set(14);
			assert_noop!(
				PeoplePallet::merge_rings(RuntimeOrigin::none(), 1, 14),
				Error::<Test>::InvalidRing
			);
			assert_noop!(
				PeoplePallet::merge_rings(RuntimeOrigin::none(), 14, 1),
				Error::<Test>::InvalidRing
			);
		});
	}

	#[test]
	fn fails_when_one_of_rings_is_half_full() {
		TestExt::new().execute_with(|| {
			// Two rings exist
			generate_people_with_index(0, 19);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), 1, None));

			// The current ring has a higher index than the ones being merged
			CurrentRingIndex::<Test>::set(14);

			assert_noop!(
				PeoplePallet::merge_rings(RuntimeOrigin::none(), 0, 1),
				Error::<Test>::RingAboveMergeThreshold
			);

			assert_noop!(
				PeoplePallet::merge_rings(RuntimeOrigin::none(), 1, 0),
				Error::<Test>::RingAboveMergeThreshold
			);
		});
	}

	#[test]
	fn fails_if_one_of_rings_has_pending_suspensions() {
		TestExt::new().execute_with(|| {
			// Two rings exist
			generate_people_with_index(0, 19);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), 1, None));

			// Suspend and remove more than half of the people in the first ring
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let suspensions: &[PersonalId] = &[1, 2, 3, 4, 5, 6];
			assert_ok!(PeoplePallet::suspend_personhood(suspensions));
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![1, 2, 3, 4, 5, 6]);
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));
			assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO));

			// Suspend a few more people in the first ring
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let suspensions: &[PersonalId] = &[7, 8];
			assert_ok!(PeoplePallet::suspend_personhood(suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// The current ring has a higher index than the ones being merged
			CurrentRingIndex::<Test>::set(14);

			assert_noop!(
				PeoplePallet::merge_rings(RuntimeOrigin::none(), 0, 1),
				Error::<Test>::SuspensionsPending
			);
		});
	}

	#[test]
	fn moves_all_keys_from_target_ring_to_base_ring() {
		TestExt::new().execute_with(|| {
			// Two rings exist
			generate_people_with_index(0, 19);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), 1, None));

			// Suspend and remove more than half of the people in both rings
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let suspensions: &[PersonalId] = &[1, 2, 3, 4, 5, 6, 11, 12, 13, 14, 15, 16];
			assert_ok!(PeoplePallet::suspend_personhood(suspensions));
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![1, 2, 3, 4, 5, 6]);
			assert_eq!(suspended_indices_list(1).into_inner(), vec![1, 2, 3, 4, 5, 6]);
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO));
			assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), 1));

			// The current ring has a higher index than the ones being merged
			CurrentRingIndex::<Test>::set(14);

			assert_ok!(PeoplePallet::merge_rings(RuntimeOrigin::none(), 0, 1));

			assert_eq!(RingKeys::<Test>::get(RI_ZERO).len(), 8);
			assert_eq!(RingKeysStatus::<Test>::get(RI_ZERO).total, 8);
			assert!(Root::<Test>::get(RI_ZERO).is_some());
			assert!(Root::<Test>::get(1).is_none());
		});
	}
}

mod suspensions {
	use super::*;

	#[test]
	fn suspending_personhood_fails_if_no_session_started() {
		TestExt::new().execute_with(|| {
			let suspensions: &[PersonalId] = &[1];
			assert_noop!(
				PeoplePallet::suspend_personhood(suspensions),
				Error::<Test>::NoMutationSession
			);
		});
	}

	#[test]
	fn suspending_personhood_fails_if_id_not_in_ring() {
		TestExt::new().execute_with(|| {
			// A ring with people exists
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			// Attempt to suspend a person fails
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let suspensions: &[PersonalId] = &[14];
			assert_noop!(
				PeoplePallet::suspend_personhood(suspensions),
				Error::<Test>::InvalidSuspensions
			);
		});
	}

	#[test]
	fn suspending_personhood_marks_people_as_suspended() {
		TestExt::new().execute_with(|| {
			// A ring with people exists
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			// Attempt to suspend a person
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let suspensions: &[PersonalId] = &[1];
			assert_ok!(PeoplePallet::suspend_personhood(suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// Makes the person's record suspended
			let personal_record = People::<Test>::get(1);
			assert!(personal_record.is_some());
			assert_eq!(personal_record.unwrap().position, RingPosition::Suspended);

			// Pending suspensions for the ring are incremented
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![1]);
		});
	}

	#[test]
	fn suspended_people_removal_fails_if_session_started() {
		TestExt::new().execute_with(|| {
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			assert_noop!(
				PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO),
				Error::<Test>::InvalidSuspensions
			);
		});
	}

	#[test]
	fn suspended_people_removal_fails_if_keys_are_migrated() {
		TestExt::new().execute_with(|| {
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_noop!(
				PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO),
				Error::<Test>::InvalidSuspensions
			);
		});
	}

	#[test]
	fn suspended_people_removal_fails_if_no_suspensions_recorded_in_ring() {
		TestExt::new().execute_with(|| {
			assert_noop!(
				PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO),
				Error::<Test>::InvalidSuspensions
			);
		});
	}

	#[test]
	fn suspended_people_removal_modifies_ring_data() {
		TestExt::new().execute_with(|| {
			// A ring exists
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));
			let initial_root = Root::<Test>::get(RI_ZERO).unwrap();

			// One person becomes suspended
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let suspensions: &[PersonalId] = &[1];
			assert_ok!(PeoplePallet::suspend_personhood(suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// Attempt to remove suspended people succeeds
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![1]);
			assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO));

			// Ring data becomes modified
			assert_eq!(RingKeysStatus::<Test>::get(RI_ZERO), RingStatus { included: 0, total: 9 });
			assert_eq!(RingKeys::<Test>::get(RI_ZERO).len(), 9);
			assert_ne!(Root::<Test>::get(RI_ZERO).unwrap().intermediate, initial_root.intermediate);

			// Pending suspensions are cleared for the ring
			assert!(suspended_indices_list(RI_ZERO).is_empty());
		});
	}

	#[test]
	fn suspending_in_multiple_sessions() {
		TestExt::new().execute_with(|| {
			// A ring with multiple people
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			// First session: some people become suspended
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			assert_ok!(PeoplePallet::suspend_personhood(&[1, 2]));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// Those people are then removed
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![1, 2]);
			assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO));

			// Second session: some more people become suspended
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			assert_ok!(PeoplePallet::suspend_personhood(&[3, 4]));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// Pending suspensions are tracked correctly
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![3, 4]);

			// Those extra people are removed too
			assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO));

			// Final ring state is correct
			assert_eq!(RingKeys::<Test>::get(RI_ZERO).len(), 6);
		});
	}

	#[test]
	fn suspending_personhood_then_resume() {
		TestExt::new().execute_with(|| {
			// A ring with people exists
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			// Attempt to suspend a person
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let suspensions: &[PersonalId] = &[1];
			assert_ok!(PeoplePallet::suspend_personhood(suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// Makes the person's record suspended
			let personal_record = People::<Test>::get(1);
			assert!(personal_record.is_some());
			assert_eq!(personal_record.unwrap().position, RingPosition::Suspended);

			// Pending suspensions for the ring are incremented
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![1]);
			assert_ok!(PeoplePallet::recognize_personhood(1, None));
			// Still needs to remove a key.
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![1]);

			assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO));
			assert!(suspended_indices_list(RI_ZERO).is_empty());
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));
		});
	}

	#[test]
	fn suspending_person_removes_associated_data() {
		TestExt::new().execute_with(|| {
			// A ring with multiple people
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			// A person associated with an account
			let person_id = 0;
			let account_id = 42;

			let id_origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(person_id));
			assert_ok!(PeoplePallet::set_personal_id_account(id_origin, account_id, 0));

			// The association exists
			assert_eq!(AccountToPersonalId::<Test>::get(account_id), Some(person_id));

			// The person becomes suspended
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			assert_ok!(PeoplePallet::suspend_personhood(&[person_id]));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// Account to personal id is removed
			assert!(AccountToPersonalId::<Test>::get(account_id).is_none());

			// The person is removed
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![person_id as u32]);
			assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO));

			// Account to personal id stays removed
			assert!(AccountToPersonalId::<Test>::get(account_id).is_none());

			// Using the account for authentication fails
			let nonce = 0;
			let tx_ext = (
				AsPerson::<Test>::new(Some(AsPersonInfo::AsPersonalIdentityWithAccount(nonce))),
				frame_system::CheckNonce::<Test>::from(nonce),
			);
			let dummy_call = Call::<Test>::unset_alias_account {};
			assert_noop!(
				exec_tx(Some(account_id), tx_ext, dummy_call),
				InvalidTransaction::BadSigner
			);
		});
	}
}

mod key_migration {
	use super::*;

	#[test]
	fn migrate_key_fails_without_old_key() {
		TestExt::new().execute_with(|| {
			let secret = Simple::new_secret([0; 32]);
			let public = Simple::member_from_secret(&secret);
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(3));
			assert_noop!(
				PeoplePallet::migrate_included_key(origin, public),
				Error::<Test>::NotPerson
			);
		});
	}

	#[test]
	fn migrate_key_fails_for_existing_key() {
		TestExt::new().execute_with(|| {
			// A ring with people exists
			generate_people_with_index(0, 19);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			let record3 = People::<Test>::get(3).unwrap();
			let record13 = People::<Test>::get(13).unwrap();
			assert!(matches!(
				record3.position,
				RingPosition::Included {
					ring_index: 0,
					ring_position: 3,
					scheduled_for_removal: false
				}
			));
			assert!(matches!(record13.position, RingPosition::Onboarding { queue_page: 0 }));

			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(3));
			// The key is already used by person 3 who is included
			assert_noop!(
				PeoplePallet::migrate_included_key(origin.clone(), record3.key),
				Error::<Test>::KeyAlreadyInUse
			);
			// The key is already used by person 13 who is onboarding
			assert_noop!(
				PeoplePallet::migrate_included_key(origin, record13.key),
				Error::<Test>::KeyAlreadyInUse
			);
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(13));
			// The key is already used by person 3 who is included
			assert_noop!(
				PeoplePallet::migrate_onboarding_key(origin.clone(), record3.key),
				Error::<Test>::KeyAlreadyInUse
			);
			// The key is already used by person 13 who is onboarding
			assert_noop!(
				PeoplePallet::migrate_onboarding_key(origin, record13.key),
				Error::<Test>::KeyAlreadyInUse
			);
		});
	}

	#[test]
	fn migrate_key_fails_for_suspended_person() {
		TestExt::new().execute_with(|| {
			// A ring exists
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			// One person becomes suspended
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let suspensions: &[PersonalId] = &[3];
			assert_ok!(PeoplePallet::suspend_personhood(suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// Attempt to remove suspended people succeeds
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![3]);
			assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO));

			assert!(matches!(People::<Test>::get(3).unwrap().position, RingPosition::Suspended));
			let new_secret = Simple::new_secret([100; 32]);
			let new_public = Simple::member_from_secret(&new_secret);
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(3));
			assert_noop!(
				PeoplePallet::migrate_included_key(origin, new_public),
				Error::<Test>::Suspended
			);
		});
	}

	#[test]
	fn migrate_key_enqueues_included_key_for_removal() {
		TestExt::new().execute_with(|| {
			// A ring with people exists
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			let initial_record = People::<Test>::get(3).unwrap();
			assert!(matches!(
				initial_record.position,
				RingPosition::Included {
					ring_index: 0,
					ring_position: 3,
					scheduled_for_removal: false
				}
			));
			let temp_secret = Simple::new_secret([100; 32]);
			let temp_public = Simple::member_from_secret(&temp_secret);
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(3));
			assert_ok!(PeoplePallet::migrate_included_key(origin, temp_public));
			assert_eq!(KeyMigrationQueue::<Test>::get(3).unwrap(), temp_public);

			// Makes the person's record scheduled for removal
			let temp_record = People::<Test>::get(3).unwrap();
			assert_eq!(temp_record.key, initial_record.key);
			assert_eq!(
				temp_record.position,
				RingPosition::Included {
					ring_index: 0,
					ring_position: 3,
					scheduled_for_removal: true
				}
			);
			assert_eq!(Keys::<Test>::get(temp_public).unwrap(), 3);
			assert_eq!(Keys::<Test>::get(initial_record.key).unwrap(), 3);

			let new_secret = Simple::new_secret([101; 32]);
			let new_public = Simple::member_from_secret(&new_secret);
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(3));

			assert_ok!(PeoplePallet::migrate_included_key(origin, new_public));
			assert_eq!(KeyMigrationQueue::<Test>::get(3).unwrap(), new_public);

			let new_record = People::<Test>::get(3).unwrap();
			assert_eq!(
				new_record.position,
				RingPosition::Included {
					ring_index: 0,
					ring_position: 3,
					scheduled_for_removal: true
				}
			);
			assert_eq!(Keys::<Test>::get(new_public).unwrap(), 3);
			assert_eq!(Keys::<Test>::get(initial_record.key).unwrap(), 3);
			assert!(!Keys::<Test>::contains_key(temp_public));
		});
	}

	#[test]
	fn migrate_key_replaces_onboarding_key() {
		TestExt::new().execute_with(|| {
			// A ring with people exists
			generate_people_with_index(0, 19);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			let initial_record = People::<Test>::get(13).unwrap();
			assert!(matches!(initial_record.position, RingPosition::Onboarding { queue_page: 0 }));
			let initial_pos = OnboardingQueue::<Test>::get(0)
				.iter()
				.position(|k| *k == initial_record.key)
				.unwrap();
			let new_secret = Simple::new_secret([100; 32]);
			let new_public = Simple::member_from_secret(&new_secret);
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(13));
			assert_ok!(PeoplePallet::migrate_onboarding_key(origin, new_public));

			// The person's record is the same but the key that they onboard with changed.
			let personal_record = People::<Test>::get(13).unwrap();
			assert_eq!(personal_record.position, RingPosition::Onboarding { queue_page: 0 });
			assert_eq!(OnboardingQueue::<Test>::get(0)[initial_pos], new_public);
			assert_eq!(Keys::<Test>::get(new_public).unwrap(), 13);
			assert!(!Keys::<Test>::contains_key(initial_record.key));

			// Calling it again with a different key replaces the previous one.
			let new_secret = Simple::new_secret([101; 32]);
			let new_public = Simple::member_from_secret(&new_secret);
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(13));
			let temp_record = personal_record;
			assert_ok!(PeoplePallet::migrate_onboarding_key(origin, new_public));
			assert_eq!(Keys::<Test>::get(new_public).unwrap(), 13);
			assert_eq!(OnboardingQueue::<Test>::get(0)[initial_pos], new_public);
			assert!(!Keys::<Test>::contains_key(temp_record.key));
		});
	}

	#[test]
	fn migrate_key_invalid_state() {
		TestExt::new().execute_with(|| {
			// A ring with people exists
			generate_people_with_index(0, 19);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			let new_secret = Simple::new_secret([100; 32]);
			let new_public = Simple::member_from_secret(&new_secret);

			let initial_record = People::<Test>::get(13).unwrap();
			assert!(matches!(initial_record.position, RingPosition::Onboarding { queue_page: 0 }));
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(13));
			assert_noop!(
				PeoplePallet::migrate_included_key(origin, new_public),
				Error::<Test>::InvalidKeyMigration
			);

			let initial_record = People::<Test>::get(3).unwrap();
			assert!(matches!(
				initial_record.position,
				RingPosition::Included {
					ring_index: 0,
					ring_position: 3,
					scheduled_for_removal: false
				}
			));
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(3));
			assert_noop!(
				PeoplePallet::migrate_onboarding_key(origin, new_public),
				Error::<Test>::InvalidKeyMigration
			);

			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(3));
			assert_ok!(PeoplePallet::migrate_included_key(origin, new_public));
			let new_secret = Simple::new_secret([101; 32]);
			let new_public = Simple::member_from_secret(&new_secret);
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(13));
			assert_ok!(PeoplePallet::migrate_onboarding_key(origin, new_public));
		});
	}

	#[test]
	fn migrate_keys_mutates_record() {
		TestExt::new().execute_with(|| {
			// A ring with people exists
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			let initial_record = People::<Test>::get(3).unwrap();
			assert!(matches!(
				initial_record.position,
				RingPosition::Included {
					ring_index: 0,
					ring_position: 3,
					scheduled_for_removal: false
				}
			));
			let secret1 = Simple::new_secret([100; 32]);
			let public1 = Simple::member_from_secret(&secret1);
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(3));
			assert_ok!(PeoplePallet::migrate_included_key(origin, public1));

			assert_eq!(KeyMigrationQueue::<Test>::get(3).unwrap(), public1);

			// Handle migrations
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			assert_eq!(Keys::<Test>::get(public1).unwrap(), 3);
			let new_record = People::<Test>::get(3).unwrap();
			assert_eq!(new_record.key, public1);
			assert!(matches!(new_record.position, RingPosition::Onboarding { queue_page: 0 }));
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![3]);

			let secret2 = Simple::new_secret([101; 32]);
			let public2 = Simple::member_from_secret(&secret2);
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(4));
			assert_ok!(PeoplePallet::migrate_included_key(origin, public2));
			assert_eq!(KeyMigrationQueue::<Test>::get(4).unwrap(), public2);

			// Handle migrations
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			assert_eq!(Keys::<Test>::get(public2).unwrap(), 4);
			let new_record = People::<Test>::get(4).unwrap();
			assert_eq!(new_record.key, public2);
			assert!(matches!(new_record.position, RingPosition::Onboarding { queue_page: 0 }));
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![3, 4]);
		});
	}

	#[test]
	fn migrate_keys_before_suspension() {
		TestExt::new().execute_with(|| {
			// A ring with people exists
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			let initial_record = People::<Test>::get(3).unwrap();
			assert!(matches!(
				initial_record.position,
				RingPosition::Included {
					ring_index: 0,
					ring_position: 3,
					scheduled_for_removal: false
				}
			));
			let secret1 = Simple::new_secret([100; 32]);
			let public1 = Simple::member_from_secret(&secret1);
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(3));
			assert_ok!(PeoplePallet::migrate_included_key(origin.clone(), public1));

			assert_eq!(KeyMigrationQueue::<Test>::get(3).unwrap(), public1);

			assert!(suspended_indices_list(RI_ZERO).is_empty());
			assert_noop!(
				PeoplePallet::migrate_keys(RuntimeOrigin::none(), None),
				Error::<Test>::NoKeyMigrationSession
			);
			// Handle migrations
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			assert_noop!(
				PeoplePallet::migrate_keys(RuntimeOrigin::none(), None),
				Error::<Test>::NoKeyMigrationSession
			);
			assert!(suspended_indices_list(RI_ZERO).is_empty());
			assert_ok!(PeoplePallet::suspend_personhood(&[3]));
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![3]);
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![3]);

			// Migrate another key without removing the other key.
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(4));
			let secret2 = Simple::new_secret([101; 32]);
			let public2 = Simple::member_from_secret(&secret2);
			assert_ok!(PeoplePallet::migrate_included_key(origin, public2));

			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			assert_noop!(
				PeoplePallet::migrate_keys(RuntimeOrigin::none(), None),
				Error::<Test>::NoKeyMigrationSession
			);
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![3]);
			assert_ok!(PeoplePallet::suspend_personhood(&[4]));
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![3, 4]);
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![3, 4]);
		});
	}

	#[test]
	fn migrate_keys_after_suspension() {
		TestExt::new().execute_with(|| {
			// A ring with people exists
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			let initial_record = People::<Test>::get(3).unwrap();
			assert!(matches!(
				initial_record.position,
				RingPosition::Included {
					ring_index: 0,
					ring_position: 3,
					scheduled_for_removal: false
				}
			));
			let secret1 = Simple::new_secret([100; 32]);
			let public1 = Simple::member_from_secret(&secret1);
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(3));
			assert_ok!(PeoplePallet::migrate_included_key(origin.clone(), public1));

			assert_eq!(KeyMigrationQueue::<Test>::get(3).unwrap(), public1);

			// Handle migrations
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			assert_ok!(PeoplePallet::suspend_personhood(&[3]));
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![3]);
			assert_noop!(
				PeoplePallet::migrate_keys(RuntimeOrigin::none(), None),
				Error::<Test>::NoKeyMigrationSession
			);
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![3]);
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![3]);

			assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO));
			assert!(suspended_indices_list(RI_ZERO).is_empty());

			let secret2 = Simple::new_secret([101; 32]);
			let public2 = Simple::member_from_secret(&secret2);
			assert_noop!(
				PeoplePallet::migrate_included_key(origin, public2),
				Error::<Test>::Suspended
			);
		});
	}

	#[test]
	fn migrate_key_and_remove_it() {
		TestExt::new().execute_with(|| {
			// A ring with people exists
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			let initial_record = People::<Test>::get(3).unwrap();
			assert!(matches!(
				initial_record.position,
				RingPosition::Included {
					ring_index: 0,
					ring_position: 3,
					scheduled_for_removal: false
				}
			));
			let secret1 = Simple::new_secret([100; 32]);
			let public1 = Simple::member_from_secret(&secret1);
			let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(3));
			assert_ok!(PeoplePallet::migrate_included_key(origin.clone(), public1));

			assert_eq!(KeyMigrationQueue::<Test>::get(3).unwrap(), public1);

			// Handle migrations
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			assert_noop!(
				PeoplePallet::migrate_keys(RuntimeOrigin::none(), None),
				Error::<Test>::NoKeyMigrationSession
			);
			assert!(suspended_indices_list(RI_ZERO).is_empty());
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), vec![3]);

			assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO));
			assert!(suspended_indices_list(RI_ZERO).is_empty());
			assert_eq!(RingKeys::<Test>::get(0).len(), 9);
			assert!(!RingKeys::<Test>::get(0)
				.into_iter()
				.any(|k| k == initial_record.key || k == public1));
		});
	}

	#[test]
	fn migrate_and_suspend_keys_then_remove_them() {
		TestExt::new().execute_with(|| {
			// A ring with people exists
			generate_people_with_index(0, 9);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			for personal_id in 0..10 {
				assert_eq!(
					People::<Test>::get(personal_id).unwrap().position.ring_index().unwrap(),
					RI_ZERO
				);
			}

			let old_keys = RingKeys::<Test>::get(RI_ZERO);

			let new_keys: Vec<_> = (100..110)
				.map(|x| Simple::new_secret([x; 32]))
				.map(|s| Simple::member_from_secret(&s))
				.collect();

			let static_people: Vec<PersonalId> = (0..10).filter(|i| i % 3 == 2).collect();

			let migrated_people: Vec<PersonalId> = (0..10).filter(|i| i % 3 == 0).collect();
			for personal_id in &migrated_people {
				let origin = RuntimeOrigin::from(PeopleOrigin::PersonalIdentity(*personal_id));
				assert_ok!(PeoplePallet::migrate_included_key(
					origin.clone(),
					new_keys[*personal_id as usize]
				));
			}

			assert!(suspended_indices_list(RI_ZERO).is_empty());

			// Handle migrations
			assert_ok!(PeoplePallet::start_people_set_mutation_session());

			let suspensions: Vec<PersonalId> = (0..10).filter(|i| i % 3 == 1).collect();
			assert_ok!(PeoplePallet::suspend_personhood(&suspensions[..]));

			assert_eq!(
				suspended_indices_list(RI_ZERO).into_inner(),
				suspensions.iter().copied().map(|x| x as u32).collect::<Vec<_>>()
			);

			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));
			assert_eq!(
				suspended_indices_list(RI_ZERO).len(),
				migrated_people.len() + suspensions.len()
			);

			let suspended_indices = suspended_indices_list(RI_ZERO);
			assert!(&suspended_indices[..].windows(2).all(|pair| pair[0] < pair[1]));
			assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO));
			assert!(suspended_indices_list(RI_ZERO).is_empty());
			let current_keys = RingKeys::<Test>::get(0).into_inner();
			assert_eq!(current_keys.len(), static_people.len());
			let static_keys: Vec<_> = old_keys
				.into_iter()
				.enumerate()
				.filter(|(i, _)| static_people.contains(&(*i as PersonalId)))
				.map(|(_, k)| k)
				.collect();
			assert_eq!(current_keys, static_keys);
		});
	}
}

mod onboard_people {
	use super::*;

	#[test]
	fn fails_if_suspensions_ongoing() {
		TestExt::new().execute_with(|| {
			// Several people are awaiting onboarding
			generate_people_with_index(0, 9);

			// Mutation session starts
			assert_ok!(PeoplePallet::start_people_set_mutation_session());

			// Attempt to onboard people fails
			assert_noop!(
				PeoplePallet::onboard_people(RuntimeOrigin::none()),
				Error::<Test>::Incomplete
			);
		});
	}

	#[test]
	fn fails_if_ring_contains_suspended_members() {
		TestExt::new().execute_with(|| {
			OnboardingSize::<Test>::set(5);

			// A ring with people exists
			generate_people_with_index(0, 4);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));

			// Several people become suspended
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let suspensions: &[PersonalId] = &[1, 2, 3];
			assert_ok!(PeoplePallet::suspend_personhood(suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// and are removed from the ring
			let suspended_indices = vec![1, 2, 3];
			assert_eq!(suspended_indices_list(RI_ZERO).into_inner(), suspended_indices);
			assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), RI_ZERO,));

			// One more person becomes suspended
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let suspensions: &[PersonalId] = &[4];
			assert_ok!(PeoplePallet::suspend_personhood(suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// More people are awaiting onboarding
			generate_people_with_index(10, 22);

			// Attempt to build on that same ring fails
			assert_noop!(
				PeoplePallet::onboard_people(RuntimeOrigin::none()),
				Error::<Test>::Incomplete
			);
		});
	}

	#[test]
	fn no_keys_in_queue() {
		TestExt::new().execute_with(|| {
			assert_noop!(
				PeoplePallet::onboard_people(RuntimeOrigin::none()),
				Error::<Test>::Incomplete
			);
		});
	}

	#[test]
	fn less_keys_in_queue_than_onboarding_size() {
		TestExt::new().execute_with(|| {
			OnboardingSize::<Test>::set(5);

			// Several people are awaiting onboarding
			generate_people_with_index(0, 3);

			// Onboarding attempt fails
			assert_noop!(
				PeoplePallet::onboard_people(RuntimeOrigin::none()),
				Error::<Test>::Incomplete
			);
		});
	}

	#[test]
	fn current_ring_empty() {
		TestExt::new().execute_with(|| {
			// Several people are awaiting onboarding
			generate_people_with_index(0, 9);

			// Onboarding attempt succeeds
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));

			// Keys are removed from the onboarding queue
			let queue = OnboardingQueue::<Test>::get(RI_ZERO);
			assert_eq!(queue.len(), 0);

			// Keys are inserted into ring keys and statuses
			assert_eq!(RingKeys::<Test>::get(RI_ZERO).len(), 10);
			assert_eq!(RingKeysStatus::<Test>::get(RI_ZERO), RingStatus { total: 10, included: 0 });

			// Personal records are updated
			for id in 0..=9 {
				let record = People::<Test>::get(id);
				assert!(record.is_some());
				match record.unwrap().position {
					RingPosition::Included { ring_index, .. } => {
						assert_eq!(ring_index, RI_ZERO);
					},
					_ => panic!("Expected RingPosition::Included variant"),
				}
			}

			// Current ring index is incremented
			assert_eq!(CurrentRingIndex::<Test>::get(), RI_ZERO + 1);
		});
	}

	#[test]
	fn current_ring_with_keys_in_it() {
		TestExt::new().execute_with(|| {
			OnboardingSize::<Test>::set(4);

			// A ring with people exists
			generate_people_with_index(0, 4);
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO, None));
			assert!(Root::<Test>::get(RI_ZERO).is_some());

			// Several people are awaiting onboarding
			generate_people_with_index(5, 9);

			// Onboarding attempt succeeds
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));

			// Newcomers are included in the same ring
			assert!(Root::<Test>::get(RI_ZERO + 1).is_none());
			assert_eq!(RingKeys::<Test>::get(RI_ZERO).len(), 10);
			assert_eq!(RingKeysStatus::<Test>::get(RI_ZERO), RingStatus { total: 10, included: 5 });
		});
	}

	#[test]
	fn multiple_queue_pages() {
		TestExt::new().execute_with(|| {
			// Note: default onboarding size: 10, default page size: 40
			let queue_page_size: u32 = <Test as Config>::OnboardingQueuePageSize::get();
			let onboarding_size: u32 = OnboardingSize::<Test>::get();
			let people_to_onboard = queue_page_size * 4;
			let expected_pages_to_fill: u32 = people_to_onboard / queue_page_size;
			let expected_rings_to_build: u32 = people_to_onboard / onboarding_size;

			// Start with empty onboarding queue
			assert_eq!(OnboardingQueue::<Test>::get(0).len(), 0);

			// Enough people in the queue to fill multiple pages
			generate_people_with_index(1, people_to_onboard as u8);

			// Keys are placed in multiple onboarding queue pages
			for page_index in 0..=expected_pages_to_fill - 1 {
				assert_eq!(
					OnboardingQueue::<Test>::get(page_index).len(),
					queue_page_size as usize
				);
			}

			// Each call to onboard people should onboard OnboardingSize number of keys
			// which is equal to the number of rings to build
			for _ in 0..=expected_rings_to_build - 1 {
				assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			}

			// Keys are removed from the onboarding queue
			for page_index in 0..=expected_pages_to_fill - 1 {
				assert_eq!(OnboardingQueue::<Test>::get(page_index).len(), 0);
			}

			// and are moved forward in the onboarding process
			let mut personal_id: u32 = 1;
			for ring_index in 0..=expected_rings_to_build - 1 {
				assert_eq!(RingKeys::<Test>::get(ring_index).len(), 10);
				assert_eq!(
					RingKeysStatus::<Test>::get(ring_index),
					RingStatus { total: 10, included: 0 }
				);
				while personal_id < onboarding_size {
					let record = People::<Test>::get(personal_id as u64);
					assert!(record.is_some());
					match record.unwrap().position {
						RingPosition::Included { ring_index: record_ring_index, .. } => {
							assert_eq!(record_ring_index, ring_index, "{}", personal_id);
						},
						_ => panic!("Expected RingPosition::Included variant"),
					}
					personal_id += 1;
				}
			}

			// When build_ring calls are executed
			for ring_index in 0..=expected_rings_to_build - 1 {
				assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), ring_index, None));

				// Ring related storage items change
				assert!(Root::<Test>::get(ring_index).is_some());
				assert_eq!(RingKeys::<Test>::get(ring_index).len(), 10);
				assert_eq!(
					RingKeysStatus::<Test>::get(ring_index),
					RingStatus { total: 10, included: 10 }
				);

				// Records stay the same
				while personal_id < onboarding_size {
					let record = People::<Test>::get(personal_id as u64);
					assert!(record.is_some());
					match record.unwrap().position {
						RingPosition::Included { ring_index: record_ring_index, .. } => {
							assert_eq!(record_ring_index, ring_index, "{}", personal_id);
						},
						_ => panic!("Expected RingPosition::Included variant"),
					}
					personal_id += 1;
				}
			}
		});
	}

	#[test]
	fn awaiting_people_in_between_rings_boundaries() {
		TestExt::new().execute_with(|| {
			// 3 rings with 9/10 people exists
			generate_people_with_index(1, 30);
			for _ in 1..=3 {
				assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			}
			for ring_index in 0..=2 {
				assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), ring_index, None));
			}

			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let suspensions: Vec<PersonalId> = vec![1, 11, 21];
			assert_ok!(PeoplePallet::suspend_personhood(&suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			for ring_index in 0..=2 {
				assert_ok!(PeoplePallet::remove_suspended_keys(RuntimeOrigin::none(), ring_index,));
			}

			for ring_index in 0..=2 {
				assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), ring_index, None));

				assert!(Root::<Test>::get(ring_index).is_some());
				assert_eq!(RingKeys::<Test>::get(ring_index).len(), 9);
				assert_eq!(
					RingKeysStatus::<Test>::get(ring_index),
					RingStatus { total: 9, included: 9 }
				);
			}

			// 10 more people in the queue, that then onboard
			generate_people_with_index(31, 40);
			assert_eq!(OnboardingQueue::<Test>::get(0).len(), 10);

			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			assert_eq!(OnboardingQueue::<Test>::get(0).len(), 0);

			// Previous rings stay intact
			for ring_index in 0..=2 {
				assert!(Root::<Test>::get(ring_index).is_some());
				assert_eq!(RingKeys::<Test>::get(ring_index).len(), 9);
				assert_eq!(
					RingKeysStatus::<Test>::get(ring_index),
					RingStatus { total: 9, included: 9 }
				);
			}

			// New onboarding group is assigned to a new ring
			assert_eq!(RingKeys::<Test>::get(3).len(), 10);
			assert_eq!(RingKeysStatus::<Test>::get(3), RingStatus { total: 10, included: 0 });
		});
	}

	#[test]
	fn onboarding_queue_index_rotates_when_overflown() {
		TestExt::new().execute_with(|| {
			// Queue page indices near the maximum u32 value
			QueuePageIndices::<Test>::set((u32::MAX - 2, u32::MAX - 1));

			// Enough people in the queue to fill current pages and trigger creation of a new page
			let queue_page_size: u32 = <Test as Config>::OnboardingQueuePageSize::get();
			let people_to_onboard: u8 = (queue_page_size * 3) as u8;
			generate_people_with_index(1, people_to_onboard);

			// The tail has overflown and wrapped around
			// The head should have advanced but not overflown yet
			let (head, tail) = QueuePageIndices::<Test>::get();
			assert_eq!(head, u32::MAX - 2);
			assert_eq!(tail, 0);

			// Onboarding people should cause the head to overflow
			let onboarding_size: u32 = OnboardingSize::<Test>::get();
			let expected_rings_to_build: u32 = people_to_onboard as u32 / onboarding_size;
			for _ in 0..=expected_rings_to_build - 1 {
				assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			}

			// Head should have overflown and wrapped around
			let (head, tail) = QueuePageIndices::<Test>::get();
			assert_eq!(head, 0);
			assert_eq!(tail, 0);

			// Just to make sure that the queue is still functional by adding more people
			// and onboarding them after the overflow
			generate_people_with_index(
				(queue_page_size * 3) as u8 + 1,
				(queue_page_size * 6) as u8,
			);

			let (head, tail) = QueuePageIndices::<Test>::get();
			assert_eq!(head, 0);
			assert_eq!(tail, 2);

			assert!(
				!OnboardingQueue::<Test>::get(head).is_empty() &&
					!OnboardingQueue::<Test>::get(tail).is_empty()
			);

			for _ in 0..=expected_rings_to_build - 1 {
				assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
			}
			assert_eq!(head, 0);
			assert_eq!(tail, 2);
		});
	}

	#[test]
	fn multi_paged_queue_left_with_one_page_after_single_onboard_call() {
		let queue_page_size: u32 = <Test as Config>::OnboardingQueuePageSize::get();
		let people_to_onboard = queue_page_size * 3;

		TestExt::new().max_ring_size(queue_page_size * 2).execute_with(|| {
			// Make the onboarding size big enough to consume 2 queue pages per single onboard call
			OnboardingSize::<Test>::set(queue_page_size * 2);

			// Enough people in the queue to fill multiple pages
			generate_people_with_index(1, people_to_onboard as u8);

			let (head, tail) = QueuePageIndices::<Test>::get();
			assert_eq!(head, 0);
			assert_eq!(tail, 2);
			assert_eq!(OnboardingQueue::<Test>::get(0).len(), queue_page_size as usize);
			assert_eq!(OnboardingQueue::<Test>::get(1).len(), queue_page_size as usize);
			assert_eq!(OnboardingQueue::<Test>::get(2).len(), queue_page_size as usize);

			// Onboard people call succeeds
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));

			// Members added
			assert_eq!(ActiveMembers::<Test>::get(), queue_page_size * 2);

			// Queue indices moved forward
			let (head, tail) = QueuePageIndices::<Test>::get();
			assert_eq!(head, 2);
			assert_eq!(tail, 2);

			// First two pages are empty now
			assert_eq!(OnboardingQueue::<Test>::get(0).len(), 0);
			assert_eq!(OnboardingQueue::<Test>::get(1).len(), 0);

			// The last one still full
			assert_eq!(OnboardingQueue::<Test>::get(2).len(), queue_page_size as usize);

			// More people come along to fill in one more page
			generate_people_with_index(
				people_to_onboard as u8 + 1,
				people_to_onboard as u8 + queue_page_size as u8,
			);

			// Previously empty pages stay empty, previously full page stays full,
			// one more page created and filled in
			assert_eq!(OnboardingQueue::<Test>::get(0).len(), 0);
			assert_eq!(OnboardingQueue::<Test>::get(1).len(), 0);
			assert_eq!(OnboardingQueue::<Test>::get(2).len(), queue_page_size as usize);
			assert_eq!(OnboardingQueue::<Test>::get(3).len(), queue_page_size as usize);

			// Indices change
			let (head, tail) = QueuePageIndices::<Test>::get();
			assert_eq!(head, 2);
			assert_eq!(tail, 3);

			// Another onboard people call
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));

			// Members added
			assert_eq!(ActiveMembers::<Test>::get(), queue_page_size * 4);

			// Queue indices move forward again
			let (head, tail) = QueuePageIndices::<Test>::get();
			assert_eq!(head, 3);
			assert_eq!(tail, 3);

			// All the pages are empty now
			for i in 0..3 {
				assert_eq!(OnboardingQueue::<Test>::get(i).len(), 0);
			}

			// Again, more people come along to fill in one more page
			generate_people_with_index(
				people_to_onboard as u8 + queue_page_size as u8 + 1,
				people_to_onboard as u8 + (queue_page_size as u8 * 2),
			);

			assert_eq!(OnboardingQueue::<Test>::get(3).len(), queue_page_size as usize);
			assert_eq!(OnboardingQueue::<Test>::get(4).len(), 0);

			let (head, tail) = QueuePageIndices::<Test>::get();
			assert_eq!(head, 3);
			assert_eq!(tail, 3);
		});
	}

	#[test]
	fn multi_paged_queue_left_with_no_queue_page_after_single_onboard_call() {
		let queue_page_size: u32 = <Test as Config>::OnboardingQueuePageSize::get();
		let people_to_onboard = queue_page_size * 4;
		let onboarding_size = 5;
		TestExt::new().execute_with(|| {
			crate::OnboardingSize::<Test>::set(onboarding_size);
			// Enough people in the queue to fill multiple pages
			generate_people_with_index(1, people_to_onboard as u8);

			let (head, tail) = QueuePageIndices::<Test>::get();
			assert_eq!(head, 0);
			assert_eq!(tail, 3);
			assert_eq!(OnboardingQueue::<Test>::get(0).len(), queue_page_size as usize);
			assert_eq!(OnboardingQueue::<Test>::get(1).len(), queue_page_size as usize);
			assert_eq!(OnboardingQueue::<Test>::get(2).len(), queue_page_size as usize);
			assert_eq!(OnboardingQueue::<Test>::get(3).len(), queue_page_size as usize);

			// Queue indices are updated
			let (head, tail) = QueuePageIndices::<Test>::get();
			assert_eq!(head, 0);
			assert_eq!(tail, 3);

			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let mut suspensions: Vec<PersonalId> = (3u64..queue_page_size as PersonalId).collect();
			suspensions
				.extend(2 + queue_page_size as PersonalId..2u64 * queue_page_size as PersonalId);
			assert_ok!(PeoplePallet::suspend_personhood(&suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			assert_eq!(OnboardingQueue::<Test>::get(0).len(), 3);
			assert_eq!(OnboardingQueue::<Test>::get(1).len(), 2);
			assert_eq!(OnboardingQueue::<Test>::get(2).len(), queue_page_size as usize);
			assert_eq!(OnboardingQueue::<Test>::get(3).len(), queue_page_size as usize);

			// Queue indices are the same because we still have the first 2 pages
			let (head, tail) = QueuePageIndices::<Test>::get();
			assert_eq!(head, 0);
			assert_eq!(tail, 3);

			// Onboard people call succeeds
			assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));

			// Members added
			assert_eq!(ActiveMembers::<Test>::get(), onboarding_size);

			// First two pages are empty now
			assert_eq!(OnboardingQueue::<Test>::get(0).len(), 0);
			assert_eq!(OnboardingQueue::<Test>::get(1).len(), 0);

			// Queue indices are updated to reflect the discarded pages
			let (head, tail) = QueuePageIndices::<Test>::get();
			assert_eq!(head, 2);
			assert_eq!(tail, 3);

			// The we still have another 2 queue pages.
			assert_eq!(OnboardingQueue::<Test>::get(2).len(), queue_page_size as usize);
		});
	}
}

mod merge_queue_pages {
	use super::*;
	use frame_support::pallet_prelude::Get;

	#[test]
	fn fails_if_queue_empty() {
		TestExt::new().execute_with(|| {
			assert_noop!(
				PeoplePallet::merge_queue_pages(RuntimeOrigin::none()),
				Error::<Test>::QueueStable
			);
		});
	}

	#[test]
	fn fails_if_only_one_page_exists() {
		TestExt::new().execute_with(|| {
			generate_people_with_index(0, 8);
			assert_noop!(
				PeoplePallet::merge_queue_pages(RuntimeOrigin::none()),
				Error::<Test>::QueueStable
			);
		});
	}

	#[test]
	fn fails_if_too_many_elements_per_page() {
		TestExt::new().execute_with(|| {
			// Two full pages exist
			let queue_size: u32 = <Test as Config>::OnboardingQueuePageSize::get();
			generate_people_with_index(1, queue_size as u8 * 2);

			// Half of the people become suspended in the first page
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let first_page_suspensions: Vec<PersonalId> = (1..20).collect();
			assert_ok!(PeoplePallet::suspend_personhood(&first_page_suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// Half of the people minus one become suspended in the second page
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let second_page_suspensions: Vec<PersonalId> = (40..60).collect();
			assert_ok!(PeoplePallet::suspend_personhood(&second_page_suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());

			// Attempt to merge pages fails
			assert_noop!(
				PeoplePallet::merge_queue_pages(RuntimeOrigin::none()),
				Error::<Test>::QueueStable
			);
		});
	}

	#[test]
	fn succeeds_if_only_two_pages_exist() {
		TestExt::new().execute_with(|| {
			// Two full pages exist
			let queue_size: u32 = <Test as Config>::OnboardingQueuePageSize::get();
			generate_people_with_index(1, queue_size as u8 * 2);

			// So the queue has two full pages in it
			assert_eq!(QueuePageIndices::<Test>::get(), (0, 1));
			assert!(OnboardingQueue::<Test>::get(0).is_full());
			assert!(OnboardingQueue::<Test>::get(1).is_full());

			// Half of the people become suspended in the first page
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let first_page_suspensions: Vec<PersonalId> = (0..19).collect();
			assert_ok!(PeoplePallet::suspend_personhood(&first_page_suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// More than half of the people become suspended in the second page
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let second_page_suspensions: Vec<PersonalId> = (40..61).collect();
			assert_ok!(PeoplePallet::suspend_personhood(&second_page_suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// Attempt to merge pages succeeds
			assert_ok!(PeoplePallet::merge_queue_pages(RuntimeOrigin::none()));

			// The queue pages have changed
			assert_eq!(QueuePageIndices::<Test>::get(), (1, 1));
			assert!(OnboardingQueue::<Test>::get(0).is_empty());
			let page = OnboardingQueue::<Test>::get(1);
			assert!(page.is_full());

			// All non-suspended people are still in the queue, in right order, with modified
			// personal records
			let first_page_remaining: Vec<PersonalId> = (19..40).collect();
			let second_page_remaining: Vec<PersonalId> = (61..80).collect();

			let mut page_iter = page.iter();
			for id in first_page_remaining.iter().chain(second_page_remaining.iter()) {
				let key = page_iter.next();
				assert!(key.is_some());
				let key = key.unwrap();

				let personal_id_from_page = Keys::<Test>::get(key).unwrap();
				assert_eq!(*id, personal_id_from_page);
				let personal_record = People::<Test>::get(personal_id_from_page).unwrap();
				assert_eq!(personal_record.position, RingPosition::Onboarding { queue_page: 1 });
			}
		});
	}

	#[test]
	fn succeeds_if_multiple_pages_exist() {
		TestExt::new().execute_with(|| {
			// Three full pages exist
			let queue_size: u32 = <Test as Config>::OnboardingQueuePageSize::get();
			generate_people_with_index(1, queue_size as u8 * 3);

			// So the queue has three full pages in it
			assert_eq!(QueuePageIndices::<Test>::get(), (0, 2));
			assert!(OnboardingQueue::<Test>::get(0).is_full());
			assert!(OnboardingQueue::<Test>::get(1).is_full());
			assert!(OnboardingQueue::<Test>::get(2).is_full());

			// Half of the people become suspended in the first page
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let first_page_suspensions: Vec<PersonalId> = (0..19).collect();
			assert_ok!(PeoplePallet::suspend_personhood(&first_page_suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// More than half of the people become suspended in the second page
			assert_ok!(PeoplePallet::start_people_set_mutation_session());
			let second_page_suspensions: Vec<PersonalId> = (40..61).collect();
			assert_ok!(PeoplePallet::suspend_personhood(&second_page_suspensions));
			assert_ok!(PeoplePallet::end_people_set_mutation_session());
			assert_ok!(PeoplePallet::migrate_keys(RuntimeOrigin::none(), None));

			// Attempt to merge pages succeeds
			assert_ok!(PeoplePallet::merge_queue_pages(RuntimeOrigin::none()));

			// The queue pages have changed
			assert_eq!(QueuePageIndices::<Test>::get(), (1, 2));
			assert!(OnboardingQueue::<Test>::get(0).is_empty());
			assert!(OnboardingQueue::<Test>::get(2).is_full());
			let page = OnboardingQueue::<Test>::get(1);
			assert!(page.is_full());

			// All non-suspended people are still in the queue, in right order, with modified
			// personal records
			let first_page_remaining: Vec<PersonalId> = (19..40).collect();
			let second_page_remaining: Vec<PersonalId> = (61..80).collect();

			let mut page_iter = page.iter();
			for id in first_page_remaining.iter().chain(second_page_remaining.iter()) {
				let key = page_iter.next();
				assert!(key.is_some());
				let key = key.unwrap();

				let personal_id_from_page = Keys::<Test>::get(key).unwrap();
				assert_eq!(*id, personal_id_from_page);
				let personal_record = People::<Test>::get(personal_id_from_page).unwrap();
				assert_eq!(personal_record.position, RingPosition::Onboarding { queue_page: 1 });
			}
		});
	}
}

#[test]
fn test_revision_in_tx_ext_as_alias_account() {
	new_test_ext().execute_with(|| {
		// Setup
		crate::OnboardingSize::<Test>::set(1);
		let (_, pk, sk) = generate_people_with_index(0, 0).pop().unwrap();
		assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
		assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), 0, None));
		let alias_account = 37;
		setup_alias_account(&pk, &sk, MOCK_CONTEXT, alias_account);

		// Use alias account successfully
		let call = frame_system::Call::remark { remark: vec![] };
		assert_ok!(exec_as_alias_tx(alias_account, call));

		// Revise the ring
		crate::Root::<Test>::mutate(0, |root| {
			root.as_mut().unwrap().revision = 1;
		});

		// Fail to alias account with outdated revision
		let call = frame_system::Call::remark { remark: vec![] };
		assert_noop!(exec_as_alias_tx(alias_account, call), BadSigner);
	});
}

#[test]
fn test_under_alias_revision_check() {
	new_test_ext().execute_with(|| {
		OnboardingSize::<Test>::set(1);

		// Setup a person and its alias account
		let (_, pk, sk) = generate_people_with_index(0, 0).pop().unwrap();
		assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
		assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), 0, None));
		let ring_info = Root::<Test>::get(0).expect("Ring must exist after building");
		assert_eq!(ring_info.revision, 0);
		let alias_account: u64 = 42;
		setup_alias_account(&pk, &sk, MOCK_CONTEXT, alias_account);

		// The account can now use `under_alias` successfully
		let dummy_call = Box::new(RuntimeCall::from(frame_system::Call::remark { remark: vec![] }));
		assert_ok!(PeoplePallet::under_alias(
			RuntimeOrigin::signed(alias_account),
			dummy_call.clone()
		));

		// Now we change the ring to revision=1, making the stored alias outdated.
		let mut ring_info = Root::<Test>::get(0).unwrap();
		ring_info.revision = 1;
		Root::<Test>::insert(0, ring_info);

		// Attempt `under_alias` again with the *outdated* revision=0 from storage => should fail.
		assert_noop!(
			PeoplePallet::under_alias(RuntimeOrigin::signed(alias_account), dummy_call),
			sp_runtime::DispatchError::BadOrigin,
		);
	});
}

#[test]
fn resetting_alias_account_for_new_revision_is_refunded() {
	new_test_ext().execute_with(|| {
		use crate::pallet::Origin as PeopleOrigin;
		OnboardingSize::<Test>::set(1);

		// Create a single person so that `build_ring` can work.
		let (_, _key, _secret) = generate_people_with_index(0, 0).pop().unwrap();

		// Build the ring with the single key we just inserted. This sets the ring revision to 0.
		assert_ok!(PeoplePallet::onboard_people(RuntimeOrigin::none()));
		assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), 0, None));
		let ring_info = Root::<Test>::get(0).expect("Ring must exist after build_ring");
		assert_eq!(ring_info.revision, 0);

		// Set an alias with revision=0 for an account for the first time.
		// We expect `Pays::No` because no alias was previously set.
		let account: u64 = 42;
		let ca = ContextualAlias { alias: [1u8; 32], context: MOCK_CONTEXT };
		let rev_ca = RevisedContextualAlias { revision: 0, ring: 0, ca: ca.clone() };
		let origin = RuntimeOrigin::from(PeopleOrigin::PersonalAlias(rev_ca.clone()));
		let result = PeoplePallet::set_alias_account(origin, account, 0);
		assert_eq!(result.unwrap(), frame_support::pallet_prelude::Pays::No.into());

		// Fail attempt to set the same alias again with the *same* revision=0 for the same account.
		let origin = RuntimeOrigin::from(PeopleOrigin::PersonalAlias(rev_ca.clone()));
		assert_noop!(
			PeoplePallet::set_alias_account(origin, account, 0),
			Error::<Test>::AliasAccountAlreadySet
		);

		// Attempt to set the same alias again with the *same* revision=0 for a different account.
		let origin = RuntimeOrigin::from(PeopleOrigin::PersonalAlias(rev_ca.clone()));
		let account2: u64 = 43;
		let result = PeoplePallet::set_alias_account(origin, account2, 0);
		assert_eq!(result.unwrap(), frame_support::pallet_prelude::Pays::Yes.into());

		// Set the ring revision to 1.
		let ring_info = Root::<Test>::get(0).unwrap();
		Root::<Test>::insert(0, RingRoot { revision: 1, ..ring_info });

		// Now set the alias account again, but *with the newer revision=1*.
		// We expect `Pays::No` because the revision of the alias <-> Account is needed.
		let rev_ca_new = RevisedContextualAlias { revision: 1, ring: 0, ca: ca.clone() };
		let origin = RuntimeOrigin::from(PeopleOrigin::PersonalAlias(rev_ca_new.clone()));
		let result = PeoplePallet::set_alias_account(origin, account, 0);
		assert_eq!(result.unwrap(), frame_support::pallet_prelude::Pays::No.into());

		// Move to a different ring.
		let ring_info = Root::<Test>::get(0).unwrap();
		Root::<Test>::insert(1, ring_info);

		// Now set the alias account again, but *with the different ring=1*.
		// We expect `Pays::No` because the revision of the alias <-> Account is needed.
		let rev_ca_new = RevisedContextualAlias { revision: 1, ring: 1, ca };
		let origin = RuntimeOrigin::from(PeopleOrigin::PersonalAlias(rev_ca_new.clone()));
		let result = PeoplePallet::set_alias_account(origin, account, 0);
		assert_eq!(result.unwrap(), frame_support::pallet_prelude::Pays::No.into());
	});
}
