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

//! Resources types

use super::*;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::fmt::Debug;
use frame_support::BoundedVec;
use scale_info::TypeInfo;

pub type SignatureOf<T> = <<T as Config>::Crypto as GenerateVerifiable>::Signature;
pub type ConsumerInfoOf<T> = ConsumerInfo<Username<T>, Credibility>;
pub type UsernameReservationOf<T> = UsernameReservation<<T as frame_system::Config>::AccountId>;
pub type PersonalUsernameChoiceOf<T> = PersonalUsernameChoice<Username<T>>;

/// A byte vec used to represent a username.
pub type Username<T> = BoundedVec<u8, <T as Config>::MaxUsernameLength>;

/// The information related to a particular consumer.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct ConsumerInfo<Username, Credibility> {
	/// An opaque key type which will be used in E2E encrypted communication between consumers.
	pub identifier_key: CommunicationIdentifier,
	/// The username associated with the consumer if they are a full person.
	pub full_username: Option<Username>,
	/// The username associated with this consumer's lite person identity.
	pub lite_username: Username,
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
pub enum PersonalUsernameChoice<Username> {
	/// Use a new username.
	Standalone(Username),
	/// Use the reserved username of the submitter.
	Reservation(Username),
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
