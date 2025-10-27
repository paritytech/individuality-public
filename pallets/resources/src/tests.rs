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

//! Unit tests for the resources pallet.

use super::{pallet::*, *};
use crate::{
	mock::*,
	types::{Credibility, PersonalUsernameChoice},
	Error,
};
use frame_support::{assert_noop, assert_ok};
use sp_core::{ed25519, Get, Pair};
use sp_runtime::AccountId32;
use sp_statement_store::{Statement, StatementSource};

#[test]
fn register_lite_person_success() {
	new_test_ext().execute_with(|| {
		let user_account = id_to_account(1);
		let origin = lite_person_origin(1);
		let uname = username::<Test>(b"testuser.12");
		let comm = comm_id(b"key1");

		assert_ok!(Resources::register_lite_person(origin.clone(), comm, uname.clone(), None));

		let consumer_info = Consumers::<Test>::get(&user_account).unwrap();
		assert_eq!(consumer_info.identifier_key, comm);
		assert_eq!(consumer_info.lite_username, uname.clone());
		assert_eq!(consumer_info.full_username, None);
		assert_eq!(consumer_info.credibility, Credibility::Lite);
		assert_eq!(UsernameOwnerOf::<Test>::get(&uname), Some(user_account.clone()));
		assert_eq!(System::sufficients(&user_account), 1); // Sufficiency increased
	});
}

#[test]
fn register_lite_person_with_reservation_success() {
	new_test_ext().execute_with(|| {
		let user_account = id_to_account(1);
		let origin = lite_person_origin(1);
		let uname = username::<Test>(b"testuser.12");
		let reserved_uname = username::<Test>(b"reserved");
		let comm = comm_id(b"key1");
		set_time_sec(100);

		assert_ok!(Resources::register_lite_person(
			origin.clone(),
			comm,
			uname.clone(),
			Some(reserved_uname.clone())
		));

		let consumer_info = Consumers::<Test>::get(&user_account).unwrap();
		assert_eq!(consumer_info.lite_username, uname.clone());
		assert_eq!(UsernameOwnerOf::<Test>::get(&uname), Some(user_account.clone()));

		let reservation = ReservedUsernames::<Test>::get(&reserved_uname).unwrap();
		assert_eq!(reservation.owner, user_account);
		assert_eq!(reservation.since, 100);
	});
}

#[test]
fn register_person_success_standalone_username() {
	new_test_ext().execute_with(|| {
		let person_id = 10;
		let person_alias = id_to_alias(person_id);
		let origin = person_origin_for(person_id, 0, 0);
		let lite_account = id_to_account(1);
		let lite_uname = username::<Test>(b"liteuser.12");
		let person_uname = username::<Test>(b"personuser");
		let comm = comm_id(b"key1");

		// Register lite person first
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			comm,
			lite_uname.clone(),
			None
		));
		assert!(Consumers::<Test>::get(&lite_account).is_some());
		set_time_sec(100);

		let proof = mock_lite_proof(lite_account.clone());

		assert_ok!(Resources::register_person(
			origin.clone(),
			lite_account.clone(),
			proof,
			PersonalUsernameChoice::Standalone(person_uname.clone())
		));

		let consumer_info = Consumers::<Test>::get(&lite_account).unwrap();
		assert_eq!(consumer_info.identifier_key, comm);
		assert_eq!(consumer_info.lite_username, lite_uname.clone());
		assert_eq!(consumer_info.full_username, Some(person_uname.clone()));
		assert_eq!(
			consumer_info.credibility,
			Credibility::Person { alias: person_alias, last_update: 100 }
		);
		assert_eq!(UsernameOwnerOf::<Test>::get(&lite_uname), Some(lite_account.clone()));
		assert_eq!(UsernameOwnerOf::<Test>::get(&person_uname), Some(lite_account.clone()));
		assert_eq!(AccountOfAlias::<Test>::get(person_alias), Some(lite_account.clone()));
		assert_eq!(System::sufficients(&lite_account), 1); // Should still be 1
	});
}

