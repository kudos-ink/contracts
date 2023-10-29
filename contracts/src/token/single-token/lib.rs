#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[openbrush::implementation(Ownable)]
#[openbrush::contract]
pub mod single_token {
    use kudos_ink_contracts::traits::workflow::{WorkflowError, *};
    use kudos_ink_contracts::traits::types::HashValue;
    use openbrush::{modifiers, traits::Storage};

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
    pub struct SingleToken {
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

    impl Workflow for SingleToken {
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
        fn claim(&mut self, contribution_id: u64) -> Result<(), WorkflowError> {
            self.claim(contribution_id)
        }
    }

    impl SingleToken {
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

            let contributor = match self.get_account(contributor_identity) {
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
        pub fn claim(&mut self, contribution_id: u64) -> Result<(), WorkflowError> {
            let contribution = self.ensure_can_claim(contribution_id)?;

            // Perform the reward claim
            if self.env().transfer(contribution.contributor, self.reward).is_err() {
                return Err(WorkflowError::PaymentFailed);
            }

            self.contribution = Some(Contribution {
                is_reward_claimed: true,
                ..contribution
            });

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

        /// Simply returns the `AccountId` of a given identity.
        #[ink(message)]
        pub fn get_account(&self, identity: HashValue) -> Option<AccountId> {
            self.identities.get(identity)
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

        use ink::env::test::EmittedEvent;
        type Event = <SingleToken as ::ink::reflect::ContractEventBase>::Type;

        /// We test if the constructor does its job.
        #[ink::test]
        fn new_works() {
            let contract = create_contract(1u128, 1u128);
            assert_eq!(contract.get_workflow(), [0; 32]);
            assert_eq!(contract.get_reward(), 1u128);
            assert_eq!(contract.get_contribution(), None);
        }

        #[ink::test]
        fn register_identity_works() {
            let accounts = default_accounts();
            let mut contract = create_contract(1u128, 1u128);
            let bob_identity = SingleToken::hash("bobby".as_bytes());
            set_next_caller(accounts.bob);
            assert_eq!(
                contract.register_identity(bob_identity),
                Ok(())
            );

            // Validate `IdentityRegistered` event emition
            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(1, emitted_events.len());
            let decoded_events = decode_events(emitted_events);
            if let Event::IdentityRegistered(IdentityRegistered { identity, caller }) = decoded_events[0] {
                assert_eq!(identity, bob_identity);
                assert_eq!(caller, accounts.bob);
            } else {
                panic!("encountered unexpected event kind: expected a IdentityRegistered event")
            }

            let maybe_account = contract.get_account(bob_identity);
            assert_eq!(
                maybe_account,
                Some(accounts.bob)
            );
        }

        #[ink::test]
        fn already_registered_identity_fails() {
            let accounts = default_accounts();
            let mut contract = create_contract(1u128, 1u128);
            let identity = SingleToken::hash("bobby".as_bytes());
            set_next_caller(accounts.bob);
            let _ = contract.register_identity(identity);
            assert_eq!(
                contract.register_identity(identity),
                Err(WorkflowError::IdentityAlreadyRegistered)
            );
        }

        #[ink::test]
        fn approve_works() {
            let accounts = default_accounts();
            let mut contract = create_contract(1u128, 1u128);
            let identity = SingleToken::hash("bobby".as_bytes());
            set_next_caller(accounts.bob);
            let _ = contract.register_identity(identity);

            let contribution_id = 1u64;
            set_next_caller(accounts.alice);
            assert_eq!(
                contract.approve(contribution_id, identity),
                Ok(())
            );

            // Validate `ContributionApproval` event emition
            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(2, emitted_events.len());
            let decoded_events = decode_events(emitted_events);
            if let Event::ContributionApproval(ContributionApproval { id, contributor }) = decoded_events[1] {
                assert_eq!(id, contribution_id);
                assert_eq!(contributor, accounts.bob);
            } else {
                panic!("encountered unexpected event kind: expected a ContributionApproval event")
            }

            let maybe_contribution = contract.get_contribution();
            assert_eq!(
                maybe_contribution,
                Some(Contribution {id: contribution_id, contributor: accounts.bob, is_reward_claimed: false})
            );
        }

        #[ink::test]
        fn only_contract_owner_can_approve() {
            let accounts = default_accounts();
            let mut contract = create_contract(1u128, 1u128);
            let identity = SingleToken::hash("bobby".as_bytes());
            set_next_caller(accounts.bob);
            let _ = contract.register_identity(identity);

            let contribution_id = 1u64;
            assert_eq!(
                contract.approve(contribution_id, identity),
                Err(WorkflowError::OwnableError(OwnableError::CallerIsNotOwner))
            );
        }

        #[ink::test]
        fn already_approved_contribution_fails() {
            let accounts = default_accounts();
            let mut contract = create_contract(1u128, 1u128);
            let identity = SingleToken::hash("bobby".as_bytes());
            let identity2 = SingleToken::hash("bobby2".as_bytes());
            set_next_caller(accounts.bob);
            let _ = contract.register_identity(identity);

            let contribution_id = 1u64;
            set_next_caller(accounts.alice);
            let _ = contract.approve(contribution_id, identity);

            assert_eq!(
                contract.approve(contribution_id, identity2),
                Err(WorkflowError::ContributionAlreadyApproved)
            );
        }

        #[ink::test]
        fn approve_unknown_contributor_identity_fails() {
            let accounts = default_accounts();
            let mut contract = create_contract(1u128, 1u128);
            let identity = SingleToken::hash("bobby".as_bytes());
            let identity2 = SingleToken::hash("bobby2".as_bytes());
            set_next_caller(accounts.bob);
            let _ = contract.register_identity(identity);

            let contribution_id = 1u64;
            set_next_caller(accounts.alice);
            assert_eq!(
                contract.approve(contribution_id, identity2),
                Err(WorkflowError::UnknownContributor)
            );
        }

        #[ink::test]
        fn can_claim_works() {
            let accounts = default_accounts();
            let mut contract = create_contract(1u128, 1u128);
            let identity = SingleToken::hash("bobby".as_bytes());
            set_next_caller(accounts.bob);
            let _ = contract.register_identity(identity);

            let contribution_id = 1u64;
            set_next_caller(accounts.alice);
            let _ = contract.approve(contribution_id, identity);
            
            set_next_caller(accounts.bob);
            assert_eq!(
                contract.can_claim(contribution_id),
                Ok(true)
            );
        }

        #[ink::test]
        fn claim_works() {
            let accounts = default_accounts();
            let single_reward = 1u128;
            let mut contract = create_contract(1u128, single_reward);
            let identity = SingleToken::hash("bobby".as_bytes());
            set_next_caller(accounts.bob);
            let _ = contract.register_identity(identity);

            let issue_id = 1u64;
            set_next_caller(accounts.alice);
            let _ = contract.approve(issue_id, identity);
            
            let bob_initial_balance = get_balance(accounts.bob);
            set_next_caller(accounts.bob);
            assert_eq!(contract.claim(issue_id), Ok(()));
            assert_eq!(
                get_balance(accounts.bob),
                bob_initial_balance + contract.reward
            );

            let maybe_contribution = contract.get_contribution();
            assert_eq!(
                maybe_contribution,
                Some(Contribution {id: issue_id, contributor: accounts.bob, is_reward_claimed: true})
            );

            // Validate `RewardClaimed` event emition
            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(3, emitted_events.len());
            let decoded_events = decode_events(emitted_events);
            if let Event::RewardClaimed(RewardClaimed { contribution_id, contributor, reward }) = decoded_events[2] {
                assert_eq!(contribution_id, issue_id);
                assert_eq!(contributor, accounts.bob);
                assert_eq!(reward, single_reward);
            } else {
                panic!("encountered unexpected event kind: expected a RewardClaimed event")
            }
        }

        #[ink::test]
        fn cannot_claim_non_approved_contribution() {
            let accounts = default_accounts();
            let contract = create_contract(1u128, 1u128);
            set_next_caller(accounts.bob);

            let contribution_id = 1u64;
            assert_eq!(
                contract.can_claim(contribution_id),
                Err(WorkflowError::NoContributionApprovedYet)
            );
        }

        #[ink::test]
        fn cannot_claim_unknown_contribution() {
            let accounts = default_accounts();
            let mut contract = create_contract(1u128, 1u128);
            let identity = SingleToken::hash("bobby".as_bytes());
            set_next_caller(accounts.bob);
            let _ = contract.register_identity(identity);

            let contribution_id = 1u64;
            set_next_caller(accounts.alice);
            let _ = contract.approve(contribution_id, identity);

            set_next_caller(accounts.bob);
            assert_eq!(
                contract.can_claim(2u64),
                Err(WorkflowError::UnknownContribution)
            );
        }

        #[ink::test]
        fn cannot_claim_if_not_contributor() {
            let accounts = default_accounts();
            let mut contract = create_contract(1u128, 1u128);
            let identity = SingleToken::hash("bobby".as_bytes());
            set_next_caller(accounts.eve);
            let _ = contract.register_identity(identity);

            let contribution_id = 1u64;
            set_next_caller(accounts.alice);
            let _ = contract.approve(contribution_id, identity);

            set_next_caller(accounts.bob);
            assert_eq!(
                contract.can_claim(contribution_id),
                Err(WorkflowError::CallerIsNotContributor)
            );
        }

        #[ink::test]
        fn cannot_claim_already_claimed_reward() {
            let accounts = default_accounts();
            let mut contract = create_contract(1u128, 1u128);
            let identity = SingleToken::hash("bobby".as_bytes());
            set_next_caller(accounts.bob);
            let _ = contract.register_identity(identity);

            let contribution_id = 1u64;
            set_next_caller(accounts.alice);
            let _ = contract.approve(contribution_id, identity);

            set_next_caller(accounts.bob);
            let _ = contract.claim(contribution_id);
            assert_eq!(
                contract.can_claim(contribution_id),
                Err(WorkflowError::AlreadyClaimed)
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

        /// Creates a new instance of `SingleToken` with `initial_balance`.
        ///
        /// Returns the `contract_instance`.
        fn create_contract(initial_balance: Balance, reward: Balance) -> SingleToken {
            let accounts = default_accounts();
            set_next_caller(accounts.alice);
            set_balance(contract_id(), initial_balance);
            SingleToken::new([0; 32], reward)
        }

        fn decode_events(emittend_events: Vec<EmittedEvent>) -> Vec<Event> {
            emittend_events
                .into_iter()
                .map(|event| {
                    <Event as scale::Decode>::decode(&mut &event.data[..]).expect("invalid data")
                })
                .collect()
        }
    }
}
