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

//! Resources.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod types;
pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;
pub use weights::WeightInfo;

use core::time::Duration;
use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	traits::{EnsureOriginWithArg, UnixTime},
};
use indiv_support::traits::{Alias, CommunicationIdentifier, ConsumerRegistrar, Context, Username};
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_statement_store::{
	runtime_api::{InvalidStatement, StatementSource, ValidStatement},
	Proof,
	SignatureVerificationResult::Valid,
	Statement,
};
use types::{
	ConsumerInfo, Credibility, InvalidStatementReason, PersonalUsernameChoice, UsernameReservation,
	UsernameReservationOf,
};
use verifiable::GenerateVerifiable;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	pub const RESOURCES_CONTEXT: Context = *b"pop:polkadot.network/resources  ";

	const LOG_TARGET: &str = "runtime::pallet-resources";
	const MIN_LITE_USERNAME_DIGITS: usize = 2;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config<
		AccountId: From<sp_statement_store::AccountId> + Into<sp_statement_store::AccountId>,
	>
	{
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// Trait allowing cryptographic proof of membership without exposing the underlying member.
		/// Normally a Ring-VRF.
		type Crypto: GenerateVerifiable<
			Proof: Send + Sync + DecodeWithMemTracking,
			Signature: Send + Sync + DecodeWithMemTracking,
			Member: DecodeWithMemTracking,
		>;

		/// The maximum length of a username, including any potential trailing digits.
		#[pallet::constant]
		type MaxUsernameLength: Get<u32>;

		/// The minimum length of a username.
		#[pallet::constant]
		type MinUsernameLength: Get<u32>;

		/// The duration of time, in seconds, for which a person's authorization is valid. After
		/// this period elapses, people will no longer be considered active, but their resource
		/// allowances should default to the same values used for lite people.
		#[pallet::constant]
		type PersonAuthDuration: Get<u32>;

		/// The minimum interval of time, in seconds, which must pass before updating a person's
		/// authorization.
		#[pallet::constant]
		type MinPersonAuthUpdateInterval: Get<u32>;

		/// How to recognise an origin representing a person.
		type EnsurePerson: EnsureOriginWithArg<OriginFor<Self>, Context, Success = Alias>;

		/// How to recognise an origin representing a lite person.
		type EnsureLitePerson: EnsureOrigin<OriginFor<Self>, Success = Self::AccountId>;

		/// The source of time.
		type Clock: UnixTime;

		/// Signature type for ensuring ownership of provided accounts in case of registrations
		/// through alias.
		type OffchainSignature: Verify<Signer: IdentifyAccount<AccountId = Self::AccountId>>
			+ Parameter;

		/// The amount of time for which a username reservation is valid, in seconds. After this
		/// time period elapses, the reservation can be voided.
		type UsernameReservationDuration: Get<u64>;

		/// The limit for the statement store usage for lite people.
		type LitePersonStatementLimit: Get<ValidStatement>;

		/// Benchmark helper trait.
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: benchmarking::BenchmarkHelper<Self>;
	}

	/// Accounts used to identify consumers mapped to their consumer information.
	#[pallet::storage]
	pub type Consumers<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, ConsumerInfo>;

	/// Reverse lookup from `username` to the `AccountId` that has registered it. The `owner` value
	/// should be a key in the `Consumers` map. There can be at most 2 usernames pointing to the
	/// same `owner`:
	/// - username associated with a consumer's lite person identity - this will always be present;
	/// - optionally another username associated with the consumer's full person identity, if
	///   applicable.
	#[pallet::storage]
	pub type UsernameOwnerOf<T: Config> =
		StorageMap<_, Blake2_128Concat, Username, T::AccountId, OptionQuery>;

	/// Reverse lookup from a reserved `username` to the `AccountId` that has registered it along
	/// with the timestamp when it happened. Old reservations can be removed from storage.
	#[pallet::storage]
	pub type ReservedUsernames<T: Config> =
		StorageMap<_, Blake2_128Concat, Username, UsernameReservationOf<T>, OptionQuery>;

	/// Reverse lookup from registered aliases to the `AccountId` used to register as a consumer.
	#[pallet::storage]
	pub type AccountOfAlias<T: Config> =
		StorageMap<_, Blake2_128Concat, Alias, T::AccountId, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A person has registered as a consumer.
		PersonRegistered { alias: Alias, account: T::AccountId },
		/// A lite person has registered as a consumer.
		LitePersonRegistered { account: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Username does not fit the requirements.
		InvalidUsername,
		/// Username is already taken.
		UsernameTaken,
		/// Consumer is already registered.
		AlreadyRegistered,
		/// Provided proof of ownership is invalid.
		InvalidProofOfOwnership,
		/// Person is not registered as a consumer.
		NotRegistered,
		/// Consumer is not a full person.
		NotFullPerson,
		/// Attempted to update person authorization too early.
		TouchNotReady,
		/// Reservation is not active.
		NoReservation,
		/// The signature for the reserved username is invalid.
		InvalidUsernameSignature,
		/// The username in the reservation request is already taken.
		UsernameReservationTaken,
		/// The reservation has not expired.
		ReservationFresh,
		/// There is no lite consumer to be linked.
		NoLinkedIdentity,
		/// The lite consumer is already linked to a full person consumer.
		AlreadyLinked,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register a lite person as a consumer.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::register_lite_person())]
		pub fn register_lite_person(
			origin: OriginFor<T>,
			identifier_key: CommunicationIdentifier,
			username: Username,
			reserved_username: Option<Username>,
		) -> DispatchResultWithPostInfo {
			// Ensure this is a lite person.
			let lite_person_account = T::EnsureLitePerson::ensure_origin(origin)?;
			Self::register_lite_consumer_inner(
				lite_person_account,
				identifier_key,
				username,
				reserved_username,
			)?;
			Ok(Pays::No.into())
		}

		/// Register a proven person as a consumer.
		///
		///	The person must link a previously recognized lite identity, which will be upgraded to a
		/// full person consumer. In order to prove they hold the lite identity they want to link,
		/// users must provide a `lite_identity_proof` signature, created by signing the alias bytes
		/// using their lite consumer account.
		///
		/// The consumer can choose if they want to have a new username or use an existing
		/// reservation made in the name of the lite consumer who will be linked.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::register_person())]
		pub fn register_person(
			origin: OriginFor<T>,
			linked_lite_identity: T::AccountId,
			lite_identity_proof: T::OffchainSignature,
			username_choice: PersonalUsernameChoice,
		) -> DispatchResultWithPostInfo {
			// Ensure this is a person.
			let alias = T::EnsurePerson::ensure_origin(origin, &RESOURCES_CONTEXT)?;
			// And they're not already registered.
			ensure!(!AccountOfAlias::<T>::contains_key(alias), Error::<T>::AlreadyRegistered);
			let username = match username_choice {
				PersonalUsernameChoice::Standalone(username) => {
					// Validate the username, including that it is not already taken.
					ensure!(
						!UsernameOwnerOf::<T>::contains_key(&username),
						Error::<T>::UsernameTaken
					);
					ensure!(
						!ReservedUsernames::<T>::contains_key(&username),
						Error::<T>::UsernameTaken
					);
					Self::validate_username(&username, true)?;
					username
				},
				PersonalUsernameChoice::Reservation(reserved) => {
					let reservation =
						ReservedUsernames::<T>::take(&reserved).ok_or(Error::<T>::NoReservation)?;
					// Check that the lite person which will be linked with this full person was the
					// one who reserved this username.
					ensure!(
						reservation.owner == linked_lite_identity,
						Error::<T>::InvalidUsernameSignature
					);
					// As the username is already reserved, there cannot be another person holding
					// this username and it must be valid.
					reserved
				},
			};

			// Verify proof of ownership of the linked lite person.
			ensure!(
				lite_identity_proof.verify(&alias[..], &linked_lite_identity),
				Error::<T>::InvalidProofOfOwnership,
			);
			// Ensure the linked lite person was not already linked to another full person.
			let mut linked_consumer_info =
				Consumers::<T>::get(&linked_lite_identity).ok_or(Error::<T>::NoLinkedIdentity)?;
			ensure!(linked_consumer_info.full_username.is_none(), Error::<T>::AlreadyLinked);
			// Update the linked lite consumer's record with the full person username.
			linked_consumer_info.full_username = Some(username.clone());
			// Update the linked lite consumer's record with the full person credibility. From this
			// moment onward, this consumer will be registered as a full person through this
			// upgrade.
			let now = T::Clock::now().as_secs();
			linked_consumer_info.credibility = Credibility::Person { alias, last_update: now };

			// Add the username to the list.
			UsernameOwnerOf::<T>::insert(&username, &linked_lite_identity);
			// Mark the alias as used.
			AccountOfAlias::<T>::insert(alias, &linked_lite_identity);

			// Set the consumer's record.
			Consumers::<T>::insert(&linked_lite_identity, linked_consumer_info);

			Self::deposit_event(Event::PersonRegistered { alias, account: linked_lite_identity });
			Ok(Pays::No.into())
		}

		/// Update a person's authorization by ensuring they can still authenticate as people.
		///
		/// This call must be performed at least `MinPersonAuthUpdateInterval` seconds after the
		/// last update in order to prevent spam.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::touch_person_authorization())]
		pub fn touch_person_authorization(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			// Ensure this is a person.
			let alias = T::EnsurePerson::ensure_origin(origin, &RESOURCES_CONTEXT)?;
			let account = AccountOfAlias::<T>::get(alias).ok_or(Error::<T>::NotRegistered)?;
			let consumer_info = Consumers::<T>::get(&account).ok_or(Error::<T>::NotRegistered)?;
			let Credibility::Person { last_update, .. } = consumer_info.credibility else {
				return Err(Error::<T>::NotFullPerson.into());
			};
			// Ensure the authorization is old enough to be touched.
			let now = T::Clock::now().as_secs();
			ensure!(
				now > last_update.saturating_add(T::MinPersonAuthUpdateInterval::get() as u64),
				Error::<T>::TouchNotReady
			);
			// Set the consumer's updated record.
			Consumers::<T>::insert(
				&account,
				ConsumerInfo {
					credibility: Credibility::Person { alias, last_update: now },
					..consumer_info
				},
			);

			Ok(Pays::No.into())
		}

		/// Remove a username reservation which expired past the `UsernameReservationDuration`.
		///
		/// The origin must be a lite person but the weight is refunded if the call is successful.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_expired_username_reservation())]
		pub fn remove_expired_username_reservation(
			origin: OriginFor<T>,
			username: Username,
		) -> DispatchResultWithPostInfo {
			let _ = T::EnsureLitePerson::ensure_origin(origin)?;
			let reservation =
				ReservedUsernames::<T>::take(&username).ok_or(Error::<T>::NoReservation)?;
			let now = T::Clock::now();
			ensure!(
				now > Duration::from_secs(reservation.since)
					.saturating_add(Duration::from_secs(T::UsernameReservationDuration::get())),
				Error::<T>::ReservationFresh
			);
			Ok(Pays::No.into())
		}

		/// Update the communication identifier key of a consumer.
		///
		/// The origin must be the account registered for that consumer, regardless of their
		/// credibility.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_expired_username_reservation())]
		pub fn update_identifier_key(
			origin: OriginFor<T>,
			identifier_key: CommunicationIdentifier,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut consumer_info = Consumers::<T>::get(&who).ok_or(Error::<T>::NotRegistered)?;
			consumer_info.identifier_key = identifier_key;
			Consumers::<T>::insert(who, consumer_info);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Ensure a username is valid depending on the owner's credibility.
		pub fn validate_username(username: &Username, person: bool) -> Result<(), Error<T>> {
			// Ensure the username is available.
			if person {
				// People can choose any username of minimum length `MinUsernameLength`, as long as
				// it's lowercase alphanumeric.
				ensure!(
					username.len() >= T::MinUsernameLength::get() as usize,
					Error::<T>::InvalidUsername
				);
				ensure!(
					username.iter().all(|byte| byte.is_ascii_lowercase()),
					Error::<T>::InvalidUsername
				);
			} else {
				// Usernames for lite people must follow a pattern of at least `MinUsernameLength`
				// lowercase letters, followed by at least `MIN_LITE_USERNAME_DIGITS` digits.
				let separator_index = username
					.iter()
					.position(|byte| *byte == b'.')
					.ok_or(Error::<T>::InvalidUsername)?;
				// Letter part must be at least `MinUsernameLength` characters long.
				ensure!(
					separator_index as u32 >= T::MinUsernameLength::get(),
					Error::<T>::InvalidUsername
				);
				// Digits part must be at least `MIN_LITE_USERNAME_DIGITS` digits long.
				ensure!(
					username.len() - separator_index > MIN_LITE_USERNAME_DIGITS,
					Error::<T>::InvalidUsername
				);
				// First segment must be all ASCII lowercase letters.
				ensure!(
					username[..(separator_index)].iter().all(|byte| byte.is_ascii_lowercase()),
					Error::<T>::InvalidUsername
				);
				// Second segment must be all ASCII digits.
				// Slice access is safe because the length of the digit part was checked above.
				ensure!(
					username[(1 + separator_index)..].iter().all(|byte| byte.is_ascii_digit()),
					Error::<T>::InvalidUsername
				);
			}
			Ok(())
		}

		/// This function will return a `LitePersonStatementLimit` statement limit for all
		/// consumers. This statement authorization does not work for any accounts which are not
		/// consumers. The statements must be signed.
		pub fn validate_statement(
			source: StatementSource,
			statement: Statement,
		) -> Result<ValidStatement, InvalidStatement> {
			log::trace!(
				target: LOG_TARGET,
				"Statement validation: Validating statement {:?}",
				statement,
			);

			Self::validate_statement_with_reason_and_account(source, statement, None).map_err(|e| {
				use InvalidStatementReason::*;
				match e {
					NotConsumer => log::trace!(
						target: LOG_TARGET,
						"Statement validation failed: not a consumer."
					),
					StatementIsNotSigned => log::trace!(
						target: LOG_TARGET,
						"Statement validation failed: statement is not signed."
					),
					InvalidSignature => log::trace!(
						target: LOG_TARGET,
						"Statement validation failed: statement signature is invalid."
					),
				}
				InvalidStatement::BadProof
			})
		}

		pub fn validate_statement_with_reason_and_account(
			_source: StatementSource,
			statement: Statement,
			maybe_verified_account: Option<T::AccountId>,
		) -> Result<ValidStatement, InvalidStatementReason> {
			let account = if let Some(account) = maybe_verified_account {
				account
			} else {
				// Only accept signed statements.
				match statement.proof() {
					Some(Proof::Ed25519 { .. }) |
					Some(Proof::Sr25519 { .. }) |
					Some(Proof::Secp256k1Ecdsa { .. }) => (),
					Some(Proof::OnChain { .. }) | None => {
						return Err(InvalidStatementReason::StatementIsNotSigned);
					},
				}

				let Valid(account) = statement.verify_signature() else {
					return Err(InvalidStatementReason::InvalidSignature);
				};

				account.into()
			};

			// For now, we allow all consumers, lite people and full people, to use the same
			// statement limit.
			let _consumer_info =
				Consumers::<T>::get(account).ok_or(InvalidStatementReason::NotConsumer)?;
			let limit = T::LitePersonStatementLimit::get();
			Ok(limit)
		}

		/// Register a lite consumer using the provided information.
		///
		/// IMPORTANT
		///
		/// This function does not check for authorization. The caller is responsible for ensuring
		/// the `account` to be registered is a lite person and that the user's consent was
		/// provided, usually through a signature verified by the caller.
		pub fn register_lite_consumer_inner(
			account: T::AccountId,
			identifier_key: CommunicationIdentifier,
			username: Username,
			reserved_username: Option<Username>,
		) -> Result<(), Error<T>> {
			// Must not already be registered.
			ensure!(!Consumers::<T>::contains_key(&account), Error::<T>::AlreadyRegistered);
			// Validate the username, including that it is not already taken.
			ensure!(!UsernameOwnerOf::<T>::contains_key(&username), Error::<T>::UsernameTaken);
			ensure!(!ReservedUsernames::<T>::contains_key(&username), Error::<T>::UsernameTaken);
			Self::validate_username(&username, false)?;
			if let Some(reserved_username) = reserved_username {
				ensure!(reserved_username != username, Error::<T>::InvalidUsername);
				ensure!(
					!UsernameOwnerOf::<T>::contains_key(&reserved_username),
					Error::<T>::UsernameReservationTaken
				);
				ensure!(
					!ReservedUsernames::<T>::contains_key(&reserved_username),
					Error::<T>::UsernameReservationTaken
				);
				Self::validate_username(&reserved_username, true)?;
				let reservation = UsernameReservation {
					owner: account.clone(),
					since: T::Clock::now().as_secs(),
				};
				ReservedUsernames::<T>::insert(reserved_username, reservation);
			}
			// Add the username to the list.
			UsernameOwnerOf::<T>::insert(&username, &account);
			// Set the consumer's record.
			Consumers::<T>::insert(
				&account,
				ConsumerInfo {
					identifier_key,
					full_username: None,
					lite_username: username,
					credibility: Credibility::Lite,
				},
			);
			frame_system::Pallet::<T>::inc_sufficients(&account);

			Self::deposit_event(Event::LitePersonRegistered { account });
			Ok(())
		}
	}

	impl<T: Config> ConsumerRegistrar<T::AccountId> for Pallet<T> {
		type Error = Error<T>;

		fn register_lite_consumer(
			account: T::AccountId,
			identifier_key: CommunicationIdentifier,
			username: Username,
			reserved_username: Option<Username>,
		) -> Result<(), Error<T>> {
			Self::register_lite_consumer_inner(account, identifier_key, username, reserved_username)
		}
	}
}
