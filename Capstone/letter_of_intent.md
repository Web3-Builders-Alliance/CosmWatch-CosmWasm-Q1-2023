# CosmWatch Capstone: Letter of Intent

>✉ I submit this letter of intent for my Capstone project as part of the Q1 CosmWasm/Rust Cohort for the WBA Institute.

## **W3BA Graduate Hiring Contract and UI**

Hiring freelancers online using centralized web2 technology is currently limited, expensive, and risky. To combat those issues, I am going to use a soulbound token (SBT) contract and DApp built by Eliseo (capstone Q4 2022) and take it a step further, enabling anyone who visits the DApp to view graduate details and faciliate the hiring process using a new, escrow-like smart contract.

---

## Hiring someone online: What are your options currently and what problems are associated with them?

### Fiverr, Upwork

- Facilitate an agreement between two parties of
    - **What** milestones/tasks are to be completed
    - **When** they will happen (milestone dates, deadlines)
    - **How much** the freelancer should be rewarded at the specified time(s)
- Charge expensive fees in order to facilitate the payout(s)
    - This can make hiring someone more expensive than originally expected
- Offer limited flexibility in how a transaction is coordinated
    - Does the party hiring want one task and one payment?
    - Is the task or project big enough for it to include milestones? 
    - These are defined by the platform and can be very standardized and inflexible
    - To be completely fair, there are benefits to standardization and limited options in this context as well as drawbacks
- Sometimes difficult to qualify a freelancer's level of skill
 
### Two private parties
- Facilitate an agreement like the above states, but the integrity and quality of the agreement is left up to the parties who created it
    - Standardization or guidelines don't exist unless one or both parties provide them
    - There are no guarantees that standardation or guidelines will exist
- Payment is at the mercy of the hiring party
    - It may come on-time, late, or not at all
    - There are no guarantees here either
- There is no support like hiring through a web2 platform
    - If a transaction is disputed or a malicious actor is uncovered, the freelancer has very limited options, if any, to resolve the conflict
- The freelancer/party getting paid is at the mercy of numerous money transferring services (PayPal, Venmo, Cash App, Zelle, etc...)
    - Fees are almost inevitable when sending money through a third-party service
    - This may mean the freelancer may not get the entire amount promised, if not explicitly defined in a contract/agreement
- Sometimes difficult to qualify a freelancer's level of skill

## The better way to hire someone online: Hiring DApp

### There will be two components to this solution (completed in order below):

1. Architect, write, test, and deploy the extended escrow smart contract with frontend interaction in mind
    - Contains the escrow-like logic of creating new agreements, updating existing agreements, and executing payouts for completed agreements and/or agreement milestones
    - These changes can either be done from the UI or direct API calls to the contract
    - If all milestones are completed and thereby paying out all funds for that agreement, the agreement is marked completed and becomes immutable automatically
2. Extend the existing UI using the both the existing SBT and newly built extended escrow smart contracts (potentially leveraging a fork of Angel Protocol's open-sourced frontend)
    - The existing UI built by Eliseo will be used to browse and filter W3BA graduate SBTs by skills, cohort, etc...
    - Extend the DApp to click on a graduate and view metadata (i.e. Image, name, description, skills, cohort, etc...) and provide a pretty and easy to use interface for hiring a graduate
    - Anyone can view and hire the graduate by connecting their wallet to the UI, but first the hiring party must propose a job consisting of a name, description, and at least one milestone (milestone = condition and payout amount) to the graduate.
    - The proposal creator will be expected to send the total payout amount to the contract to be locked until the proposal is accepted and miletones are met or the proposal is rejected by the graduate
    - The graduate can choose to either accept or reject this proposal through connecting their wallet to the UI and checking their inbox
    - In their inbox, the user can see all proposals new, in progress, or completed for the connected wallet
    - Both the graduate and hiring party will have controls in which they can agree or disagree to execute milestone payouts or cancel the agreement entirely at any time

### By developing this decentralized platform, we can solve many of the problems listed above:

- Introduce an independent and trusted third-party to replace platforms
    - This will ensure that when the appropriate conditions are met, the payout will go to the hired party automatically and directly from the contract
    - Enforces strict terms (i.e. If both parties agree that the terms have been met, then execute the payment to the recipient)
    - Holds both parties accountable at different stages of the process.
        - The hiring party must deposit the total payout amount into the contract at time of creation
        - The hired party must ensure the hiring party's needs are met before voting to execute payout
- Significantly reduce overhead by replacing platform fees with gas fees
    - Gas fees for executing a smart contract can be unpredictably pricier than sending tokens from one wallet to another
        - Despite this, the gas fees are a small fraction of the corresponding fee on a web2 hiring platform
- Certified Web3 Builders Alliance SBT that verifies that a graduate is a dedicated web3 developer with sufficient skill to complete high-quality, paid work

<br>

**Want a visual of my thought process?** Link to my initial brainstorm/mind map [here](https://docs.google.com/drawings/d/1G7j7yoXV2-VwnyyCZ5DMcuyZ5xSyws3anO7NSlVD_SI/edit?pli=1)

<br>

Thank you,<br>
Max
