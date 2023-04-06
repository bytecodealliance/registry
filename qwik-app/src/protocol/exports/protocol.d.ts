export interface ProtoEnvelopeBody {
  contentBytes: Uint8Array,
  keyId: string,
  signature: string,
}
/**
 * # Variants
 * 
 * ## `"sha256"`
 */
export type HashAlgorithm = 'sha256';
export interface DynHash {
  algo: HashAlgorithm,
  bytes?: Uint8Array,
}
export type RecordId = RecordIdDynHash;
export interface RecordIdDynHash {
  tag: 'dyn-hash',
  val: DynHash,
}
export interface Head {
  digest: RecordId,
  timestamp?: string,
}
/**
 * # Variants
 * 
 * ## `"release"`
 * 
 * ## `"yank"`
 */
export type Perm = 'release' | 'yank';
export interface PermEntry {
  keyId: string,
  perms: Perm[],
}
export interface Released {
  content: DynHash,
}
export interface Yanked {
  by: string,
  timestamp: string,
}
export type ReleaseState = ReleaseStateReleased | ReleaseStateYanked;
export interface ReleaseStateReleased {
  tag: 'released',
  val: Released,
}
export interface ReleaseStateYanked {
  tag: 'yanked',
  val: Yanked,
}
export interface Release {
  version: string,
  by: string,
  timestamp: string,
  state: ReleaseState,
}
export interface KeyEntry {
  keyId: string,
  publicKey: string,
}
export interface Validator {
  algorithm?: HashAlgorithm,
  head?: Head,
  permissions: PermEntry[],
  releases: Release[],
  keys?: KeyEntry[],
}
export interface PackageInfo {
  name: string,
  checkpoint?: string,
  state: Validator,
}
export namespace Protocol {
  export function validate(packageRecords: ProtoEnvelopeBody[]): PackageInfo;
}