#[test]
fn register_person_success_with_reservation() {
	new_test_ext().execute_with(|| {
		let person_id = 10;
		let person_alias = id_to_alias(person_id);
		let origin = person_origin_for(person_id, 0, 0);
		let lite_account = id_to_account(1);
		let lite_uname = username::<Test>(b"liteuser.12");
		let reserved_uname = username::<Test>(b"reserved");
		let comm = comm_id(b"key1");
		set_time_sec(50);

		// Register lite person first with reservation
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			comm,
			lite_uname.clone(),
			Some(reserved_uname.clone())
		));
		assert!(ReservedUsernames::<Test>::get(&reserved_uname).is_some());
		set_time_sec(100);

		let proof = mock_lite_proof(lite_account.clone());

		assert_ok!(Resources::register_person(
			origin.clone(),
			lite_account.clone(),
			proof,
			PersonalUsernameChoice::Reservation(reserved_uname.clone())
		));

		let consumer_info = Consumers::<Test>::get(&lite_account).unwrap();
		assert_eq!(consumer_info.full_username, Some(reserved_uname.clone()));
		assert_eq!(
			consumer_info.credibility,
			Credibility::Person { alias: person_alias, last_update: 100 }
		);
		assert_eq!(UsernameOwnerOf::<Test>::get(&reserved_uname), Some(lite_account.clone()));
		assert_eq!(AccountOfAlias::<Test>::get(person_alias), Some(lite_account.clone()));
		assert!(ReservedUsernames::<Test>::get(&reserved_uname).is_none()); // Reservation consumed
	});
}

#[test]
fn register_fails_if_already_registered() {
	new_test_ext().execute_with(|| {
		let user_account = id_to_account(1);
		let origin = lite_person_origin(1);
		let uname = username::<Test>(b"testuser.12");
		let comm = comm_id(b"key1");

		// Register once
		assert_ok!(Resources::register_lite_person(origin.clone(), comm, uname.clone(), None));

		// Try registering lite again
		let second_uname = username::<Test>(b"secondtestuser.12");
		let second_comm = comm_id(b"key2");
		assert_noop!(
			Resources::register_lite_person(
				origin.clone(),
				second_comm,
				second_uname.clone(),
				None
			),
			Error::<Test>::AlreadyRegistered
		);

		// Try registering as full person (linking this lite person)
		let person_id = 10;
		let person_alias = id_to_alias(person_id);
		let person_origin = person_origin_for(person_id, 0, 0);
		let person_uname = username::<Test>(b"personuser");
		let proof = mock_lite_proof(user_account.clone());

		// Need to upgrade credibility before this check
		Consumers::<Test>::mutate(&user_account, |c| {
			if let Some(info) = c {
				info.credibility = Credibility::Person { alias: person_alias, last_update: 100 };
			}
		});
		AccountOfAlias::<Test>::insert(person_alias, user_account.clone()); // Mock alias already used

		assert_noop!(
			Resources::register_person(
				person_origin.clone(),
				user_account.clone(),
				proof,
				PersonalUsernameChoice::Standalone(person_uname.clone())
			),
			Error::<Test>::AlreadyRegistered // Because alias is already registered
		);
	});
}

