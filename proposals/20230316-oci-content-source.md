---
title: OCI Content Source
authors:
- @devigned
reviewers:
- tbd
creation-date: 2023-03-16
last-updated: 2023-03-16
status: implementable
---


# OCI Content Source

## Table of Contents
- [Summary](#summary)
- [Motivation](#motivation)
    - [Goals](#goals)
    - [Non-Goals / Future Work](#non-goals--future-work)
- [Proposal](#proposal)
    - [User Stories](#user-stories)
        - [Story 1 - Publishing a Component](#story-1---publishing-a-component)
        - [Story 2 - Publishing a Component Interface](#story-2---publishing-a-component-interface)
        - [Story 3 - Publishing a Bundled Component](#story-3---publishing-a-bundled-component)
        - [Story 4 - Fetching a Component](#story-4---fetching-a-component)
    - [Requirements](#requirements)
        - [Functional](#functional)
        - [Non-Functional](#non-functional)
    - [Implementation Details](#implementation-details)
      - [Artifact Types](#artifact-types)
      - [Image Manifest for Components](#image-manifest-for-components)
      - [Image Manifest for Interface Components](#image-manifest-for-interface-components)
      - [Image Manifest for Bundled Components](#image-manifest-for-bundled-components)
      - [Image Manifest for Signing and SBOMs](#image-manifest-for-signing-and-sboms)
      - [Image Manifests for Additional Metadata](#image-manifests-for-additional-metadata)
      - [Warg Registry Implementation](#warg-registry-implementation)
- [Alternative Options](#alternative-options)
    - [Publish Artifacts Using an Artifact Manifest](#publish-artifacts-using-an-artifact-manifest)
        - [Pros](#pros)
        - [Cons](#cons)
- [Conclusions](#conclusions)
- [Additional Details](#additional-details)
    - [Test Plan](#test-plan)
- [Implementation History](#implementation-history)

## Summary

This proposal introduces a new content source kind to Warg, which will enable Warg to store and retrieve packages from OCI registries. OCI registries make it simple to store, share and manage package content, are broadly accessible from local environments to cloud service providers whom run managed OCI registries, and have a large, established set of tools built around the OCI registries and images.

## Motivation

At the time of writing this, Warg only supports a single content source kind `ContentSourceKind::HttpAnonymous`, which retrieves and persists package content via unauthenticated HTTP requests stored to the local file system. This works well for demo purposes, but is not a robust solution for long-term, distributed, scalable storage of package content.

OCI registries are a great match for the addressable content that Warg needs to store. Additionally, their nearly ubiquitous usage means that Warg registries will not need additional infrastructure to store package content, and related metadata and cryptographic assurances.

### Goals
- Define metadata and layer media types for storing Warg packages in OCI registries.
- Define a strategy for attaching software bill of materials (SBOMs), attestations, and signatures for Warg packages stored in OCI registries.
- Introduce `ContentSourceKind::OCIv1_1` to persist and fetch package content from OCI v1.1 compliant registries.
- Implement persistence and retrieval logic for `ContentSourceKind::OCIv1_1`
- Describe a pattern for attaching additional metadata to be attached to components stored in an OCI registry. Examples of additional metadata are debugging symbols, documentation, WIT, etc.

### Non-Goals
- Exhaustively describe specification for additional metadata to be attached to components stored in an OCI registry.

## Glossary
- [Component](https://github.com/bytecodealliance/SIG-Registries/blob/main/glossary.md#component): A component is defined by the (emerging) [W3C WebAssembly Component Model specification](https://github.com/WebAssembly/component-model) which defines a component as a portable binary built from WebAssembly core modules with statically-analyzable, capability-safe, language-agnostic interfaces. A component package is a type of [package](https://github.com/bytecodealliance/SIG-Registries/blob/main/glossary.md#package) whose contents are a component.
- [Bundled Component and Bundling](https://github.com/bytecodealliance/SIG-Registries/blob/main/glossary.md#bundled-component-and-bundling): A "bundled component" is a [component](https://github.com/bytecodealliance/SIG-Registries/blob/main/glossary.md#component) that only has interface dependencies and can thus run directly on a wasm engine that natively implements those interfaces
without requiring any registry access. "Bundling" is an automatic transformation on a [component](https://github.com/bytecodealliance/SIG-Registries/blob/main/glossary.md#component) that replaces [imports](https://github.com/bytecodealliance/SIG-Registries/blob/main/glossary.md#imports) of other components (in the
registry) with inline copies of those components (fetched from the registry at the time of bundling) to produce a bundled component.

## Proposal
### User Stories
#### Story 1 - Publishing a Component
Alex is an engineer working in a large organization which is building applications using Wasm components. Alex would like share the component that their team has built with others within their company. The company Alex works for has a lot of folks with experience running containers, and they have OCI registries already provisioned to store container images. Alex would like to publish the component their team has built into one of their company's OCI registry.

#### Story 2 - Publishing a Component Interface
Erin is an engineer working on an open source project that allows users of the project to extend the functionality of the project using Wasm components as plugins. Erin would like to publish the interface specification for their plugin, so that other developers can easily find the interface and develop plugins for their project. Erin would like to use the GitHub Container Registry to store the Wasm component interface.

#### Story 3 - Publishing a Bundled Component
Alex is an engineer working in a large, security focused organization which runs a lot of Linux containers in production. The security team at Alex's company requires Linux containers be signed and provide a software bill of materials. Alex has recently built a new application that instead of being packaged as a Linux container image, they have built their application targeting Wasm. In fact, Alex built their application using many Wasm components. Alex and their team have finalized the feature set for their first release, tested the application, and locked the version for all the dependencies. Alex would like to publish this version of their application with all the application dependencies bundled together. Alex would also like to sign the bundled application and includes a software bill of materials of the components bundled. 

#### Story 4 - Fetching a Component
Erin is an engineer working on an open source project that uses Wasm components. Erin needs to pad the left side of strings in their application and rather than write this functionality, Erin wants to find and use a component that implements this functionality. Erin's friend Alex told them about their awesome leftpad component. Erin adds a dependency in their project for Alex's leftpad component. When the dependency is added, Erin's computer fetches the component from GitHub Container Registry, validates the signature on the component, and provides Erin a software bill of materials for the contents of the leftpad component.

### Requirements
#### Functional
- FR1. Warg MUST support publishing components to an OCIv1 compatible registry.
- FR2. Warg MUST support propagating signatures and bills of materials to OCIv1 content stores.
- FR3. Warg SHOULD support existing container image secure supply chain tooling allowing existing investments in container secure supply chains to be leveraged for Wasm components.
#### Non-functional
- TODO

### Implementation Details
At the time of authoring this proposal, the OCI v1.1 image specification is still in release candidate state, and there is a bit of flux with regard to artifacts. This proposal will take into account the most up-to-date state of the image specification and may evolve as changes in the OCI specifications evolve.

#### Artifact Types
Image manifests will be differentiated based on the `artifactType` field in the image manifest. The following media types will be used.
- Component: "application/vnd.wasm.component.v1"
- Interface: "application/vnd.wasm.component.interface.v1"
- Bundled Component: "application/vnd.wasm.component.bundled.v1"

#### Image Manifest for Components
The following is an example image manifest for a component containing a configuration structure, a layer containing the `my-component.wasm` binary.
```json
{
  "schemaVersion": 2,
  "mediaType": "application/vnd.oci.image.manifest.v1+json",
  "artifactType": "application/vnd.wasm.component.v1",
  "config": {
    "mediaType": "application/vnd.wasm.component.config.v1+json",
    "digest": "sha256:5587da2246a78f08c447bff2ac91ee5c2b57be2f2a15244b5e618ac0be626885",
    "size": 331
  },
  "layers": [
    {
      "mediaType": "application/vnd.wasm.content.layer.v1+wasm",
      "digest": "sha256:b36aa5d0111a488937361fdb35432510d50675a11d566c5d9e82a147fb9ff552",
      "size": 2087464,
      "annotations": {
        "org.opencontainers.image.title": "my-component"
      }
    }
  ]
}
```

The following is an example of the configuration structure referenced in the preceding image manifest.
```json
{
  "mediaType": "application/vnd.wasm.component.config.v1+json",
  "architecture": "wasm32",
  "os": "wasi"
}
```

#### Image Manifest for Interface Components
The following is an example image manifest for a component containing a configuration structure and a layer containing the `my-component-interface.wasm` binary.
```json
{
  "schemaVersion": 2,
  "mediaType": "application/vnd.oci.image.manifest.v1+json",
  "artifactType": "application/vnd.wasm.component.interface.v1",
  "config": {
    "mediaType": "application/vnd.wasm.component.config.v1+json",
    "digest": "sha256:c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4",
    "size": 331
  },
  "layers": [
    {
      "mediaType": "application/vnd.wasm.content.layer.v1+wasm",
      "digest": "sha256:dcf07c6bb395d6e1d40505b77e70af04f5fae0d54c9573fd379c1e7355a18cf3",
      "size": 2087464,
      "annotations": {
        "org.opencontainers.image.title": "my-component-interface"
      }
    }
  ]
}
```

The following is an example of the configuration structure referenced in the preceding image manifest.
```json
{
  "mediaType": "application/vnd.wasm.component.config.v1+json",
  "architecture": "wasm32",
  "os": "wasi"
}
```

#### Image Manifest for Bundled Components
The following is an example image manifest for a component containing a configuration structure, a layer containing the `my-bundled-component.wasm` binary, and a data layer containing some a static asset.
```json
{
  "schemaVersion": 2,
  "mediaType": "application/vnd.oci.image.manifest.v1+json",
  "artifactType": "application/vnd.wasm.component.bundled.v1",
  "config": {
    "mediaType": "application/vnd.wasm.component.config.v1+json",
    "digest": "sha256:105ab3237b4f0d885700892a0f4b3482d1146dff27c88d46f02b8bd7ef67c3de",
    "size": 331
  },
  "layers": [
    {
      "mediaType": "application/vnd.wasm.content.layer.v1+wasm",
      "digest": "sha256:2e94e0582fb925e89515435513496819dc8f364f2da400059a64d6d1412ca2ad",
      "size": 2087464,
      "annotations": {
        "org.opencontainers.image.title": "my-bundled-component"
      }
    },
    {
      "mediaType": "application/vnd.wasm.content.layer.v1+data",
      "digest": "sha256:8c69a84ec5adec97e47d4250410a7689046762aaa8e89f82ddbb4a89acb7388e",
      "size": 96
    }
  ]
}
```

The following is an example of the configuration structure referenced in the preceding image manifest.
```json
{
  "mediaType": "application/vnd.wasm.component.config.v1+json",
  "architecture": "wasm32",
  "os": "wasi",
  "wasi": {
    "environment": {
      "env1": "first",
      "env2": "second"
    },
    "files": [
      {
        "guest": "cat.png",
        "digest": "sha256:8c69a84ec5adec97e47d4250410a7689046762aaa8e89f82ddbb4a89acb7388e"
      }
    ]
  }
}

```

#### Image Manifest for Signing and SBOMs
The following example illustrates signing a component using Notary V2. Use of Notary V2 could be replaced with any other signing implementation.
- The signature manifest `mediaType` MUST be an image manifest. 
- The subject descriptor digest MUST point to an image manifest for a component.
```json
{
    "schemaVersion": 2,
    "mediaType": "application/vnd.oci.image.manifest.v1+json",
    "config": {
        "mediaType": "application/vnd.cncf.notary.signature",
        "size": 2,
        "digest": "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
    },
    "layers": [
        {
            "mediaType": "application/jose+json",
            "digest": "sha256:9834876dcfb05cb167a5c24953eba58c4ac89b1adf57f28f2f9d09af107ee8f0",
            "size": 32654
        }
    ],
    "subject": {
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "digest": "sha256:e41e72e96cf23dc26baa6931e5534c7fe4b16157d485cc36bbbbd000fe37477d",
        "size": 16724
    },
    "annotations": {
        "io.cncf.notary.x509chain.thumbprint#S256": 
        "[\"B7A69A70992AE4F9FF103EBE04A2C3BA6C777E439253CE36562E6E98375068C3\",\"932EB6F5598435D4EF23F97B0B5ACB515FAE2B8D8FAC046AB813DDC419DD5E89\"]"
    }
}
```

For additional information about Notary V2 image manifest and payload, see the [Notary V2 Signature Specification](https://github.com/notaryproject/notaryproject/blob/v1.0.0-rc.2/specs/signature-specification.md#backward-compatibility).

SBOMs are to be applied in a similar manner as signatures.

#### Image Manifests for Additional Metadata
There will likely be a need to provide additional metadata for a component. For example, debugging symbols, documentation, WIT file representing the textual description of the component interface. This proposal will not exhaustively address each of these additional pieces of metadata. However, if additional metadata is to be applied, the additional metadata MUST be specified using an image manifest which subject descriptor points to the component image manifest and MUST have an `artifactType` specified.

#### Warg Registry Implementation
TODO

## Alternative Options
### Publish Artifacts Using an Artifact Manifest
An alternative option to using image manifests to describe components is to use an artifact manifest. An artifact manifest would have `mediaType` not equal to "application/vnd.oci.image.manifest.v1+json".
#### Pros
  - Specifying a custom `mediaType` could provide more opportunity to creatively describe components.
#### Cons
  - It seems likely, at the time of authoring, that artifact manifests will not be part of the OCIv1.1 specification: https://github.com/opencontainers/image-spec/pull/999.
  - Using artifact manifests would likely lead to less portability across registry implementations.

## Conclusions
TODO

## Additional Details
### Test Plan
TODO

## Implementation History
- 2023-04-04: Initial draft 


