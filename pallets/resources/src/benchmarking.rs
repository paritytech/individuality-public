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

//! People lite pallet benchmarks

#![allow(unused)]

use super::*;
use crate::Pallet as Resources;
use frame_benchmarking::v2::{benchmarks, *};
use frame_support::{
	dispatch::{DispatchInfo, PostDispatchInfo},
	pallet_prelude::ValidateUnsigned,
	traits::{fungible::InspectHold, UnfilteredDispatchable},
	BoundedVec,
};
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin as SystemOrigin};
use sp_core::Get;
use sp_runtime::traits::{AsTransactionAuthorizedOrigin, DispatchTransaction, Dispatchable, Zero};

#[benchmarks]
mod benches {
	use super::*;

	#[benchmark]
	fn register_lite_person() -> Result<(), BenchmarkError> {
		// TODO TODO

		#[block]
		{}

		Ok(())
	}

	#[benchmark]
	fn register_person() -> Result<(), BenchmarkError> {
		// TODO TODO

		#[block]
		{}

		Ok(())
	}

	#[benchmark]
	fn touch_person_authorization() -> Result<(), BenchmarkError> {
		// TODO TODO

		#[block]
		{}

		Ok(())
	}

	#[benchmark]
	fn remove_expired_username_reservation() -> Result<(), BenchmarkError> {
		// TODO TODO

		#[block]
		{}

		Ok(())
	}

	#[benchmark]
	fn update_identifier_key() -> Result<(), BenchmarkError> {
		// TODO TODO

		#[block]
		{}

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