#[test]
fn register_fails_if_username_taken() {
	new_test_ext().execute_with(|| {
		let uname = username::<Test>(b"takenuser.12");
		let comm = comm_id(b"key1");

		// User 1 registers with uname
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			comm,
			uname.clone(),
			None
		));

		// User 2 tries to register lite with same uname
		assert_noop!(
			Resources::register_lite_person(lite_person_origin(2), comm, uname.clone(), None),
			Error::<Test>::UsernameTaken
		);

		// User 3 (full person) tries to register with same uname (standalone)
		let person_id = 10;
		let person_origin = person_origin_for(person_id, 0, 0);
		let lite_account_for_person = id_to_account(3); // A different lite account to link
		let proof = mock_lite_proof(lite_account_for_person.clone());
		// Need to register the lite account first
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(3),
			comm,
			username::<Test>(b"another.34"), // different username
			None
		));
		assert_noop!(
			Resources::register_person(
				person_origin.clone(),
				lite_account_for_person.clone(),
				proof.clone(),
				PersonalUsernameChoice::Standalone(uname.clone()) // Trying taken username
			),
			Error::<Test>::UsernameTaken
		);

		// User 3 tries to register using a reservation for the taken username
		// (This scenario shouldn't happen if reservation checks work, but test anyway)
		assert_noop!(
			Resources::register_person(
				person_origin.clone(),
				lite_account_for_person.clone(),
				proof.clone(),
				PersonalUsernameChoice::Reservation(uname.clone()) // Trying taken username
			),
			Error::<Test>::NoReservation // It will fail here first as reservation doesn't exist
		);
	});
}

#[test]
fn register_fails_if_username_invalid() {
	new_test_ext().execute_with(|| {
		let comm = comm_id(b"key1");

		// Lite person invalid names
		let invalid_lite_unames = [
			b"invalid".to_vec(),  // No digits
			b"invalid.".to_vec(), // No digits
			b"invalid.1".to_vec(), /* Not enough
			                       * digits */
			b"Invalid.12".to_vec(),  // Uppercase
			b"in_valid.12".to_vec(), // Underscore
			b"short.12".to_vec(),    // Too short base
		];
		for uname_bytes in invalid_lite_unames {
			let uname = username::<Test>(&uname_bytes);
			assert_noop!(
				Resources::register_lite_person(lite_person_origin(1), comm, uname.clone(), None),
				Error::<Test>::InvalidUsername,
			);
		}

		// Person invalid names
		let person_id = 10;
		let person_origin = person_origin_for(person_id, 0, 0);
		let lite_account_for_person = id_to_account(2);
		let proof = mock_lite_proof(lite_account_for_person.clone());
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(2),
			comm,
			username::<Test>(b"linking.12"),
			None
		)); // Setup linked account

		let invalid_person_unames = [
			b"Invalid".to_vec(),  // Uppercase
			b"in_valid".to_vec(), // Underscore
			b"invalid.12".to_vec(), /* Dot separator (only allowed for
			                       * lite) */
			b"short".to_vec(), // Too short
		];
		for uname_bytes in invalid_person_unames {
			let uname = username::<Test>(&uname_bytes);
			assert_noop!(
				Resources::register_person(
					person_origin.clone(),
					lite_account_for_person.clone(),
					proof.clone(),
					PersonalUsernameChoice::Standalone(uname.clone())
				),
				Error::<Test>::InvalidUsername
			);
		}

		// Test invalid reserved username for lite person
		let valid_uname = username::<Test>(b"gooduser.12");
		let invalid_reserved = username::<Test>(b"Invalid");
		assert_noop!(
			Resources::register_lite_person(
				lite_person_origin(1),
				comm,
				valid_uname.clone(),
				Some(invalid_reserved)
			),
			Error::<Test>::InvalidUsername // validate_username runs even for reservation
		);

		// Test invalid reserved username for full person
		let valid_reserved = username::<Test>(b"reserved");
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			comm,
			valid_uname.clone(),
			Some(valid_reserved)
		),);
		let invalid_reserved = username::<Test>(b"Invalid");
		// reservation doesn't exist; it it did, the reserved username must have been validated when
		// the reservation was made, as checked above
		assert_noop!(
			Resources::register_person(
				person_origin.clone(),
				lite_account_for_person.clone(),
				proof.clone(),
				PersonalUsernameChoice::Reservation(invalid_reserved)
			),
			Error::<Test>::NoReservation
		);
	});
}

