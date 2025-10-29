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

use std::collections::BTreeSet;

use crate::{mock::*, ReservedIds};
use frame_support::assert_ok;
use indiv_support::traits::{AddOnlyPeopleTrait, PersonalId};
use verifiable::{demo_impls::Simple, GenerateVerifiable};

#[test]
fn id_registration_works() {
	TestExt::new().execute_with(|| {
		assert_ok!(DummyDim::reserve_ids(RuntimeOrigin::root(), 100));
		let dummy_ids: BTreeSet<_> = ReservedIds::<Test>::iter_keys().collect();
		let people_ids: BTreeSet<_> =
			indiv_pallet_people::ReservedPersonalId::<Test>::iter_keys().collect();
		assert_eq!(dummy_ids, people_ids);
		let mut independent_ids = vec![];
		for _ in 0..100 {
			let id = People::reserve_new_id();
			independent_ids.push(id);
		}
		assert_ok!(DummyDim::reserve_ids(RuntimeOrigin::root(), 100));
		let dummy_ids: BTreeSet<_> = ReservedIds::<Test>::iter_keys().collect();
		let expected_ids: BTreeSet<_> = (0..100).chain(200..300).collect();
		assert_eq!(dummy_ids, expected_ids);

		for id in 0..100 {
			assert_ok!(DummyDim::cancel_id_reservation(RuntimeOrigin::root(), id));
		}
		let dummy_ids: BTreeSet<_> = ReservedIds::<Test>::iter_keys().collect();
		let expected_ids: BTreeSet<_> = (200..300).collect();
		assert_eq!(dummy_ids, expected_ids);

		for id in 100..150 {
			assert_ok!(People::cancel_id_reservation(id));
		}

		for id in 50..150 {
			assert_ok!(DummyDim::renew_id_reservation(RuntimeOrigin::root(), id));
		}
		let dummy_ids: BTreeSet<_> = ReservedIds::<Test>::iter_keys().collect();
		let expected_ids: BTreeSet<_> = (50..150).chain(200..300).collect();
		assert_eq!(dummy_ids, expected_ids);
	});
}

#[test]
fn personhood_recognition_and_suspension_works() {
	TestExt::new().execute_with(|| {
		assert_ok!(DummyDim::reserve_ids(RuntimeOrigin::root(), 200));
		let ids_and_keys: Vec<_> = (0..100)
			.map(|i| (i as PersonalId, Simple::member_from_secret(&[i; 32])))
			.collect();
		assert_ok!(DummyDim::recognize_personhood(
			RuntimeOrigin::root(),
			ids_and_keys.clone().try_into().unwrap()
		));
		for (id, key) in ids_and_keys {
			assert_eq!(key, crate::People::<Test>::get(id).unwrap().key);
			assert!(indiv_pallet_people::Keys::<Test>::contains_key(key));
		}

		let new_ids_and_keys: Vec<_> = (100..150)
			.map(|i| (i as PersonalId, Simple::member_from_secret(&[i; 32])))
			.collect();
		assert_ok!(DummyDim::recognize_personhood(
			RuntimeOrigin::root(),
			new_ids_and_keys.clone().try_into().unwrap()
		));
		for (id, key) in new_ids_and_keys {
			assert_eq!(key, crate::People::<Test>::get(id).unwrap().key);
			assert!(indiv_pallet_people::Keys::<Test>::contains_key(key));
		}

		assert_ok!(DummyDim::start_mutation_session(RuntimeOrigin::root()));

		let suspended_ids: Vec<_> = (50..125).collect();
		assert_ok!(DummyDim::suspend_personhood(
			RuntimeOrigin::root(),
			suspended_ids.try_into().unwrap()
		));
		for id in (0..50).chain(125..150) {
			assert!(!crate::People::<Test>::get(id).unwrap().suspended);
			assert!(!indiv_pallet_people::People::<Test>::get(id).unwrap().position.suspended());
		}
		for id in 50..125 {
			assert!(crate::People::<Test>::get(id).unwrap().suspended);
			assert!(indiv_pallet_people::People::<Test>::get(id).unwrap().position.suspended());
		}

		assert_ok!(DummyDim::end_mutation_session(RuntimeOrigin::root()));
		indiv_pallet_people::RingsState::<Test>::mutate(|s| {
			*s = s.clone().end_key_migration().unwrap()
		});
		assert_ok!(DummyDim::start_mutation_session(RuntimeOrigin::root()));

		for id in 50..100 {
			assert_ok!(DummyDim::resume_personhood(RuntimeOrigin::root(), id));
		}

		for id in (0..100).chain(125..150) {
			assert!(!crate::People::<Test>::get(id).unwrap().suspended);
			assert!(!indiv_pallet_people::People::<Test>::get(id).unwrap().position.suspended());
		}
		for id in 100..125 {
			assert!(crate::People::<Test>::get(id).unwrap().suspended);
			assert!(indiv_pallet_people::People::<Test>::get(id).unwrap().position.suspended());
		}

		assert_ok!(DummyDim::end_mutation_session(RuntimeOrigin::root()));
	});
}
