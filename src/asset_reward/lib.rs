#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod asset_reward {
    use ink::env::hash::{HashOutput, Sha2x256};
    use ink::storage::Mapping;

    /// Type alias for hashes.
    pub type HashValue = [u8; 32];
    /// Type alias for the contract's `Result` type.
    pub type Result<T> = core::result::Result<T, Error>;

    /// A Workflow is represented by:
    /// - the public address of the account transferring the reward.
    /// - the hash of the workflow file.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(ink::storage::traits::StorageLayout, scale_info::TypeInfo)
    )]
    pub struct Workflow {
        account: AccountId,
        hash: HashValue,
    }

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
    pub struct AssetReward {
        // The registered workflow.
        workflow: Workflow,

        // The contribution reward amount
        reward: Balance,

        // The used workflow run ids database.
        used_run_ids: Mapping<u64, bool>,

        // The approved contribution ids database.
        contributions: Mapping<u64, Contribution>,

        // The registered contributors ids database.
        identities: Mapping<HashValue, AccountId>, // HashValue refers to the contributo id (e.g. github ID)
    }

    /// Errors that can occur upon calling this contract.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Error {
        /// An aspiring contributor identity is already registered in the DB.
        IdentityAlreadyRegistered,
        /// Contribution is already approved in the DB.
        ContributionAlreadyApproved,
        // Run id is already used in the DB.
        RunIdAlreadyUsed,
        /// Contributor identity is not registered in the DB.
        UnknownContributor,
        /// Contribution is not in the DB.
        UnknownContribution,
        /// Attempted reward payment to a contributor failed.
        PaymentFailed,
        /// Returned if caller is not the workflow `owner` while required to.
        CallerIsNotWorkflowOwner,
        /// Returned if caller is not the `contributor` while required to.
        CallerIsNotContributor,
        /// Returned when attempting to claim an already claimed reward.
        AlreadyClaimed,
    }

    impl AssetReward {
        /// Constructor that initializes an asset reward for a given workflow
        #[ink(constructor)]
        pub fn new(workflow: Workflow, reward: Balance) -> Self {
            Self {
                workflow,
                reward,
                used_run_ids: Mapping::default(),
                contributions: Mapping::default(),
                identities: Mapping::default(),
            }
        }

        /// Verify that the correct workflow run is executed before approving a contribution.
        /// To do so:
        /// - it reconstructs a message using the stored hash of the workflow file with a specific workflow run id from arguments
        /// - recovers the signer public address using the generated message and the signature from arguments
        /// - ensure that the public address recovered is the trusted signer address in storage
        /// Reward contracts extend this method to the implementation of their respective reward mechanisms.
        ///
        /// Constraint(s):
        /// 1. The caller has to be the workflow `owner`.
        /// 2. The workflow `run_id` must not have been used previously.
        /// 3. The signature has to be valid.
        #[ink(message)]
        pub fn verify_workflow_run(&mut self, run_id: u64, signature: HashValue) -> Result<bool> {
            if self.env().caller() != self.workflow.account {
                return Err(Error::CallerIsNotWorkflowOwner);
            }

            if self.run_id_is_known(run_id) {
                return Err(Error::RunIdAlreadyUsed);
            }

            // bytes32 message = prefixed(keccak256(abi.encodePacked(githubWorkflows[_name].fileHash, _runId)));
            // address recovered = recoverSigner(message, _signature);

            // require(recovered == githubWorkflowSigner, "Invalid signature.");

            Ok(true)
        }

        /// Register an aspiring contributor.
        ///
        /// Constraint(s):
        /// 1. The `identity` id should not already be registered.
        ///
        /// A `Registered` event is emitted.
        #[ink(message)]
        pub fn register_identity(&mut self, account: AccountId, identity: HashValue) -> Result<()> {
            if self.identity_is_known(identity) {
                return Err(Error::IdentityAlreadyRegistered);
            }

            self.identities.insert(identity, &account);
            Ok(())
        }

        /// Approve contribution. This is triggered by a workflow run.
        ///
        /// Constraint(s):
        /// 1. The `run_id` & `signature` must come from a verified workflow run.
        /// 2. The `contribution_id` should not already be approved.
        /// 3. The `contributor_identity` must be registered.
        ///
        /// An `Approval` event is emitted.
        #[ink(message)]
        pub fn approve(
            &mut self,
            contribution_id: u64,
            contributor_identity: HashValue,
            _run_id: u64,
            _signature: HashValue,
        ) -> Result<()> {
            // TODO
            // verifyWorkflowRun(_runId, "withdraw", _signature);

            if self.contribution_is_known(contribution_id) {
                return Err(Error::ContributionAlreadyApproved);
            }

            let maybe_contributor = self.identities.get(contributor_identity);
            if maybe_contributor.is_none() {
                return Err(Error::UnknownContributor);
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
        pub fn claim(&self, contribution_id: u64) -> Result<bool> {
            if !self.contribution_is_known(contribution_id) {
                return Err(Error::UnknownContribution);
            }

            let contribution = self.contributions.get(contribution_id).unwrap();
            let contributor = contribution.contributor;
            if self.env().caller() != contributor {
                return Err(Error::CallerIsNotContributor);
            }

            if contribution.is_claimed {
                return Err(Error::AlreadyClaimed);
            }

            match self.env().transfer(contributor, self.reward) {
                Ok(_) => Ok(true),
                Err(_) => Err(Error::PaymentFailed),
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

        /// A helper function to detect whether a `run_id` has been already used
        pub fn run_id_is_known(&self, run_id: u64) -> bool {
            self.used_run_ids.get(run_id).is_some()
        }

        pub fn hash_sha_256(input: &[u8]) -> HashValue {
            let mut hash_value = <Sha2x256 as HashOutput>::Type::default();
            ink::env::hash_encoded::<Sha2x256, _>(&input, &mut hash_value);
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
            let identity = AssetReward::hash_sha_256("bobby".as_bytes());
            let contributor = accounts.bob;
            set_next_caller(accounts.bob);
            assert_eq!(
                contract.register_identity(contributor.clone(), identity),
                Ok(())
            );
            set_next_caller(accounts.alice);
            assert_eq!(
                contract.approve(contribution_id, identity, 0, [0; 32]),
                Ok(())
            );
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

        /// Creates a new instance of `AssetReward` with `initial_balance`.
        ///
        /// Returns the `contract_instance`.
        fn create_contract(initial_balance: Balance, reward: Balance) -> AssetReward {
            let accounts = default_accounts();
            set_next_caller(accounts.alice);
            set_balance(contract_id(), initial_balance);
            AssetReward::new(
                Workflow {
                    account: default_accounts().alice,
                    hash: [0; 32],
                },
                reward,
            )
        }
    }
}