#[test]
fn register_person_fails_invalid_proof() {
	new_test_ext().execute_with(|| {
		let person_id = 10;
		let origin = person_origin_for(person_id, 0, 0);
		let lite_account = id_to_account(1);
		let person_uname = username::<Test>(b"personuser");
		let comm = comm_id(b"key1");

		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			comm,
			username::<Test>(b"linking.12"),
			None
		));

		// Create proof using a different account's authority
		let invalid_proof = mock_lite_proof(id_to_account(99)); // Proof for account 99

		assert_noop!(
			Resources::register_person(
				origin.clone(),
				lite_account.clone(),
				invalid_proof, // Incorrect proof
				PersonalUsernameChoice::Standalone(person_uname.clone())
			),
			Error::<Test>::InvalidProofOfOwnership
		);
	});
}

#[test]
fn register_fails_reserved_username_taken() {
	new_test_ext().execute_with(|| {
		let uname1 = username::<Test>(b"userone.12");
		let uname2 = username::<Test>(b"usertwo.12");
		let reserved_uname = username::<Test>(b"reserved");
		let comm = comm_id(b"key1");
		set_time_sec(100);

		// User 1 registers and reserves "reserved"
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			comm,
			uname1.clone(),
			Some(reserved_uname.clone())
		));

		// User 2 tries to reserve the same name
		assert_noop!(
			Resources::register_lite_person(
				lite_person_origin(2),
				comm,
				uname2.clone(),
				Some(reserved_uname.clone())
			),
			Error::<Test>::UsernameReservationTaken
		);

		// User 3 (full person) tries to use the reservation made by User 1
		let person_id = 10;
		let person_origin = person_origin_for(person_id, 0, 0);
		let lite_account_for_person = id_to_account(3); // Link User 3's lite account
		let proof = mock_lite_proof(lite_account_for_person.clone());
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(3),
			comm,
			username::<Test>(b"userthree.12"),
			None
		)); // Setup linked account
		assert_noop!(
			Resources::register_person(
				person_origin.clone(),
				lite_account_for_person.clone(), // User 3's lite account
				proof.clone(),
				PersonalUsernameChoice::Reservation(reserved_uname.clone()) // User 1's reservation
			),
			Error::<Test>::InvalidUsernameSignature // Fails because reservation owner mismatch
		);

		// Test scenario where reserved name is already taken as a primary name
		let uname4 = username::<Test>(b"userfour.12");
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(4),
			comm,
			uname4.clone(), // User 4 takes this name
			None
		));
		// User 5 tries to reserve it
		assert_noop!(
			Resources::register_lite_person(
				lite_person_origin(5),
				comm,
				username::<Test>(b"userfive.12"),
				Some(uname4)
			),
			Error::<Test>::UsernameReservationTaken
		);
	});
}

#[test]
fn register_person_fails_no_linked_lite_identity() {
	new_test_ext().execute_with(|| {
		let person_id = 10;
		let origin = person_origin_for(person_id, 0, 0);
		let non_existent_lite_account = id_to_account(99); // This account is not registered
		let person_uname = username::<Test>(b"personuser");
		let proof = mock_lite_proof(non_existent_lite_account.clone());

		assert_noop!(
			Resources::register_person(
				origin.clone(),
				non_existent_lite_account.clone(), // Non-existent account
				proof,
				PersonalUsernameChoice::Standalone(person_uname.clone())
			),
			Error::<Test>::NoLinkedIdentity
		);
	});
}

