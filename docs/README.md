# WebAssembly Registry (WARG)

This document introduces the registry project's scope and
design proposed by the Bytecode Alliance SIG Registries group,
commonly known as ***WebAssembly Registry (WARG)***.


## Process

SIG Registeries meetings are held weekly.
To attend, please [join the Google group](https://groups.google.com/g/ba-sig-registries) for the calendar invite.
The calendar invite provides instructions for adding to the agenda.
See [past meeting notes](https://github.com/bytecodealliance/meetings/tree/main/sig-registries).

Design process [phases](phases.md).


## Design Decisions

#### WebAssembly binary packages
*WARG* is primarily designed for publishing and fetching
WebAssembly binary packages, both core modules as well as
components and component interfaces.

#### From development to deployment
Both libraries and interfaces published for software
developers as well as deployment artifacts can be published
to *WARG* registries.

#### Federation of registries
Anyone can run their own registry and import (and optionally
mirror) packages from other registries. Creating your own
private or public registry allows you to implement your own
policies for publishing packages, define access controls,
and secure network traffic.

#### Verifiable logs
Package releases are published to immutable logs signed by
their maintainers. Clients, third-party monitors, and
importing registries can cryptographically verify a registry's
state and history of state changes to detect a compromised or
malicious registry.

#### Content hosting optionality
*WARG* primarily interacts with package release logs with content
hashes. The registry itself or other services, such as blob
stores and OCI registries, can host the package contents.


## Scope for the Specification and Reference Implementation
Anything that involves client-to-registry, monitor-to-registry,
or registry-to-registry interaction needs to be clearly defined
in the *WARG Specification*.

The reference implementation prioritizes a minimal design that
is usable but not necessarily highly scalable or full-featured.
This encourages other *WARG Specification* compliant
implementations to be developed for more extensive requirements.
