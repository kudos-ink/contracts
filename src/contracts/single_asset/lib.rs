#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[openbrush::implementation(Ownable)]
#[openbrush::contract]
pub mod single_asset_reward {
    use kudos_ink::traits::workflow::{*, WorkflowError};
    use openbrush::{contracts::traits::ownable::OwnableError, modifiers, traits::Storage};

    use ink::env::hash::{HashOutput, Sha2x256};
    use ink::storage::Mapping;

    /// A Contribution is represented by:
    /// - a unique id.
    /// - the contributor; allowed to claim the reward.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(ink::storage::traits::StorageLayout, scale_info::TypeInfo)
    )]
    pub struct Contribution {
        id: u64,
        contributor: AccountId,
        is_claimed: bool,
    }

    #[ink(storage)]
    #[derive(Default, Storage)]
    pub struct SingleAssetReward {
        #[storage_field]
        ownable: ownable::Data,

        // The registered workflow.
        workflow: HashValue,

        // The contribution reward amount
        reward: Balance,

        // The approved contribution ids database.
        contributions: Mapping<u64, Contribution>,

        // The registered contributors ids database.
        identities: Mapping<HashValue, AccountId>, // HashValue refers to the contributo id (e.g. github ID)
    }

    impl Workflow for SingleAssetReward {
/// Register an aspiring contributor.
        ///
        /// Constraint(s):
        /// 1. The `identity` id should not already be registered.
        ///
        /// A `Registered` event is emitted.
        #[ink(message)]
        fn register_identity(
            &mut self,
            account: AccountId,
            identity: HashValue,
        ) -> Result<(), WorkflowError> {
            if self.identity_is_known(identity) {
                return Err(WorkflowError::IdentityAlreadyRegistered);
            }

            self.identities.insert(identity, &account);
            Ok(())
        }

        /// Approve contribution. This is triggered by a workflow run.
        ///
        /// Constraint(s):
        /// 1. The `contribution_id` should not already be approved.
        /// 2. The `contributor_identity` must be registered.
        ///
        /// An `Approval` event is emitted.
        #[ink(message)]
        #[modifiers(only_owner)]
        fn approve(
            &mut self,
            contribution_id: u64,
            contributor_identity: HashValue,
        ) -> Result<(), WorkflowError> {
            if self.contribution_is_known(contribution_id) {
                return Err(WorkflowError::ContributionAlreadyApproved);
            }

            let maybe_contributor = self.identities.get(contributor_identity);
            if maybe_contributor.is_none() {
                return Err(WorkflowError::UnknownContributor);
            }

            let contribution = Contribution {
                id: contribution_id,
                contributor: maybe_contributor.unwrap(),
                is_claimed: false,
            };
            self.contributions.insert(contribution_id, &contribution);
            Ok(())
        }

        /// Claim reward for a contributor.
        ///
        /// Constraint(s):
        /// 1. The `contribution_id` must be mapped to an existing approved contribution in `contributions`.
        /// 2. The caller has to be the contributor of the approved contribution.
        /// 3. The claim must be available (marked as false in the claims mapping).
        ///
        /// A `Claim` event is emitted.
        #[ink(message)]
        fn claim(&self, contribution_id: u64) -> Result<bool, WorkflowError> {
            self.claim_reward(contribution_id)
        }
    }

    impl SingleAssetReward {
        /// Constructor that initializes an asset reward for a given workflow
        #[ink(constructor)]
        pub fn new(workflow: HashValue, reward: Balance) -> Self {
            let mut instance = Self::default();
            let caller = instance.env().caller();
            ownable::Internal::_init_with_owner(&mut instance, caller);
            Self {
                workflow,
                reward,
                ..instance
            }
        }

        /// Claim reward for a contributor.
        ///
        /// Constraint(s):
        /// 1. The `contribution_id` must be mapped to an existing approved contribution in `contributions`.
        /// 2. The caller has to be the contributor of the approved contribution.
        /// 3. The claim must be available (marked as false in the claims mapping).
        ///
        /// A `Claim` event is emitted.
        #[ink(message)]
        pub fn claim_reward(&self, contribution_id: u64) -> Result<bool, WorkflowError> {
            if !self.contribution_is_known(contribution_id) {
                return Err(WorkflowError::UnknownContribution);
            }

            let contribution = self.contributions.get(contribution_id).unwrap();
            let contributor = contribution.contributor;
            if Self::env().caller() != contributor {
                return Err(WorkflowError::CallerIsNotContributor);
            }

            if contribution.is_claimed {
                return Err(WorkflowError::AlreadyClaimed);
            }

            match self.env().transfer(contributor, self.reward) {
                Ok(_) => Ok(true),
                Err(_) => Err(WorkflowError::PaymentFailed),
            }
        }

        /// Simply returns the current workflow.
        #[ink(message)]
        pub fn get_reward(&self) -> Balance {
            self.reward
        }

        /// A helper function to detect whether a contribution exists in the storage
        pub fn contribution_is_known(&self, contribution_id: u64) -> bool {
            self.contributions.get(contribution_id).is_some()
        }

        /// A helper function to detect whether an aspiring contributor identity has been registered in the storage
        pub fn identity_is_known(&self, identity: HashValue) -> bool {
            self.identities.get(identity).is_some()
        }

        /// A helper function to hash bytes (e.g. identities or workflow file sha)
        pub fn hash(input: &[u8]) -> HashValue {
            let mut hash_value = <Sha2x256 as HashOutput>::Type::default();
            ink::env::hash_bytes::<Sha2x256>(input, &mut hash_value);
            hash_value
        }
    }

    #[cfg(test)]
    mod tests {
        /// Accounts
        /// ALICE -> contract owner
        /// BOB -> contributor

        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the constructor does its job.
        #[ink::test]
        fn new_works() {
            let contract = create_contract(1u128, 1u128);
            assert_eq!(contract.get_reward(), 1u128);
        }

        /// We test if a reward for an approved contribution can be claimed from the contributor
        #[ink::test]
        fn claim_works() {
            let accounts = default_accounts();
            let mut contract = create_contract(10u128, 1u128);
            let contribution_id = 1u64;
            let identity = SingleAssetReward::hash("bobby".as_bytes());
            let contributor = accounts.bob;
            set_next_caller(accounts.bob);
            assert_eq!(
                contract.register_identity(contributor.clone(), identity),
                Ok(())
            );
            set_next_caller(accounts.alice);
            assert_eq!(contract.approve(contribution_id, identity), Ok(()));
            let bob_initial_balance = get_balance(accounts.bob);
            set_next_caller(accounts.bob);
            assert_eq!(contract.claim(contribution_id), Ok(true));
            assert_eq!(
                get_balance(accounts.bob),
                bob_initial_balance + contract.reward
            );
        }

        fn default_accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<Environment>()
        }

        fn contract_id() -> AccountId {
            ink::env::test::callee::<ink::env::DefaultEnvironment>()
        }

        fn set_next_caller(caller: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(caller);
        }

        fn set_balance(account_id: AccountId, balance: Balance) {
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(account_id, balance)
        }

        fn get_balance(account: AccountId) -> Balance {
            ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(account)
                .expect("Cannot get account balance")
        }

        /// Creates a new instance of `SingleAssetReward` with `initial_balance`.
        ///
        /// Returns the `contract_instance`.
        fn create_contract(initial_balance: Balance, reward: Balance) -> SingleAssetReward {
            let accounts = default_accounts();
            set_next_caller(accounts.alice);
            set_balance(contract_id(), initial_balance);
            SingleAssetReward::new([0; 32], reward)
        }
    }
}