#[test]
fn register_person_fails_lite_identity_already_linked() {
	new_test_ext().execute_with(|| {
		let person1_id = 10;
		let person1_origin = person_origin_for(person1_id, 0, 0);

		let person2_id = 20;
		let person2_origin = person_origin_for(person2_id, 1, 0); // Different ring/rev

		let lite_account = id_to_account(1);
		let lite_uname = username::<Test>(b"liteuser.12");
		let person1_uname = username::<Test>(b"firstpersonuser");
		let person2_uname = username::<Test>(b"secondpersonuser");
		let comm = comm_id(b"key1");

		// Register lite person
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			comm,
			lite_uname.clone(),
			None
		));

		// Person 1 links the lite account
		let proof1 = mock_lite_proof(lite_account.clone());
		assert_ok!(Resources::register_person(
			person1_origin.clone(),
			lite_account.clone(),
			proof1,
			PersonalUsernameChoice::Standalone(person1_uname.clone())
		));

		// Person 2 tries to link the same lite account
		let proof2 = mock_lite_proof(lite_account.clone());
		assert_noop!(
			Resources::register_person(
				person2_origin.clone(),
				lite_account.clone(), // Already linked lite account
				proof2,
				PersonalUsernameChoice::Standalone(person2_uname.clone())
			),
			Error::<Test>::AlreadyLinked
		);
	});
}

#[test]
fn touch_authorization_success() {
	new_test_ext().execute_with(|| {
		let person_id = 10;
		let person_alias = id_to_alias(person_id);
		let person_origin = person_origin_for(person_id, 0, 0);
		let lite_account = id_to_account(1);

		// Register as person
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			comm_id(b"key"),
			username::<Test>(b"liteper.12"),
			None
		));
		let proof = mock_lite_proof(lite_account.clone());
		set_time_sec(100);
		assert_ok!(Resources::register_person(
			person_origin.clone(),
			lite_account.clone(),
			proof,
			PersonalUsernameChoice::Standalone(username::<Test>(b"fullperson"))
		));
		assert_eq!(
			Consumers::<Test>::get(&lite_account).unwrap().credibility,
			Credibility::Person { alias: person_alias, last_update: 100 }
		);

		// Advance time past the minimum interval but within duration
		let duration: u32 = <Test as Config>::MinPersonAuthUpdateInterval::get();
		advance_time_sec(duration as u64 + 1);
		let new_time = TestClock::now().as_secs();

		// Touch authorization
		assert_ok!(Resources::touch_person_authorization(person_origin.clone()));

		// Check last_update is updated
		assert_eq!(
			Consumers::<Test>::get(&lite_account).unwrap().credibility,
			Credibility::Person { alias: person_alias, last_update: new_time }
		);
	});
}

#[test]
fn touch_authorization_fails_not_registered() {
	new_test_ext().execute_with(|| {
		let person_id = 10;
		let person_origin = person_origin_for(person_id, 0, 0); // Origin exists, but not registered in Resources

		assert_noop!(
			Resources::touch_person_authorization(person_origin.clone()),
			Error::<Test>::NotRegistered
		);
	});
}

#[test]
fn touch_authorization_fails_not_full_person() {
	new_test_ext().execute_with(|| {
		let lite_account = id_to_account(1);
		let lite_origin = lite_person_origin(1); // Use lite origin

		// Register as lite person
		assert_ok!(Resources::register_lite_person(
			lite_origin.clone(),
			comm_id(b"key"),
			username::<Test>(b"liteper.12"),
			None
		));

		// Try touching with a Person origin (that isn't linked) - should fail NotRegistered
		let person_id = 10;
		let person_origin = person_origin_for(person_id, 0, 0);
		assert_noop!(
			Resources::touch_person_authorization(person_origin),
			Error::<Test>::NotRegistered
		);

		// Even if we somehow got a Person origin linked to the lite account's alias
		// (this shouldn't happen without register_person), the check inside touch
		// requires `Credibility::Person`.
		// Manually insert the alias mapping to test the inner check.
		let alias = id_to_alias(person_id);
		AccountOfAlias::<Test>::insert(alias, lite_account.clone());
		let origin = person_origin_for(person_id, 0, 0);
		assert_noop!(
			Resources::touch_person_authorization(origin),
			Error::<Test>::NotFullPerson // Fails because credibility is `Lite`
		);
	});
}

