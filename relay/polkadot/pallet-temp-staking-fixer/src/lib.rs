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

//! # Temporary pallet to fix staking ledgers.
//!
//! > TODO

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Currency;
use frame_system::ensure_signed;
use sp_staking::StakingAccount;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::{pallet_prelude::*, RawOrigin};

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_staking::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// TODO
		Restored { stash: T::AccountId },
		RestoredUnstaked { stash: T::AccountId },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn restore_ledger_temp(
			origin: OriginFor<T>,
			stash: T::AccountId,
			maybe_slashing_spans: Option<u32>,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			// calls `Staking::restore_ledger` as `Root`.
			pallet_staking::Pallet::<T>::restore_ledger(
				RawOrigin::Root.into(),
				stash.clone(),
				None,
				None,
				None,
			)?;

			let ledger = pallet_staking::Pallet::<T>::ledger(StakingAccount::Stash(stash.clone()))?;

			// check if stash's free balance covers the current ledger's total amount. If not,
			// force unstake the ledger.
			if ledger.total > T::Currency::free_balance(&stash) {
				pallet_staking::Pallet::<T>::force_unstake(
					RawOrigin::Root.into(),
					stash.clone(),
					maybe_slashing_spans.unwrap_or_default(),
				)?;

				Self::deposit_event(Event::<T>::RestoredUnstaked { stash });
				Ok(())
			} else {
				Self::deposit_event(Event::<T>::Restored { stash });
				Ok(())
			}
		}
	}
}
