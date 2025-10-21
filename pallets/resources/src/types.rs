// This file is part of Substrate.

// Copyright (C) 2017-2022 Parity Technologies (UK) Ltd.
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

//! Resources types

use super::*;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::fmt::Debug;
use frame_support::BoundedVec;
use scale_info::TypeInfo;

pub type SignatureOf<T> = <<T as Config>::Crypto as GenerateVerifiable>::Signature;
pub type ConsumerInfoOf<T> = ConsumerInfo<Username<T>, Credibility>;
pub type UsernameReservationOf<T> = UsernameReservation<<T as frame_system::Config>::AccountId>;
pub type PersonalUsernameChoiceOf<T> =
	PersonalUsernameChoice<Username<T>, <T as Config>::OffchainSignature>;

/// A byte vec used to represent a username.
pub type Username<T> = BoundedVec<u8, <T as Config>::MaxUsernameLength>;

/// The information related to a particular consumer.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct ConsumerInfo<Username, Credibility> {
	/// An opaque key type which will be used in E2E encrypted communication between consumers.
	pub identifier_key: CommunicationIdentifier,
	/// The username associated with this consumer.
	pub username: Username,
	/// The credibility of a consumer.
	pub credibility: Credibility,
}

/// The credibility of a consumer.
#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Debug)]
pub enum Credibility {
	/// Recognized as a lite person.
	Lite,
	/// Recognized as a full person with an alias. Since personhood can be suspended, in order to
	/// ensure fair access to the resources, we record a timestamp of the last interaction with
	/// this consumer using the person authentication.
	Person { alias: Alias, last_update: u64 },
}

/// The information related to a username reservation.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct UsernameReservation<Account> {
	/// The owner of the reserved username.
	pub owner: Account,
	/// The timestamp at which the username was reserved, in seconds since the UNIX epoch.
	pub since: u64,
}

/// The username configuration for a full person's registration.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub enum PersonalUsernameChoice<Username, Signature> {
	/// Use a new username.
	Standalone(Username),
	/// Take a reserved username with the signature of the reservation's submitter.
	Reservation(Username, Signature),
}

/// The reason why a statement is invalid for pallet-resources.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Clone, PartialEq)]
pub enum InvalidStatementReason {
	/// The account is not associated with a consumer.
	NotConsumer,
	/// The statement is not signed, only signed statements are valid.
	StatementIsNotSigned,
	/// The signature of the statement failed to verify.
	InvalidSignature,
}
