#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[openbrush::implementation(Ownable)]
#[openbrush::contract]
pub mod single_asset_reward {
    use kudos_ink::traits::workflow::{WorkflowError, *};
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
        // The unique contribution ID (e.g. the Github issue #id).
        id: u64,
        // The contributor public key (e.g. extract from the `identities` mapping).
        contributor: AccountId,
        is_reward_claimed: bool,
    }

    #[ink(storage)]
    #[derive(Default, Storage)]
    pub struct SingleAssetReward {
        #[storage_field]
        ownable: ownable::Data,

        // The registered workflow.
        // It is usually represented with the SHA hash of the workflow file (e.g. Github Workflow file).
        workflow: HashValue,

        // The contribution reward amount.
        reward: Balance,

        // The approved `Contribution`.
        contribution: Option<Contribution>,

        // The registered contributors ids database.
        // The key refers to a registered and unique contribution ID (e.g. the Github issue #id).
        // The value is the associated registered `AccountId` (public key) of the contributor.
        identities: Mapping<HashValue, AccountId>, // HashValue refers to the contributo id (e.g. github ID)
    }

    /// Emitted when an `identity` is registered by an aspiring contributor.
    #[ink(event)]
    pub struct IdentityRegistered {
        identity: HashValue,
        caller: AccountId,
    }

    /// Emitted when a `contribution` is approved.
    #[ink(event)]
    pub struct ContributionApproval {
        id: u64,
        contributor: AccountId,
    }

    /// Emitted when the reward associated with the `contribution` is claimed.
    #[ink(event)]
    pub struct RewardClaimed {
        contribution_id: u64,
        contributor: AccountId,
        reward: Balance,
    }

    impl Workflow for SingleAssetReward {
        /// Register the caller as an aspiring contributor.
        ///
        /// Constraint(s):
        /// 1. The `identity` id should not already be registered.
        ///
        /// A `IdentityRegistered` event is emitted.
        #[ink(message)]
        fn register_identity(&mut self, identity: HashValue) -> Result<(), WorkflowError> {
            self.register_identity(identity)
        }

        /// Approve contribution. This is triggered by a workflow run.
        ///
        /// Constraint(s):
        /// 1. The `contribution_id` should not already be approved.
        /// 2. The `contributor_identity` must be registered.
        ///
        /// An `ContributionApproval` event is emitted.
        #[ink(message)]
        #[modifiers(only_owner)]
        fn approve(
            &mut self,
            contribution_id: u64,
            contributor_identity: HashValue,
        ) -> Result<(), WorkflowError> {
            self.approve(contribution_id, contributor_identity)
        }

        /// Check the ability to claim for a given `contribution_id`.
        ///
        /// Constraint(s):
        /// 1. A `contribution` must be approved.
        /// 2. The `contribution_id` must be the same as the one in the approved `contribution`.
        /// 3. The caller has to be the contributor of the approved `contribution`.
        /// 4. The claim must be available (marked as false in the claims mapping).
        #[ink(message)]
        fn can_claim(&self, contribution_id: u64) -> Result<bool, WorkflowError> {
            self.can_claim(contribution_id)
        }

        /// Claim reward for a given `contribution_id`.
        ///
        /// Constraint(s): Ensure `can_claim`.
        ///
        /// A `RewardClaimed` event is emitted.
        #[ink(message)]
        fn claim(&self, contribution_id: u64) -> Result<(), WorkflowError> {
            self.claim(contribution_id)
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

        /// Register the caller as an aspiring contributor.
        #[ink(message)]
        pub fn register_identity(&mut self, identity: HashValue) -> Result<(), WorkflowError> {
            if self.identity_is_known(identity) {
                return Err(WorkflowError::IdentityAlreadyRegistered);
            }

            let caller = Self::env().caller();
            self.identities.insert(identity, &caller);

            self.env()
                .emit_event(IdentityRegistered { identity, caller });

            Ok(())
        }

        /// Approve contribution. This is triggered by a workflow run.
        #[ink(message)]
        #[modifiers(only_owner)]
        pub fn approve(
            &mut self,
            contribution_id: u64,
            contributor_identity: HashValue,
        ) -> Result<(), WorkflowError> {
            if self.contribution.is_some() {
                return Err(WorkflowError::ContributionAlreadyApproved);
            }

            let contributor = match self.identities.get(contributor_identity) {
                Some(contributor) => contributor,
                None => return Err(WorkflowError::UnknownContributor),
            };

            let contribution = Contribution {
                id: contribution_id,
                contributor,
                is_reward_claimed: false,
            };
            self.contribution = Some(contribution);

            self.env().emit_event(ContributionApproval {
                id: contribution_id,
                contributor,
            });

            Ok(())
        }

        /// Check the ability to claim for a given `contribution_id`.
        #[ink(message)]
        pub fn can_claim(&self, contribution_id: u64) -> Result<bool, WorkflowError> {
            self.ensure_can_claim(contribution_id)?;

            Ok(true)
        }

        /// Claim reward for a given `contribution_id`.
        #[ink(message)]
        pub fn claim(&self, contribution_id: u64) -> Result<(), WorkflowError> {
            let contribution = self.ensure_can_claim(contribution_id)?;

            // Perform the reward claim
            if let Err(_) = self.env().transfer(contribution.contributor, self.reward) {
                return Err(WorkflowError::PaymentFailed);
            }

            self.env().emit_event(RewardClaimed {
                contribution_id,
                contributor: contribution.contributor,
                reward: self.reward,
            });

            Ok(())
        }

        /// Simply returns the workflow hash.
        #[ink(message)]
        pub fn get_workflow(&self) -> HashValue {
            self.workflow
        }

        /// Simply returns the reward amount.
        #[ink(message)]
        pub fn get_reward(&self) -> Balance {
            self.reward
        }

        /// Simply returns the aprroved `contribution` if some.
        #[ink(message)]
        pub fn get_contribution(&self) -> Option<Contribution> {
            self.contribution
        }

        /// A helper function to ensure a contributor can claim the reward.
        pub fn ensure_can_claim(
            &self,
            contribution_id: u64,
        ) -> Result<Contribution, WorkflowError> {
            // Check if a contribution is set
            let contribution = match &self.contribution {
                Some(contribution) => contribution,
                None => return Err(WorkflowError::NoContributionApprovedYet),
            };

            // Verify the contribution ID
            if contribution_id != contribution.id {
                return Err(WorkflowError::UnknownContribution);
            }

            // Verify the caller is the contributor
            if Self::env().caller() != contribution.contributor {
                return Err(WorkflowError::CallerIsNotContributor);
            }

            // Check if the reward has already been claimed
            if contribution.is_reward_claimed {
                return Err(WorkflowError::AlreadyClaimed);
            }

            Ok(*contribution)
        }

        /// A helper function to detect whether an aspiring contributor identity has been registered in the storage.
        pub fn identity_is_known(&self, identity: HashValue) -> bool {
            self.identities.get(identity).is_some()
        }

        /// A helper function to hash bytes (e.g. identities or workflow file sha).
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
