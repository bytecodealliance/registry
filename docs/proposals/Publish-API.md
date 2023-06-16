# Publish API

> This API focuses on "component" publishing, but should be generalizable to other "package types" (e.g. interfaces).

## Create Unpublished Release
`POST /releases` (or `/components/bytecodealliance.org:dog-facts`)

This is the "release manifest":
```jsonc
{
   "packageType": "component",
   "name": "bytecodealliance.org:dog-facts",
   "version": "1.0.0",
   "contentDigest": {"sha256": "f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"}
   // ...more fields populated by publisher
}
```
`201 Created` -> `/components/bytecodealliance.org:dog-facts/v1.0.0/unpublished`

### Content Digest

* The `contentDigest` field value is the lower-hex-encoded "content digest"
* The "content digest" is the SHA-256 digest of the "content index"
* The "content index" consists of one or more "content entries" concatenated together
 * Within the index, entries are ordered (byte-wise, lexicographically) by path
* Each "content entry" consists of the SHA-256 digest of the content and a path
 * Paths must conform to [these rules](https://fuchsia.dev/fuchsia-src/development/source_code/archive_format#path_data)

```
<content-digest> := SHA-256(<content-index>)
<content-index>  := <content-entry>+
<content-entry>  := <entry-digest> <entry-path-vec>
<entry-digest>   := SHA-256([entry content])
<entry-path-vec> := LENGTH(<entry-path>) <entry-path>
<entry-path>     := [see link above]


SHA-256: binary output of the FIPS 180-4 SHA256 algorithm
LENGTH: size of argument in bytes as a 16-bit, unsigned, little-endian int
```
> The exact contents for particular package types are not fully defined in this proposal, but for components in particular the component Wasm binary entry must be named exactly `component.wasm`.

## Unpublished Release 
`GET /components/bytecodealliance.org:dog-facts/v1.0.0/unpublished`
```jsonc
{
   "release": { <release manifest (above)> },
   // Server-authenticated identity
   "creator": "github:bytecodealliance-ci",
   // Enum of e.g. "pending", "processing", "expired"
   "status": "pending",
   // Unpublished releases are transient
   "expiresAt": "2022-04-27T12:34:56Z",
   // Each source points to identical content
   "contentSources": [],
   // Optional: registry-managed storage to upload component content
   "uploadUrl": "https://storage.example.com/abc123",
}
```

## Add release content source(s)

* Direct upload to `uploadUrl` (`application/wasm`):
   `POST https://objects.example.com/abc123`
* Indirect, by URL:
   `POST /components/bytecodealliance.org:dog-facts/v1.0.0/content-sources`
   ```jsonc
   {"contentSources": [
       {"url": "https://github.com/bytecodealliance/dog-facts/releases/download/v1.0.0/dog-facts-1.0.0.wasm"},
       {"url": "ipfs://x9jrSiRbxkH82quqonfkiLn3/dog-facts-1.0.0.wasm"}
   ]}
   ```
   * Allows more complex schemes like signed upload URLs
   * Different registries may accept different content source types (transports, hosts)

> This list can be modified after publishing.

## Publish Release
`POST /components/bytecodealliance.org:dog-facts/v1.0.0`
```jsonc
{
   // Redundant (optional?), but allows for better error handling ("bad digest" vs "bad signature")
   // e.g. SHA-256("WASM-COMPONENT-REGISTRY-RELEASE-V1" || <raw release manifest>)
   "releaseDigest": "abc123...",
   // e.g. Ed25519Sign(<publisher key>, <raw releaseDigest>)
   "releaseSignature": "..."
}
```

> Registry will need to fetch / validate / extract metadata from component, so actual publication may be asynchronous (e.g. Unpublished Release `"status": "processing"` above)

> This proposal suggests signing the exact release manifest content POSTed by the publisher, but a different canonical encoding could be used instead.

## Get Release
`GET /components/bytecodealliance.org:dog-facts/v1.0.0`
```jsonc
{
   "release": { <release manifest> },
   "releaseSignature": "...",
   "creator": "github:bytecodealliance-ci",
   "contentSources": [{"url": "https://storage.example.com/abc123"}],
   // ...more fields populated by registry
}
```