#[test]
fn touch_authorization_fails_too_early() {
	new_test_ext().execute_with(|| {
		let person_id = 10;
		let person_origin = person_origin_for(person_id, 0, 0);
		let lite_account = id_to_account(1);

		// Register as person
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			comm_id(b"key"),
			username::<Test>(b"liteper.12"),
			None
		));
		let proof = mock_lite_proof(lite_account.clone());
		set_time_sec(100);
		assert_ok!(Resources::register_person(
			person_origin.clone(),
			lite_account.clone(),
			proof,
			PersonalUsernameChoice::Standalone(username::<Test>(b"fullperson"))
		));

		// Advance time less than the minimum interval
		let duration: u32 = <Test as Config>::MinPersonAuthUpdateInterval::get();
		advance_time_sec(duration as u64 - 1);

		// Touch authorization should fail
		assert_noop!(
			Resources::touch_person_authorization(person_origin.clone()),
			Error::<Test>::TouchNotReady
		);
	});
}

#[test]
fn remove_username_reservation_success() {
	new_test_ext().execute_with(|| {
		let reserved_uname = username::<Test>(b"reserved");
		let reservation_time = 100;
		set_time_sec(reservation_time);

		// Reserve username
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			comm_id(b"key"),
			username::<Test>(b"liteper.12"),
			Some(reserved_uname.clone())
		));
		assert!(ReservedUsernames::<Test>::contains_key(&reserved_uname));

		// Advance time past expiry
		let duration: u64 = <Test as Config>::UsernameReservationDuration::get();
		advance_time_sec(duration + 1u64);

		// Remove reservation (origin doesn't matter much here, just needs to be signed)
		assert_ok!(Resources::remove_expired_username_reservation(
			lite_person_origin(2), // Another user removes it
			reserved_uname.clone()
		));

		// Check reservation is gone
		assert!(!ReservedUsernames::<Test>::contains_key(&reserved_uname));
	});
}

#[test]
fn remove_username_reservation_fails_no_reservation() {
	new_test_ext().execute_with(|| {
		let non_reserved_uname = username::<Test>(b"notreserved");

		assert_noop!(
			Resources::remove_expired_username_reservation(
				lite_person_origin(1),
				non_reserved_uname.clone()
			),
			Error::<Test>::NoReservation
		);
	});
}

#[test]
fn remove_username_reservation_fails_too_early() {
	new_test_ext().execute_with(|| {
		let reserved_uname = username::<Test>(b"reserved");
		let reservation_time = 100;
		set_time_sec(reservation_time);

		// Reserve username
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			comm_id(b"key"),
			username::<Test>(b"liteper.12"),
			Some(reserved_uname.clone())
		));

		// Advance time, but not past expiry
		let duration: u64 = <Test as Config>::UsernameReservationDuration::get();
		advance_time_sec(duration - 1u64);

		// Try removing reservation
		assert_noop!(
			Resources::remove_expired_username_reservation(
				lite_person_origin(2),
				reserved_uname.clone()
			),
			Error::<Test>::ReservationFresh
		);
	});
}

#[test]
fn update_identifier_key_success() {
	new_test_ext().execute_with(|| {
		let user_account = id_to_account(1);
		let origin = RuntimeOrigin::signed(user_account.clone());
		let initial_comm = comm_id(b"key1");
		let new_comm = comm_id(b"key2");

		// Register lite person
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			initial_comm,
			username::<Test>(b"liteper.12"),
			None
		));

		// Update key
		assert_ok!(Resources::update_identifier_key(origin.clone(), new_comm));

		// Check key updated
		assert_eq!(Consumers::<Test>::get(&user_account).unwrap().identifier_key, new_comm);
	});
}

#[test]
fn update_identifier_key_fails_not_registered() {
	new_test_ext().execute_with(|| {
		let non_registered_account = id_to_account(99);
		let origin = RuntimeOrigin::signed(non_registered_account.clone());
		let new_comm = comm_id(b"key2");

		assert_noop!(
			Resources::update_identifier_key(origin.clone(), new_comm),
			Error::<Test>::NotRegistered
		);
	});
}

