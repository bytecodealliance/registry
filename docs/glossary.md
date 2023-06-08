# Glossary

As with most software projects, terms are often overloaded. The goal of this document is to provide
an unambiguous definition of terms frequently used by SIG Registries.

## Terms

### Annotations

Annotations are typed metadata added to a package after its creation. For example, "yank" and "takedown" annotation types.

### Component

A component is defined by the (emerging) [W3C WebAssembly Component Model specification](https://github.com/WebAssembly/component-model) which defines a component as a portable 
binary built from WebAssembly core modules with statically-analyzable, capability-safe, language-agnostic interfaces.

A component package is a type of [package](#package) whose contents are a component.

### Bundled Component and Bundling

A "bundled component" is a [component](#component) that only has interface dependencies and can thus run directly on a wasm engine that natively implements those interfaces 
without requiring any registry access. "Bundling" is an automatic transformation on a [component](#component) that replaces [imports](#imports) of other components (in the 
registry) with inline copies of those components (fetched from the registry at the time of bundling) to produce a bundled component.

### Exports

An "export" is a function, value, type, or [interface](#interfaces) that is implemented by a [component](#component) and exposed to the outside world with a given string name and declared type.

### Imports

An "import" is a function, value, type, or [interface](#interfaces) that must be given by the outside world to a component with a given string name and matching type in order to use that component.

### Interface

An "interface" is a named collection of functions, values, types, and other interfaces that can collectively be [imported](#imports) or [exported](#exports) by a component or 
[world](#world).  Each member of an interface is described with a name and a type.  A single component can import and/or export the same interface one or more times.

### Library

A library package is a package containg a WebAssembly [module](#module) implementing a shared executable library. Library packages will commonly be used for sharing language
runtime libraries and interpreters.

### Module

A common question is "What is the difference between a component and a module?". A Wasm module is a core Wasm module and compatible with Wasm 1.0 whereas a component adheres to 
the evolving Component Model specification with support for interfaces definitions (beyond i32's in core Wasm).  Modules are like `.dll`s in native systems, allowing low-level 
sharing of pointers to a shared memory but not providing isolation and requiring additional out-of-band information to reuse.  Components are more like microservices that 
supply an OpenAPI: they encapsulate their low-level state and self-describe their interface in a language-agnostic manner.  In the context of the registry, module packages are 
useful for factoring low-level runtime code out of components that would otherwise be statically duplicated.

### Namespace

A namespace is a named-definition of scope. A registry instance defines a disjoint namespace such that no registry instance's package names ever shadow or backstop another.

### Package

A package is type of content bundle uploaded to the registry. The registry architecture defines a number of built-in package types. Some of the package types include 
[Component](#component), [WIT](#wit), and [Library](#library) packages.

### Policies

The registry architecture provides mechanisms for registry instances to apply their own appropriate policies.

### Provenance

Provenance is a record of ownership of a package. The state of a registry is provenantial when it is *internally consistent* and every package release has provenance.

### Publisher

A publisher is a role that interacts with the publisher API, and has the ability to create a new package releases signed by a current maintainer.

### Registry

There isn’t just one registry: there is a single registry architecture which consists of a common set of tools and building blocks, and many registry instances, which are live services implemented in terms of the registry architecture. Registry instances can be general and global or specific to individual projects, companies, teams or accounts.

### Signatures

Signatures are cryptographic bindings to signing identities.

### Takedown

Takedown is an assertion by the [publisher](#publisher) or registry operator that "this release was removed for legal or policy reasons". This is unique from [yank](#yank)'s 
behavior in that the release's content URLs and potentially release metadata are removed from the registry. The primary use-case for a takedown is for *legal* reasons (DMCA et 
al).

### WIT

A WIT package is a binary-encoded representation of a [WIT](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md) document that may contain
any number of [interfaces](#interface) and [worlds](#world). Component authoring tooling uses WIT packages for generating language-specific bindings when authoring
a component.

### World

A "world" is a named collection of [imports](#imports) and [exports](#exports) used to describe both an individual component's type and also a host environment in which
a component may be run. For components, it describes the component's imports and exports. For host environments, it describes the maximal set of imports and the minimal
set of exports required for a component to run in that environment.

### Yank

Yank is an assertion by the [publisher](#publisher) that "this release is not fit for use". When a package is "yanked", the release is not altered but the release may be 
excluded from default query results. An example of when a package might be "yanked" is after an accidental or unintentional release.

## Governance and community terminology

### Phases of agreement

Phases of agreement are the levels of agreement required to advance proposals to the next stage. For more details, see [phases.md](phases.md).
