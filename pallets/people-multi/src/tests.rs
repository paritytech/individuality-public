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
use frame_support::{assert_noop, assert_ok, pallet_prelude::Pays};
use individuality_support::traits::RI_ZERO;
use sp_runtime::transaction_validity::InvalidTransaction;
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
			RingPosition::Included { ring_index: 0, ring_position: 0 }
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
		assert_ok!(PeoplePallet::remove_suspended_people(
			RuntimeOrigin::signed(0),
			0,
			vec![0].try_into().unwrap()
		));

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
	fn submits_transaction_only_at_interval_ticks() {
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
			let block = 0;
			System::set_block_number(block);
			PeoplePallet::offchain_worker(block);
			assert_eq!(state.read().transactions.len(), 0);
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
}

mod validate_unsigned {
	use super::*;
	use frame_support::BoundedVec;
	use sp_runtime::{
		traits::ValidateUnsigned,
		transaction_validity::{InvalidTransaction, TransactionSource},
	};

	#[test]
	fn only_works_for_build_ring_calls() {
		TestExt::new().execute_with(|| {
			// Set-up to make the checks validating the need of ring build pass
			OnboardingSize::<Test>::set(1);
			let member_key = Simple::member_from_secret(&Simple::new_secret([0u8; 32]));
			let keys: BoundedVec<MemberOf<Test>, <Test as Config>::MaxRingSize> =
				BoundedVec::try_from(vec![member_key]).expect("failed to init members");
			let ring_status = RingStatus { total: keys.len().saturated_into(), included: 0 };
			RingKeys::<Test>::insert(0, keys);
			RingKeysStatus::<Test>::insert(0, ring_status);

			// build_ring call should succeed
			let valid_call = Call::<Test>::build_ring {
				ring_index: 0,
				limit: Some(OnboardingSize::<Test>::get()),
			};
			assert_ok!(PeoplePallet::validate_unsigned(TransactionSource::Local, &valid_call),);

			// Other calls should fail
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
