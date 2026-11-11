# Security Policy

Tillstone handles money and business records, so we take security reports seriously.

## Reporting a vulnerability

**Please don't open a public issue.** Instead, report it privately through GitHub:

1. Go to the repository's **Security** tab.
2. Choose **Report a vulnerability**.
3. Describe the problem, how to reproduce it, and what an attacker could do with it.

You'll get an acknowledgement within 7 days. We'll keep you updated while we work on a fix, and credit you in the advisory unless you'd rather stay anonymous.

## Supported versions

Tillstone has no releases yet. Once it does, security fixes will be made for the **latest release**. Installed copies are notified of security releases through a signed security notice (see [the plan, §9](docs/PLAN.md#9-security-design)).

## Scope

In scope: anything in this repository, including the app, the core, sync, payment and fiscal adapters, and the release process.

Out of scope: vulnerabilities in third-party services (such as payment providers) themselves. Report those to the provider. Issues in how Tillstone *uses* them are in scope.

## Disclosure

We aim to publish a fix and an advisory within 90 days of a report, sooner for actively exploited issues. Please give us that time before disclosing publicly.
