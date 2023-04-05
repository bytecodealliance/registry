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
export interface KeyEntry {
  keyId: string,
  publicKey: string,
}
export interface Validator {
  algorithm?: HashAlgorithm,
  head?: Head,
  permissions: PermEntry[],
  keys?: KeyEntry[],
}
export interface PackageInfo {
  name: string,
  checkpoint?: string,
  state: Validator,
}
export namespace Protocol {
  export function validate(packageRecord: ProtoEnvelopeBody): PackageInfo;
}
