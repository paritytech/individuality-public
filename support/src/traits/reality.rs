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

//! Traits concerned with modelling reality.


use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::Parameter;
use scale_info::TypeInfo;
use sp_runtime::{DispatchError, DispatchResult, RuntimeDebug};

/// Identity of personhood.
///
/// This is a persistent identifier for every individual. Regardless of what
/// else the individual changes within the system (such as identity documents, cryptographic keys,
/// etc...) this does not change. As such, it should never be used in application code.
pub type PersonalId = u64;

/// Identifier for a specific application in which we may wish to track individual people.
///
/// NOTE: This MUST remain equivalent to the type `Context` in the crate `verifiable`.
pub type Context = [u8; 32];

/// Identifier for a specific individual within an application context.
///
/// NOTE: This MUST remain equivalent to the type `Alias` in the crate `verifiable`.
pub type Alias = [u8; 32];

/// The type we use to identify different rings.
pub type RingIndex = u32;

/// The ring index 0.
pub const RI_ZERO: RingIndex = 0;

/// Identifier used for communication among people, usually a public key of a crypto type allowing
/// for symmetric key generation using public keys.
pub type CommunicationIdentifier = [u8; 65];

#[derive(
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	Encode,
	Decode,
	MaxEncodedLen,
	TypeInfo,
	DecodeWithMemTracking,
)]
pub struct ContextualAlias {
	pub alias: Alias,
	pub context: Context,
}

/// Trait to recognize people and handle personal id.
///
/// `PersonalId` goes through multiple state: free, reserved, used; a used personal id can belong
/// to a recognized person or a suspended person.
pub trait AddOnlyPeopleTrait {
	type Member: Parameter + MaxEncodedLen;
	/// Reserve a new id for a future person. This id is not recognized, not reserved, and has
	/// never been reserved in the past.
	fn reserve_new_id() -> PersonalId;
	/// Renew a reservation for a personal id. The id is not recognized, but has been reserved in
	/// the past.
	///
	/// An error is returned if the id is used or wasn't reserved before.
	fn renew_id_reservation(personal_id: PersonalId) -> Result<(), DispatchError>;
	/// Cancel the reservation for a personal id
	///
	/// An error is returned if the id wasn't reserved in the first place.
	fn cancel_id_reservation(personal_id: PersonalId) -> Result<(), DispatchError>;
	/// Recognized a person.
	///
	/// The personal id must be reserved or the person must have already been recognized and
	/// suspended in the past.
	/// If recognizing a new person, a key must be provided. If resuming the personhood then no key
	/// must be provided.
	///
	/// An error is returned if:
	/// * `maybe_key` is some and the personal id was not reserved or is used by a recognized or
	///   suspended person.
	/// * `maybe_key` is none and the personal id was not recognized before.
	fn recognize_personhood(
		who: PersonalId,
		maybe_key: Option<Self::Member>,
	) -> Result<(), DispatchError>;
	// All stuff for benchmarks.
	#[cfg(feature = "runtime-benchmarks")]
	type Secret;
	#[cfg(feature = "runtime-benchmarks")]
	fn mock_key(who: PersonalId) -> (Self::Member, Self::Secret);
}

/// Trait to recognize and suspend people.
pub trait PeopleTrait: AddOnlyPeopleTrait {
	/// Suspend a set of people. This operation must be called within a mutation session.
	///
	/// An error is returned if:
	/// * a suspended personal id was already suspended.
	/// * a personal id doesn't belong to any person.
	fn suspend_personhood(suspensions: &[PersonalId]) -> DispatchResult;
	/// Return whether the mutation session can be started.
	///
	/// The result of this operation holds until any call to `start_people_set_mutation_session`
	/// and `end_people_set_mutation_session` is made.
	fn can_start_people_set_mutation_session() -> bool;
	/// Start a mutation session for setting people.
	///
	/// An error is returned if the mutation session can be started at the moment. It is expected
	/// to become startable later.
	fn start_people_set_mutation_session() -> DispatchResult;
	/// End a mutation session for setting people.
	///
	/// An error is returned if there is no mutation session ongoing.
	fn end_people_set_mutation_session() -> DispatchResult;
}

impl AddOnlyPeopleTrait for () {
	type Member = ();
	fn reserve_new_id() -> PersonalId {
		0
	}
	fn renew_id_reservation(_: PersonalId) -> Result<(), DispatchError> {
		Ok(())
	}
	fn cancel_id_reservation(_: PersonalId) -> Result<(), DispatchError> {
		Ok(())
	}
	fn recognize_personhood(_: PersonalId, _: Option<Self::Member>) -> Result<(), DispatchError> {
		Ok(())
	}

	#[cfg(feature = "runtime-benchmarks")]
	type Secret = PersonalId;
	#[cfg(feature = "runtime-benchmarks")]
	fn mock_key(who: PersonalId) -> (Self::Member, Self::Secret) {
		((), who)
	}
}

impl PeopleTrait for () {
	fn suspend_personhood(_: &[PersonalId]) -> DispatchResult {
		Ok(())
	}
	fn start_people_set_mutation_session() -> DispatchResult {
		Ok(())
	}
	fn end_people_set_mutation_session() -> DispatchResult {
		Ok(())
	}
	fn can_start_people_set_mutation_session() -> bool {
		true
	}
}

/// Trait to get the total number of active members in a set.
pub trait CountedMembers {
	/// Returns the number of active members in the set.
	fn active_count() -> u32;

	/// Sets the number of active members in the set.
	#[cfg(feature = "runtime-benchmarks")]
	fn set_active_count(count: u32);
}
