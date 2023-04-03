/**
 * # Variants
 * 
 * ## `"sha256"`
 */
export type HashAlgorithm = 'sha256';
export interface DynHash {
  algo: HashAlgorithm,
  bytes: Uint8Array,
}
export interface MapCheckpoint {
  logRoot: DynHash,
  logLength: number,
  mapRoot: DynHash,
}
export namespace Protocol {
  export function validate(privateKey: string, contents: MapCheckpoint): void;
}
