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

use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};
use individuality_support::traits::RI_ZERO;
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
			PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO),
			Error::<Test>::StillFresh
		);

		// Not enough for a queue
		generate_people_with_index(0, 3);

		assert_noop!(
			PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO),
			Error::<Test>::Incomplete
		);

		// Now we have enough to build one.
		generate_people_with_index(4, 4);

		assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO));

		// We can add 5 more people
		generate_people_with_index(5, 9);

		assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), RI_ZERO));

		// We can add 26 more people
		generate_people_with_index(10, 35);

		assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), 1));

		assert_ok!(PeoplePallet::build_ring(RuntimeOrigin::none(), 2));

		// Can't build 3, because then there are only 4 spots left which is less than onboarding
		// size of 5
		assert_noop!(PeoplePallet::build_ring(RuntimeOrigin::none(), 3), Error::<Test>::Incomplete);
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
		let secret_b = Simple::new_secret([2; 32]);
		let key_a = Simple::member_from_secret(&secret_a);
		let key_b = Simple::member_from_secret(&secret_b);
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
		assert_ok!(PeoplePallet::recognize_personhood(person_a, Some(key_b)));
	});
}

// #[test]
// fn recognize_person_with_duplicate_key_after_suspend() {
// 	TestExt::new().execute_with(|| {
// 		// Recognize person A
// 		let person_a = 1u64;
// 		let secret_a = Simple::new_secret([1; 32]);
// 		let key_a = Simple::member_from_secret(&secret_a);
// 		assert_ok!(PeoplePallet::recognize_personhood(person_a, Some(key_a)));

// 		// Suspend person A
// 		assert_ok!(PeoplePallet::suspend_index(person_a));

// 		// Recognize person B with same key
// 		let person_b = 2u64;
// 		assert_ok!(PeoplePallet::recognize_personhood(person_b, Some(key_a)));

// 		// Recognize person A again with no key
// 		assert_noop!(
// 			PeoplePallet::recognize_personhood(person_a, None),
// 			Error::<Test>::KeyAlreadyInUse
// 		);
// 	});
// }

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
