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
use sp_runtime::{testing::UintAuthorityId, transaction_validity::InvalidTransaction};

/// Test that a non-restricted origin (`NON_RESTRICTED_ORIGIN`) is never tracked, i.e., no usage.
#[test]
fn non_restricted_origin_is_not_charged() {
	new_test_ext().execute_with(|| {
		advance_by(1);

		assert_ok!(exec_signed_tx(NON_RESTRICTED_ORIGIN, MockPalletCall::do_something {}));

		assert!(
			Usages::<Test>::iter().next().is_none(),
			"Non-restricted origin should have no tracked usage."
		);
	});
}

/// Test that restricted origins (`RESTRICTED_ORIGIN_1`, `RESTRICTED_ORIGIN_2`) have their usage
/// tracked, refunded on Pays::No, and also can exceed the limit one time if the call is
/// whitelisted.
#[test]
fn restricted_origin_works() {
	new_test_ext().execute_with(|| {
		// length of the extrinsic.
		let len = {
			let tx_ext = (RestrictOrigin::<Test>::new(true),);
			let tx = UncheckedExtrinsic::new_signed(
				MockPalletCall::do_something {}.into(),
				RESTRICTED_ORIGIN_1,
				UintAuthorityId(RESTRICTED_ORIGIN_1),
				tx_ext,
			);
			tx.encoded_size() as u64
		};

		let mut previous_used = 0;

		assert_eq!(ALLOWANCE_RECOVERY_PER_BLOCK, 5);
		assert_eq!(CALL_WEIGHT, 15);
		assert_eq!(MAX_ALLOWANCE, 100);

		// Move beyond block 0 for events
		advance_by(1);

		// Normal call => usage increases
		assert_ok!(exec_signed_tx(RESTRICTED_ORIGIN_1, MockPalletCall::do_something {}));
		let usage = Usages::<Test>::get(RuntimeRestrictedEntity::A).unwrap();
		assert_eq!(usage.used, previous_used + CALL_WEIGHT + len);
		assert_eq!(usage.at_block, 1);

		// A call with `Pays::No` => usage is refunded
		previous_used = Usages::<Test>::get(RuntimeRestrictedEntity::A).unwrap().used;
		assert_ok!(exec_signed_tx(RESTRICTED_ORIGIN_1, MockPalletCall::do_something_refunded {}));
		let usage_after = Usages::<Test>::get(RuntimeRestrictedEntity::A).unwrap();
		assert_eq!(usage_after.used, previous_used);

		// Again a normal call => usage increases
		assert_ok!(exec_signed_tx(RESTRICTED_ORIGIN_1, MockPalletCall::do_something {}));
		let usage = Usages::<Test>::get(RuntimeRestrictedEntity::A).unwrap();
		assert_eq!(usage.used, previous_used + CALL_WEIGHT + len);

		// Now we have reached the limit
		// Normal calls that push usage above the max should fail.
		assert_noop!(
			exec_signed_tx(RESTRICTED_ORIGIN_1, MockPalletCall::do_something {}),
			InvalidTransaction::Payment
		);

		// Advance a few blocks to partially recover usage
		advance_by(1);
		// Still not enough to do another normal call if we haven't recovered enough.
		assert_noop!(
			exec_signed_tx(RESTRICTED_ORIGIN_1, MockPalletCall::do_something {}),
			InvalidTransaction::Payment
		);

		// Advance one more block => total 5 blocks.
		advance_by(1);
		previous_used = Usages::<Test>::get(RuntimeRestrictedEntity::A).unwrap().used;
		assert_ok!(exec_signed_tx(RESTRICTED_ORIGIN_1, MockPalletCall::do_something {}));
		let current_usage = Usages::<Test>::get(RuntimeRestrictedEntity::A).unwrap();
		let recovered_amount = 2 * ALLOWANCE_RECOVERY_PER_BLOCK;

		// Usage = (previous_used - recovered_amount) + (CALL_WEIGHT + len).
		assert_eq!(current_usage.used, previous_used + CALL_WEIGHT + len - recovered_amount);
		assert_eq!(current_usage.at_block, 3);
	});
}

#[test]
fn one_time_excess_works_and_works_only_one_time() {
	new_test_ext().execute_with(|| {
		advance_by(1);

		// Given usage is 0, RESTRICTED_ORIGIN_1 can exceed once.
		assert_ok!(exec_signed_tx(
			RESTRICTED_ORIGIN_1,
			MockPalletCall::do_something_allowed_excess {}
		));
		let current_usage = Usages::<Test>::get(RuntimeRestrictedEntity::A).unwrap();
		assert!(current_usage.used > MAX_ALLOWANCE);

		// Now that usage has exceeded the max, even the "allowed excess" call should fail.
		assert_noop!(
			exec_signed_tx(RESTRICTED_ORIGIN_1, MockPalletCall::do_something_allowed_excess {}),
			InvalidTransaction::Payment
		);
	});
}

#[test]
fn one_time_excess_is_origin_specific() {
	new_test_ext().execute_with(|| {
		advance_by(1);

		// try the "allowed_excess" call from RESTRICTED_ORIGIN_2:
		// It should *fail*, because RESTRICTED_ORIGIN_2 is not in `OperationAllowedOneTimeExcess`.
		assert_noop!(
			exec_signed_tx(RESTRICTED_ORIGIN_2, MockPalletCall::do_something_allowed_excess {}),
			InvalidTransaction::Payment
		);

		// Demonstrate that RESTRICTED_ORIGIN_1 *can* exceed (for completeness).
		assert_ok!(exec_signed_tx(
			RESTRICTED_ORIGIN_1,
			MockPalletCall::do_something_allowed_excess {}
		));
		let current_usage = Usages::<Test>::get(RuntimeRestrictedEntity::A).unwrap();
		assert!(current_usage.used > MAX_ALLOWANCE);
	});
}

#[test]
fn one_time_excess_requires_usage_zero() {
	new_test_ext().execute_with(|| {
		advance_by(1);

		// We use a bit of the allowance.
		assert_ok!(exec_signed_tx(RESTRICTED_ORIGIN_1, MockPalletCall::do_something {}));
		let usage = Usages::<Test>::get(RuntimeRestrictedEntity::A).unwrap();
		assert!(usage.used < MAX_ALLOWANCE);
		assert!(usage.used > 0);

		// Now that usage is non-zero, we can call exceeding operations.
		assert_noop!(
			exec_signed_tx(RESTRICTED_ORIGIN_1, MockPalletCall::do_something_allowed_excess {}),
			InvalidTransaction::Payment
		);
	});
}

// TODO: Gui: a test to assert the behavior of the extension when disabled.
// TODO: Gui: a test for `clean_usage` call.