#[test]
fn validate_username_variants() {
	new_test_ext().execute_with(|| {
		// Valid lite
		assert_ok!(Resources::validate_username(&username::<Test>(b"abcdefg.12"), false));
		assert_ok!(Resources::validate_username(&username::<Test>(b"userlongname.12345"), false));

		// Invalid lite

		// Too short base
		assert!(matches!(
			Resources::validate_username(&username::<Test>(b"abc.12"), false),
			Err(Error::<Test>::InvalidUsername)
		));
		// Too few digits
		assert!(matches!(
			Resources::validate_username(&username::<Test>(b"abcdefg.1"), false),
			Err(Error::<Test>::InvalidUsername)
		));
		// No digits
		assert!(matches!(
			Resources::validate_username(&username::<Test>(b"abcdefg."), false),
			Err(Error::<Test>::InvalidUsername)
		));
		// No separator/digits
		assert!(matches!(
			Resources::validate_username(&username::<Test>(b"abcdefg"), false),
			Err(Error::<Test>::InvalidUsername)
		));
		// Uppercase base
		assert!(matches!(
			Resources::validate_username(&username::<Test>(b"abcDefg.12"), false),
			Err(Error::<Test>::InvalidUsername)
		));
		// Non-digit suffix
		assert!(matches!(
			Resources::validate_username(&username::<Test>(b"abcdefg.1a"), false),
			Err(Error::<Test>::InvalidUsername)
		));
		// Underscore
		assert!(matches!(
			Resources::validate_username(&username::<Test>(b"abc_defg.12"), false),
			Err(Error::<Test>::InvalidUsername)
		));

		// Valid person
		assert_ok!(Resources::validate_username(&username::<Test>(b"abcdefg"), true)); // Meets min length if MinUsernameLength is <= 7

		// Invalid person

		// Too short
		assert!(matches!(
			Resources::validate_username(&username::<Test>(b"abc"), true),
			Err(Error::<Test>::InvalidUsername)
		));
		// Uppercase
		assert!(matches!(
			Resources::validate_username(&username::<Test>(b"Abcdefg"), true),
			Err(Error::<Test>::InvalidUsername)
		));
		// Hyphen
		assert!(matches!(
			Resources::validate_username(&username::<Test>(b"abc-defg"), true),
			Err(Error::<Test>::InvalidUsername)
		));
		// Dot separator not allowed
		assert!(matches!(
			Resources::validate_username(&username::<Test>(b"abcdefg.12"), true),
			Err(Error::<Test>::InvalidUsername)
		));
	});
}

// --- Statement Validation Tests ---

#[test]
fn validate_statement_success_for_consumer() {
	new_test_ext().execute_with(|| {
		let pair = ed25519::Pair::generate_with_phrase(None).0;

		let consumer_account: AccountId32 = pair.public().into();
		// Register the consumer
		assert_ok!(Resources::register_lite_person(
			RuntimeOrigin::from(OriginCaller::PeopleLite(pallet_people_lite::Origin::LitePerson(
				consumer_account.clone()
			))),
			comm_id(b"key"),
			username::<Test>(b"consumer.12"),
			None
		));

		// Mock a signed statement from this consumer
		let mut statement = Statement::new();
		statement.set_plain_data(b"deadbeef".to_vec());
		statement.sign_ed25519_private(&pair);

		// Validate the statement
		// Don't provide the account in order to test the signature verification
		let result = Resources::validate_statement_with_reason_and_account(
			StatementSource::Local,
			statement,
			None,
		);
		assert_ok!(result.clone());
		assert_eq!(result.unwrap(), <Test as Config>::LitePersonStatementLimit::get());
	});
}

