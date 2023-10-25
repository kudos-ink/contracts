#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod asset_reward {
    pub type HashValue = [u8; 32];

    /// A Workflow is represented by:
    /// - the public address of the account transferring the reward.
    /// - the hash of the workflow file.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout, scale_info::TypeInfo))]
    pub struct Workflow {
        account: AccountId,
        hash: HashValue
    }

    #[ink(storage)]
    pub struct AssetReward {
        // The registered workflow.
	    workflow: Workflow,

        // The contribution reward amount
	    reward: Balance,
    }

    impl AssetReward {
        /// Constructor that initializes an asset reward for a given workflow
        #[ink(constructor)]
        pub fn new(workflow: Workflow, reward: Balance) -> Self {
            Self {
                workflow,
                reward
          }
        }
        /// Simply returns the current workflow.
        #[ink(message)]
        pub fn get_reward(&self) -> Balance {
            self.reward
        }
    }

    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        fn default_accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<Environment>()
        }

        /// We test if the constructor does its job.
        #[ink::test]
        fn new_works() {
            let default_accounts = default_accounts();
            let asset_reward = AssetReward::new(Workflow { account: default_accounts.alice, hash: [0; 32] }, 1u128);
            assert_eq!(asset_reward.get_reward(), 1u128);
        }
    }
}
