# Kudos Ink!

> This project is a POC developed for the [Polkadot ink! Hackathon 
powered by Encode Club](https://www.encode.club/polkadot-ink-hackathon)

## Summary

Kudos Ink! is a platform that helps developers to find their next open-source software (OSS) tasks with AI assistance based on their skills and interests.

> As a project, use your community to get stuff done

## Abstract

Kudos Ink! addresses several significant pain points in the OSS ecosystem. First, it tackles the challenge of finding relevant OSS contributions, which is often a daunting task for developers. Additionally, the project aims to decentralize payments, providing a trustless environment where contributors can receive their dues without intermediaries. Lastly, Kudos Ink! offers a solution for the diversity and customization of rewards, overcoming the limitations of conventional transfers and airdrops.

Two existing solutions in the OSS community address decentralized contribution rewards: [NEAR Crowd](https://nearcrowd.com/), a platform that manages contribution bounties and reviews through smart contracts with community involvement, although it lacks reward customization; and [Web3 Actions](https://web3actions.github.io/), a system offering customizable payment workflows for OSS, using smart contracts and GitHub workflow signers to establish trustless reward systems; however it uses Kovan network which is deprecated today.

We drew inspiration from the second approach to develop **Kudos Ink!: a solution for customizable contribution rewards and decentralized payment workflows**.

> This article was a solid source of inspiration: [Ethereum on Github](https://medium.com/geekculture/ethereum-on-github-a752e33d6f19), by *Markus Kottländer*.

## How it works?

The reward contribution flow relies on two main characters:
- An `organization` calling for contributors to address certain Github issues.issues 
- `Aspiring contributors`

### Step 0: Prerequisities

- The organization creates or adds a **reward workflow** to its Github project repository.
- The organization instantiates a reward contract. It can be either an existing **reward contract** (we provide some templates [here](https://github.com/kudos-ink/contracts/tree/main/contracts/src)) or a custom one. The contract needs to implement the interface defined by the [workflow trait](https://github.com/kudos-ink/contracts/blob/main/contracts/src/traits/workflow.rs).

The contract is instanciated by calling `new` and providing at least the **reward workflow** file hash as an argument (in the [SingleToken](https://github.com/kudos-ink/contracts/blob/main/contracts/src/token/single-token/lib.rs) reward contract template we also provide the reward amount as an argument).

### Step 1: Issue Opening

- The organization creates an issue marked as payable, triggering the previously established **reward workflow**.

### Step 2: Aspiring Contributor

- An aspiring contributor registers their identity calling `register_contributor` with their Github ID on the **reward contract**.
- The aspiring contributor must be assigned to the opened issue.
- The contributor opens a pull request (PR) to resolve the issue.

### Step 3: Approval

- The organization reviews, approves, and merges the PR, thereby closing the issue.
- The **reward workflow** is triggered and calls `approve` on the **reward contract** with the given issue #ID and the contributor Github ID as their identity.

### Step 4: Claim

- The contributor claims its reward by using the `claim` method on the **reward contract** with the issue #ID. This action triggers the custom reward flow specified in the contract (e.g. a single bounty transfer, an NFT minting, ..)

## Customization aspects

This approach offers a high level of customization. The `claim` method, the final step in the process, allows for a wide range of reward mechanisms, making it highly adaptable to the specific needs and preferences of the organization and contributors. This flexibility empowers projects to implement diverse and tailored reward structures, whether it's a straightforward balance transfer, the issuance of non-fungible tokens (NFTs), or other creative and unique reward systems, ensuring that the Kudos Ink! platform can seamlessly accommodate a variety of open-source project needs and preferences. 

Kudos Ink! provides some templates ready to use as [reward workflow](https://github.com/kudos-ink/workflow-example/blob/main/.github/workflows/issue-closed.yml) with dedicated Github actions and [reward contract](https://github.com/kudos-ink/contracts/blob/main/contracts/src/token/single-token/lib.rs)

## Getting Started

To use Kudos Ink! in your project follow these steps:

1. Add a **reward workflow** to your Github repository; Kudos Ink! provides the following [example](https://github.com/kudos-ink/workflow-example/blob/main/.github/workflows/issue-closed.yml)

2. Create and instanciate a **reward contract**; Kudos Ink! provides the following [example](https://github.com/kudos-ink/contracts/blob/main/contracts/src/token/single-token/lib.rs)

It can be any **reward contract** implementing the following [trait](https://github.com/kudos-ink/contracts/blob/main/contracts/src/traits/workflow.rs):

```rust
#[openbrush::trait_definition]
pub trait Workflow: Ownable {
    /// Register the caller as an aspiring contributor.
    #[ink(message)]
    fn register_identity(&mut self, identity: HashValue) -> Result<(), WorkflowError>;

    /// Approve contribution. This is triggered by a workflow run.
    #[ink(message)]
    #[modifiers(only_owner)]
    fn approve(
        &mut self,
        contribution_id: u64,
        contributor_identity: HashValue,
    ) -> Result<(), WorkflowError>;

    /// Check the ability to claim for a given `contribution_id`.
    #[ink(message)]
    fn can_claim(&self, contribution_id: u64) -> Result<bool, WorkflowError>;

    /// Claim reward for a given `contribution_id`.
    #[ink(message)]
    fn claim(&mut self, contribution_id: u64) -> Result<(), WorkflowError>;
}
```

### Open brush support

Kudos Ink! supports [OpenBrush](https://github.com/Brushfam/openbrush-contracts). The `approve` method extends the [Ownable](https://learn.brushfam.io/docs/OpenBrush/smart-contracts/ownable) contract from OpenBrush.

## Existing Reward Contracts

### SingleToken

A contract to automatize single contribution rewards with a predefined bounty.

[Source](https://github.com/kudos-ink/contracts/blob/main/contracts/src/token/single-token/lib.rs)

## Future Ideas

Kudos Ink!'s goal is to deliver a developer-friendly hub for contributions. This benefits the ecosystem as a all.

In the next months we want to expand the array of available reward contracts, offering a wider selection of customizable options to cater to the diverse needs of open-source projects.

Additionally, a dedicated frontend application is on the horizon, designed to streamline and simplify various aspects of the platform's functionality. This application will allow organizations to instantiate reward contracts in bulk, providing a more efficient and scalable solution for project management. It will also introduce Github OAuth authentication for seamless contributor identity registration, making it easier for aspiring contributors to participate in the ecosystem. The app will further enable contributors to claim their rewards effortlessly and track their contribution metrics.

Kudos Ink! has ambitious plans to implement AI assistance, leveraging contributors' skills, interests, and experience to provide personalized suggestions for their next ideal open-source contribution, thereby enhancing the platform's utility and contributing to the growth of the open-source community.

These future ideas promise to make Kudos Ink! an even more robust and user-friendly platform for both organizations and contributors.