#[test]
fn validate_statement_fails_for_non_consumer() {
	new_test_ext().execute_with(|| {
		// Mock a signed statement from a non-consumer, some random account
		let pair = ed25519::Pair::generate_with_phrase(None).0;
		let mut statement = Statement::new();
		statement.sign_ed25519_private(&pair);

		// Validate the statement
		let result = Resources::validate_statement_with_reason_and_account(
			StatementSource::Local,
			statement,
			None,
		);
		assert_eq!(result, Err(InvalidStatementReason::NotConsumer));
	});
}

#[test]
fn validate_statement_fails_for_unsigned() {
	new_test_ext().execute_with(|| {
		let statement = Statement::new();

		// Validate the statement
		let result = Resources::validate_statement_with_reason_and_account(
			StatementSource::Local,
			statement,
			None,
		);
		assert_eq!(result, Err(InvalidStatementReason::StatementIsNotSigned));
	});
}

#[test]
fn validate_statement_fails_for_invalid_signature() {
	new_test_ext().execute_with(|| {
		let consumer_account = id_to_account(1);

		// Register the consumer
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			comm_id(b"key"),
			username::<Test>(b"consumer.12"),
			None
		));

		// Generate a new random key pair
		let pair = ed25519::Pair::generate_with_phrase(None).0;
		// Mock a statement signed by someone else
		let mut statement = Statement::new();
		statement.sign_ed25519_private(&pair);

		// Try to validate as if it was signed by consumer_account
		// Manually passing the verified account to simulate failed signature verification
		// Normally, statement.verify_signature() would return Err or Valid(other_account)
		// Here we test the case where verify_signature itself fails internally
		// A more direct test:
		let invalid_sig_statement = Statement::new_with_proof(Proof::Sr25519 {
			signature: [0; 64],
			signer: sp_statement_store::AccountId::from(consumer_account),
		});

		let result = Resources::validate_statement_with_reason_and_account(
			StatementSource::Local,
			invalid_sig_statement,
			None, // Let the function try to verify
		);
		assert_eq!(result, Err(InvalidStatementReason::InvalidSignature));
	});
}

#[test]
fn validate_statement_uses_verified_account_if_provided() {
	new_test_ext().execute_with(|| {
		let consumer_account = id_to_account(1);
		// Register the consumer
		assert_ok!(Resources::register_lite_person(
			lite_person_origin(1),
			comm_id(b"key"),
			username::<Test>(b"consumer.12"),
			None
		));

		// Mock a statement (signature doesn't matter here as we provide the account)
		let statement = Statement::new();

		// Validate, providing the already verified account
		let result = Resources::validate_statement_with_reason_and_account(
			StatementSource::Local,
			statement.clone(),
			Some(consumer_account.clone()),
		);
		assert_ok!(result.clone());
		assert_eq!(result.unwrap(), <Test as Config>::LitePersonStatementLimit::get());

		// Validate with a non-consumer account provided
		let non_consumer_account = id_to_account(99);
		let result_non_consumer = Resources::validate_statement_with_reason_and_account(
			StatementSource::Local,
			statement,
			Some(non_consumer_account),
		);
		assert_eq!(result_non_consumer, Err(InvalidStatementReason::NotConsumer));
	});
}

#[test]
fn test_username_basic() {
	Resources::validate_username(&b"abcdefg.12".to_vec().try_into().unwrap(), false).unwrap();
	Resources::validate_username(&b"abcdefg.1".to_vec().try_into().unwrap(), false).unwrap_err();
	Resources::validate_username(&b"abcdef.12".to_vec().try_into().unwrap(), false).unwrap_err();
	Resources::validate_username(&b"abcdef1.12".to_vec().try_into().unwrap(), false).unwrap_err();
	Resources::validate_username(&b"abcdefg.a2".to_vec().try_into().unwrap(), false).unwrap_err();
	Resources::validate_username(&b"abcdefgh12".to_vec().try_into().unwrap(), false).unwrap_err();
	Resources::validate_username(&b"abcdefghij".to_vec().try_into().unwrap(), false).unwrap_err();
}
