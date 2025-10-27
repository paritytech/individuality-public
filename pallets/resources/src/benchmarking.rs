// This file is part of Substrate.

// Copyright (C) 2020-2022 Parity Technologies (UK) Ltd.
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
