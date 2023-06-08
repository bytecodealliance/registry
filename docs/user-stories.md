# Bytecode Alliance Components Registry — MVP Product definition

The purpose of this document is to collaborate on and produce a specification
for a registry for publishing, consuming, storing, and sharing Web Assembly
components. This includes a set of user stories, requirements, non goals, and
the HTTP API for the registry.

## Requirement categories

### Authoring components (A, B, C, I)

- Must be able to author a component in Rust, JavaScript, C/C++
- Must be able to import other components and interfaces
- Must be able to bootstrap a component from a template
- Must be able to add a cryptographic signature

### Publishing components (G, N, P)

- Must be able to cryptographically sign artifacts
- Must be able to distribute components separately
- Must be able to yank components

### Discovering components (D, E, T)

- Must be able to search for interfaces and components
- Must be able to search for components that implement an interface
- Must be able to view dependencies of a component
- Must be able to view the cryptographic signature of a component.
- Must be able to list authors of component
- Must be able to view LICENSE of component
- Must be able to view what system capabilities a component will require
- Must be able to get the name and version of a component
- Must be able to see available versions of a component given the component name

### Fetching components (H, R, S)

- Must be able to fetch a component by name and version
- Must be able to cryptographically validate components prior to installing them
  (though perhaps after downloading them)
- Must be able to cache components and compare cached component to component in
  registry to know if they are the same (e.g. avoid re-downloading the same
  components)

### Inspecting components (K, M)

- Must be able to list signatures on a component prior to installing
- Must be able to see LICENSE (and maybe README) of component without installing
- Must be able to determine size of component prior to installing

### Deploying components (F, L, O, Q)

- It must be easy to run your own
  [Deployable Registry](https://docs.google.com/document/d/1FxSuSYL0LkGb2jueUAcUu-h4sKEsdiWgMd4axsR95F8/edit#heading=h.hibsxupp1y4n)
- We must have implemented the Registry Spec except for the provenance parts

### Running components (J)

- Must be able to have tooling to dynamically link and instantiate components
- Must be able to reject components that don’t cryptographically verify

## User stories

A. As a Rust developer, I want to be able to use cargo-based tooling to develop
a WebAssembly Component that consumes Components and Interfaces from the
registry through auto-generated Rust APIs, and publish the resulting Component
to the registry.

B. As a JS developer, I want to be able to use npm-based tooling to develop a
WebAssembly Component that consumes Components and Interfaces from the registry
through auto-generated TypeScript type definitions, and publish the resulting
Component to the registry.

C. As a JS or Rust developer, I want to target a Profile published to the
registry to ensure that the resulting component will work in a runtime
environment implementing that Profile.

D. As a developer, I want to be able to search the registry by component /
interface name or description contents with an optional version constraint.

E. As a developer, I want to be able to search the registry for Components
implementing a specific Interface with an optional version constraint.

F. As a system administrator or platform operator, I want to be able to run an
instance of the registry implementation, and have the ability to make a filtered
subset of other registries available, either their live contents, or a mirrored
snapshot.

G. As a package author, I want to be able to associate sets of structured, but
open-ended metadata with the package when uploading it to the registry along
with signatures proving authenticity.

H. As a package consumer, I want to be able to retrieve the metadata sets
associated with a package as well as their signatures.

I. As a component developer, I want to be able to package my component along
with a set of hierarchical static contents from a local directory into my
component as an encapsulated implementation detail not visible to my clients or
other components my client is using.

J. As an application deployer, I want to be able to deploy (or know how to
unpack) a wasm component with my mounted static assets from a package in a
registry.

K. As a forensic analyst or auditor, I want to re-compose the full execution
environment for a past job and reconstruct the source code used for all compiled
artifacts.

L. As an individual or organization, I want to collect my
components/interfaces/profiles into a namespace grouping.

M. As a host environment provider, I want to have the APIs exposed necessary to
analyze whether I can run a specific component purely based on statically
analyzing its interface.

N. As a package owner, I can mark a package as “yanked” to indicate it should
not be used

O. As a repository owner, I can “delete” a package that is deemed in violation
of registries practices.

P. As a developer, I want to be able to satisfy licenses that require making
source code and/or other information available for binaries.

Q. As a registry owner, I can enable a requirement that all packages must have a
license field/annotation that contains a valid
[SPDX identifier](https://spdx.org/licenses/).

R. As a registry client, I want to have reproducible version resolution of my
version-constrained dependencies to the same contents over time (or a failure in
case a dependency was deleted).

S. As a registry client, I want to have a way to update the reproducible version
resolution of my version-constrained dependencies.

T. As a registry client, I want to be able to look up and retrieve a package in
a registry based on its content hash.

### References

- [original user stories document](https://docs.google.com/document/d/1QV0iXQBEqnE9CtNAhwH-oD7PBRnfeREj2nWZmw_zO8M/edit#)
- [registry architecture requirements](https://docs.google.com/document/d/1jv4Vh9o4LNT_XV9sklY1N840vElSI_dilnenzuwkraM/edit#heading=h.s6m06pefqgfb)
